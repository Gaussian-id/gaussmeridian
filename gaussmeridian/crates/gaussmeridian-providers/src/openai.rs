//! OpenAI provider implementation
//!
//! Provides full support for OpenAI's API including:
//! - Chat completions (GPT-4, GPT-3.5-turbo, etc.)
//! - Text completions
//! - Embeddings (text-embedding-ada-002, text-embedding-3-small, etc.)
//! - Streaming support
//! - Function/tool calling
//! - Vision capabilities

use crate::common::BaseProviderConfig;
use futures::{Stream, StreamExt};
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use std::pin::Pin;
use tracing::{debug, error};

/// OpenAI-specific configuration
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub base_config: BaseProviderConfig,
    model_allowlist_is_authoritative: bool,
    /// Organization ID (optional)
    pub organization_id: Option<String>,
    /// Project ID (optional)
    pub project_id: Option<String>,
    /// Default max tokens for completions
    pub default_max_tokens: u32,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            base_config: BaseProviderConfig::new(
                "openai".to_string(),
                std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            )
            .with_base_url("https://api.openai.com/v1".to_string())
            .with_models(vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-3.5-turbo".to_string(),
                "text-embedding-3-small".to_string(),
                "text-embedding-3-large".to_string(),
                "text-embedding-ada-002".to_string(),
            ]),
            model_allowlist_is_authoritative: false,
            organization_id: std::env::var("OPENAI_ORG_ID").ok(),
            project_id: std::env::var("OPENAI_PROJECT_ID").ok(),
            default_max_tokens: 4096,
        }
    }
}

impl OpenAIConfig {
    /// Create a new OpenAI configuration with API key
    pub fn new(api_key: String) -> Self {
        Self {
            base_config: BaseProviderConfig::new("openai".to_string(), api_key)
                .with_base_url("https://api.openai.com/v1".to_string()),
            ..Default::default()
        }
    }

    /// Set organization ID
    pub fn with_organization(mut self, org_id: String) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    /// Set project ID
    pub fn with_project(mut self, project_id: String) -> Self {
        self.project_id = Some(project_id);
        self
    }

