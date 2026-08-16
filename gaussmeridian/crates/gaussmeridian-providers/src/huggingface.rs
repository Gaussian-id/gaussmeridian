//! HuggingFace provider implementation

use crate::common::BaseProviderConfig;
use futures::Stream;
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct HuggingFaceConfig {
    pub base_config: BaseProviderConfig,
    pub inference_endpoint: String,
    pub max_tokens: u32,
}

impl Default for HuggingFaceConfig {
    fn default() -> Self {
        Self {
            base_config: BaseProviderConfig::new("huggingface".to_string(), "".to_string()),
            inference_endpoint: "https://api-inference.huggingface.co".to_string(),
            max_tokens: 2048,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HuggingFaceProvider {
    config: HuggingFaceConfig,
    client: Client,
}

impl HuggingFaceProvider {
    pub fn new(config: HuggingFaceConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    async fn estimate_tokens(&self, text: &str) -> u32 {
        text.len() as u32 / 4
    }
}

#[async_trait::async_trait]
impl LLMProvider for HuggingFaceProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        // HuggingFace doesn't support chat completions in the same way
        Err(ProviderError::Internal(
            "Chat completions not supported".to_string(),
        ))
    }

    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, Self::Error> {
        let estimated_tokens = self.estimate_tokens(&request.prompt).await;

        let start_time = std::time::Instant::now();

        let hf_request = serde_json::json!({
            "inputs": request.prompt,
            "parameters": {
                "max_new_tokens": request.max_tokens.unwrap_or(self.config.max_tokens),
                "temperature": request.temperature.unwrap_or(1.0) as f64,
                "top_p": request.top_p.unwrap_or(1.0) as f64,
                "do_sample": true,
                "return_full_text": false
            }
        });

        let response = self
            .client
            .post(&format!(
                "{}/models/{}",
                self.config.inference_endpoint, request.model
            ))
            .header(
                "Authorization",
                &format!("Bearer {}", self.config.base_config.api_key),
            )
            .header("Content-Type", "application/json")
            .json(&hf_request)
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

        let generated_text = response_data[0]["generated_text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let choices = vec![CompletionChoice {
            text: generated_text,
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
            id: format!("hf_{}", chrono::Utc::now().timestamp()),
            object: "text_completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model,
            choices,
            usage: Some(usage),
        })
    }

    async fn chat_completion_stream(
        &self,
        _request: ChatCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        Err(ProviderError::Internal(
            "Streaming not supported".to_string(),
        ))
    }

    async fn completion_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>, Self::Error>
    {
        Err(ProviderError::Internal(
            "Streaming not supported".to_string(),
        ))
    }

    async fn embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, Self::Error> {
        let input_texts = match &request.input {
            EmbeddingInput::String(s) => vec![s.clone()],
            EmbeddingInput::Array(arr) => arr.clone(),
            EmbeddingInput::ArrayOfArrays(arrs) => {
                // For array of arrays (token IDs), convert to strings
                arrs.iter()
                    .map(|arr| {
                        // Convert token IDs to a placeholder string
                        format!("[{} tokens]", arr.len())
                    })
                    .collect()
            }
        };

        let estimated_tokens = input_texts.iter().map(|s| s.len() as u32 / 4).sum();

        let start_time = std::time::Instant::now();

        let hf_request = serde_json::json!({
            "inputs": input_texts
        });

        let response = self
            .client
            .post(&format!(
                "{}/models/{}",
                self.config.inference_endpoint, request.model
            ))
            .header(
                "Authorization",
                &format!("Bearer {}", self.config.base_config.api_key),
            )
            .header("Content-Type", "application/json")
            .json(&hf_request)
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

        let embeddings = response_data
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
            .map(|(index, embedding_array)| {
                let embedding_values = embedding_array
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

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        Ok(vec![
            Model {
                id: "gpt2".to_string(),
                object: "model".to_string(),
                created: 1556740080,
                owned_by: "huggingface".to_string(),
                permission: None,
                root: None,
                parent: None,
            },
            Model {
                id: "bert-base-uncased".to_string(),
                object: "model".to_string(),
                created: 1556740080,
                owned_by: "huggingface".to_string(),
                permission: None,
                root: None,
                parent: None,
            },
        ])
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "HuggingFace".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec!["completions".to_string(), "embeddings".to_string()],
            rate_limits: Some(RateLimits {
                requests_per_minute: Some(100),
                tokens_per_minute: Some(10000),
            }),
            pricing: None,
            models: vec![], // Empty for now since we don't have ModelInfo structs
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        let response = self
            .client
            .get(&format!("{}/models", self.config.inference_endpoint))
            .header(
                "Authorization",
                &format!("Bearer {}", self.config.base_config.api_key),
            )
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
            supports_streaming: false,
            supports_functions: false,
            supports_vision: false,
            supports_embeddings: true,
            max_context_length: Some(4096),
            max_tokens_per_request: Some(2048),
            supported_models: vec!["gpt2".to_string(), "bert-base-uncased".to_string()],
        }
    }

    async fn get_cost_info(&self, _model: &str) -> Result<CostInfo, Self::Error> {
        Ok(CostInfo {
            input_cost_per_1k_tokens: 0.0,
            output_cost_per_1k_tokens: 0.0,
            currency: "USD".to_string(),
            model: "huggingface".to_string(),
        })
    }

    async fn supports_model(&self, model: &str) -> bool {
        self.config.base_config.models.contains(&model.to_string())
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
