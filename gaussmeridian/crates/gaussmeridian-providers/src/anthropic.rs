//! Anthropic Claude provider implementation

use crate::common::BaseProviderConfig;
// RateLimit is not used in this module
use futures::StreamExt;
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use std::pin::Pin;
// Arc is not used in this module

/// Claude models assumed when no allowlist is configured. Every name here is one the generated
/// catalog actually seeds — the previous defaults were `claude-3-*-2024*` ids that appear in no
/// catalog and that `supports_model()` could never route to.
const DEFAULT_MODELS: [&str; 3] = ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4.5"];

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub base_config: BaseProviderConfig,
    pub api_version: String,
    pub max_tokens: u32,
    /// Set once `with_model_allowlist` installs a non-empty list, exactly as in `OpenAIConfig`
    /// and `GeminiConfig`. Kept private so the only way to make the allowlist authoritative is
    /// through the builder.
    model_allowlist_is_authoritative: bool,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            base_config: BaseProviderConfig::new("anthropic".to_string(), "".to_string()),
            api_version: "2023-06-01".to_string(),
            max_tokens: 4096,
            model_allowlist_is_authoritative: false,
        }
    }
}

impl AnthropicConfig {
    pub fn new(api_key: String) -> Self {
        Self {
            base_config: BaseProviderConfig::new("anthropic".to_string(), api_key)
                .with_base_url("https://api.anthropic.com".to_string()),
            ..Default::default()
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_config = self.base_config.with_base_url(base_url);
        self
    }

    /// Install the generated `[providers.anthropic].models` allowlist from `gaussmeridian.toml`.
    ///
    /// Mirrors `OpenAIConfig::with_model_allowlist` / `GeminiConfig::with_model_allowlist`.
    /// Anthropic previously had no such builder, so `app.rs` hardcoded a two-entry
    /// `claude-3-*-20240229` list inline and the generated allowlist was ignored entirely —
    /// `supports_model()` is an exact match against `base_config.models`, so every catalog name
    /// was unroutable while the two hardcoded ids the catalog never seeds were the only
    /// matches. That is MG-RI-BUG-010 still live for this one provider.
    pub fn with_model_allowlist(mut self, models: Vec<String>) -> Self {
        if !models.is_empty() {
            self.base_config = self.base_config.with_models(models);
            self.model_allowlist_is_authoritative = true;
        }
        self
    }

    /// The models this adapter will accept — the configured allowlist when one is authoritative,
    /// otherwise [`DEFAULT_MODELS`]. Single source for `capabilities()` and `list_models()` so
    /// the two can never disagree.
    fn effective_models(&self) -> Vec<String> {
        if self.model_allowlist_is_authoritative {
            self.base_config.models.clone()
        } else {
            DEFAULT_MODELS.iter().map(|m| m.to_string()).collect()
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Resolve the base URL, env-overridable: `ANTHROPIC_API_BASE` → config `base_url` → the
    /// public Anthropic API. The env override lets a deployment (or local dev / a mock — e.g. when
    /// the account has no credits) repoint the provider without editing `gaussmeridian.toml`.
    /// Mirrors `OpenAIProvider::base_url`.
    fn base_url(&self) -> String {
        std::env::var("ANTHROPIC_API_BASE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| self.config.base_config.base_url.clone())
            .unwrap_or_else(|| "https://api.anthropic.com".to_string())
    }

    async fn estimate_tokens(&self, messages: &[Message]) -> u32 {
        messages
            .iter()
            .map(|msg| {
                match &msg.content {
                    Content::Text(text) => text.len() as u32 / 4,
                    Content::Parts(parts) => parts
                        .iter()
                        .map(|part| {
                            match part {
                                ContentPart::Text { text } => text.len() as u32 / 4,
                                ContentPart::ImageUrl { .. } => 1000, // Rough estimate for images
                            }
                        })
                        .sum::<u32>(),
                }
            })
            .sum()
    }
}

#[async_trait::async_trait]
impl LLMProvider for AnthropicProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        let estimated_tokens = self.estimate_tokens(&request.messages).await;

        let start_time = std::time::Instant::now();

        let anthropic_request = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens.unwrap_or(self.config.max_tokens),
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": false
        });

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url()))
            .header("x-api-key", &self.config.base_config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(ProviderError::BadRequest(format!(
                "API request failed with status: {}",
                response.status()
            )));
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        // Convert Anthropic response to our format
        let choices = vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content: Content::Text(
                    response_data["content"][0]["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                ),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
                confidence: None,
            },
            finish_reason: Some("stop".to_string()),
            logprobs: None,
        }];

        let usage = Usage {
            prompt_tokens: estimated_tokens,
            completion_tokens: estimated_tokens / 2, // Rough estimate
            total_tokens: estimated_tokens + estimated_tokens / 2,
        };

        Ok(ChatCompletionResponse {
            id: response_data["id"].as_str().unwrap_or("").to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices,
            usage: Some(usage),
            system_fingerprint: None,
        })
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Self::Error> {
        let estimated_tokens = request.prompt.len() as u32 / 4;

        let start_time = std::time::Instant::now();

        let anthropic_request = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "max_tokens": request.max_tokens.unwrap_or(self.config.max_tokens),
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": false
        });

        let response = self
            .client
            .post(format!("{}/v1/complete", self.base_url()))
            .header("x-api-key", &self.config.base_config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(ProviderError::BadRequest(format!(
                "API request failed with status: {}",
                response.status()
            )));
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        let choices = vec![CompletionChoice {
            text: response_data["completion"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            index: 0,
            logprobs: None,
            finish_reason: Some("stop".to_string()),
        }];

        let usage = Usage {
            prompt_tokens: estimated_tokens,
            completion_tokens: estimated_tokens / 2,
            total_tokens: estimated_tokens + estimated_tokens / 2,
        };

        Ok(CompletionResponse {
            id: response_data["id"].as_str().unwrap_or("").to_string(),
            object: "text_completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices,
            usage: Some(usage),
        })
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        let _estimated_tokens = self.estimate_tokens(&request.messages).await;

        let anthropic_request = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "max_tokens": request.max_tokens.unwrap_or(self.config.max_tokens),
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stream": true
        });

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url()))
            .header("x-api-key", &self.config.base_config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(ProviderError::BadRequest(format!(
                "API request failed with status: {}",
                response.status()
            )));
        }

        let model_for_stream = request.model.clone();
        let stream = response.bytes_stream().map(move |chunk| {
            let model = model_for_stream.clone();
            chunk
                .map_err(|e| ProviderError::Unavailable(format!("Stream error: {}", e)))
                .and_then(|bytes| {
                    let text = String::from_utf8(bytes.to_vec())
                        .map_err(|e| ProviderError::Internal(format!("Invalid UTF-8: {}", e)))?;

                    // Parse SSE format and convert to ChatCompletionChunk
                    if text.starts_with("data: ") {
                        let data = &text[6..];
                        if data == "[DONE]" {
                            return Ok(ChatCompletionChunk {
                                id: "".to_string(),
                                object: "chat.completion.chunk".to_string(),
                                created: chrono::Utc::now().timestamp(),
                                model: model.clone(),
                                choices: vec![ChoiceDelta {
                                    index: 0,
                                    delta: Some(MessageDelta {
                                        role: Some(Role::Assistant),
                                        content: Some("".to_string()),
                                        function_call: None,
                                        tool_calls: None,
                                    }),
                                    finish_reason: Some("stop".to_string()),
                                    logprobs: None,
                                }],
                                usage: None,
                                system_fingerprint: None,
                            });
                        }

                        // Parse JSON and convert
                        let json: serde_json::Value = serde_json::from_str(data)
                            .map_err(|e| ProviderError::Internal(format!("Invalid JSON: {}", e)))?;

                        Ok(ChatCompletionChunk {
                            id: json["id"].as_str().unwrap_or("").to_string(),
                            object: "chat.completion.chunk".to_string(),
                            created: chrono::Utc::now().timestamp(),
                            model: model.clone(),
                            choices: vec![ChoiceDelta {
                                index: 0,
                                delta: Some(MessageDelta {
                                    role: Some(Role::Assistant),
                                    content: Some(
                                        json["delta"]["content"][0]["text"]
                                            .as_str()
                                            .unwrap_or("")
                                            .to_string(),
                                    ),
                                    function_call: None,
                                    tool_calls: None,
                                }),
                                finish_reason: None,
                                logprobs: None,
                            }],
                            usage: None,
                            system_fingerprint: None,
                        })
                    } else {
                        Ok(ChatCompletionChunk {
                            id: "".to_string(),
                            object: "chat.completion.chunk".to_string(),
                            created: chrono::Utc::now().timestamp(),
                            model: model.clone(),
                            choices: vec![],
                            usage: None,
                            system_fingerprint: None,
                        })
                    }
                })
        });

        Ok(Box::pin(stream))
    }

    async fn completion_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<
        Pin<Box<dyn futures::Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        Err(ProviderError::Internal(
            "Completion streaming not supported".to_string(),
        ))
    }

    async fn embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Self::Error> {
        let estimated_tokens = match &request.input {
            EmbeddingInput::String(s) => s.len() as u32 / 4,
            EmbeddingInput::Array(arr) => arr.iter().map(|s| s.len() as u32 / 4).sum(),
            EmbeddingInput::ArrayOfArrays(_) => 1000, // Rough estimate
        };

        let start_time = std::time::Instant::now();

        let input = match &request.input {
            EmbeddingInput::String(s) => vec![s.clone()],
            EmbeddingInput::Array(arr) => arr.clone(),
            EmbeddingInput::ArrayOfArrays(_) => vec!["".to_string()], // Placeholder
        };

        let anthropic_request = serde_json::json!({
            "model": request.model,
            "input": input
        });

        let response = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url()))
            .header("x-api-key", &self.config.base_config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(ProviderError::BadRequest(format!(
                "API request failed with status: {}",
                response.status()
            )));
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        let embeddings = response_data["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| EmbeddingData {
                object: "embedding".to_string(),
                embedding: item["embedding"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .collect(),
                index: item["index"].as_u64().unwrap_or(0) as u32,
            })
            .collect();

        let usage = Usage {
            prompt_tokens: estimated_tokens,
            completion_tokens: 0,
            total_tokens: estimated_tokens,
        };

        Ok(EmbeddingResponse {
            object: "list".to_string(),
            data: embeddings,
            model: request.model,
            usage: Some(usage),
        })
    }

    /// Report exactly what `supports_model()` will accept — the configured allowlist.
    ///
    /// This used to return three hardcoded `claude-3-*-2024*` ids with a frozen
    /// `created: 1709251200`, which no catalog seeds and `supports_model()` does not match. So
    /// `GET /v1/models` advertised three ids that `GET /v1/models/:id` answers 404 for and that
    /// routing can never select, while every real Claude model stayed invisible (MG-RI-BUG-012).
    /// Deriving the list from `base_config.models` makes the two agree by construction.
    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        Ok(self
            .config
            .effective_models()
            .into_iter()
            .map(|id| Model {
                id,
                object: "model".to_string(),
                created: 0,
                owned_by: "anthropic".to_string(),
                permission: None,
                root: None,
                parent: None,
            })
            .collect())
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "Anthropic".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec![
                "chat_completions".to_string(),
                "completions".to_string(),
                "embeddings".to_string(),
                "streaming".to_string(),
            ],
            rate_limits: Some(RateLimits {
                requests_per_minute: Some(1000),
                tokens_per_minute: Some(100000),
            }),
            pricing: None,
            models: vec![], // Empty for now since we don't have ModelInfo structs
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url()))
            .header("x-api-key", &self.config.base_config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Health check failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::BadRequest("Health check failed".to_string()))
        }
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            supports_functions: false,
            supports_vision: true,
            supports_embeddings: true,
            max_context_length: Some(200000),
            max_tokens_per_request: Some(4096),
            supported_models: self.config.effective_models(),
        }
    }

    async fn get_cost_info(&self, model: &str) -> Result<CostInfo, Self::Error> {
        let (input_cost_per_1k, output_cost_per_1k) = match model {
            "claude-3-opus-20240229" => (0.015, 0.075),
            "claude-3-sonnet-20240229" => (0.003, 0.015),
            "claude-3-haiku-20240307" => (0.00025, 0.00125),
            _ => (0.003, 0.015), // Default to sonnet pricing
        };

        Ok(CostInfo {
            input_cost_per_1k_tokens: input_cost_per_1k,
            output_cost_per_1k_tokens: output_cost_per_1k,
            currency: "USD".to_string(),
            model: model.to_string(),
        })
    }

    /// Routes through `effective_models()` — the same source as `capabilities()` and
    /// `list_models()`. Reading `base_config.models` directly meant an unconfigured adapter
    /// (empty list) accepted nothing at all, while the list endpoint still advertised models.
    async fn supports_model(&self, model: &str) -> bool {
        self.config.effective_models().iter().any(|m| m == model)
    }

    fn get_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.config.base_config.base_url.clone().unwrap_or_default(),
            api_key: Some(self.config.base_config.api_key.clone()),
            timeout: self.config.base_config.timeout.unwrap_or(30),
            max_retries: self.config.base_config.max_retries.unwrap_or(3),
            custom_headers: std::collections::HashMap::new(),
        }
    }
}
