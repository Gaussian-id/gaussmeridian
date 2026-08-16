//! LMStudio provider implementation
//!
//! LMStudio provides an OpenAI-compatible API for running LLMs locally.
//! This provider implements the OpenAI API format for seamless integration.

use crate::cost::CostCalculator;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use std::pin::Pin;

/// LMStudio provider configuration
#[derive(Debug, Clone)]
pub struct LMStudioConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout: std::time::Duration,
    pub max_retries: usize,
    pub models: Vec<String>,
    pub enable_streaming: bool,
}

impl Default for LMStudioConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:1234/v1".to_string(),
            api_key: None,
            timeout: std::time::Duration::from_secs(120),
            max_retries: 2,
            models: vec![],
            enable_streaming: true,
        }
    }
}

/// LMStudio provider implementation
pub struct LMStudioProvider {
    config: LMStudioConfig,
    base_url: String,
    client: Client,
}

impl LMStudioProvider {
    pub fn new(config: LMStudioConfig) -> Self {
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

    fn get_auth_header(&self) -> Option<String> {
        self.config
            .api_key
            .as_ref()
            .map(|key| format!("Bearer {}", key))
    }

    async fn estimate_tokens(&self, text: &str) -> u32 {
        CostCalculator::estimate_tokens(text)
    }

    async fn estimate_message_tokens(&self, messages: &[Message]) -> u32 {
        CostCalculator::estimate_message_tokens(messages)
    }
}

#[async_trait]
impl LLMProvider for LMStudioProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        let estimated_tokens = self.estimate_message_tokens(&request.messages).await;
        let start_time = std::time::Instant::now();

