//! Common provider utilities and base classes

use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use gaussmeridian_core::{LLMProvider, ProviderError};

#[derive(Debug, Clone)]
pub struct BaseProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub timeout: Option<u64>,
    pub max_retries: Option<u32>,
    pub rate_limit: Option<u32>,
    pub models: Vec<String>,
}

impl BaseProviderConfig {
    pub fn new(name: String, api_key: String) -> Self {
        Self {
            name,
            api_key,
            base_url: None,
            timeout: None,
            max_retries: None,
            rate_limit: None,
            models: Vec::new(),
        }
    }

    /// Set the base URL for the provider
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// Set the timeout in milliseconds
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Set the rate limit
    pub fn with_rate_limit(mut self, rate_limit: u32) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Set the supported models
    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub burst_size: u32,
}

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderHealth {
    Healthy,
    Degraded {
        latency: Duration,
        error_rate: f64,
    },
    Unhealthy {
        last_error: String,
        consecutive_failures: u32,
    },
    Unknown,
}

/// Provider statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_latency: Duration,
    pub last_request_time: Option<chrono::DateTime<chrono::Utc>>,
    pub error_rate: f64,
    pub health_status: ProviderHealth,
}

/// Base provider implementation
pub struct BaseProvider {
    config: BaseProviderConfig,
    client: Client,
    stats: Arc<RwLock<ProviderStats>>,
    health_status: Arc<RwLock<ProviderHealth>>,
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl BaseProvider {
    pub fn new(
        config: BaseProviderConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::builder()
            .timeout(
                config
                    .timeout
                    .map(Duration::from_millis)
                    .unwrap_or(Duration::from_secs(30)),
            )
            .build()?;

        let stats = ProviderStats {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_latency: Duration::ZERO,
            last_request_time: None,
            error_rate: 0.0,
            health_status: ProviderHealth::Unknown,
        };

        let rate_limiter = if let Some(rate_limit) = &config.rate_limit {
            RateLimiter::new(*rate_limit, *rate_limit, *rate_limit)
        } else {
            RateLimiter::unlimited()
        };

        Ok(Self {
            config,
            client,
            stats: Arc::new(RwLock::new(stats)),
            health_status: Arc::new(RwLock::new(ProviderHealth::Unknown)),
            rate_limiter: Arc::new(RwLock::new(rate_limiter)),
        })
    }

    /// Get provider configuration
    pub fn config(&self) -> &BaseProviderConfig {
        &self.config
    }

    /// Get HTTP client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get provider statistics
    pub async fn get_stats(&self) -> ProviderStats {
        self.stats.read().await.clone()
    }

    /// Get health status
    pub async fn get_health(&self) -> ProviderHealth {
        self.health_status.read().await.clone()
    }

    /// Update health status
    pub async fn update_health(&self, health: ProviderHealth) {
        *self.health_status.write().await = health;
    }

    /// Record successful request
    pub async fn record_success(&self, latency: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.successful_requests += 1;
        stats.last_request_time = Some(chrono::Utc::now());

        // Update average latency
        let total_latency = stats.average_latency.as_nanos()
            * (stats.successful_requests - 1) as u128
            + latency.as_nanos();
        stats.average_latency =
            Duration::from_nanos((total_latency / stats.successful_requests as u128) as u64);

        stats.error_rate = stats.failed_requests as f64 / stats.total_requests as f64;
    }

    /// Record failed request
    pub async fn record_failure(&self, _error: &str) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.failed_requests += 1;
        stats.last_request_time = Some(chrono::Utc::now());
        stats.error_rate = stats.failed_requests as f64 / stats.total_requests as f64;
    }

    /// Check rate limit
    pub async fn check_rate_limit(
        &self,
        tokens: u32,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        self.rate_limiter
            .write()
            .await
            .check_rate_limit(tokens)
            .await
    }

    /// Perform health check
    pub async fn health_check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Default health check - ping the base URL
        let response = self
            .client
            .get(self.config.base_url.as_ref().ok_or("Base URL not set")?)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            self.update_health(ProviderHealth::Healthy).await;
            Ok(())
        } else {
            self.update_health(ProviderHealth::Unhealthy {
                last_error: format!("HTTP {}", response.status()),
                consecutive_failures: 1,
            })
            .await;
            Err("Health check failed".into())
        }
    }
}

/// Rate limiter implementation
pub struct RateLimiter {
    requests_per_minute: u32,
    tokens_per_minute: u32,
    burst_size: u32,
    request_tokens: Vec<(chrono::DateTime<chrono::Utc>, u32)>,
    unlimited: bool,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32, tokens_per_minute: u32, burst_size: u32) -> Self {
        Self {
            requests_per_minute,
            tokens_per_minute,
            burst_size,
            request_tokens: Vec::new(),
            unlimited: false,
        }
    }

    pub fn unlimited() -> Self {
        Self {
            requests_per_minute: 0,
            tokens_per_minute: 0,
            burst_size: 0,
            request_tokens: Vec::new(),
            unlimited: true,
        }
    }

    pub async fn check_rate_limit(
        &mut self,
        tokens: u32,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        if self.unlimited {
            return Ok(true);
        }

        let now = chrono::Utc::now();
        let window_start = now - chrono::Duration::minutes(1);

        // Clean up old entries
        self.request_tokens
            .retain(|(timestamp, _)| *timestamp > window_start);

        // Check request rate
        if self.request_tokens.len() >= self.requests_per_minute as usize {
            return Ok(false);
        }

        // Check token rate
        let total_tokens: u32 = self.request_tokens.iter().map(|(_, tokens)| tokens).sum();
        if total_tokens + tokens > self.tokens_per_minute {
            return Ok(false);
        }

        // Check burst size
        if tokens > self.burst_size {
            return Ok(false);
        }

        // Add current request
        self.request_tokens.push((now, tokens));
        Ok(true)
    }
}

