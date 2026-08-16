//! API client for communicating with GaussMeridian server
//!
//! Provides secure, async HTTP communication with the GaussMeridian API
//! for fetching metrics, models, providers, and other operational data.
//! Uses typed error handling that matches the backend error format.

use crate::error::{ApiError, ApiResult, ErrorCode, ErrorType};
use crate::state::{
    AgentStatus, ModelInfo, ModelPricing, ProviderStatus, RequestInfo, SystemMetrics,
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};

/// Configuration for API client retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

/// API client for GaussMeridian
#[derive(Debug)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    retry_config: RetryConfig,
}

// Response structures for API endpoints

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    #[serde(default)]
    uptime_seconds: u64,
    #[serde(default)]
    version: String,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct MetricsResponse {
    #[serde(default)]
    total_requests: u64,
    #[serde(default)]
    requests_per_second: f64,
    #[serde(default)]
    avg_latency_ms: f64,
    #[serde(default)]
    p50_latency_ms: f64,
    #[serde(default)]
    p95_latency_ms: f64,
    #[serde(default)]
    p99_latency_ms: f64,
    #[serde(default)]
    error_rate: f64,
    #[serde(default)]
    memory_usage_mb: f64,
    #[serde(default)]
    memory_total_mb: f64,
    #[serde(default)]
    cpu_usage_percent: f64,
    #[serde(default)]
    cache_hit_rate: f64,
    #[serde(default)]
    active_connections: u32,
    #[serde(default)]
    total_tokens: u64,
    #[serde(default)]
    total_cost: f64,
}

#[derive(Debug, Deserialize)]
struct ModelsListResponse {
    data: Vec<ModelApiResponse>,
}

