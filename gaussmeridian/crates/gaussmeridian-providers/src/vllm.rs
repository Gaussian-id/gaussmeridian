//! vLLM provider implementation
//!
//! vLLM provides an OpenAI-compatible API for running LLMs locally.
//! This provider implements the OpenAI API format for seamless integration.

use crate::cost::CostCalculator;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use gaussmeridian_core::LLMProvider;
use gaussmeridian_models::*;
use reqwest::Client;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{error, info, warn};

/// GPU resource information
#[derive(Debug, Clone)]
pub struct GPUResourceInfo {
    pub device_id: usize,
    pub memory_total_gb: f64,
    pub memory_used_gb: f64,
    pub memory_free_gb: f64,
    pub utilization_percent: f32,
}

/// Request queue entry
#[derive(Debug, Clone)]
struct QueuedRequest {
    request: ChatCompletionRequest,
    priority: u32,
    queued_at: std::time::Instant,
}

/// vLLM provider configuration
#[derive(Debug, Clone)]
pub struct VLLMConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout: std::time::Duration,
    pub max_retries: usize,
    pub models: Vec<String>,
    pub enable_streaming: bool,
    pub max_concurrent_requests: Option<usize>,
    /// Enable GPU resource management
    pub enable_gpu_management: bool,
    /// GPU device IDs to use (empty means use all available)
    pub gpu_device_ids: Vec<usize>,
    /// Enable request queuing
    pub enable_request_queue: bool,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Queue timeout
    pub queue_timeout: std::time::Duration,
}

impl Default for VLLMConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8000/v1".to_string(),
            api_key: None,
            timeout: std::time::Duration::from_secs(120), // Longer timeout for local models
            max_retries: 2,
            models: vec![],
            enable_streaming: true,
            max_concurrent_requests: Some(10),
            enable_gpu_management: false,
            gpu_device_ids: vec![],
            enable_request_queue: false,
            max_queue_size: 100,
            queue_timeout: std::time::Duration::from_secs(300),
        }
    }
}

/// vLLM provider implementation with native integration features
pub struct VLLMProvider {
    config: VLLMConfig,
    base_url: String,
    client: Client,
    /// Request queue for managing concurrent requests
    request_queue: Option<tokio::sync::mpsc::UnboundedSender<QueuedRequest>>,
    /// Current active request count
    active_requests: Arc<std::sync::atomic::AtomicUsize>,
    /// GPU resource information cache
    gpu_info: Arc<tokio::sync::RwLock<Vec<GPUResourceInfo>>>,
}

impl VLLMProvider {
    pub fn new(config: VLLMConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        let active_requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let gpu_info = Arc::new(tokio::sync::RwLock::new(Vec::new()));

        // Initialize request queue if enabled
        let request_queue = if config.enable_request_queue {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<QueuedRequest>();
            let client_clone = client.clone();
            let base_url_clone = config.base_url.clone();
            let active_requests_clone = active_requests.clone();
            let max_concurrent = config.max_concurrent_requests.unwrap_or(10);

            // Spawn queue processor
            tokio::spawn(async move {
                let mut processing = Vec::new();
                loop {
                    tokio::select! {
                        // Receive new request
                        request = rx.recv() => {
                            if let Some(queued) = request {
                                if processing.len() < max_concurrent {
                                    let client = client_clone.clone();
                                    let base_url = base_url_clone.clone();
                                    let active = active_requests_clone.clone();
                                    active.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                                    let handle = tokio::spawn(async move {
                                        // Process request
                                        let _ = Self::process_queued_request(
                                            &client,
                                            &base_url,
                                            queued.request,
                                        ).await;
                                        active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                                    });
                                    processing.push(handle);
                                } else {
                                    // Queue is full, drop request
                                    warn!("Request queue full, dropping request");
                                }
                            } else {
                                // Channel closed
                                break;
                            }
                        }
                        // Check for completed tasks
                        _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                            processing.retain(|handle| !handle.is_finished());
                        }
                    }
                }
            });

            Some(tx)
        } else {
            None
        };

