//! Ollama provider implementation
//!
//! Provides support for locally hosted models via Ollama including:
//! - Chat completions with any Ollama-supported model
//! - Text completions
//! - Embeddings
//! - Streaming support
//! - Automatic model detection

use crate::common::BaseProviderConfig;
use futures::{Stream, StreamExt};
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use std::pin::Pin;
use tracing::{debug, error, warn};

/// Ollama-specific configuration
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_config: BaseProviderConfig,
    /// Default number of context tokens
    pub num_ctx: Option<u32>,
    /// Number of threads to use
    pub num_thread: Option<u32>,
    /// Keep model loaded in memory
    pub keep_alive: Option<String>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_config: BaseProviderConfig::new("ollama".to_string(), String::new())
                .with_base_url("http://localhost:11434".to_string())
                .with_timeout(300) // Local models may be slow to load
                .with_models(vec![
                    "llama3.2".to_string(),
                    "llama3.1".to_string(),
                    "llama2".to_string(),
                    "codellama".to_string(),
                    "mistral".to_string(),
                    "mixtral".to_string(),
                    "phi3".to_string(),
                    "qwen2.5".to_string(),
                    "gemma2".to_string(),
                    "nomic-embed-text".to_string(),
                    "mxbai-embed-large".to_string(),
                ]),
            num_ctx: Some(4096),
            num_thread: None,
            keep_alive: Some("5m".to_string()),
        }
    }
}

impl OllamaConfig {
    /// Create a new Ollama configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set custom base URL
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_config = self.base_config.with_base_url(base_url);
        self
    }

    /// Set context size
    pub fn with_num_ctx(mut self, num_ctx: u32) -> Self {
        self.num_ctx = Some(num_ctx);
        self
    }

    /// Set thread count
    pub fn with_num_thread(mut self, num_thread: u32) -> Self {
        self.num_thread = Some(num_thread);
        self
    }

    /// Set keep alive duration
    pub fn with_keep_alive(mut self, keep_alive: String) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }
}

