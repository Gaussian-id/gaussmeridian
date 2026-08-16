//! Cohere provider implementation

use crate::cost::CostCalculator;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use std::pin::Pin;

/// Cohere provider configuration
#[derive(Debug, Clone)]
pub struct CohereConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout: std::time::Duration,
    pub max_retries: usize,
    pub models: Vec<String>,
    pub enable_streaming: bool,
}

impl Default for CohereConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.cohere.ai/v1".to_string(),
            api_key: None,
            timeout: std::time::Duration::from_secs(30),
            max_retries: 3,
            models: vec![
                "command".to_string(),
                "command-light".to_string(),
                "command-nightly".to_string(),
                "command-light-nightly".to_string(),
            ],
            enable_streaming: true,
        }
    }
}

/// Cohere provider implementation
pub struct CohereProvider {
    config: CohereConfig,
    base_url: String,
    client: Client,
}

impl CohereProvider {
    pub fn new(config: CohereConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            base_url: config.base_url.clone(),
            config,
            client,
        }
    }

    fn get_auth_header(&self) -> String {
        format!("Bearer {}", self.config.api_key.as_deref().unwrap_or(""))
    }

    async fn estimate_tokens(&self, text: &str) -> u32 {
        CostCalculator::estimate_tokens(text)
    }

    async fn estimate_message_tokens(&self, messages: &[Message]) -> u32 {
        CostCalculator::estimate_message_tokens(messages)
    }

    fn convert_messages_to_prompt(&self, messages: &[Message]) -> String {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::System => "System",
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::Function | Role::Tool => "Assistant", // Treat as assistant
                };
                let content = match &msg.content {
                    Content::Text(text) => text.clone(),
                    Content::Parts(parts) => {
                        parts
                            .iter()
                            .filter_map(|part| match part {
                                ContentPart::Text { text } => Some(text.clone()),
                                ContentPart::ImageUrl { .. } => None, // Cohere doesn't support images
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                };
                format!("{}: {}", role, content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl LLMProvider for CohereProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        let total_tokens = self.estimate_message_tokens(&request.messages).await;
        let start_time = std::time::Instant::now();

        // Convert messages to prompt format (Cohere uses prompt-based API)
        let prompt = self.convert_messages_to_prompt(&request.messages);

        let cohere_request = serde_json::json!({
            "model": request.model,
            "prompt": prompt,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature.unwrap_or(0.7),
            "p": request.top_p.unwrap_or(0.9),
            "stop_sequences": request.stop.as_ref().and_then(|s| match s {
                StopSequence::String(stop) => Some(vec![stop.clone()]),
                StopSequence::Array(stops) => Some(stops.clone()),
            }),
            "stream": false,
        });

        let response = self
            .client
            .post(&format!("{}/generate", self.base_url))
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/json")
            .json(&cohere_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(crate::error_utils::handle_http_error(response).await);
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        let generated_text = response_data["generations"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let choices = vec![Choice {
            index: 0,
            message: Message {
                role: Role::Assistant,
                content: Content::Text(generated_text),
                name: None,
                function_call: None,
                tool_calls: None,
                tool_call_id: None,
            },
            finish_reason: response_data["generations"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
            logprobs: None,
        }];

        let completion_text = match &choices[0].message.content {
            Content::Text(text) => text.clone(),
            Content::Parts(_) => String::new(),
        };
        let completion_tokens = self.estimate_tokens(&completion_text).await;
        let usage = Usage {
            prompt_tokens: total_tokens,
            completion_tokens,
            total_tokens: total_tokens + completion_tokens,
        };

        Ok(ChatCompletionResponse {
            id: response_data["id"]
                .as_str()
                .unwrap_or(&format!("cohere_{}", chrono::Utc::now().timestamp()))
                .to_string(),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices,
            usage: Some(usage),
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
        if !self.config.enable_streaming {
            return Err(ProviderError::BadRequest(
                "Streaming is not enabled for this provider".to_string(),
            ));
        }

        let prompt = self.convert_messages_to_prompt(&request.messages);
        let model = request.model.clone();

        let cohere_request = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature.unwrap_or(0.7),
            "p": request.top_p.unwrap_or(0.9),
            "stream": true,
        });

        let request_builder = self
            .client
            .post(&format!("{}/generate", self.base_url))
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/json")
            .json(&cohere_request);

        let response = request_builder
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(crate::error_utils::handle_http_error(response).await);
        }

        let stream = response
            .bytes_stream()
            .map(move |chunk| {
                let chunk = chunk
                    .map_err(|e| ProviderError::Internal(format!("Stream chunk error: {}", e)))?;

                let text = String::from_utf8_lossy(&chunk);
                let lines: Vec<&str> = text.lines().collect();

                for line in lines {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(text) = json["generations"][0]["text"].as_str() {
                                let chunk = ChatCompletionChunk {
                                    id: json["id"].as_str().unwrap_or("cohere_stream").to_string(),
                                    object: "chat.completion.chunk".to_string(),
                                    created: chrono::Utc::now().timestamp(),
                                    model: model.clone(),
                                    choices: vec![ChoiceDelta {
                                        index: 0,
                                        delta: Some(MessageDelta {
                                            role: None,
                                            content: Some(text.to_string()),
                                            function_call: None,
                                            tool_calls: None,
                                        }),
                                        finish_reason: json["generations"][0]["finish_reason"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                        logprobs: None,
                                    }],
                                    usage: None,
                                    system_fingerprint: None,
                                };
                                return Ok(Ok(chunk));
                            }
                        }
                    }
                }

                Ok(Err(ProviderError::Internal(
                    "Failed to parse stream chunk".to_string(),
                )))
            })
            .filter_map(|result| async move {
                match result {
                    Ok(Ok(chunk)) => Some(Ok(chunk)),
                    Ok(Err(e)) => Some(Err(e)),
                    Err(e) => Some(Err(e)),
                }
            });

        Ok(Box::pin(stream))
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Self::Error> {
        let total_tokens = self.estimate_tokens(&request.prompt).await;
        let start_time = std::time::Instant::now();

        let cohere_request = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature.unwrap_or(0.7),
            "p": request.top_p.unwrap_or(0.9),
            "stop_sequences": request.stop.as_ref().and_then(|s| match s {
                StopSequence::String(stop) => Some(vec![stop.clone()]),
                StopSequence::Array(stops) => Some(stops.clone()),
            }),
            "stream": false,
        });

        let response = self
            .client
            .post(&format!("{}/generate", self.base_url))
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/json")
            .json(&cohere_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(crate::error_utils::handle_http_error(response).await);
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        let generated_text = response_data["generations"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let choices = vec![CompletionChoice {
            text: generated_text.clone(),
            index: 0,
            logprobs: None,
            finish_reason: response_data["generations"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
        }];

        let usage = Usage {
            prompt_tokens: total_tokens,
            completion_tokens: self.estimate_tokens(&generated_text).await,
            total_tokens: total_tokens + self.estimate_tokens(&generated_text).await,
        };

        Ok(CompletionResponse {
            id: response_data["id"]
                .as_str()
                .unwrap_or(&format!("cohere_{}", chrono::Utc::now().timestamp()))
                .to_string(),
            object: "text_completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices,
            usage: Some(usage),
        })
    }

    async fn completion_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>, Self::Error>
    {
        if !self.config.enable_streaming {
            return Err(ProviderError::BadRequest(
                "Streaming is not enabled for this provider".to_string(),
            ));
        }

        let prompt = request.prompt.clone();
        let model = request.model.clone();

        let cohere_request = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "temperature": request.temperature.unwrap_or(0.7),
            "p": request.top_p.unwrap_or(0.9),
            "stream": true,
        });

        let request_builder = self
            .client
            .post(&format!("{}/generate", self.base_url))
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/json")
            .json(&cohere_request);

        let response = request_builder
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(crate::error_utils::handle_http_error(response).await);
        }

        let stream = response
            .bytes_stream()
            .map(move |chunk| {
                let chunk = chunk
                    .map_err(|e| ProviderError::Internal(format!("Stream chunk error: {}", e)))?;

                let text = String::from_utf8_lossy(&chunk);
                let lines: Vec<&str> = text.lines().collect();

                for line in lines {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(text) = json["generations"][0]["text"].as_str() {
                                let chunk = CompletionChunk {
                                    id: json["id"].as_str().unwrap_or("cohere_stream").to_string(),
                                    object: "text_completion.chunk".to_string(),
                                    created: chrono::Utc::now().timestamp(),
                                    model: model.clone(),
                                    choices: vec![CompletionChoiceDelta {
                                        text: text.to_string(),
                                        index: 0,
                                        logprobs: None,
                                        finish_reason: json["generations"][0]["finish_reason"]
                                            .as_str()
                                            .map(|s| s.to_string()),
                                    }],
                                    usage: None,
                                };
                                return Ok(Ok(chunk));
                            }
                        }
                    }
                }

                Ok(Err(ProviderError::Internal(
                    "Failed to parse stream chunk".to_string(),
                )))
            })
            .filter_map(|result| async move {
                match result {
                    Ok(Ok(chunk)) => Some(Ok(chunk)),
                    Ok(Err(e)) => Some(Err(e)),
                    Err(e) => Some(Err(e)),
                }
            });

        Ok(Box::pin(stream))
    }

    async fn embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Self::Error> {
        let input_texts = match &request.input {
            EmbeddingInput::String(s) => vec![s.clone()],
            EmbeddingInput::Array(arr) => arr.clone(),
            EmbeddingInput::ArrayOfArrays(_) => {
                return Err(ProviderError::BadRequest(
                    "Array of arrays not supported".to_string(),
                ))
            }
        };

        let mut total_tokens = 0u32;
        for text in input_texts.iter() {
            total_tokens += self.estimate_tokens(text).await;
        }
        let start_time = std::time::Instant::now();

        let cohere_request = serde_json::json!({
            "model": request.model,
            "texts": input_texts,
            "truncate": "END", // Cohere-specific parameter
        });

        let response = self
            .client
            .post(&format!("{}/embed", self.base_url))
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/json")
            .json(&cohere_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            return Err(crate::error_utils::handle_http_error(response).await);
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        let embeddings = response_data["embeddings"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
            .map(|(index, embedding_array)| {
                let embedding_values: Vec<f64> = embedding_array
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .collect();

                EmbeddingData {
                    object: "embedding".to_string(),
                    embedding: embedding_values,
                    index: index as u32,
                }
            })
            .collect();

        let usage = Usage {
            prompt_tokens: total_tokens,
            completion_tokens: 0,
            total_tokens: total_tokens,
        };

        Ok(EmbeddingResponse {
            object: "list".to_string(),
            data: embeddings,
            model: request.model,
            usage: Some(usage),
        })
    }

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        // Cohere doesn't have a public models endpoint, so we return configured models
        let models = self
            .config
            .models
            .iter()
            .map(|model_id| Model {
                id: model_id.clone(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "cohere".to_string(),
                permission: None,
                root: Some(model_id.clone()),
                parent: None,
            })
            .collect();

        Ok(models)
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "Cohere".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec![
                "chat_completion".to_string(),
                "completion".to_string(),
                "embeddings".to_string(),
            ],
            rate_limits: Some(RateLimits {
                requests_per_minute: Some(100),
                tokens_per_minute: Some(10000),
            }),
            pricing: None,
            models: vec![],
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        // Use a simple tokenize endpoint as health check
        let response = self
            .client
            .post(&format!("{}/tokenize", self.base_url))
            .header("Authorization", self.get_auth_header())
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "text": "health check",
                "model": "command"
            }))
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Health check failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::BadRequest(format!(
                "Health check failed with status: {}",
                response.status()
            )))
        }
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: self.config.enable_streaming,
            supports_functions: false,
            supports_vision: false,
            supports_embeddings: true,
            max_context_length: Some(4096),
            max_tokens_per_request: Some(4096),
            supported_models: self.config.models.clone(),
        }
    }

    async fn get_cost_info(&self, model: &str) -> Result<CostInfo, Self::Error> {
        // Cohere pricing (approximate, as of 2024)
        let (input_cost, output_cost) = match model {
            "command" | "command-nightly" => (0.0015, 0.002),
            "command-light" | "command-light-nightly" => (0.0005, 0.0005),
            _ => (0.001, 0.001),
        };

        Ok(CostInfo {
            input_cost_per_1k_tokens: input_cost,
            output_cost_per_1k_tokens: output_cost,
            currency: "USD".to_string(),
            model: model.to_string(),
        })
    }

    async fn supports_model(&self, model: &str) -> bool {
        self.config.models.contains(&model.to_string())
    }

    fn get_config(&self) -> ProviderConfig {
        ProviderConfig {
            base_url: self.base_url.clone(),
            api_key: self.config.api_key.clone(),
            timeout: self.config.timeout.as_secs(),
            max_retries: self.config.max_retries as u32,
            custom_headers: std::collections::HashMap::new(),
        }
    }
}