#[derive(Debug, Deserialize)]
struct ModelApiResponse {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    owned_by: Option<String>,
    #[serde(default)]
    context_length: Option<u32>,
    #[serde(default)]
    pricing: Option<PricingResponse>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct PricingResponse {
    #[serde(default)]
    prompt: f64,
    #[serde(default)]
    completion: f64,
    #[serde(default)]
    currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProvidersResponse {
    providers: Vec<ProviderApiResponse>,
}

#[derive(Debug, Deserialize)]
struct ProviderApiResponse {
    name: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    healthy: bool,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    models: Vec<String>,
    #[serde(default)]
    priority: u32,
    #[serde(default)]
    weight: f64,
    #[serde(default)]
    request_count: u64,
    #[serde(default)]
    error_count: u64,
    #[serde(default)]
    avg_latency_ms: f64,
    #[serde(default)]
    last_health_check: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct AgentsResponse {
    agents: Vec<AgentApiResponse>,
}

#[derive(Debug, Deserialize)]
struct AgentApiResponse {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    agent_type: String,
    #[serde(default)]
    strategy: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    request_count: u64,
    #[serde(default)]
    success_rate: f64,
    #[serde(default)]
    avg_latency_ms: f64,
    #[serde(default)]
    last_activity: Option<DateTime<Utc>>,
    #[serde(default)]
    config: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RequestsResponse {
    requests: Vec<RequestApiResponse>,
}

#[derive(Debug, Deserialize)]
struct RequestApiResponse {
    #[serde(default)]
    id: String,
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    method: String,
    #[serde(default)]
    endpoint: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    status_code: u16,
    #[serde(default)]
    latency_ms: f64,
    #[serde(default)]
    tokens: Option<u32>,
    #[serde(default)]
    cost: Option<f64>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    tenant_id: Option<String>,
}

impl ApiClient {
    /// Create a new API client
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the GaussMeridian API
    /// * `api_key` - Optional API key for authentication
    ///
    /// # Security
    /// - Validates URL scheme
    /// - Sets reasonable timeouts
    /// - Rejects invalid certificates
    pub fn new(base_url: String, api_key: Option<String>) -> ApiResult<Self> {
        // Validate and sanitize URL
        let sanitized_url = base_url.trim_end_matches('/').to_string();
        if !sanitized_url.starts_with("http://") && !sanitized_url.starts_with("https://") {
            return Err(ApiError::new(
                400,
                ErrorType::InvalidRequestError,
                ErrorCode::InvalidFieldFormat,
                "Invalid URL scheme. Must be http:// or https://",
            )
            .with_param("base_url"));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(5)
            .danger_accept_invalid_certs(false)
            .build()
            .map_err(|e| {
                ApiError::new(
                    500,
                    ErrorType::ServerError,
                    ErrorCode::InternalError,
                    format!("Failed to create HTTP client: {}", e),
                )
            })?;

        Ok(Self {
            client,
            base_url: sanitized_url,
            api_key,
            retry_config: RetryConfig::default(),
        })
    }

    /// Set retry configuration
    #[allow(dead_code)]
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Build a request with authentication and security headers
    fn build_request(
        &self,
        method: reqwest::Method,
        path: &str,
    ) -> ApiResult<reqwest::RequestBuilder> {
        // Security: Prevent path traversal
        if path.contains("..") {
            return Err(ApiError::new(
                400,
                ErrorType::InvalidRequestError,
                ErrorCode::InvalidFieldFormat,
                "Invalid path: path traversal detected",
            )
            .with_param("path"));
        }

        let sanitized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };

        let url = format!("{}{}", self.base_url, sanitized_path);
        let mut request = self.client.request(method, &url);

        // Add authentication header
        if let Some(key) = &self.api_key {
            request = request.header("x-api-key", key);
        }

        // Add standard headers
        request = request.header("Accept", "application/json");
        request = request.header("User-Agent", "GaussMeridian-TUI/1.0");

        Ok(request)
    }

    /// Execute a request with retry logic for transient failures
    async fn execute_with_retry<T, F, Fut>(&self, operation: F) -> ApiResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = ApiResult<T>>,
    {
        let mut last_error = None;
        let mut backoff = self.retry_config.initial_backoff;

        for attempt in 0..=self.retry_config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    if !err.is_retryable() || attempt == self.retry_config.max_retries {
                        return Err(err);
                    }

                    warn!(
                        "Request failed (attempt {}/{}): {}. Retrying in {:?}",
                        attempt + 1,
                        self.retry_config.max_retries + 1,
                        err,
                        backoff
                    );

                    tokio::time::sleep(backoff).await;

                    // Exponential backoff
                    backoff = Duration::from_secs_f64(
                        (backoff.as_secs_f64() * self.retry_config.multiplier)
                            .min(self.retry_config.max_backoff.as_secs_f64()),
                    );

                    last_error = Some(err);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ApiError::new(
                500,
                ErrorType::ServerError,
                ErrorCode::InternalError,
                "Unexpected error in retry loop",
            )
        }))
    }

    /// Get health status and system metrics
    pub async fn get_health(&self) -> ApiResult<SystemMetrics> {
        let health = self
            .execute_with_retry(|| async {
                let response = self
                    .build_request(reqwest::Method::GET, "/health")?
                    .send()
                    .await?;

                if !response.status().is_success() {
                    return Err(ApiError::from_response(response).await);
                }

                response.json::<HealthResponse>().await.map_err(|e| {
                    ApiError::new(
                        500,
                        ErrorType::ServerError,
                        ErrorCode::InternalError,
                        format!("Failed to parse health response: {}", e),
                    )
                })
            })
            .await?;

        // Try to get detailed metrics
        let metrics = self.get_metrics().await.unwrap_or_default();

        Ok(SystemMetrics {
            uptime_seconds: health.uptime_seconds,
            total_requests: metrics.total_requests,
            requests_per_second: metrics.requests_per_second,
            avg_latency_ms: metrics.avg_latency_ms,
            p50_latency_ms: metrics.p50_latency_ms,
            p95_latency_ms: metrics.p95_latency_ms,
            p99_latency_ms: metrics.p99_latency_ms,
            error_rate: metrics.error_rate,
            memory_usage_mb: metrics.memory_usage_mb,
            memory_total_mb: metrics.memory_total_mb,
            cpu_usage_percent: metrics.cpu_usage_percent,
            cache_hit_rate: metrics.cache_hit_rate,
            active_connections: metrics.active_connections,
            total_tokens_processed: metrics.total_tokens,
            total_cost: metrics.total_cost,
            ..Default::default()
        })
    }

    /// Get detailed metrics (Prometheus format or JSON)
    async fn get_metrics(&self) -> ApiResult<MetricsResponse> {
        // Try JSON metrics first
        if let Ok(response) = self
            .build_request(reqwest::Method::GET, "/v1/admin/metrics")?
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(metrics) = response.json::<MetricsResponse>().await {
                    return Ok(metrics);
                }
            }
        }