    /// Set custom base URL (for Azure OpenAI or compatible endpoints)
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

/// OpenAI o-series reasoning models (o1/o3/o4 families) take `max_completion_tokens`
/// instead of `max_tokens` and reject non-default `temperature`. Matched on the
/// model-name prefix so date-stamped variants (e.g. `o4-mini-2025-04-16`) are covered.
fn is_reasoning_model(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    ["o1", "o3", "o4"]
        .iter()
        .any(|p| m == *p || m.starts_with(&format!("{p}-")))
}

fn request_stream_usage(body: &mut serde_json::Value) {
    body["stream_options"] = serde_json::json!({"include_usage": true});
}

fn chat_request_body(request: &ChatCompletionRequest, stream: bool) -> serde_json::Value {
    let mut body = serde_json::json!({
        "model": &request.model,
        "messages": &request.messages,
        "stream": stream
    });
    if stream {
        request_stream_usage(&mut body);
    }

    if let Some(max_tokens) = request.max_tokens {
        if is_reasoning_model(&request.model) {
            body["max_completion_tokens"] = serde_json::json!(max_tokens);
        } else {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
    }
    if let Some(temperature) = request.temperature {
        if !is_reasoning_model(&request.model) {
            body["temperature"] = serde_json::json!(temperature);
        }
    }
    if let Some(top_p) = request.top_p {
        body["top_p"] = serde_json::json!(top_p);
    }
    if let Some(n) = request.n {
        body["n"] = serde_json::json!(n);
    }
    if let Some(stop) = &request.stop {
        body["stop"] = serde_json::json!(stop);
    }
    if let Some(presence_penalty) = request.presence_penalty {
        body["presence_penalty"] = serde_json::json!(presence_penalty);
    }
    if let Some(frequency_penalty) = request.frequency_penalty {
        body["frequency_penalty"] = serde_json::json!(frequency_penalty);
    }
    if let Some(user) = &request.user {
        body["user"] = serde_json::json!(user);
    }
    if let Some(logit_bias) = &request.logit_bias {
        body["logit_bias"] = serde_json::json!(logit_bias);
    }
    if let Some(functions) = &request.functions {
        body["functions"] = serde_json::json!(functions);
    }
    if let Some(tools) = &request.tools {
        body["tools"] = serde_json::json!(tools);
    }
    if let Some(response_format) = request.extra.get("response_format") {
        body["response_format"] = response_format.clone();
    }

    body
}

fn stream_usage(chunk: &serde_json::Value) -> Option<Usage> {
    let usage = chunk.get("usage")?;
    Some(Usage {
        prompt_tokens: usage.get("prompt_tokens")?.as_u64()?.try_into().ok()?,
        completion_tokens: usage.get("completion_tokens")?.as_u64()?.try_into().ok()?,
        total_tokens: usage.get("total_tokens")?.as_u64()?.try_into().ok()?,
    })
}

/// OpenAI retains `/v1/completions` only for legacy instruct-style models. Modern models in
/// Meridian's catalog are chat models, so this adapter translates the public text-completion
/// compatibility surface instead of dispatching them to an incompatible upstream endpoint.
fn uses_legacy_text_completion(model: &str) -> bool {
    let model = model.to_ascii_lowercase();
    model.contains("instruct")
        || model.starts_with("text-davinci-")
        || matches!(model.as_str(), "davinci" | "curie" | "babbage" | "ada")
}

fn text_request_as_chat(
    request: CompletionRequest,
) -> Result<ChatCompletionRequest, ProviderError> {
    let mut unsupported = Vec::new();
    if request.suffix.is_some() {
        unsupported.push("suffix");
    }
    if request.logprobs.is_some() {
        unsupported.push("logprobs");
    }
    if request.echo.is_some() {
        unsupported.push("echo");
    }
    if request.best_of.is_some() {
        unsupported.push("best_of");
    }
    if !unsupported.is_empty() {
        return Err(ProviderError::BadRequest(format!(
            "modern OpenAI text compatibility does not support: {}",
            unsupported.join(", ")
        )));
    }

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

    Ok(ChatCompletionRequest {
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
    })
}

fn chat_message_text(content: Content) -> String {
    match content {
        Content::Text(text) => text,
        Content::Parts(parts) => parts
            .into_iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text),
                ContentPart::ImageUrl { .. } => None,
            })
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn chat_response_as_text(response: ChatCompletionResponse) -> CompletionResponse {
    CompletionResponse {
        id: response.id,
        object: "text_completion".to_string(),
        created: response.created,
        model: response.model,
        choices: response
            .choices
            .into_iter()
            .map(|choice| CompletionChoice {
                text: chat_message_text(choice.message.content),
                index: choice.index,
                logprobs: choice.logprobs,
                finish_reason: choice.finish_reason,
            })
            .collect(),
        usage: response.usage,
    }
}

fn chat_chunk_as_text(chunk: ChatCompletionChunk) -> CompletionChunk {
    CompletionChunk {
        id: chunk.id,
        object: "text_completion".to_string(),
        created: chunk.created,
        model: chunk.model,
        choices: chunk
            .choices
            .into_iter()
            .map(|choice| CompletionChoiceDelta {
                text: choice
                    .delta
                    .and_then(|delta| delta.content)
                    .unwrap_or_default(),
                index: choice.index,
                logprobs: choice.logprobs,
                finish_reason: choice.finish_reason,
            })
            .collect(),
        usage: chunk.usage,
    }
}

/// OpenAI provider implementation
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider with the given configuration
    pub fn new(config: OpenAIConfig) -> Self {
        let timeout = config.base_config.timeout.unwrap_or(120);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout as u64))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Create a new OpenAI provider with just an API key
    pub fn with_api_key(api_key: String) -> Self {
        Self::new(OpenAIConfig::new(api_key))
    }

    fn retain_configured_models(&self, models: Vec<Model>) -> Vec<Model> {
        if !self.config.model_allowlist_is_authoritative {
            return models;
        }
        models
            .into_iter()
            .filter_map(|mut model| {
                if self.config.base_config.models.contains(&model.id) {
                    model.owned_by = "openai".to_string();
                    Some(model)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the base URL for API requests
    /// Resolve the base URL, env-overridable: `OPENAI_API_BASE` → config `base_url` → the public
    /// API. The env override lets a deployment (or local dev / a mock) repoint the provider
    /// without editing `gaussmeridian.toml`.
    fn base_url(&self) -> String {
        std::env::var("OPENAI_API_BASE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| self.config.base_config.base_url.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
    }

    /// Build headers for API requests
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        headers.insert(
            "Authorization",
            format!("Bearer {}", self.config.base_config.api_key)
                .parse()
                .unwrap(),
        );
        headers.insert("Content-Type", "application/json".parse().unwrap());

        if let Some(ref org_id) = self.config.organization_id {
            headers.insert("OpenAI-Organization", org_id.parse().unwrap());
        }

        if let Some(ref project_id) = self.config.project_id {
            headers.insert("OpenAI-Project", project_id.parse().unwrap());
        }

        headers
    }

    /// Estimate token count for messages (approximate)
    fn estimate_tokens(&self, messages: &[Message]) -> u32 {
        messages
            .iter()
            .map(|msg| {
                let content_tokens = match &msg.content {
                    Content::Text(text) => text.len() as u32 / 4, // ~4 chars per token
                    Content::Parts(parts) => parts
                        .iter()
                        .map(|part| match part {
                            ContentPart::Text { text } => text.len() as u32 / 4,
                            ContentPart::ImageUrl { .. } => 85, // Base tokens for image
                        })
                        .sum::<u32>(),
                };
                // Add overhead for message structure
                content_tokens + 4
            })
            .sum::<u32>()
            + 3 // Add 3 for priming
    }

    /// Parse SSE data into ChatCompletionChunk
    fn parse_sse_chunk(
        &self,
        data: &str,
        model: &str,
    ) -> Result<Option<ChatCompletionChunk>, ProviderError> {
        if data == "[DONE]" {
            return Ok(None);
        }

        let json: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| ProviderError::Internal(format!("Failed to parse SSE data: {}", e)))?;

        let choices: Vec<ChoiceDelta> = json["choices"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|choice| {
                let delta = &choice["delta"];
                ChoiceDelta {
                    index: choice["index"].as_u64().unwrap_or(0) as u32,
                    delta: Some(MessageDelta {
                        role: delta["role"].as_str().and_then(|r| match r {
                            "assistant" => Some(Role::Assistant),
                            "user" => Some(Role::User),
                            "system" => Some(Role::System),
                            "function" => Some(Role::Function),
                            "tool" => Some(Role::Tool),
                            _ => None,
                        }),
                        content: delta["content"].as_str().map(String::from),
                        function_call: None,
                        tool_calls: None,
                    }),
                    finish_reason: choice["finish_reason"].as_str().map(String::from),
                    logprobs: None,
                }
            })
            .collect();

        Ok(Some(ChatCompletionChunk {
            id: json["id"].as_str().unwrap_or("").to_string(),
            object: "chat.completion.chunk".to_string(),
            created: json["created"]
                .as_i64()
                .unwrap_or_else(|| chrono::Utc::now().timestamp()),
            model: model.to_string(),
            choices,
            usage: None,
            system_fingerprint: json["system_fingerprint"].as_str().map(String::from),
        }))
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        let url = format!("{}/chat/completions", self.base_url());
        debug!("OpenAI chat completion request to {}", url);

        let body = chat_request_body(&request, false);

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!("OpenAI API error {}: {}", status, error_body);

            return match status.as_u16() {
                401 => Err(ProviderError::Authentication("Invalid API key".to_string())),
                429 => Err(ProviderError::RateLimit("Rate limit exceeded".to_string())),
                400 => Err(ProviderError::BadRequest(error_body)),
                500..=599 => Err(ProviderError::Unavailable(format!(
                    "Server error: {}",
                    status
                ))),
                _ => Err(ProviderError::Internal(format!(
                    "HTTP {}: {}",
                    status, error_body
                ))),
            };
        }

        let response_data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        Ok(response_data)
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        let url = format!("{}/chat/completions", self.base_url());
        debug!("OpenAI streaming chat completion request to {}", url);

        let body = chat_request_body(&request, true);

        let model = request.model.clone();

        let req = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body);

        let es = EventSource::new(req).map_err(|e| {
            ProviderError::Unavailable(format!("Failed to create event source: {}", e))
        })?;

        // Drive the EventSource manually so we can TERMINATE the stream on `[DONE]` / clean EOF.
        // `reqwest_eventsource::EventSource` auto-reconnects on connection close (SSE reconnect
        // behavior); the previous `filter_map` never ended the stream, so after each `[DONE]` it
        // reconnected and re-hit the provider ~1/sec forever (observed live: 2 client requests →
        // 277 upstream calls, each surfaced as "Stream error: Stream ended"). Ending the
        // async-stream drops `es`, which stops the reconnect. `Error::StreamEnded` is a clean EOF,
        // not an error; any other error is surfaced ONCE and then the stream ends (no reconnect
        // loop) — mirroring `GeminiProvider::chat_completion_stream`.
        let stream = async_stream::stream! {
            let mut es = es;
            let mut pending_terminal = None;
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => continue,
                    Ok(Event::Message(msg)) => {
                        let data = msg.data.trim();
                        if data.is_empty() {
                            continue;
                        }
                        if data == "[DONE]" {
                            if let Some(chunk) = pending_terminal.take() {
                                yield Ok(chunk);
                            }
                            return; // terminal sentinel — end the stream (drops es, no reconnect)
                        }
                        match serde_json::from_str::<serde_json::Value>(data) {
                            Ok(json) => {
                                let choices: Vec<ChoiceDelta> = json["choices"]
                                    .as_array()
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .map(|choice| {
                                        let delta = &choice["delta"];
                                        ChoiceDelta {
                                            index: choice["index"].as_u64().unwrap_or(0) as u32,
                                            delta: Some(MessageDelta {
                                                role: delta["role"].as_str().and_then(|r| match r {
                                                    "assistant" => Some(Role::Assistant),
                                                    "user" => Some(Role::User),
                                                    "system" => Some(Role::System),
                                                    _ => None,
                                                }),
                                                content: delta["content"].as_str().map(String::from),
                                                function_call: None,
                                                tool_calls: None,
                                            }),
                                            finish_reason: choice["finish_reason"]
                                                .as_str()
                                                .map(String::from),
                                            logprobs: None,
                                        }
                                    })
                                    .collect();

                                let chunk = ChatCompletionChunk {
                                    id: json["id"].as_str().unwrap_or("").to_string(),
                                    object: "chat.completion.chunk".to_string(),
                                    created: json["created"]
                                        .as_i64()
                                        .unwrap_or_else(|| chrono::Utc::now().timestamp()),
                                    model: model.clone(),
                                    choices,
                                    usage: stream_usage(&json),
                                    system_fingerprint: json["system_fingerprint"]
                                        .as_str()
                                        .map(String::from),
                                };
                                let has_finish = chunk
                                    .choices
                                    .iter()
                                    .any(|choice| choice.finish_reason.is_some());
                                if has_finish && chunk.usage.is_none() {
                                    pending_terminal = Some(chunk);
                                } else if chunk.usage.is_some() {
                                    if let Some(mut terminal) = pending_terminal.take() {
                                        terminal.usage = chunk.usage;
                                        yield Ok(terminal);
                                    } else {
                                        yield Ok(chunk);
                                    }
                                } else {
                                    yield Ok(chunk);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse SSE chunk: {}", e);
                                continue;
                            }
                        }
                    }
                    Err(reqwest_eventsource::Error::StreamEnded) => {
                        if let Some(chunk) = pending_terminal.take() {
                            yield Ok(chunk);
                        }
                        return; // clean end-of-stream (EOF), not an error — do NOT reconnect
                    }
                    Err(e) => {
                        yield Err(ProviderError::Unavailable(format!("Stream error: {}", e)));
                        return; // surface the real error once, then end (no reconnect loop)
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
        if !uses_legacy_text_completion(&request.model) {
            let request = text_request_as_chat(request)?;
            return self
                .chat_completion(request)
                .await
                .map(chat_response_as_text);
        }

        let url = format!("{}/completions", self.base_url());
        debug!("OpenAI completion request to {}", url);

        let mut body = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": false
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }
        if let Some(n) = request.n {
            body["n"] = serde_json::json!(n);
        }
        if let Some(ref stop) = request.stop {
            body["stop"] = serde_json::json!(stop);
        }
        if let Some(presence_penalty) = request.presence_penalty {
            body["presence_penalty"] = serde_json::json!(presence_penalty);
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            body["frequency_penalty"] = serde_json::json!(frequency_penalty);
        }

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProviderError::BadRequest(format!(
                "HTTP {}: {}",
                status, error_body
            )));
        }

        let response_data: CompletionResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        Ok(response_data)
    }

    async fn completion_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>, Self::Error>
    {
        if !uses_legacy_text_completion(&request.model) {
            let request = text_request_as_chat(request)?;
            let stream = self.chat_completion_stream(request).await?;
            return Ok(Box::pin(stream.map(|chunk| chunk.map(chat_chunk_as_text))));
        }

        let url = format!("{}/completions", self.base_url());
        debug!("OpenAI streaming completion request to {}", url);

        let mut body = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": true
        });
        request_stream_usage(&mut body);

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let model = request.model.clone();

        let req = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body);

        let es = EventSource::new(req).map_err(|e| {
            ProviderError::Unavailable(format!("Failed to create event source: {}", e))
        })?;

        // Same termination fix as `chat_completion_stream`: end the stream on `[DONE]`/clean EOF so
        // the EventSource doesn't reconnect-loop and re-hit the provider. See that method's comment.
        let stream = async_stream::stream! {
            let mut es = es;
            let mut pending_terminal = None;
            while let Some(event) = es.next().await {
                match event {
                    Ok(Event::Open) => continue,
                    Ok(Event::Message(msg)) => {
                        let data = msg.data.trim();
                        if data.is_empty() {
                            continue;
                        }
                        if data == "[DONE]" {
                            if let Some(chunk) = pending_terminal.take() {
                                yield Ok(chunk);
                            }
                            return;
                        }
                        match serde_json::from_str::<serde_json::Value>(data) {
                            Ok(json) => {
                                let choices: Vec<CompletionChoiceDelta> = json["choices"]
                                    .as_array()
                                    .unwrap_or(&vec![])
                                    .iter()
                                    .map(|choice| CompletionChoiceDelta {
                                        text: choice["text"].as_str().unwrap_or("").to_string(),
                                        index: choice["index"].as_u64().unwrap_or(0) as u32,
                                        logprobs: None,
                                        finish_reason: choice["finish_reason"]
                                            .as_str()
                                            .map(String::from),
                                    })
                                    .collect();

                                let chunk = CompletionChunk {
                                    id: json["id"].as_str().unwrap_or("").to_string(),
                                    object: "text_completion".to_string(),
                                    created: json["created"]
                                        .as_i64()
                                        .unwrap_or_else(|| chrono::Utc::now().timestamp()),
                                    model: model.clone(),
                                    choices,
                                    usage: stream_usage(&json),
                                };
                                let has_finish = chunk
                                    .choices
                                    .iter()
                                    .any(|choice| choice.finish_reason.is_some());
                                if has_finish && chunk.usage.is_none() {
                                    pending_terminal = Some(chunk);
                                } else if chunk.usage.is_some() {
                                    if let Some(mut terminal) = pending_terminal.take() {
                                        terminal.usage = chunk.usage;
                                        yield Ok(terminal);
                                    } else {
                                        yield Ok(chunk);
                                    }
                                } else {
                                    yield Ok(chunk);
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                    Err(reqwest_eventsource::Error::StreamEnded) => {
                        if let Some(chunk) = pending_terminal.take() {
                            yield Ok(chunk);
                        }
                        return;
                    }
                    Err(e) => {
                        yield Err(ProviderError::Unavailable(format!("Stream error: {}", e)));
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Self::Error> {
        let url = format!("{}/embeddings", self.base_url());
        debug!("OpenAI embedding request to {}", url);

        let input = match &request.input {
            EmbeddingInput::String(s) => serde_json::json!(s),
            EmbeddingInput::Array(arr) => serde_json::json!(arr),
            EmbeddingInput::ArrayOfArrays(arr) => serde_json::json!(arr),
        };

        let mut body = serde_json::json!({
            "model": request.model,
            "input": input
        });

        // Add optional parameters from extra field if present
        if let Some(encoding_format) = request.extra.get("encoding_format") {
            body["encoding_format"] = encoding_format.clone();
        }
        if let Some(dimensions) = request.extra.get("dimensions") {
            body["dimensions"] = dimensions.clone();
        }

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProviderError::BadRequest(format!(
                "HTTP {}: {}",
                status, error_body
            )));
        }

        let response_data: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        Ok(response_data)
    }

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        let url = format!("{}/models", self.base_url());
        debug!("OpenAI list models request to {}", url);

        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(ProviderError::BadRequest(format!(
                "HTTP {}: {}",
                status, error_body
            )));
        }

        #[derive(serde::Deserialize)]
        struct ModelsResponse {
            data: Vec<Model>,
        }

        let response_data: ModelsResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        Ok(self.retain_configured_models(response_data.data))
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "OpenAI".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec![
                "chat_completions".to_string(),
                "completions".to_string(),
                "embeddings".to_string(),
                "streaming".to_string(),
                "function_calling".to_string(),
                "vision".to_string(),
                "json_mode".to_string(),
            ],
            rate_limits: Some(RateLimits {
                requests_per_minute: Some(10000),
                tokens_per_minute: Some(2000000),
            }),
            pricing: None,
            models: vec![],
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        let url = format!("{}/models", self.base_url());

        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
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
        let supported_models = if self.config.model_allowlist_is_authoritative {
            self.config.base_config.models.clone()
        } else {
            vec![
                "gpt-4o".to_string(),
                "gpt-4o-mini".to_string(),
                "gpt-4-turbo".to_string(),
                "gpt-4".to_string(),
                "gpt-3.5-turbo".to_string(),
                "text-embedding-3-small".to_string(),
                "text-embedding-3-large".to_string(),
                "text-embedding-ada-002".to_string(),
            ]
        };
        ProviderCapabilities {
            supports_streaming: true,
            supports_functions: true,
            supports_vision: true,
            supports_embeddings: true,
            max_context_length: Some(128000), // GPT-4 Turbo
            max_tokens_per_request: Some(4096),
            supported_models,
        }
    }

    async fn get_cost_info(&self, model: &str) -> Result<CostInfo, Self::Error> {
        // Pricing as of 2024 (per 1K tokens)
        let (input_cost, output_cost) = match model {
            "gpt-4o" => (0.005, 0.015),
            "gpt-4o-mini" => (0.00015, 0.0006),
            "gpt-4-turbo" | "gpt-4-turbo-preview" => (0.01, 0.03),
            "gpt-4" | "gpt-4-0613" => (0.03, 0.06),
            "gpt-4-32k" => (0.06, 0.12),
            "gpt-3.5-turbo" | "gpt-3.5-turbo-0125" => (0.0005, 0.0015),
            "gpt-3.5-turbo-instruct" => (0.0015, 0.002),
            "text-embedding-3-small" => (0.00002, 0.0),
            "text-embedding-3-large" => (0.00013, 0.0),
            "text-embedding-ada-002" => (0.0001, 0.0),
            _ => (0.002, 0.002), // Default fallback
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

        // Check for model prefix matches (e.g., "gpt-4o-" for dated versions)
        let prefixes = [
            "gpt-4o",
            "gpt-4-turbo",
            "gpt-4",
            "gpt-3.5-turbo",
            "text-embedding",
        ];
        prefixes.iter().any(|prefix| model.starts_with(prefix))
    }

    fn get_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self
                .config
                .base_config
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            api_key: Some(self.config.base_config.api_key.clone()),
            timeout: self.config.base_config.timeout.unwrap_or(120),
            max_retries: self.config.base_config.max_retries.unwrap_or(3),
            custom_headers: {
                let mut headers = std::collections::HashMap::new();
                if let Some(ref org) = self.config.organization_id {
                    headers.insert("OpenAI-Organization".to_string(), org.clone());
                }
                headers
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn discovered_model(id: &str) -> Model {
        Model {
            id: id.to_string(),
            object: "model".to_string(),
            created: 0,
            owned_by: "openai".to_string(),
            permission: None,
            root: None,
            parent: None,
        }
    }

    #[tokio::test]
    async fn configured_model_allowlist_is_strict_across_provider_surfaces() {
        let provider = OpenAIProvider::new(
            OpenAIConfig::new("test-key".to_string())
                .with_model_allowlist(vec!["gpt-4o-mini".to_string(), "gpt-4o".to_string()]),
        );

        assert_eq!(
            provider.capabilities().supported_models,
            vec!["gpt-4o-mini".to_string(), "gpt-4o".to_string()]
        );
        assert!(provider.supports_model("gpt-4o-mini").await);
        assert!(provider.supports_model("gpt-4o").await);
        assert!(!provider.supports_model("gpt-4o-2024-11-20").await);
        assert!(!provider.supports_model("gpt-4-turbo").await);

        let discovered = provider.retain_configured_models(vec![
            Model {
                owned_by: "system".to_string(),
                ..discovered_model("gpt-4o")
            },
            discovered_model("gpt-4-turbo"),
            Model {
                owned_by: "system".to_string(),
                ..discovered_model("gpt-4o-mini")
            },
        ]);
        assert_eq!(
            discovered
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            vec!["gpt-4o", "gpt-4o-mini"]
        );
        assert!(discovered.iter().all(|model| model.owned_by == "openai"));
    }

    #[tokio::test]
    async fn default_model_configuration_preserves_prefix_compatibility() {
        let provider = OpenAIProvider::with_api_key("test-key".to_string());

        assert!(provider.supports_model("gpt-4o-2024-11-20").await);
        assert_eq!(
            provider
                .retain_configured_models(vec![discovered_model("gpt-4-turbo")])
                .len(),
            1
        );
    }

    #[test]
    fn streaming_request_and_terminal_chunk_preserve_provider_usage() {
        let mut body = serde_json::json!({"model": "gpt-4o-mini", "stream": true});
        request_stream_usage(&mut body);
        assert_eq!(body["stream_options"]["include_usage"], true);

        let chunk = serde_json::json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 17,
                "completion_tokens": 5,
                "total_tokens": 22
            }
        });
        let usage = stream_usage(&chunk).expect("terminal usage must parse");
        assert_eq!(usage.prompt_tokens, 17);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 22);
    }

    #[test]
    fn modern_chat_models_use_chat_transport_for_text_compatibility() {
        for model in ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-4"] {
            assert!(
                !uses_legacy_text_completion(model),
                "{model} must use the chat-completions compatibility adapter"
            );
        }
        assert!(uses_legacy_text_completion("gpt-3.5-turbo-instruct"));
    }

    #[test]
    fn text_request_maps_to_one_user_chat_message_without_losing_controls() {
        let request: CompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gpt-4o",
            "prompt": "Explain the invariant.",
            "max_tokens": 17,
            "temperature": 0.2,
            "top_p": 0.8,
            "n": 2,
            "stream": false,
            "stop": ["END"],
            "presence_penalty": 0.1,
            "frequency_penalty": 0.3,
            "user": "acceptance"
        }))
        .expect("valid completion request");

        let mapped = text_request_as_chat(request).expect("supported controls map");

        assert_eq!(mapped.model, "gpt-4o");
        assert_eq!(mapped.messages.len(), 1);
        assert!(matches!(mapped.messages[0].role, Role::User));
        assert!(matches!(
            &mapped.messages[0].content,
            Content::Text(text) if text == "Explain the invariant."
        ));
        assert_eq!(mapped.max_tokens, Some(17));
        assert_eq!(mapped.temperature, Some(0.2));
        assert_eq!(mapped.top_p, Some(0.8));
        assert_eq!(mapped.n, Some(2));
        assert_eq!(mapped.stream, Some(false));
        assert_eq!(mapped.presence_penalty, Some(0.1));
        assert_eq!(mapped.frequency_penalty, Some(0.3));
        assert_eq!(mapped.user.as_deref(), Some("acceptance"));
    }

    #[tokio::test]
    async fn modern_text_controls_fail_before_buffered_or_streaming_dispatch() {
        let provider = OpenAIProvider::new(
            OpenAIConfig::new("test-key".to_string())
                .with_base_url("http://127.0.0.1:1".to_string()),
        );

        for (field, value) in [
            ("suffix", serde_json::json!("tail")),
            ("logprobs", serde_json::json!(2)),
            ("echo", serde_json::json!(true)),
            ("best_of", serde_json::json!(2)),
        ] {
            let mut payload = serde_json::json!({
                "model": "gpt-4o-mini",
                "prompt": "hello"
            });
            payload[field] = value;
            let request: CompletionRequest =
                serde_json::from_value(payload).expect("valid public request");

            let buffered_error = provider
                .completion(request.clone())
                .await
                .expect_err("buffered request must reject unsupported control");
            let streaming_error = match provider.completion_stream(request).await {
                Err(error) => error,
                Ok(_) => panic!("streaming request must reject unsupported control"),
            };
            for error in [buffered_error, streaming_error] {
                assert!(
                    matches!(error, ProviderError::BadRequest(ref message) if message.contains(field)),
                    "{field} must produce a typed field-specific rejection, got {error:?}"
                );
            }
        }
    }

    #[test]
    fn modern_text_mapping_preserves_logit_bias() {
        let request: CompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gpt-4o",
            "prompt": "hello",
            "logit_bias": {"42": -10.0}
        }))
        .expect("valid completion request");

        let mapped = text_request_as_chat(request).expect("shared control maps");
        assert_eq!(
            mapped
                .logit_bias
                .as_ref()
                .and_then(|bias| bias.get("42"))
                .copied(),
            Some(-10.0)
        );
    }

    #[test]
    fn buffered_chat_outbound_body_preserves_supported_controls() {
        let request: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [{"role": "user", "content": "hello"}],
            "user": "acceptance",
            "logit_bias": {"42": -10.0},
            "response_format": {"type": "json_object"}
        }))
        .expect("valid chat request");

        let body = chat_request_body(&request, false);

        assert_eq!(body["stream"], false);
        assert_eq!(body["user"], "acceptance");
        assert_eq!(body["logit_bias"]["42"], -10.0);
        assert_eq!(body["response_format"]["type"], "json_object");
    }

    #[test]
    fn streaming_chat_outbound_body_preserves_supported_controls() {
        let request: ChatCompletionRequest = serde_json::from_value(serde_json::json!({
            "model": "gpt-4o-mini",
            "messages": [{"role": "user", "content": "hello"}],
            "user": "acceptance",
            "logit_bias": {"42": -10.0},
            "response_format": {"type": "json_object"}
        }))
        .expect("valid chat request");

        let body = chat_request_body(&request, true);

        assert_eq!(body["stream"], true);
        assert_eq!(body["stream_options"]["include_usage"], true);
        assert_eq!(body["user"], "acceptance");
        assert_eq!(body["logit_bias"]["42"], -10.0);
        assert_eq!(body["response_format"]["type"], "json_object");
    }

    #[test]
    fn chat_response_maps_back_to_text_completion_schema() {
        let response: ChatCompletionResponse = serde_json::from_value(serde_json::json!({
            "id": "chatcmpl-1",
            "object": "chat.completion",
            "created": 7,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Mapped answer."},
                "finish_reason": "stop",
                "logprobs": null
            }],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 3,
                "total_tokens": 8
            }
        }))
        .expect("valid chat response");

        let mapped = chat_response_as_text(response);

        assert_eq!(mapped.object, "text_completion");
        assert_eq!(mapped.choices.len(), 1);
        assert_eq!(mapped.choices[0].text, "Mapped answer.");
        assert_eq!(mapped.choices[0].finish_reason.as_deref(), Some("stop"));
        assert_eq!(mapped.usage.expect("usage preserved").total_tokens, 8);
    }

    #[test]
    fn chat_stream_chunk_maps_back_to_text_completion_schema() {
        let chunk: ChatCompletionChunk = serde_json::from_value(serde_json::json!({
            "id": "chatcmpl-1",
            "object": "chat.completion.chunk",
            "created": 7,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "delta": {"content": "Mapped"},
                "finish_reason": null,
                "logprobs": null
            }],
            "usage": null
        }))
        .expect("valid chat chunk");

        let mapped = chat_chunk_as_text(chunk);

        assert_eq!(mapped.object, "text_completion");
        assert_eq!(mapped.choices.len(), 1);
        assert_eq!(mapped.choices[0].text, "Mapped");
        assert!(mapped.choices[0].finish_reason.is_none());
    }

    #[test]
    fn test_openai_config_default() {
        let config = OpenAIConfig::default();
        assert_eq!(config.base_config.name, "openai");
        assert!(config.base_config.models.contains(&"gpt-4o".to_string()));
    }

    #[test]
    fn reasoning_models_are_detected_by_prefix() {
        for m in ["o4-mini", "o4-mini-2025-04-16", "o1", "o3-pro", "O4-MINI"] {
            assert!(is_reasoning_model(m), "{m} should be a reasoning model");
        }
        for m in [
            "gpt-4o",
            "gpt-4o-mini",
            "o200k-something",
            "codestral",
            "claude-sonnet-4-5",
        ] {
            assert!(
                !is_reasoning_model(m),
                "{m} should NOT be a reasoning model"
            );
        }
    }

    #[test]
    fn test_openai_config_builder() {
        let config = OpenAIConfig::new("test-key".to_string())
            .with_organization("org-123".to_string())
            .with_base_url("https://custom.api.com/v1".to_string());

        assert_eq!(config.base_config.api_key, "test-key");
        assert_eq!(config.organization_id, Some("org-123".to_string()));
        assert_eq!(
            config.base_config.base_url,
            Some("https://custom.api.com/v1".to_string())
        );
    }

    #[test]
    fn test_provider_creation() {
        let config = OpenAIConfig::new("test-key".to_string());
        let provider = OpenAIProvider::new(config);

        let metadata = provider.metadata();
        assert_eq!(metadata.name, "OpenAI");
        assert!(metadata
            .supported_features
            .contains(&"streaming".to_string()));
    }

    #[tokio::test]
    async fn test_supports_model() {
        let provider = OpenAIProvider::with_api_key("test-key".to_string());

        assert!(provider.supports_model("gpt-4o").await);
        assert!(provider.supports_model("gpt-4-turbo-2024-04-09").await);
        assert!(provider.supports_model("text-embedding-3-small").await);
    }

    #[tokio::test]
    async fn test_cost_info() {
        let provider = OpenAIProvider::with_api_key("test-key".to_string());

        let cost = provider.get_cost_info("gpt-4o").await.unwrap();
        assert_eq!(cost.model, "gpt-4o");
        assert!(cost.input_cost_per_1k_tokens > 0.0);
    }
}
