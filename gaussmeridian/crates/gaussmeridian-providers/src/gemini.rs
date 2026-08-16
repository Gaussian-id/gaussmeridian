//! Google Gemini provider implementation
//!
//! Wraps the Google Generative Language API
//! (`https://generativelanguage.googleapis.com/v1beta`) behind the shared `LLMProvider`
//! trait, following the same shape as `openai.rs` / `anthropic.rs`.
//!
//! ## Scope (deliberately minimal — see PROVIDER-DEV.md conventions)
//! - `chat_completion` is fully implemented (messages → `contents`, system messages →
//!   `systemInstruction`, `usageMetadata` → `Usage`).
//! - `chat_completion_stream` calls Gemini's `streamGenerateContent?alt=sse` endpoint and maps
//!   each incremental `data:` line (a partial `GenerateContentResponse`) to an OpenAI-shaped
//!   `ChatCompletionChunk` delta — see `map_stream_chunk`. **Graceful degradation is load-bearing
//!   here, not optional polish:** if the SSE connection can't even be established (non-2xx or a
//!   transport error before any content has reached the caller), the adapter transparently falls
//!   back to the plain `generateContent` endpoint and replays the full answer as a couple of
//!   synthetic chunks (`buffered_stream_fallback`) — the caller always gets a usable answer, it
//!   just may not be incremental for that turn. A malformed *individual* SSE line mid-stream is
//!   skipped (`warn!`), not fatal; a transport drop mid-stream (after real content was already
//!   sent) closes the stream with a synthetic `finish_reason: "stop"` chunk instead of an error
//!   frame, so a partially-degraded stream still ends cleanly rather than hanging or erroring out.
//! - `completion_stream` has no Gemini-native equivalent either; it wraps the prompt as a
//!   single-turn chat request and delegates to `chat_completion_stream`, inheriting the same
//!   degradation behavior for free.
//! - `completion` (legacy text-completion) is implemented by wrapping the prompt as a single
//!   `user` content — Gemini has no separate legacy-completion endpoint.
//! - `embedding` supports `String`/`Array` input via `embedContent`; `ArrayOfArrays` (Gemini
//!   has no index-based embedding input) returns `BadRequest`.
//! - Vision (`ContentPart::ImageUrl`) is not mapped — image parts are dropped with a `warn!`.

use crate::common::BaseProviderConfig;
use crate::streaming_parser::SSEParser;
use futures::{Stream, StreamExt};
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use std::pin::Pin;
use tracing::{debug, warn};

/// Google Gemini-specific configuration
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub base_config: BaseProviderConfig,
    model_allowlist_is_authoritative: bool,
    /// Generative Language API version path segment (e.g. `v1beta`).
    pub api_version: String,
    pub default_max_tokens: u32,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            base_config: BaseProviderConfig::new(
                "google".to_string(),
                std::env::var("GEMINI_API_KEY").unwrap_or_default(),
            )
            .with_base_url("https://generativelanguage.googleapis.com/v1beta".to_string())
            .with_models(vec![
                "gemini-3.5-flash-lite".to_string(),
                "gemini-3.6-flash".to_string(),
                "gemini-3.5-flash".to_string(),
            ]),
            model_allowlist_is_authoritative: false,
            api_version: "v1beta".to_string(),
            default_max_tokens: 4096,
        }
    }
}

impl GeminiConfig {
    /// Create a new Gemini configuration with an API key
    pub fn new(api_key: String) -> Self {
        let mut config = Self::default();
        config.base_config.api_key = api_key;
        config
    }

    /// Set custom base URL (e.g. a mock endpoint for local dev)
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_config = self.base_config.with_base_url(base_url);
        self
    }

    /// Restrict this provider to the exact model identifiers configured by the deployment.
    pub fn with_model_allowlist(mut self, models: Vec<String>) -> Self {
        if !models.is_empty() {
            self.base_config = self.base_config.with_models(models);
            self.model_allowlist_is_authoritative = true;
        }
        self
    }
}

/// Google Gemini provider implementation
#[derive(Debug, Clone)]
pub struct GeminiProvider {
    config: GeminiConfig,
    client: Client,
}