/// Ollama provider implementation
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    config: OllamaConfig,
    client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(config: OllamaConfig) -> Self {
        let timeout = config.base_config.timeout.unwrap_or(300);
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout as u64))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(OllamaConfig::default())
    }

    /// Get base URL
    fn base_url(&self) -> &str {
        self.config
            .base_config
            .base_url
            .as_deref()
            .unwrap_or("http://localhost:11434")
    }

    /// Convert OpenAI-style messages to Ollama format
    fn convert_messages(&self, messages: &[Message]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Function | Role::Tool => "assistant",
                };

                let content = match &msg.content {
                    Content::Text(text) => text.clone(),
                    Content::Parts(parts) => parts
                        .iter()
                        .filter_map(|part| match part {
                            ContentPart::Text { text } => Some(text.clone()),
                            ContentPart::ImageUrl { .. } => None, // TODO: Handle images
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                };

                serde_json::json!({
                    "role": role,
                    "content": content
                })
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl LLMProvider for OllamaProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        let url = format!("{}/api/chat", self.base_url());
        debug!("Ollama chat completion request to {}", url);

        let messages = self.convert_messages(&request.messages);

        let mut options = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(top_p) = request.top_p {
            options.insert("top_p".to_string(), serde_json::json!(top_p));
        }
        if let Some(num_ctx) = self.config.num_ctx {
            options.insert("num_ctx".to_string(), serde_json::json!(num_ctx));
        }
        if let Some(num_thread) = self.config.num_thread {
            options.insert("num_thread".to_string(), serde_json::json!(num_thread));
        }

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": false
        });

        if !options.is_empty() {
            body["options"] = serde_json::Value::Object(options);
        }
        if let Some(ref keep_alive) = self.config.keep_alive {
            body["keep_alive"] = serde_json::json!(keep_alive);
        }

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!("Ollama API error {}: {}", status, error_body);
            return Err(ProviderError::BadRequest(format!(
                "HTTP {}: {}",
                status, error_body
            )));
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        // Convert Ollama response to OpenAI format
        let content = response_data["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let prompt_tokens = response_data["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = response_data["eval_count"].as_u64().unwrap_or(0) as u32;

        Ok(ChatCompletionResponse {
            id: format!("ollama-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: Role::Assistant,
                    content: Content::Text(content),
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
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }),
            system_fingerprint: None,
        })
    }

    async fn chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        let url = format!("{}/api/chat", self.base_url());
        debug!("Ollama streaming chat completion request to {}", url);

        let messages = self.convert_messages(&request.messages);

        let mut options = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(top_p) = request.top_p {
            options.insert("top_p".to_string(), serde_json::json!(top_p));
        }

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "stream": true
        });

        if !options.is_empty() {
            body["options"] = serde_json::Value::Object(options);
        }

        let model = request.model.clone();

        let response = self
            .client
            .post(&url)
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

        let stream = response.bytes_stream().map(move |chunk| {
            let model = model.clone();
            chunk
                .map_err(|e| ProviderError::Unavailable(format!("Stream error: {}", e)))
                .and_then(move |bytes| {
                    let text = String::from_utf8(bytes.to_vec())
                        .map_err(|e| ProviderError::Internal(format!("Invalid UTF-8: {}", e)))?;

                    // Ollama sends newline-delimited JSON
                    let json: serde_json::Value = serde_json::from_str(text.trim())
                        .map_err(|e| ProviderError::Internal(format!("Invalid JSON: {}", e)))?;

                    let content = json["message"]["content"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let done = json["done"].as_bool().unwrap_or(false);

                    Ok(ChatCompletionChunk {
                        id: format!("ollama-{}", uuid::Uuid::new_v4()),
                        object: "chat.completion.chunk".to_string(),
                        created: chrono::Utc::now().timestamp(),
                        model: model.clone(),
                        choices: vec![ChoiceDelta {
                            index: 0,
                            delta: Some(MessageDelta {
                                role: Some(Role::Assistant),
                                content: Some(content),
                                function_call: None,
                                tool_calls: None,
                            }),
                            finish_reason: if done {
                                Some("stop".to_string())
                            } else {
                                None
                            },
                            logprobs: None,
                        }],
                        usage: None,
                        system_fingerprint: None,
                    })
                })
        });

        Ok(Box::pin(stream))
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Self::Error> {
        let url = format!("{}/api/generate", self.base_url());
        debug!("Ollama completion request to {}", url);

        let mut options = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }

        let mut body = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": false
        });

        if !options.is_empty() {
            body["options"] = serde_json::Value::Object(options);
        }

        let response = self
            .client
            .post(&url)
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

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let text = response_data["response"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let prompt_tokens = response_data["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = response_data["eval_count"].as_u64().unwrap_or(0) as u32;

        Ok(CompletionResponse {
            id: format!("ollama-{}", uuid::Uuid::new_v4()),
            object: "text_completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices: vec![CompletionChoice {
                text,
                index: 0,
                logprobs: None,
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }),
        })
    }

    async fn completion_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        let url = format!("{}/api/generate", self.base_url());
        debug!("Ollama streaming completion request to {}", url);

        let body = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": true
        });

        let model = request.model.clone();

        let response = self
            .client
            .post(&url)
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

        let stream = response.bytes_stream().map(move |chunk| {
            let model = model.clone();
            chunk
                .map_err(|e| ProviderError::Unavailable(format!("Stream error: {}", e)))
                .and_then(move |bytes| {
                    let text = String::from_utf8(bytes.to_vec())
                        .map_err(|e| ProviderError::Internal(format!("Invalid UTF-8: {}", e)))?;

                    let json: serde_json::Value = serde_json::from_str(text.trim())
                        .map_err(|e| ProviderError::Internal(format!("Invalid JSON: {}", e)))?;

                    let response_text = json["response"].as_str().unwrap_or("").to_string();
                    let done = json["done"].as_bool().unwrap_or(false);

                    Ok(CompletionChunk {
                        id: format!("ollama-{}", uuid::Uuid::new_v4()),
                        object: "text_completion".to_string(),
                        created: chrono::Utc::now().timestamp(),
                        model: model.clone(),
                        choices: vec![CompletionChoiceDelta {
                            text: response_text,
                            index: 0,
                            logprobs: None,
                            finish_reason: if done {
                                Some("stop".to_string())
                            } else {
                                None
                            },
                        }],
                        usage: None,
                    })
                })
        });

        Ok(Box::pin(stream))
    }

    async fn embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Self::Error> {
        let url = format!("{}/api/embed", self.base_url());
        debug!("Ollama embedding request to {}", url);

        let input = match &request.input {
            EmbeddingInput::String(s) => vec![s.clone()],
            EmbeddingInput::Array(arr) => arr.clone(),
            EmbeddingInput::ArrayOfArrays(_) => {
                return Err(ProviderError::BadRequest(
                    "Ollama does not support array of arrays input".to_string(),
                ))
            }
        };

        let body = serde_json::json!({
            "model": request.model,
            "input": input
        });

        let response = self
            .client
            .post(&url)
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

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        // Ollama returns embeddings in the "embeddings" field
        let embeddings = response_data["embeddings"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .map(|(idx, emb)| EmbeddingData {
                        object: "embedding".to_string(),
                        embedding: emb
                            .as_array()
                            .unwrap_or(&vec![])
                            .iter()
                            .filter_map(|v| v.as_f64())
                            .collect(),
                        index: idx as u32,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let prompt_tokens = response_data["prompt_eval_count"].as_u64().unwrap_or(0) as u32;

        Ok(EmbeddingResponse {
            object: "list".to_string(),
            data: embeddings,
            model: request.model,
            usage: Some(Usage {
                prompt_tokens,
                completion_tokens: 0,
                total_tokens: prompt_tokens,
            }),
        })
    }

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        let url = format!("{}/api/tags", self.base_url());
        debug!("Ollama list models request to {}", url);

        let response = self
            .client
            .get(&url)
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

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let models = response_data["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|m| Model {
                        id: m["name"].as_str().unwrap_or("").to_string(),
                        object: "model".to_string(),
                        created: chrono::Utc::now().timestamp(),
                        owned_by: "local".to_string(),
                        permission: None,
                        root: None,
                        parent: None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "Ollama".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec![
                "chat_completions".to_string(),
                "completions".to_string(),
                "embeddings".to_string(),
                "streaming".to_string(),
            ],
            rate_limits: None, // Local provider, no rate limits
            pricing: None,     // Free local inference
            models: vec![],
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        let url = format!("{}/api/tags", self.base_url());

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
            supports_streaming: true,
            supports_functions: false, // Depends on model
            supports_vision: true,     // Some models support vision
            supports_embeddings: true,
            max_context_length: self.config.num_ctx,
            max_tokens_per_request: None, // Depends on model
            supported_models: self.config.base_config.models.clone(),
        }
    }

    async fn get_cost_info(&self, model: &str) -> Result<CostInfo, Self::Error> {
        // Ollama is free (local inference)
        Ok(CostInfo {
            input_cost_per_1k_tokens: 0.0,
            output_cost_per_1k_tokens: 0.0,
            currency: "USD".to_string(),
            model: model.to_string(),
        })
    }

    async fn supports_model(&self, model: &str) -> bool {
        // Check if model is installed by querying the tags endpoint
        if let Ok(models) = self.list_models().await {
            return models.iter().any(|m| m.id == model || m.id.starts_with(&format!("{}:", model)));
        }
        false
    }

    fn get_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self
                .config
                .base_config
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string()),
            api_key: None,
            timeout: self.config.base_config.timeout.unwrap_or(300),
            max_retries: self.config.base_config.max_retries.unwrap_or(3),
            custom_headers: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_config.name, "ollama");
        assert_eq!(
            config.base_config.base_url,
            Some("http://localhost:11434".to_string())
        );
    }

    #[test]
    fn test_ollama_config_builder() {
        let config = OllamaConfig::new()
            .with_base_url("http://remote:11434".to_string())
            .with_num_ctx(8192)
            .with_num_thread(8);

        assert_eq!(
            config.base_config.base_url,
            Some("http://remote:11434".to_string())
        );
        assert_eq!(config.num_ctx, Some(8192));
        assert_eq!(config.num_thread, Some(8));
    }

    #[test]
    fn test_provider_creation() {
        let provider = OllamaProvider::default_config();
        let metadata = provider.metadata();
        assert_eq!(metadata.name, "Ollama");
    }

    #[tokio::test]
    async fn test_cost_info() {
        let provider = OllamaProvider::default_config();
        let cost = provider.get_cost_info("llama2").await.unwrap();
        assert_eq!(cost.input_cost_per_1k_tokens, 0.0);
        assert_eq!(cost.output_cost_per_1k_tokens, 0.0);
    }
}