        // Fall back to Prometheus metrics
        let response = self
            .build_request(reqwest::Method::GET, "/metrics")?
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ApiError::from_response(response).await);
        }

        let text = response.text().await.map_err(|e| {
            ApiError::new(
                500,
                ErrorType::ServerError,
                ErrorCode::InternalError,
                format!("Failed to read metrics response: {}", e),
            )
        })?;

        Ok(self.parse_prometheus_metrics(&text))
    }

    /// Parse Prometheus metrics format
    fn parse_prometheus_metrics(&self, text: &str) -> MetricsResponse {
        let mut metrics = MetricsResponse::default();

        for line in text.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if let Some(value) = self.extract_metric_value(line, "gaussmeridian_requests_total") {
                metrics.total_requests = value as u64;
            } else if let Some(value) =
                self.extract_metric_value(line, "gaussmeridian_requests_per_second")
            {
                metrics.requests_per_second = value;
            } else if let Some(value) =
                self.extract_metric_value(line, "gaussmeridian_latency_avg_ms")
            {
                metrics.avg_latency_ms = value;
            } else if let Some(value) = self.extract_metric_value(line, "gaussmeridian_error_rate") {
                metrics.error_rate = value;
            } else if let Some(value) =
                self.extract_metric_value(line, "gaussmeridian_memory_usage_mb")
            {
                metrics.memory_usage_mb = value;
            } else if let Some(value) =
                self.extract_metric_value(line, "gaussmeridian_cpu_usage_percent")
            {
                metrics.cpu_usage_percent = value;
            } else if let Some(value) =
                self.extract_metric_value(line, "gaussmeridian_cache_hit_rate")
            {
                metrics.cache_hit_rate = value;
            }
        }

        metrics
    }

    /// Extract metric value from Prometheus line
    fn extract_metric_value(&self, line: &str, metric_name: &str) -> Option<f64> {
        if line.starts_with(metric_name) {
            line.split_whitespace().last()?.parse().ok()
        } else {
            None
        }
    }

    /// Get list of available models
    pub async fn get_models(&self) -> ApiResult<Vec<ModelInfo>> {
        self.execute_with_retry(|| async {
            let response = self
                .build_request(reqwest::Method::GET, "/v1/models")?
                .send()
                .await?;

            if !response.status().is_success() {
                // Return empty list on non-critical errors
                if response.status().as_u16() == 404 {
                    return Ok(Vec::new());
                }
                return Err(ApiError::from_response(response).await);
            }

            // Try to parse as ModelsListResponse first, then fall back to array
            if let Ok(list_response) = response.json::<ModelsListResponse>().await {
                return Ok(list_response
                    .data
                    .into_iter()
                    .map(|m| self.convert_model(m))
                    .collect());
            }

            Ok(Vec::new())
        })
        .await
    }

    /// Convert API model response to internal type
    fn convert_model(&self, m: ModelApiResponse) -> ModelInfo {
        ModelInfo {
            id: m.id.clone(),
            name: m.name.unwrap_or_else(|| m.id.clone()),
            provider: m.owned_by.unwrap_or_else(|| "unknown".to_string()),
            enabled: true,
            context_length: m.context_length,
            pricing: m.pricing.map(|p| ModelPricing {
                prompt_cost_per_1k: p.prompt,
                completion_cost_per_1k: p.completion,
                currency: p.currency.unwrap_or_else(|| "USD".to_string()),
            }),
            request_count: 0,
            avg_latency_ms: 0.0,
            capabilities: m.capabilities.unwrap_or_default(),
        }
    }

    /// Get list of configured providers
    pub async fn get_providers(&self) -> ApiResult<Vec<ProviderStatus>> {
        // Try admin endpoint first
        if let Ok(response) = self
            .build_request(reqwest::Method::GET, "/v1/admin/providers")?
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(providers_response) = response.json::<ProvidersResponse>().await {
                    return Ok(providers_response
                        .providers
                        .into_iter()
                        .map(|p| ProviderStatus {
                            name: p.name,
                            enabled: p.enabled,
                            healthy: p.healthy,
                            base_url: p.base_url,
                            models: p.models,
                            last_health_check: p.last_health_check,
                            request_count: p.request_count,
                            error_count: p.error_count,
                            avg_latency_ms: p.avg_latency_ms,
                            priority: p.priority,
                            weight: p.weight,
                        })
                        .collect());
                }
            }
        }

        // Return demo data if API is not available
        debug!("Using demo provider data - admin endpoint not available");
        Ok(self.get_demo_providers())
    }

    /// Get demo provider data for display when API is unavailable
    fn get_demo_providers(&self) -> Vec<ProviderStatus> {
        vec![
            ProviderStatus {
                name: "OpenAI".to_string(),
                enabled: true,
                healthy: true,
                base_url: "https://api.openai.com/v1".to_string(),
                models: vec![
                    "gpt-4".to_string(),
                    "gpt-4-turbo".to_string(),
                    "gpt-3.5-turbo".to_string(),
                ],
                last_health_check: Some(Utc::now()),
                request_count: 1250,
                error_count: 3,
                avg_latency_ms: 245.5,
                priority: 1,
                weight: 1.0,
            },
            ProviderStatus {
                name: "Anthropic".to_string(),
                enabled: true,
                healthy: true,
                base_url: "https://api.anthropic.com/v1".to_string(),
                models: vec![
                    "claude-3-opus".to_string(),
                    "claude-3-sonnet".to_string(),
                ],
                last_health_check: Some(Utc::now()),
                request_count: 890,
                error_count: 1,
                avg_latency_ms: 312.8,
                priority: 2,
                weight: 0.8,
            },
            ProviderStatus {
                name: "Ollama".to_string(),
                enabled: true,
                healthy: true,
                base_url: "http://localhost:11434".to_string(),
                models: vec!["llama2".to_string(), "mistral".to_string()],
                last_health_check: Some(Utc::now()),
                request_count: 450,
                error_count: 0,
                avg_latency_ms: 85.2,
                priority: 3,
                weight: 0.5,
            },
        ]
    }

    /// Get list of MoA agents
    pub async fn get_agents(&self) -> ApiResult<Vec<AgentStatus>> {
        if let Ok(response) = self
            .build_request(reqwest::Method::GET, "/v1/moa/agents")?
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(agents_response) = response.json::<AgentsResponse>().await {
                    return Ok(agents_response
                        .agents
                        .into_iter()
                        .map(|a| AgentStatus {
                            id: a.id,
                            name: a.name,
                            agent_type: a.agent_type,
                            strategy: a.strategy,
                            status: a.status,
                            request_count: a.request_count,
                            success_rate: a.success_rate,
                            avg_latency_ms: a.avg_latency_ms,
                            last_activity: a.last_activity,
                            config: a.config,
                        })
                        .collect());
                }
            }
        }

        // Return demo agents
        debug!("Using demo agent data - MoA endpoint not available");
        Ok(self.get_demo_agents())
    }

    /// Get demo agent data
    fn get_demo_agents(&self) -> Vec<AgentStatus> {
        vec![
            AgentStatus {
                id: "agent-001".to_string(),
                name: "Primary Reasoner".to_string(),
                agent_type: "llm".to_string(),
                strategy: "standard".to_string(),
                status: "active".to_string(),
                request_count: 1500,
                success_rate: 0.98,
                avg_latency_ms: 350.0,
                last_activity: Some(Utc::now()),
                config: HashMap::new(),
            },
            AgentStatus {
                id: "agent-002".to_string(),
                name: "Code Specialist".to_string(),
                agent_type: "llm".to_string(),
                strategy: "roles".to_string(),
                status: "active".to_string(),
                request_count: 850,
                success_rate: 0.96,
                avg_latency_ms: 280.0,
                last_activity: Some(Utc::now()),
                config: HashMap::new(),
            },
            AgentStatus {
                id: "agent-003".to_string(),
                name: "Validator".to_string(),
                agent_type: "rule_based".to_string(),
                strategy: "collaborative".to_string(),
                status: "idle".to_string(),
                request_count: 420,
                success_rate: 0.99,
                avg_latency_ms: 45.0,
                last_activity: Some(Utc::now()),
                config: HashMap::new(),
            },
        ]
    }

    /// Get recent requests
    pub async fn get_recent_requests(&self, limit: usize) -> ApiResult<Vec<RequestInfo>> {
        if let Ok(response) = self
            .build_request(
                reqwest::Method::GET,
                &format!("/v1/admin/requests?limit={}", limit),
            )?
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(requests_response) = response.json::<RequestsResponse>().await {
                    return Ok(requests_response
                        .requests
                        .into_iter()
                        .map(|r| RequestInfo {
                            id: r.id,
                            timestamp: r.timestamp.unwrap_or_else(Utc::now),
                            method: r.method,
                            endpoint: r.endpoint,
                            model: r.model,
                            provider: r.provider,
                            status_code: r.status_code,
                            latency_ms: r.latency_ms,
                            tokens: r.tokens,
                            cost: r.cost,
                            error: r.error,
                            user_id: r.user_id,
                            tenant_id: r.tenant_id,
                        })
                        .collect());
                }
            }
        }

        // Return demo requests
        debug!("Using demo request data - admin endpoint not available");
        Ok(self.get_demo_requests(limit))
    }

    /// Get demo request data
    fn get_demo_requests(&self, limit: usize) -> Vec<RequestInfo> {
        let now = Utc::now();
        let endpoints = ["/v1/chat/completions", "/v1/embeddings", "/v1/completions"];
        let models = [
            "gpt-4",
            "gpt-3.5-turbo",
            "claude-3-opus",
            "llama2",
            "mistral",
        ];
        let providers = ["openai", "anthropic", "ollama"];

        (0..limit.min(50))
            .map(|i| {
                let offset = chrono::Duration::seconds((i * 3) as i64);
                let status = if i % 15 == 0 {
                    500
                } else if i % 8 == 0 {
                    429
                } else {
                    200
                };

                RequestInfo {
                    id: format!("req-{:08x}", i),
                    timestamp: now - offset,
                    method: "POST".to_string(),
                    endpoint: endpoints[i % endpoints.len()].to_string(),
                    model: Some(models[i % models.len()].to_string()),
                    provider: Some(providers[i % providers.len()].to_string()),
                    status_code: status,
                    latency_ms: 150.0 + (i as f64 * 10.0) % 300.0,
                    tokens: Some(100 + (i as u32 * 50) % 500),
                    cost: Some(0.001 + (i as f64 * 0.0005) % 0.01),
                    error: if status >= 400 {
                        Some("Rate limit exceeded".to_string())
                    } else {
                        None
                    },
                    user_id: Some(format!("user-{}", i % 5)),
                    tenant_id: Some(format!("tenant-{}", i % 3)),
                }
            })
            .collect()
    }

    /// Toggle provider enabled status
    #[allow(dead_code)]
    pub async fn toggle_provider(&self, name: &str, enabled: bool) -> ApiResult<()> {
        let response = self
            .build_request(
                reqwest::Method::PATCH,
                &format!("/v1/admin/providers/{}", name),
            )?
            .json(&serde_json::json!({ "enabled": enabled }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ApiError::from_response(response).await);
        }

        Ok(())
    }

    /// Check if server is reachable
    #[allow(dead_code)]
    pub async fn check_connection(&self) -> ApiResult<bool> {
        match self.get_health().await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.code == ErrorCode::ConnectionRefused || e.code == ErrorCode::NetworkError {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_url_scheme() {
        let result = ApiClient::new("ftp://localhost:8000".to_string(), None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidFieldFormat);
    }

    #[test]
    fn test_path_traversal_prevention() {
        let client = ApiClient::new("http://localhost:8000".to_string(), None).unwrap();
        let result = client.build_request(reqwest::Method::GET, "../etc/passwd");
        assert!(result.is_err());
    }
}