        // Initialize GPU monitoring if enabled
        if config.enable_gpu_management {
            let gpu_info_clone = gpu_info.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    // Update GPU info (placeholder - would need actual GPU monitoring)
                    let mut info = gpu_info_clone.write().await;
                    *info = Self::get_gpu_info().await;
                }
            });
        }

        Self {
            base_url: config.base_url.clone(),
            config,
            client,
            request_queue,
            active_requests,
            gpu_info,
        }
    }

    /// Process a queued request with timeout and error handling
    async fn process_queued_request(
        client: &Client,
        base_url: &str,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, ProviderError> {
        // Security: Validate request size
        let request_size = serde_json::to_string(&request)
            .map(|s| s.len())
            .unwrap_or(0);
        if request_size > 10_000_000 {
            // 10MB limit
            return Err(ProviderError::BadRequest("Request too large".to_string()));
        }

        // Implementation would call the actual API
        // This is a placeholder - would implement full request processing here
        Err(ProviderError::Internal(
            "Queue processing not fully implemented".to_string(),
        ))
    }

    /// Get GPU resource information
    async fn get_gpu_info() -> Vec<GPUResourceInfo> {
        // Placeholder - would integrate with nvidia-smi or similar
        // For now, return empty vector
        vec![]
    }

    /// Get current GPU utilization
    pub async fn get_gpu_utilization(&self) -> Vec<GPUResourceInfo> {
        self.gpu_info.read().await.clone()
    }

    /// Check if GPU resources are available
    pub async fn check_gpu_availability(&self) -> bool {
        if !self.config.enable_gpu_management {
            return true; // GPU management disabled, assume available
        }

        let gpu_info = self.gpu_info.read().await;
        if gpu_info.is_empty() {
            return true; // No GPU info available, assume available
        }

        // Check if any GPU has free memory
        gpu_info.iter().any(|gpu| gpu.memory_free_gb > 1.0)
    }

    /// Get current active request count
    pub fn get_active_request_count(&self) -> usize {
        self.active_requests
            .load(std::sync::atomic::Ordering::SeqCst)
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
impl LLMProvider for VLLMProvider {
    type Error = ProviderError;

    async fn chat_completion(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        let estimated_tokens = self.estimate_message_tokens(&request.messages).await;
        let start_time = std::time::Instant::now();

        // vLLM uses OpenAI-compatible API
        let vllm_request = serde_json::json!({
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
            .json(&vllm_request)
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

        // Parse OpenAI-compatible response
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
                .unwrap_or(&format!("vllm_{}", chrono::Utc::now().timestamp()))
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

        let vllm_request = serde_json::json!({
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
            .json(&vllm_request)
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
                                id: json["id"].as_str().unwrap_or("vllm_stream").to_string(),
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

        let vllm_request = serde_json::json!({
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
            .json(&vllm_request)
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
                .unwrap_or(&format!("vllm_{}", chrono::Utc::now().timestamp()))
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

        let vllm_request = serde_json::json!({
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
            .json(&vllm_request)
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
                                id: json["id"].as_str().unwrap_or("vllm_stream").to_string(),
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
            "vLLM does not support embeddings".to_string(),
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
            // If models endpoint fails, return configured models
            let models = self
                .config
                .models
                .iter()
                .map(|model_id| Model {
                    id: model_id.clone(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "vllm".to_string(),
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
                // Fallback to configured models
                self.config
                    .models
                    .iter()
                    .map(|model_id| Model {
                        id: model_id.clone(),
                        object: "model".to_string(),
                        created: chrono::Utc::now().timestamp(),
                        owned_by: "vllm".to_string(),
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
            name: "vLLM".to_string(),
            version: "1.0.0".to_string(),
            supported_features: vec!["chat_completion".to_string(), "completion".to_string()],
            rate_limits: None,
            pricing: None,
            models: vec![],
        }
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        // Use models endpoint as health check
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
            max_context_length: Some(32768), // vLLM typically supports large contexts
            max_tokens_per_request: Some(8192),
            supported_models: self.config.models.clone(),
        }
    }

    async fn get_cost_info(&self, _model: &str) -> Result<CostInfo, Self::Error> {
        // vLLM is local, so cost is effectively zero
        Ok(CostInfo {
            input_cost_per_1k_tokens: 0.0,
            output_cost_per_1k_tokens: 0.0,
            currency: "USD".to_string(),
            model: _model.to_string(),
        })
    }

    async fn supports_model(&self, model: &str) -> bool {
        if self.config.models.is_empty() {
            true // If no models configured, assume all are supported
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