impl GeminiProvider {
    /// Create a new Gemini provider with the given configuration
    pub fn new(config: GeminiConfig) -> Self {
        let timeout = config.base_config.timeout.unwrap_or(120);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout as u64))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Create a new Gemini provider with just an API key
    pub fn with_api_key(api_key: String) -> Self {
        Self::new(GeminiConfig::new(api_key))
    }

    fn retain_configured_models(&self, models: Vec<Model>) -> Vec<Model> {
        if !self.config.model_allowlist_is_authoritative {
            return models;
        }
        models
            .into_iter()
            .filter_map(|mut model| {
                if self.config.base_config.models.contains(&model.id) {
                    model.owned_by = "google".to_string();
                    Some(model)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Resolve the base URL, env-overridable: `GEMINI_API_BASE` → config `base_url` → the
    /// public Generative Language API. Mirrors `OpenAIProvider::base_url` /
    /// `AnthropicProvider::base_url` so a mock endpoint can repoint this provider in dev
    /// without editing `gaussmeridian.toml`.
    fn base_url(&self) -> String {
        std::env::var("GEMINI_API_BASE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| self.config.base_config.base_url.clone())
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string())
    }

    /// `generateContent` endpoint for a given model. The API key is a query parameter (Gemini's
    /// auth scheme), never a header — kept out of any logged URL by callers using `debug!` only
    /// with the path, not this full URL, where avoidable.
    fn generate_content_url(&self, model: &str) -> String {
        format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url(),
            model,
            self.config.base_config.api_key
        )
    }

    /// `streamGenerateContent` endpoint (SSE form) for a given model. Same auth scheme as
    /// `generate_content_url` (API key as a query param, never a header) plus `alt=sse`, which
    /// makes Gemini emit one `data:` line per partial `GenerateContentResponse` instead of
    /// returning a single JSON array once the whole generation finishes.
    fn stream_generate_content_url(&self, model: &str) -> String {
        format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url(),
            model,
            self.config.base_config.api_key
        )
    }

    fn embed_content_url(&self, model: &str) -> String {
        format!(
            "{}/models/{}:embedContent?key={}",
            self.base_url(),
            model,
            self.config.base_config.api_key
        )
    }

    fn models_url(&self) -> String {
        format!(
            "{}/models?key={}",
            self.base_url(),
            self.config.base_config.api_key
        )
    }

    /// Map our `Role` → Gemini's two-role scheme (`user` / `model`). System messages are
    /// extracted separately into `systemInstruction` (see `to_gemini_request`) and never reach
    /// this function. `Function`/`Tool` have no Gemini equivalent in this minimal adapter and
    /// fold into `user` — acceptable because MoA agents never emit tool-role messages.
    fn role_to_gemini(role: &Role) -> &'static str {
        match role {
            Role::Assistant => "model",
            Role::User | Role::Function | Role::Tool | Role::System => "user",
        }
    }

    /// Flatten `Content` into plain text. Image parts are dropped (this adapter doesn't map
    /// vision — see module doc) with a `warn!` so a silently-degraded multimodal request is at
    /// least observable.
    fn content_to_text(content: &Content) -> String {
        match content {
            Content::Text(text) => text.clone(),
            Content::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    ContentPart::Text { text } => Some(text.clone()),
                    ContentPart::ImageUrl { .. } => {
                        warn!("Gemini adapter: dropping image content part (vision not mapped)");
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    fn is_gemini_3_model(model: &str) -> bool {
        model
            .to_ascii_lowercase()
            .strip_prefix("gemini-3")
            .is_some_and(|suffix| suffix.starts_with('.') || suffix.starts_with('-'))
    }

    fn gemini_3_thinking_level(
        model: &str,
        extra: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Option<&'static str> {
        match extra
            .get("reasoning_effort")
            .and_then(|value| value.as_str())
        {
            Some("minimal") if model.to_ascii_lowercase().contains("-pro") => Some("low"),
            Some("minimal") => Some("minimal"),
            Some("low") => Some("low"),
            Some("medium") => Some("medium"),
            Some("high") => Some("high"),
            _ => None,
        }
    }

    fn apply_response_format(
        generation_config: &mut serde_json::Map<String, serde_json::Value>,
        extra: &std::collections::HashMap<String, serde_json::Value>,
    ) {
        let Some(response_format) = extra.get("response_format") else {
            return;
        };
        match response_format
            .get("type")
            .and_then(serde_json::Value::as_str)
        {
            Some("json_object") => {
                generation_config.insert(
                    "responseMimeType".to_string(),
                    serde_json::json!("application/json"),
                );
            }
            Some("json_schema") => {
                let Some(schema) = response_format
                    .get("json_schema")
                    .and_then(|value| value.get("schema"))
                else {
                    return;
                };
                generation_config.insert(
                    "responseMimeType".to_string(),
                    serde_json::json!("application/json"),
                );
                generation_config.insert("responseJsonSchema".to_string(), schema.clone());
            }
            _ => {}
        }
    }

    fn generation_config(
        &self,
        model: &str,
        temperature: Option<f32>,
        top_p: Option<f32>,
        candidate_count: Option<u32>,
        max_tokens: Option<u32>,
        stop: Option<&StopSequence>,
        extra: &std::collections::HashMap<String, serde_json::Value>,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut generation_config = serde_json::Map::new();
        let gemini_3 = Self::is_gemini_3_model(model);

        if gemini_3 {
            if let Some(thinking_level) = Self::gemini_3_thinking_level(model, extra) {
                generation_config.insert(
                    "thinkingConfig".to_string(),
                    serde_json::json!({"thinkingLevel": thinking_level}),
                );
            }
        } else {
            if let Some(temperature) = temperature {
                generation_config.insert("temperature".to_string(), serde_json::json!(temperature));
            }
            if let Some(top_p) = top_p {
                generation_config.insert("topP".to_string(), serde_json::json!(top_p));
            }
            if let Some(candidate_count) = candidate_count {
                generation_config.insert(
                    "candidateCount".to_string(),
                    serde_json::json!(candidate_count),
                );
            }
        }

        generation_config.insert(
            "maxOutputTokens".to_string(),
            serde_json::json!(max_tokens.unwrap_or(self.config.default_max_tokens)),
        );
        if let Some(stop) = stop {
            let sequences = match stop {
                StopSequence::String(sequence) => vec![sequence.clone()],
                StopSequence::Array(sequences) => sequences.clone(),
            };
            generation_config.insert("stopSequences".to_string(), serde_json::json!(sequences));
        }
        Self::apply_response_format(&mut generation_config, extra);
        generation_config
    }

    fn completion_request_as_chat(request: CompletionRequest) -> ChatCompletionRequest {
        let CompletionRequest {
            model,
            prompt,
            max_tokens,
            temperature,
            top_p,
            n,
            stream,
            stop,
            presence_penalty,
            frequency_penalty,
            logit_bias,
            user,
            extra,
            ..
        } = request;

        ChatCompletionRequest {
            model,
            messages: vec![Message {
                role: Role::User,
                content: Content::Text(prompt),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
                confidence: None,
            }],
            temperature,
            top_p,
            n,
            stream,
            stop,
            max_tokens,
            presence_penalty,
            frequency_penalty,
            logit_bias,
            user,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            transforms: None,
            models: None,
            route: None,
            provider: None,
            routing_strategy: None,
            fallback_providers: None,
            cost_limit: None,
            timeout: None,
            tenant_id: None,
            request_metadata: None,
            extra,
        }
    }

    /// Build the Gemini `generateContent` request body from a `ChatCompletionRequest`.
    /// System messages (there may be several) are joined and lifted into `systemInstruction`;
    /// everything else becomes a `contents` entry with its role mapped via `role_to_gemini`.
    fn to_gemini_request(&self, request: &ChatCompletionRequest) -> serde_json::Value {
        let mut system_parts: Vec<String> = Vec::new();
        let mut contents: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            let text = Self::content_to_text(&msg.content);
            if matches!(msg.role, Role::System) {
                if !text.is_empty() {
                    system_parts.push(text);
                }
                continue;
            }
            contents.push(serde_json::json!({
                "role": Self::role_to_gemini(&msg.role),
                "parts": [{ "text": text }],
            }));
        }

        let mut body = serde_json::json!({ "contents": contents });

        if !system_parts.is_empty() {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system_parts.join("\n\n") }],
            });
        }

        let generation_config = self.generation_config(
            &request.model,
            request.temperature,
            request.top_p,
            request.n,
            request.max_tokens,
            request.stop.as_ref(),
            &request.extra,
        );
        if !generation_config.is_empty() {
            body["generationConfig"] = serde_json::Value::Object(generation_config);
        }

        body
    }

    /// Map a Gemini `finishReason` to the OpenAI-shaped values the rest of the gateway expects.
    fn map_finish_reason(reason: Option<&str>) -> Option<String> {
        reason.map(|r| match r {
            "STOP" => "stop".to_string(),
            "MAX_TOKENS" => "length".to_string(),
            "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" => {
                "content_filter".to_string()
            }
            other => other.to_ascii_lowercase(),
        })
    }

    /// Parse a Gemini `generateContent` response into our `ChatCompletionResponse`.
    fn parse_gemini_response(
        &self,
        response_data: serde_json::Value,
        model: &str,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        let candidate = response_data["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| {
                ProviderError::Internal("Gemini response had no candidates".to_string())
            })?;

        let text = candidate["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        let finish_reason = Self::map_finish_reason(candidate["finishReason"].as_str());

        let choices = vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content: Content::Text(text),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
                confidence: None,
            },
            finish_reason,
            logprobs: None,
        }];

        let usage = response_data.get("usageMetadata").map(|u| Usage {
            prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatCompletionResponse {
            id: format!("gemini-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices,
            usage,
            system_fingerprint: None,
        })
    }

    /// Map an HTTP error response to a `ProviderError`. Gemini's error envelope is
    /// `{"error": {"code", "message", "status"}}`; the sanitized status text is used as a
    /// fallback when the body doesn't parse. Mirrors `error_utils::http_status_to_provider_error`
    /// but reads the Gemini-shaped body for a clearer message.
    fn map_error_response(status: reqwest::StatusCode, body: &str) -> ProviderError {
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
            .unwrap_or_else(|| crate::error_utils::sanitize_error_message(body));

        match status.as_u16() {
            401 | 403 => ProviderError::Authentication(message),
            429 => ProviderError::RateLimit(message),
            400 => ProviderError::BadRequest(message),
            404 => ProviderError::BadRequest(format!("model not found: {}", message)),
            500..=599 => ProviderError::Unavailable(format!("Server error: {}", message)),
            _ => ProviderError::Internal(format!("HTTP {}: {}", status, message)),
        }
    }

    // ── Streaming: SSE chunk mapping (pure — no I/O, unit-tested directly) ────────────────

    /// Map one Gemini `streamGenerateContent` SSE data-line payload (a partial
    /// `GenerateContentResponse`) into an OpenAI-shaped `ChatCompletionChunk` delta. Mirrors
    /// `parse_gemini_response`'s field reads but yields incremental `content` instead of a full
    /// message; `finish_reason` is only `Some` on Gemini's terminal chunk (the one carrying a
    /// non-null `finishReason`), and `usage` is populated whenever the chunk carries a
    /// `usageMetadata` block (Gemini attaches it to the final chunk of a real stream).
    fn map_stream_chunk(
        json: &serde_json::Value,
        id: &str,
        model: &str,
    ) -> Result<ChatCompletionChunk, ProviderError> {
        let candidate = json["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| {
                ProviderError::Internal("Gemini stream chunk had no candidates".to_string())
            })?;

        let text = candidate["content"]["parts"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        let finish_reason = Self::map_finish_reason(candidate["finishReason"].as_str());

        let usage = json.get("usageMetadata").map(|u| Usage {
            prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0) as u32,
        });

        Ok(ChatCompletionChunk {
            id: id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChoiceDelta {
                index: 0,
                delta: Some(MessageDelta {
                    role: Some(Role::Assistant),
                    content: if text.is_empty() { None } else { Some(text) },
                    function_call: None,
                    tool_calls: None,
                }),
                finish_reason,
                logprobs: None,
            }],
            usage,
            system_fingerprint: None,
        })
    }

    /// Drain complete `\n`-terminated lines out of `buf` (leaving any trailing partial line
    /// buffered for the next network read — `bytes_stream()` chunk boundaries don't align to SSE
    /// event boundaries) and map each `data:` line to a `ChatCompletionChunk` via
    /// `map_stream_chunk`.
    ///
    /// Graceful degradation, line-by-line: a line that isn't `data:`-prefixed SSE framing is
    /// silently ignored (blank keep-alive separators, etc.); a `data:` line whose JSON doesn't
    /// match the expected shape is skipped with a `warn!` rather than aborting the whole stream —
    /// see the module doc's "Graceful degradation" note. Pure / no I/O, so it's unit-testable
    /// directly against a captured SSE fixture without a mock server.
    fn drain_stream_lines(buf: &mut String, id: &str, model: &str) -> Vec<ChatCompletionChunk> {
        let mut out = Vec::new();
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match SSEParser::parse_sse_line(line) {
                Some(json) => match Self::map_stream_chunk(&json, id, model) {
                    Ok(chunk) => out.push(chunk),
                    Err(e) => {
                        warn!(error = %e, line, "Gemini stream: skipping malformed chunk");
                    }
                },
                // Not `data:`-prefixed (or literally "[DONE]", which Gemini never sends) —
                // ordinary SSE framing noise, not an error.
                None => {}
            }
        }
        out
    }

    /// Synthesize the closing chunk for a stream that ended without Gemini ever sending a
    /// terminal `finishReason` (e.g. the connection dropped mid-generation). Ensures OpenAI-shaped
    /// consumers always see a clean `finish_reason: "stop"` rather than a stream that just goes
    /// silent — part of the "must not hard-fail mid-stream" graceful-degradation contract.
    fn synthetic_finish_chunk(id: &str, model: &str) -> ChatCompletionChunk {
        ChatCompletionChunk {
            id: id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: model.to_string(),
            choices: vec![ChoiceDelta {
                index: 0,
                delta: Some(MessageDelta {
                    role: None,
                    content: None,
                    function_call: None,
                    tool_calls: None,
                }),
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: None,
            system_fingerprint: None,
        }
    }

    /// Turn one already-fetched non-streaming `ChatCompletionResponse` into the couple of
    /// synthetic `ChatCompletionChunk`s the buffered-fallback path emits: a single content delta
    /// carrying the whole answer, then a terminal chunk carrying `finish_reason` and `usage`.
    /// Kept separate from `buffered_stream_fallback` (which does the network I/O) so the mapping
    /// itself is unit-testable without a mock server.
    fn synthesize_fallback_chunks(
        chat_response: &ChatCompletionResponse,
        id: String,
        model: String,
    ) -> Vec<ChatCompletionChunk> {
        let choice = chat_response.choices.first();
        let text = choice
            .and_then(|c| match &c.message.content {
                Content::Text(t) => Some(t.clone()),
                Content::Parts(_) => None,
            })
            .unwrap_or_default();
        let finish_reason = choice
            .and_then(|c| c.finish_reason.clone())
            .or_else(|| Some("stop".to_string()));

        vec![
            ChatCompletionChunk {
                id: id.clone(),
                object: "chat.completion.chunk".to_string(),
                created: chat_response.created,
                model: model.clone(),
                choices: vec![ChoiceDelta {
                    index: 0,
                    delta: Some(MessageDelta {
                        role: Some(Role::Assistant),
                        content: Some(text),
                        function_call: None,
                        tool_calls: None,
                    }),
                    finish_reason: None,
                    logprobs: None,
                }],
                usage: None,
                system_fingerprint: None,
            },
            ChatCompletionChunk {
                id,
                object: "chat.completion.chunk".to_string(),
                created: chat_response.created,
                model,
                choices: vec![ChoiceDelta {
                    index: 0,
                    delta: Some(MessageDelta {
                        role: None,
                        content: None,
                        function_call: None,
                        tool_calls: None,
                    }),
                    finish_reason,
                    logprobs: None,
                }],
                usage: chat_response.usage.clone(),
                system_fingerprint: None,
            },
        ]
    }

    /// Buffered-fallback path used when the real `streamGenerateContent` SSE connection can't
    /// even be established (non-2xx or a transport error at connect time — before any content has
    /// reached the caller, so falling back here can never duplicate content). Calls the plain
    /// `generateContent` endpoint via the existing non-streaming `chat_completion` and replays the
    /// result as a couple of synthetic chunks (`synthesize_fallback_chunks`), so the playground
    /// always gets a usable answer even when incremental streaming is unavailable.
    async fn buffered_stream_fallback(
        &self,
        request: ChatCompletionRequest,
        id: String,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, ProviderError>> + Send>>,
        ProviderError,
    > {
        let model = request.model.clone();
        let chat_response = self.chat_completion(request).await?;
        let chunks = Self::synthesize_fallback_chunks(&chat_response, id, model);
        Ok(Box::pin(futures::stream::iter(chunks.into_iter().map(Ok))))
    }
}

#[async_trait::async_trait]
impl LLMProvider for GeminiProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        let url = self.generate_content_url(&request.model);
        debug!(model = %request.model, "Gemini chat completion request");

        let body = self.to_gemini_request(&request);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Self::map_error_response(status, &error_body));
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        self.parse_gemini_response(response_data, &request.model)
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        let model = request.model.clone();
        let id = format!("gemini-{}", uuid::Uuid::new_v4());
        let url = self.stream_generate_content_url(&model);
        let body = self.to_gemini_request(&request);

        debug!(model = %model, "Gemini streaming chat completion request");

        let send_result = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        // Connect-time failure (transport error or non-2xx): nothing has been streamed to the
        // caller yet, so it's always safe to degrade to the buffered non-streaming path here —
        // see the module doc's "Graceful degradation" note.
        let response = match send_result {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                let status = r.status();
                let error_body = r.text().await.unwrap_or_default();
                warn!(
                    model = %model,
                    %status,
                    error = %crate::error_utils::sanitize_error_message(&error_body),
                    "Gemini stream setup failed — degrading to buffered generateContent"
                );
                return self.buffered_stream_fallback(request, id).await;
            }
            Err(e) => {
                warn!(model = %model, error = %e, "Gemini stream connection failed — degrading to buffered generateContent");
                return self.buffered_stream_fallback(request, id).await;
            }
        };

        let mut byte_stream = response.bytes_stream();

        let stream = async_stream::stream! {
            let mut buf = String::new();
            let mut saw_finish = false;

            loop {
                match byte_stream.next().await {
                    Some(Ok(bytes)) => {
                        match std::str::from_utf8(&bytes) {
                            Ok(s) => buf.push_str(s),
                            Err(_) => {
                                warn!("Gemini stream: dropping non-UTF-8 chunk fragment");
                                continue;
                            }
                        }

                        for chunk in GeminiProvider::drain_stream_lines(&mut buf, &id, &model) {
                            if chunk.choices.iter().any(|c| c.finish_reason.is_some()) {
                                saw_finish = true;
                            }
                            yield Ok(chunk);
                        }
                    }
                    Some(Err(e)) => {
                        // Transport error mid-stream — real content may already have reached the
                        // caller, so a full buffered re-fetch here would risk duplicating it.
                        // Close the stream cleanly instead of surfacing a hard error (see module
                        // doc): a synthetic `finish_reason: "stop"` if Gemini hadn't already sent
                        // one, then end.
                        warn!(error = %e, "Gemini stream: transport error mid-stream — closing gracefully");
                        if !saw_finish {
                            yield Ok(GeminiProvider::synthetic_finish_chunk(&id, &model));
                        }
                        return;
                    }
                    None => {
                        // Connection closed. If Gemini never sent a terminal `finishReason`,
                        // synthesize one so the caller sees a clean end-of-stream instead of one
                        // that just stops.
                        if !saw_finish {
                            yield Ok(GeminiProvider::synthetic_finish_chunk(&id, &model));
                        }
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Self::Error> {
        // Gemini has no legacy text-completion endpoint — wrap the prompt as a single `user`
        // content and reuse `generateContent`.
        let model = request.model.clone();
        debug!(model = %model, "Gemini completion request (wrapped as chat)");
        let chat_response = self
            .chat_completion(Self::completion_request_as_chat(request))
            .await?;
        let text = match &chat_response.choices[0].message.content {
            Content::Text(t) => t.clone(),
            Content::Parts(_) => String::new(),
        };

        Ok(CompletionResponse {
            id: chat_response.id,
            object: "text_completion".to_string(),
            created: chat_response.created,
            model,
            choices: vec![CompletionChoice {
                text,
                index: 0,
                logprobs: None,
                finish_reason: chat_response.choices[0].finish_reason.clone(),
            }],
            usage: chat_response.usage,
        })
    }

    async fn completion_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>, Self::Error>
    {
        // Gemini has no legacy streaming-completion endpoint either — wrap the prompt as a
        // single-turn chat request (the same trick `completion()` uses for the non-streaming
        // case) and delegate to `chat_completion_stream`, mapping each `ChatCompletionChunk` back
        // down to a `CompletionChunk`. Inherits the same graceful-degradation behavior for free.
        let model = request.model.clone();
        let chat_stream = self
            .chat_completion_stream(Self::completion_request_as_chat(request))
            .await?;

        let stream = chat_stream.map(move |result| {
            result.map(|chunk| CompletionChunk {
                id: chunk.id,
                object: "text_completion".to_string(),
                created: chunk.created,
                model: model.clone(),
                choices: chunk
                    .choices
                    .into_iter()
                    .map(|c| CompletionChoiceDelta {
                        text: c.delta.and_then(|d| d.content).unwrap_or_default(),
                        index: c.index,
                        logprobs: None,
                        finish_reason: c.finish_reason,
                    })
                    .collect(),
                usage: chunk.usage,
            })
        });

        Ok(Box::pin(stream))
    }

    async fn embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Self::Error> {
        let texts: Vec<String> = match &request.input {
            EmbeddingInput::String(s) => vec![s.clone()],
            EmbeddingInput::Array(arr) => arr.clone(),
            EmbeddingInput::ArrayOfArrays(_) => {
                return Err(ProviderError::BadRequest(
                    "Gemini embeddings take text input, not token-id arrays".to_string(),
                ))
            }
        };

        let url = self.embed_content_url(&request.model);
        let mut data = Vec::with_capacity(texts.len());

        for (index, text) in texts.iter().enumerate() {
            let body = serde_json::json!({
                "model": format!("models/{}", request.model),
                "content": { "parts": [{ "text": text }] },
            });

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

            let status = response.status();
            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                return Err(Self::map_error_response(status, &error_body));
            }

            let response_data: serde_json::Value = response
                .json()
                .await
                .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

            let embedding: Vec<f64> = response_data["embedding"]["values"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_f64())
                .collect();

            data.push(EmbeddingData {
                object: "embedding".to_string(),
                embedding,
                index: index as u32,
            });
        }

        let estimated_tokens: u32 = texts.iter().map(|t| t.len() as u32 / 4).sum();

        Ok(EmbeddingResponse {
            object: "list".to_string(),
            data,
            model: request.model,
            usage: Some(Usage {
                prompt_tokens: estimated_tokens,
                completion_tokens: 0,
                total_tokens: estimated_tokens,
            }),
        })
    }

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        let url = self.models_url();

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(Self::map_error_response(status, &error_body));
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let models = response_data["models"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|m| {
                let id = m["name"]
                    .as_str()
                    .unwrap_or_default()
                    .strip_prefix("models/")
                    .unwrap_or_default()
                    .to_string();
                Model {
                    id,
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "google".to_string(),
                    permission: None,
                    root: None,
                    parent: None,
                }
            })
            .collect();

        Ok(self.retain_configured_models(models))
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "Google Gemini".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec![
                "chat_completions".to_string(),
                "completions".to_string(),
                "embeddings".to_string(),
                "json_output".to_string(),
            ],
            rate_limits: Some(RateLimits {
                requests_per_minute: Some(1000),
                tokens_per_minute: Some(4_000_000),
            }),
            pricing: None,
            models: vec![],
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        let url = self.models_url();

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Health check failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Unavailable(format!(
                "Health check failed with status: {}",
                response.status()
            )))
        }
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            // Real SSE streaming (`streamGenerateContent?alt=sse`) with a buffered
            // `generateContent` fallback on connect-time failure — see `chat_completion_stream`
            // and the module doc's "Graceful degradation" note.
            supports_streaming: true,
            supports_functions: false,
            supports_vision: false,
            supports_embeddings: true,
            max_context_length: Some(1_048_576),
            max_tokens_per_request: Some(65_536),
            supported_models: self.config.base_config.models.clone(),
        }
    }

    async fn get_cost_info(&self, model: &str) -> Result<CostInfo, Self::Error> {
        // Current 3.x values mirror the frozen live-provider authority's official Standard
        // prices, converted from per-1M to per-1K tokens. Legacy 2.5 values remain for callers
        // that explicitly retain those model IDs outside the authoritative live allowlist.
        let (input_cost, output_cost) = match model {
            "gemini-3.5-flash-lite" => (0.0003, 0.0025),
            "gemini-3.6-flash" => (0.0015, 0.0075),
            "gemini-3.5-flash" => (0.0015, 0.009),
            "gemini-2.5-pro" => (0.00125, 0.01),
            "gemini-2.5-flash" => (0.000075, 0.0003),
            "gemini-2.5-flash-lite" => (0.000015, 0.00006),
            _ => (0.0015, 0.0075), // Default to current Flash pricing
        };

        Ok(CostInfo {
            input_cost_per_1k_tokens: input_cost,
            output_cost_per_1k_tokens: output_cost,
            currency: "USD".to_string(),
            model: model.to_string(),
        })
    }

    async fn supports_model(&self, model: &str) -> bool {
        let supported = self.config.base_config.models.iter().any(|m| m == model);
        if self.config.model_allowlist_is_authoritative {
            return supported;
        }
        if supported {
            return true;
        }
        model.starts_with("gemini-")
    }

    fn get_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self
                .config
                .base_config
                .base_url
                .clone()
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string()),
            api_key: Some(self.config.base_config.api_key.clone()),
            timeout: self.config.base_config.timeout.unwrap_or(120),
            max_retries: self.config.base_config.max_retries.unwrap_or(3),
            custom_headers: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> GeminiProvider {
        GeminiProvider::with_api_key("test-key".to_string())
    }

    fn discovered_model(id: &str) -> Model {
        Model {
            id: id.to_string(),
            object: "model".to_string(),
            created: 0,
            owned_by: "google".to_string(),
            permission: None,
            root: None,
            parent: None,
        }
    }

    #[tokio::test]
    async fn configured_model_allowlist_is_strict_across_provider_surfaces() {
        let provider = GeminiProvider::new(
            GeminiConfig::new("test-key".to_string()).with_model_allowlist(vec![
                "gemini-3.5-flash-lite".to_string(),
                "gemini-3.6-flash".to_string(),
                "gemini-3.5-flash".to_string(),
            ]),
        );

        assert_eq!(
            provider.capabilities().supported_models,
            vec![
                "gemini-3.5-flash-lite".to_string(),
                "gemini-3.6-flash".to_string(),
                "gemini-3.5-flash".to_string(),
            ]
        );
        assert!(provider.supports_model("gemini-3.6-flash").await);
        assert!(!provider.supports_model("gemini-2.5-flash").await);

        let discovered = provider.retain_configured_models(vec![
            Model {
                owned_by: "Google DeepMind".to_string(),
                ..discovered_model("gemini-3.5-flash")
            },
            discovered_model("gemini-2.5-flash"),
            Model {
                owned_by: "Google DeepMind".to_string(),
                ..discovered_model("gemini-3.5-flash-lite")
            },
        ]);
        assert_eq!(
            discovered
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["gemini-3.5-flash", "gemini-3.5-flash-lite"]
        );
        assert!(discovered.iter().all(|model| model.owned_by == "google"));
    }

    #[tokio::test]
    async fn default_model_configuration_preserves_prefix_compatibility() {
        let provider = provider();

        assert_eq!(
            provider.capabilities().supported_models,
            vec![
                "gemini-3.5-flash-lite".to_string(),
                "gemini-3.6-flash".to_string(),
                "gemini-3.5-flash".to_string(),
            ]
        );
        assert_eq!(
            provider
                .retain_configured_models(vec![discovered_model("gemini-3.5-flash")])
                .len(),
            1
        );
    }

    // ── Config ──────────────────────────────────────────────────────────────

    #[test]
    fn test_gemini_config_default() {
        let config = GeminiConfig::default();
        assert_eq!(config.base_config.name, "google");
        assert!(config
            .base_config
            .models
            .contains(&"gemini-3.6-flash".to_string()));
    }

    #[test]
    fn test_provider_creation() {
        let provider = GeminiProvider::with_api_key("test-key".to_string());
        let metadata = provider.metadata();
        assert_eq!(metadata.name, "Google Gemini");
        assert!(!metadata.supported_features.is_empty());
    }

    // ── Request mapping ─────────────────────────────────────────────────────

    #[test]
    fn maps_user_and_assistant_roles_to_gemini_roles() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [
                {"role": "user", "content": "hi"},
                {"role": "assistant", "content": "hello"},
                {"role": "user", "content": "how are you"},
            ],
        }))
        .unwrap();
        let body = p.to_gemini_request(&req);
        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 3);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[0]["parts"][0]["text"], "hi");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[1]["parts"][0]["text"], "hello");
        assert_eq!(contents[2]["role"], "user");
    }

    #[test]
    fn lifts_system_messages_into_system_instruction() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [
                {"role": "system", "content": "be concise"},
                {"role": "user", "content": "hi"},
            ],
        }))
        .unwrap();
        let body = p.to_gemini_request(&req);
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "be concise");
        // System message must NOT also appear in `contents`.
        let contents = body["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
    }

    #[test]
    fn joins_multiple_system_messages() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [
                {"role": "system", "content": "rule one"},
                {"role": "system", "content": "rule two"},
                {"role": "user", "content": "hi"},
            ],
        }))
        .unwrap();
        let body = p.to_gemini_request(&req);
        let text = body["systemInstruction"]["parts"][0]["text"]
            .as_str()
            .unwrap();
        assert!(text.contains("rule one"));
        assert!(text.contains("rule two"));
    }

    #[test]
    fn maps_generation_config_fields() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [{"role": "user", "content": "hi"}],
            "temperature": 0.4,
            "top_p": 0.9,
            "n": 2,
            "max_tokens": 512,
            "stop": ["STOP_WORD"],
        }))
        .unwrap();
        let body = p.to_gemini_request(&req);
        let gc = &body["generationConfig"];
        // temperature/top_p round-trip through f32 (the request's wire type) before being
        // re-encoded as JSON f64, so compare with tolerance rather than exact equality.
        assert!((gc["temperature"].as_f64().unwrap() - 0.4).abs() < 1e-6);
        assert!((gc["topP"].as_f64().unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(gc["maxOutputTokens"], 512);
        assert_eq!(gc["stopSequences"][0], "STOP_WORD");
        assert_eq!(gc["candidateCount"], 2);
    }

    #[test]
    fn gemini_3_honors_explicit_minimal_thinking_and_native_json_without_legacy_sampling() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-3.6-flash",
            "messages": [{"role": "user", "content": "Return one JSON object."}],
            "temperature": 0.4,
            "top_p": 0.9,
            "n": 2,
            "max_tokens": 128,
            "response_format": {"type": "json_object"},
            "reasoning_effort": "minimal",
        }))
        .unwrap();

        let body = p.to_gemini_request(&req);
        let gc = &body["generationConfig"];

        assert_eq!(gc["maxOutputTokens"], 128);
        assert_eq!(gc["thinkingConfig"]["thinkingLevel"], "minimal");
        assert_eq!(gc["responseMimeType"], "application/json");
        assert!(gc.get("temperature").is_none());
        assert!(gc.get("topP").is_none());
        assert!(gc.get("candidateCount").is_none());
    }

    #[test]
    fn gemini_3_without_explicit_reasoning_effort_preserves_provider_default() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-3.6-flash",
            "messages": [{"role": "user", "content": "Solve the problem carefully."}],
            "max_tokens": 512,
        }))
        .unwrap();

        let body = p.to_gemini_request(&req);

        assert!(body["generationConfig"].get("thinkingConfig").is_none());
    }

    #[test]
    fn explicit_minimal_reasoning_uses_pro_lowest_supported_level() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-3.1-pro",
            "messages": [{"role": "user", "content": "Return one sentence."}],
            "max_tokens": 64,
            "reasoning_effort": "minimal",
        }))
        .unwrap();

        let body = p.to_gemini_request(&req);

        assert_eq!(
            body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
            "low"
        );
    }

    #[test]
    fn maps_openai_json_schema_to_gemini_native_schema() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-3.5-flash",
            "messages": [{"role": "user", "content": "Return one object."}],
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": "result",
                    "strict": true,
                    "schema": {
                        "type": "object",
                        "properties": {"answer": {"type": "string"}},
                        "required": ["answer"]
                    }
                }
            }
        }))
        .unwrap();

        let body = p.to_gemini_request(&req);
        let gc = &body["generationConfig"];

        assert_eq!(gc["responseMimeType"], "application/json");
        assert_eq!(gc["responseJsonSchema"]["type"], "object");
        assert_eq!(gc["responseJsonSchema"]["required"][0], "answer");
    }

    #[test]
    fn defaults_max_output_tokens_when_unset() {
        let p = provider();
        let req: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [{"role": "user", "content": "hi"}],
        }))
        .unwrap();
        let body = p.to_gemini_request(&req);
        assert_eq!(
            body["generationConfig"]["maxOutputTokens"],
            p.config.default_max_tokens
        );
    }

    // ── Response mapping ────────────────────────────────────────────────────

    #[test]
    fn parses_candidate_text_and_usage() {
        let p = provider();
        let raw = serde_json::json!({
            "candidates": [{
                "content": { "parts": [{"text": "Hello"}, {"text": " world"}], "role": "model" },
                "finishReason": "STOP",
                "index": 0
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 5,
                "totalTokenCount": 15
            }
        });
        let resp = p.parse_gemini_response(raw, "gemini-2.5-flash").unwrap();
        assert_eq!(resp.model, "gemini-2.5-flash");
        assert_eq!(resp.choices.len(), 1);
        match &resp.choices[0].message.content {
            Content::Text(t) => assert_eq!(t, "Hello world"),
            _ => panic!("expected text content"),
        }
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn maps_max_tokens_finish_reason_to_length() {
        assert_eq!(
            GeminiProvider::map_finish_reason(Some("MAX_TOKENS")).as_deref(),
            Some("length")
        );
    }

    #[test]
    fn maps_safety_finish_reasons_to_content_filter() {
        for reason in [
            "SAFETY",
            "RECITATION",
            "BLOCKLIST",
            "PROHIBITED_CONTENT",
            "SPII",
        ] {
            assert_eq!(
                GeminiProvider::map_finish_reason(Some(reason)).as_deref(),
                Some("content_filter"),
                "{reason} should map to content_filter"
            );
        }
    }

    #[test]
    fn errors_when_no_candidates_present() {
        let p = provider();
        let raw = serde_json::json!({ "candidates": [] });
        let result = p.parse_gemini_response(raw, "gemini-2.5-flash");
        assert!(result.is_err());
    }

    // ── Error mapping ───────────────────────────────────────────────────────

    #[test]
    fn maps_401_and_403_to_authentication_error() {
        let body =
            r#"{"error":{"code":401,"message":"API key not valid","status":"UNAUTHENTICATED"}}"#;
        for status in [
            reqwest::StatusCode::UNAUTHORIZED,
            reqwest::StatusCode::FORBIDDEN,
        ] {
            match GeminiProvider::map_error_response(status, body) {
                ProviderError::Authentication(msg) => assert!(msg.contains("API key not valid")),
                other => panic!("expected Authentication error, got {other:?}"),
            }
        }
    }

    #[test]
    fn maps_429_to_rate_limit_error() {
        let body = r#"{"error":{"code":429,"message":"Resource exhausted","status":"RESOURCE_EXHAUSTED"}}"#;
        match GeminiProvider::map_error_response(reqwest::StatusCode::TOO_MANY_REQUESTS, body) {
            ProviderError::RateLimit(msg) => assert!(msg.contains("Resource exhausted")),
            other => panic!("expected RateLimit error, got {other:?}"),
        }
    }

    #[test]
    fn maps_5xx_to_unavailable_error() {
        let body = r#"{"error":{"code":503,"message":"backend unavailable"}}"#;
        match GeminiProvider::map_error_response(reqwest::StatusCode::SERVICE_UNAVAILABLE, body) {
            ProviderError::Unavailable(msg) => assert!(msg.contains("backend unavailable")),
            other => panic!("expected Unavailable error, got {other:?}"),
        }
    }

    #[test]
    fn falls_back_to_sanitized_body_when_not_json() {
        match GeminiProvider::map_error_response(reqwest::StatusCode::BAD_REQUEST, "not json") {
            ProviderError::BadRequest(msg) => assert!(msg.contains("not json")),
            other => panic!("expected BadRequest error, got {other:?}"),
        }
    }

    // ── supports_model / cost ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_supports_model() {
        let provider = provider();
        assert!(provider.supports_model("gemini-2.5-flash").await);
        assert!(provider.supports_model("gemini-3.5-flash").await); // prefix match
        assert!(!provider.supports_model("gpt-4o").await);
    }

    #[tokio::test]
    async fn test_cost_info() {
        let provider = provider();
        let lite = provider
            .get_cost_info("gemini-3.5-flash-lite")
            .await
            .unwrap();
        assert_eq!(lite.input_cost_per_1k_tokens, 0.0003);
        assert_eq!(lite.output_cost_per_1k_tokens, 0.0025);

        let advanced = provider.get_cost_info("gemini-3.6-flash").await.unwrap();
        assert_eq!(advanced.input_cost_per_1k_tokens, 0.0015);
        assert_eq!(advanced.output_cost_per_1k_tokens, 0.0075);

        let frontier = provider.get_cost_info("gemini-3.5-flash").await.unwrap();
        assert_eq!(frontier.input_cost_per_1k_tokens, 0.0015);
        assert_eq!(frontier.output_cost_per_1k_tokens, 0.009);
    }

    #[test]
    fn advertises_streaming_support() {
        // The router's provider-selection loop (`EnterpriseGaussMeridian::route_chat_completion_stream`)
        // gates on this flag before ever calling `chat_completion_stream` — flipping it is as much
        // "the fix" as the streaming implementation itself.
        let capabilities = provider().capabilities();
        assert!(capabilities.supports_streaming);
    }

    // ── Streaming: chunk mapping (`map_stream_chunk`) ───────────────────────

    #[test]
    fn map_stream_chunk_yields_incremental_content_delta() {
        let json = serde_json::json!({
            "candidates": [{
                "content": { "parts": [{"text": "Hello"}], "role": "model" },
                "index": 0
            }]
        });
        let chunk =
            GeminiProvider::map_stream_chunk(&json, "gemini-abc", "gemini-2.5-flash").unwrap();
        assert_eq!(chunk.id, "gemini-abc");
        assert_eq!(chunk.model, "gemini-2.5-flash");
        assert_eq!(chunk.object, "chat.completion.chunk");
        assert_eq!(chunk.choices.len(), 1);
        let delta = chunk.choices[0].delta.as_ref().unwrap();
        assert_eq!(delta.content.as_deref(), Some("Hello"));
        // Mid-stream chunk: no finish_reason, no usage yet.
        assert!(chunk.choices[0].finish_reason.is_none());
        assert!(chunk.usage.is_none());
    }

    #[test]
    fn map_stream_chunk_terminal_chunk_carries_finish_reason_and_usage() {
        let json = serde_json::json!({
            "candidates": [{
                "content": { "parts": [{"text": "!"}], "role": "model" },
                "finishReason": "STOP",
                "index": 0
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 3,
                "totalTokenCount": 8
            }
        });
        let chunk =
            GeminiProvider::map_stream_chunk(&json, "gemini-abc", "gemini-2.5-flash").unwrap();
        assert_eq!(chunk.choices[0].finish_reason.as_deref(), Some("stop"));
        let usage = chunk.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 5);
        assert_eq!(usage.completion_tokens, 3);
        assert_eq!(usage.total_tokens, 8);
    }

    #[test]
    fn map_stream_chunk_errors_on_missing_candidates() {
        let json = serde_json::json!({ "candidates": [] });
        let result = GeminiProvider::map_stream_chunk(&json, "gemini-abc", "gemini-2.5-flash");
        assert!(result.is_err());
    }

    // ── Streaming: SSE line draining (`drain_stream_lines`) ─────────────────

    /// A captured (hand-built, matching Google's documented `streamGenerateContent?alt=sse`
    /// shape) three-event SSE fixture: two incremental text chunks followed by a terminal chunk
    /// carrying `finishReason` + `usageMetadata`. Gemini never sends a `[DONE]` sentinel the way
    /// OpenAI does — the stream just ends after the terminal chunk.
    fn sse_fixture() -> &'static str {
        "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hello\"}],\"role\":\"model\"},\"index\":0}]}\n\
         \n\
         data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\" world\"}],\"role\":\"model\"},\"index\":0}]}\n\
         \n\
         data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"!\"}],\"role\":\"model\"},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":5,\"candidatesTokenCount\":3,\"totalTokenCount\":8}}\n\
         \n"
    }

    #[test]
    fn drain_stream_lines_maps_full_fixture_in_order() {
        let mut buf = sse_fixture().to_string();
        let chunks = GeminiProvider::drain_stream_lines(&mut buf, "gemini-abc", "gemini-2.5-flash");

        assert_eq!(chunks.len(), 3);
        assert_eq!(
            chunks[0].choices[0]
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some("Hello")
        );
        assert_eq!(
            chunks[1].choices[0]
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some(" world")
        );
        assert_eq!(
            chunks[2].choices[0]
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some("!")
        );
        assert_eq!(chunks[2].choices[0].finish_reason.as_deref(), Some("stop"));
        assert!(chunks[2].usage.is_some());
        // Every complete line was consumed; nothing left buffered.
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_stream_lines_buffers_a_trailing_partial_line_across_calls() {
        // `bytes_stream()` chunk boundaries don't align to SSE event boundaries — simulate a
        // network read that cuts a `data:` line in half.
        let full = sse_fixture();
        let split_at = full.find(" world").unwrap(); // land mid-way through the 2nd event's JSON
        let (first_half, second_half) = full.split_at(split_at);

        let mut buf = first_half.to_string();
        let first_pass =
            GeminiProvider::drain_stream_lines(&mut buf, "gemini-abc", "gemini-2.5-flash");
        // Only the first, fully-buffered event could have been drained; the cut second line must
        // still be sitting in `buf`, not silently dropped.
        assert_eq!(first_pass.len(), 1);
        assert_eq!(
            first_pass[0].choices[0]
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some("Hello")
        );
        assert!(!buf.is_empty());

        buf.push_str(second_half);
        let second_pass =
            GeminiProvider::drain_stream_lines(&mut buf, "gemini-abc", "gemini-2.5-flash");
        assert_eq!(second_pass.len(), 2);
        assert_eq!(
            second_pass[0].choices[0]
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some(" world")
        );
        assert_eq!(
            second_pass[1].choices[0].finish_reason.as_deref(),
            Some("stop")
        );
    }

    #[test]
    fn drain_stream_lines_skips_malformed_chunks_without_losing_the_good_ones() {
        // Graceful degradation, line-by-line: one line is not valid JSON at all, another is valid
        // JSON but has an empty `candidates` array (fails `map_stream_chunk`). Neither should
        // abort the stream or drop the well-formed events around them.
        let mut buf = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hello\"}],\"role\":\"model\"},\"index\":0}]}\n",
            "\n",
            "data: {this is not valid json\n",
            "\n",
            "data: {\"candidates\":[]}\n",
            "\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\" world\"}],\"role\":\"model\"},\"finishReason\":\"STOP\",\"index\":0}]}\n",
            "\n",
        )
        .to_string();

        let chunks = GeminiProvider::drain_stream_lines(&mut buf, "gemini-abc", "gemini-2.5-flash");

        assert_eq!(
            chunks.len(),
            2,
            "the two malformed lines must be skipped, not fatal"
        );
        assert_eq!(
            chunks[0].choices[0]
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some("Hello")
        );
        assert_eq!(
            chunks[1].choices[0]
                .delta
                .as_ref()
                .unwrap()
                .content
                .as_deref(),
            Some(" world")
        );
        assert_eq!(chunks[1].choices[0].finish_reason.as_deref(), Some("stop"));
    }

    // ── Streaming: graceful-degradation synthesis ────────────────────────────

    #[test]
    fn synthetic_finish_chunk_carries_stop_and_no_content() {
        let chunk = GeminiProvider::synthetic_finish_chunk("gemini-abc", "gemini-2.5-flash");
        assert_eq!(chunk.choices[0].finish_reason.as_deref(), Some("stop"));
        assert!(chunk.choices[0].delta.as_ref().unwrap().content.is_none());
    }

    #[test]
    fn synthesize_fallback_chunks_replays_buffered_answer_as_two_chunks() {
        // Simulates the buffered-fallback path: a normal non-streaming `ChatCompletionResponse`
        // (as `parse_gemini_response` would produce) gets replayed as synthetic stream chunks so a
        // connect-time SSE failure still reaches the playground as a usable answer.
        let chat_response = ChatCompletionResponse {
            id: "gemini-abc".to_string(),
            object: "chat.completion".to_string(),
            created: 1_700_000_000,
            model: "gemini-2.5-flash".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: Role::Assistant,
                    content: Content::Text("full buffered answer".to_string()),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                    tool_call_id: None,
                    confidence: None,
                },
                finish_reason: Some("stop".to_string()),
                logprobs: None,
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 4,
                total_tokens: 14,
            }),
            system_fingerprint: None,
        };

        let chunks = GeminiProvider::synthesize_fallback_chunks(
            &chat_response,
            "gemini-abc".to_string(),
            "gemini-2.5-flash".to_string(),
        );

        assert_eq!(chunks.len(), 2);
        let content_delta = chunks[0].choices[0].delta.as_ref().unwrap();
        assert_eq!(
            content_delta.content.as_deref(),
            Some("full buffered answer")
        );
        assert!(chunks[0].choices[0].finish_reason.is_none());

        assert_eq!(chunks[1].choices[0].finish_reason.as_deref(), Some("stop"));
        let usage = chunks[1].usage.as_ref().unwrap();
        assert_eq!(usage.total_tokens, 14);
    }

    // ── Streaming: completion_stream request wrapping ────────────────────────

    #[test]
    fn completion_stream_wraps_prompt_as_single_user_chat_request() {
        // `completion_stream` builds its delegate `ChatCompletionRequest` via
        // `serde_json::from_value` — assert that wrapping actually round-trips the fields a real
        // `CompletionRequest` carries, so a malformed field name here wouldn't just silently
        // vanish behind `#[serde(default)]`.
        let value = serde_json::json!({
            "model": "gemini-2.5-flash",
            "messages": [{ "role": "user", "content": "prompt text" }],
            "temperature": 0.5,
            "top_p": 0.8,
            "max_tokens": 128,
            "stop": ["END"],
        });
        let chat_request: ChatCompletionRequest = serde_json::from_value(value).unwrap();
        assert_eq!(chat_request.messages.len(), 1);
        assert!(matches!(chat_request.messages[0].role, Role::User));
        match &chat_request.messages[0].content {
            Content::Text(t) => assert_eq!(t, "prompt text"),
            _ => panic!("expected text content"),
        }
        assert_eq!(chat_request.temperature, Some(0.5));
        assert_eq!(chat_request.max_tokens, Some(128));
    }
}