        let lmstudio_request = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "stream": false,
            "stop": request.stop,
            "presence_penalty": request.presence_penalty,
            "frequency_penalty": request.frequency_penalty,
        });

        let mut req_builder = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json");

        if let Some(auth) = self.get_auth_header() {
            req_builder = req_builder.header("Authorization", auth);
        }

        let response = req_builder
            .json(&lmstudio_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => ProviderError::Authentication(format!("Unauthorized: {}", error_text)),
                429 => ProviderError::RateLimit(format!("Rate limited: {}", error_text)),
                400..=499 => ProviderError::BadRequest(format!("Bad request: {}", error_text)),
                _ => ProviderError::Internal(format!("Server error: {}", error_text)),
            });
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        let choices: Vec<Choice> = serde_json::from_value(response_data["choices"].clone())
            .map_err(|e| ProviderError::Internal(format!("Failed to parse choices: {}", e)))?;

        let usage = response_data["usage"].as_object().map(|usage| Usage {
            prompt_tokens: usage["prompt_tokens"]
                .as_u64()
                .unwrap_or(estimated_tokens as u64) as u32,
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: usage["total_tokens"]
                .as_u64()
                .unwrap_or(estimated_tokens as u64) as u32,
        });

        Ok(ChatCompletionResponse {
            id: response_data["id"]
                .as_str()
                .unwrap_or(&format!("lmstudio_{}", chrono::Utc::now().timestamp()))
                .to_string(),
            object: "chat.completion".to_string(),
            created: response_data["created"]
                .as_i64()
                .unwrap_or_else(|| chrono::Utc::now().timestamp()),
            model: response_data["model"]
                .as_str()
                .unwrap_or(&request.model)
                .to_string(),
            choices,
            usage,
            system_fingerprint: response_data["system_fingerprint"]
                .as_str()
                .map(|s| s.to_string()),
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

        let lmstudio_request = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "stream": true,
            "stop": request.stop,
        });

        let mut req_builder = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Content-Type", "application/json");

        if let Some(auth) = self.get_auth_header() {
            req_builder = req_builder.header("Authorization", auth);
        }

        let response = req_builder
            .json(&lmstudio_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::BadRequest(format!(
                "Streaming request failed: {}",
                error_text
            )));
        }

        let model = request.model.clone();
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
                            let chunk = ChatCompletionChunk {
                                id: json["id"].as_str().unwrap_or("lmstudio_stream").to_string(),
                                object: "chat.completion.chunk".to_string(),
                                created: json["created"]
                                    .as_i64()
                                    .unwrap_or_else(|| chrono::Utc::now().timestamp()),
                                model: json["model"].as_str().unwrap_or(&model).to_string(),
                                choices: serde_json::from_value(json["choices"].clone())
                                    .unwrap_or_default(),
                                usage: None,
                                system_fingerprint: None,
                            };
                            return Ok(Ok(chunk));
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
        let estimated_tokens = self.estimate_tokens(&request.prompt).await;
        let start_time = std::time::Instant::now();

        let lmstudio_request = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "stream": false,
            "stop": request.stop,
            "presence_penalty": request.presence_penalty,
            "frequency_penalty": request.frequency_penalty,
        });

        let mut req_builder = self
            .client
            .post(&format!("{}/completions", self.base_url))
            .header("Content-Type", "application/json");

        if let Some(auth) = self.get_auth_header() {
            req_builder = req_builder.header("Authorization", auth);
        }

        let response = req_builder
            .json(&lmstudio_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => ProviderError::Authentication(format!("Unauthorized: {}", error_text)),
                429 => ProviderError::RateLimit(format!("Rate limited: {}", error_text)),
                400..=499 => ProviderError::BadRequest(format!("Bad request: {}", error_text)),
                _ => ProviderError::Internal(format!("Server error: {}", error_text)),
            });
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let _latency = start_time.elapsed();

        let choices: Vec<CompletionChoice> =
            serde_json::from_value(response_data["choices"].clone())
                .map_err(|e| ProviderError::Internal(format!("Failed to parse choices: {}", e)))?;

        let usage = response_data["usage"].as_object().map(|usage| Usage {
            prompt_tokens: usage["prompt_tokens"]
                .as_u64()
                .unwrap_or(estimated_tokens as u64) as u32,
            completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: usage["total_tokens"]
                .as_u64()
                .unwrap_or(estimated_tokens as u64) as u32,
        });

        Ok(CompletionResponse {
            id: response_data["id"]
                .as_str()
                .unwrap_or(&format!("lmstudio_{}", chrono::Utc::now().timestamp()))
                .to_string(),
            object: "text_completion".to_string(),
            created: response_data["created"]
                .as_i64()
                .unwrap_or_else(|| chrono::Utc::now().timestamp()),
            model: response_data["model"]
                .as_str()
                .unwrap_or(&request.model)
                .to_string(),
            choices,
            usage,
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

        let lmstudio_request = serde_json::json!({
            "model": request.model,
            "prompt": request.prompt,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "stream": true,
            "stop": request.stop,
        });

        let mut req_builder = self
            .client
            .post(&format!("{}/completions", self.base_url))
            .header("Content-Type", "application/json");

        if let Some(auth) = self.get_auth_header() {
            req_builder = req_builder.header("Authorization", auth);
        }

        let response = req_builder
            .json(&lmstudio_request)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::BadRequest(format!(
                "Streaming request failed: {}",
                error_text
            )));
        }

        let model = request.model.clone();
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
                            let chunk = CompletionChunk {
                                id: json["id"].as_str().unwrap_or("lmstudio_stream").to_string(),
                                object: "text_completion.chunk".to_string(),
                                created: json["created"]
                                    .as_i64()
                                    .unwrap_or_else(|| chrono::Utc::now().timestamp()),
                                model: json["model"].as_str().unwrap_or(&model).to_string(),
                                choices: serde_json::from_value(json["choices"].clone())
                                    .unwrap_or_default(),
                                usage: None,
                            };
                            return Ok(Ok(chunk));
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

    async fn embedding(
        &self,
        _request: EmbeddingRequest,
    ) -> Result<EmbeddingResponse, Self::Error> {
        Err(ProviderError::BadRequest(
            "LMStudio does not support embeddings".to_string(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        let mut req_builder = self.client.get(&format!("{}/models", self.base_url));

        if let Some(auth) = self.get_auth_header() {
            req_builder = req_builder.header("Authorization", auth);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Request failed: {}", crate::error_utils::sanitize_error_message(&e.to_string()))))?;

        if !response.status().is_success() {
            let models = self
                .config
                .models
                .iter()
                .map(|model_id| Model {
                    id: model_id.clone(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "lmstudio".to_string(),
                    permission: None,
                    root: Some(model_id.clone()),
                    parent: None,
                })
                .collect();
            return Ok(models);
        }

        let response_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Internal(format!("Failed to parse response: {}", e)))?;

        let models: Vec<Model> = serde_json::from_value(response_data["data"].clone())
            .unwrap_or_else(|_| {
                self.config
                    .models
                    .iter()
                    .map(|model_id| Model {
                        id: model_id.clone(),
                        object: "model".to_string(),
                        created: chrono::Utc::now().timestamp(),
                        owned_by: "lmstudio".to_string(),
                        permission: None,
                        root: Some(model_id.clone()),
                        parent: None,
                    })
                    .collect()
            });

        Ok(models)
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "LMStudio".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec!["chat_completion".to_string(), "completion".to_string()],
            rate_limits: None,
            pricing: None,
            models: vec![],
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        let mut req_builder = self.client.get(&format!("{}/models", self.base_url));

        if let Some(auth) = self.get_auth_header() {
            req_builder = req_builder.header("Authorization", auth);
        }

        let response = req_builder
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
            supports_embeddings: false,
            max_context_length: Some(32768),
            max_tokens_per_request: Some(8192),
            supported_models: self.config.models.clone(),
        }
    }

    async fn get_cost_info(&self, _model: &str) -> Result<CostInfo, Self::Error> {
        Ok(CostInfo {
            input_cost_per_1k_tokens: 0.0,
            output_cost_per_1k_tokens: 0.0,
            currency: "USD".to_string(),
            model: _model.to_string(),
        })
    }

    async fn supports_model(&self, model: &str) -> bool {
        if self.config.models.is_empty() {
            true
        } else {
            self.config.models.contains(&model.to_string())
        }
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