/// Provider factory trait
#[async_trait]
pub trait ProviderFactory: Send + Sync {
    type Provider: LLMProvider;
    type Config: Clone + Send + Sync;

    async fn create_provider(
        &self,
        config: Self::Config,
    ) -> Result<Self::Provider, Box<dyn std::error::Error + Send + Sync>>;

    fn provider_name(&self) -> &'static str;

    fn supported_features(&self) -> Vec<&'static str>;
}

/// Provider registry for managing multiple providers
pub struct ProviderRegistry {
    providers:
        Arc<RwLock<HashMap<String, Arc<dyn LLMProvider<Error = ProviderError> + Send + Sync>>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a provider directly
    pub async fn register_provider(
        &self,
        name: String,
        provider: Arc<dyn LLMProvider<Error = ProviderError> + Send + Sync>,
    ) {
        self.providers.write().await.insert(name, provider);
    }

    /// Get a provider by name
    pub async fn get_provider(
        &self,
        name: &str,
    ) -> Option<Arc<dyn LLMProvider<Error = ProviderError> + Send + Sync>> {
        self.providers.read().await.get(name).cloned()
    }

    /// List all registered providers
    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Remove a provider
    pub async fn remove_provider(&self, name: &str) -> bool {
        self.providers.write().await.remove(name).is_some()
    }

    /// Get provider statistics
    pub async fn get_provider_stats(&self, _name: &str) -> Option<ProviderStats> {
        // This would need to be implemented by each provider
        None
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP request builder for providers
pub struct RequestBuilder {
    client: Client,
    base_url: String,
    headers: HashMap<String, String>,
}

impl RequestBuilder {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            headers: HashMap::new(),
        }
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn with_auth(mut self, token: String) -> Self {
        self.headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }

    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.headers
            .insert("Content-Type".to_string(), content_type);
        self
    }

    pub fn build_get(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.get(&url);

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        request
    }

    pub fn build_post(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.post(&url);

        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        request
    }
}

// Stream transformation utilities
pub fn transform_stream<S, F>(
    stream: S,
    transform: F,
) -> impl Stream<Item = Result<S::Item, ProviderError>>
where
    S: Stream + Unpin,
    S::Item: Clone,
    F: Fn(S::Item) -> Result<S::Item, ProviderError> + Send + Sync + 'static,
{
    use futures::StreamExt;
    stream.map(move |item| transform(item))
}

pub fn filter_stream<S, F>(stream: S, predicate: F) -> impl Stream<Item = S::Item>
where
    S: Stream + Unpin,
    F: Fn(&S::Item) -> bool + Send + Sync + 'static,
{
    use futures::StreamExt;
    stream.filter(move |item| std::future::ready(predicate(item)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::iter;

    #[tokio::test]
    async fn test_provider_registry() {
        let registry = ProviderRegistry::new();

        // Test empty registry
        assert_eq!(registry.list_providers().await.len(), 0);
        assert!(registry.get_provider("test").await.is_none());

        // Test removing non-existent provider
        assert!(!registry.remove_provider("test").await);
    }

    #[tokio::test]
    async fn test_stream_transformations() {
        use futures::StreamExt;
        // Use simple integers for testing - the transform_stream function requires Clone
        let stream = iter(vec![1i32, 2, 3]);
        let transformed = transform_stream(stream, |x| Ok(x));

        let results: Vec<_> = transformed.collect().await;
        assert_eq!(results.len(), 3);
        // Verify each result is Ok
        assert!(results.iter().all(|r| r.is_ok()));
    }
}

/// Error handling utilities
pub mod error {

    /// Convert provider errors to common error type
    pub fn convert_provider_error<E>(error: E) -> gaussmeridian_models::ProviderError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        gaussmeridian_models::ProviderError::Internal(error.to_string())
    }

    /// Handle HTTP errors
    pub fn handle_http_error(
        status: reqwest::StatusCode,
        body: String,
    ) -> gaussmeridian_models::ProviderError {
        match status.as_u16() {
            400..=499 => gaussmeridian_models::ProviderError::BadRequest(format!(
                "HTTP {}: {}",
                status, body
            )),
            500..=599 => {
                gaussmeridian_models::ProviderError::Internal(format!("HTTP {}: {}", status, body))
            }
            _ => gaussmeridian_models::ProviderError::Unavailable(format!(
                "HTTP {}: {}",
                status, body
            )),
        }
    }
}

/// Cost calculation utilities
pub mod cost {

    /// Calculate cost based on tokens and model
    pub fn calculate_cost(input_tokens: u32, output_tokens: u32, model: &str) -> f64 {
        // This would be implemented with actual pricing data
        let input_cost_per_1k = get_input_cost_per_1k(model);
        let output_cost_per_1k = get_output_cost_per_1k(model);

        let input_cost = (input_tokens as f64 / 1000.0) * input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * output_cost_per_1k;

        input_cost + output_cost
    }

    fn get_input_cost_per_1k(model: &str) -> f64 {
        match model {
            "gpt-4" => 0.03,
            "gpt-4-turbo" => 0.01,
            "gpt-3.5-turbo" => 0.0015,
            _ => 0.001,
        }
    }

    fn get_output_cost_per_1k(model: &str) -> f64 {
        match model {
            "gpt-4" => 0.06,
            "gpt-4-turbo" => 0.03,
            "gpt-3.5-turbo" => 0.002,
            _ => 0.002,
        }
    }
}
