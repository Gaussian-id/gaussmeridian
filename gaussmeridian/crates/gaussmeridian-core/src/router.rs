//! Router implementations and main router logic

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use gaussmeridian_auth::AuthContext;
use gaussmeridian_cache::Cache;
#[cfg(feature = "db")]
use gaussmeridian_db::{
    request_repository::{RequestRepository, RequestRepositoryTrait},
    response_repository::{ResponseRepository, ResponseRepositoryTrait},
    tenant_repository::{TenantRepository, TenantRepositoryTrait},
    DatabaseClient,
};
use gaussmeridian_metrics::MainMetricsCollector;
use gaussmeridian_models::*;

use crate::{
    balancer::AdvancedLoadBalancer,
    cache::{CacheKey, CacheValue},
    circuit_breaker::CircuitBreaker,
    connection_pool::ConnectionPool,
    error::GaussMeridianError,
    provider_registry::{ProviderEntry, ProviderRegistry},
    rate_limiter::RateLimiter,
    request_batcher::RequestBatcher,
    types::{BalanceInfo, UsageInfo},
};

/// Resolve the ordered set of registered providers that both (a) actually serve
/// `model` and (b) advertise streaming support.
///
/// This mirrors the model-aware lookup already used elsewhere in this crate
/// (`EnterpriseGaussMeridian::get_model_info`, `ProviderRegistry::validate_model`,
/// `RequestBatcher::process_model_batch`) — each iterates providers and asks
/// `provider.supports_model(model)` rather than trying providers blind. Streaming
/// selection previously skipped this check entirely (see `route_chat_completion_stream`),
/// which let it hand a request to a provider that doesn't own the model at all.
///
/// That matters more for streaming than for the buffered path: several providers
/// (OpenAI, Anthropic) build their stream lazily via `EventSource::new()`, which
/// does not touch the network and therefore returns `Ok(stream)` unconditionally —
/// the wrong-provider 404 only surfaces later, as an error item inside the stream,
/// once it has already been handed back to the caller. By the time that happens the
/// fallback loop has already committed to this stream and cannot retry.
///
/// Returns:
/// - `Ok(candidates)` — non-empty, in registry-iteration order, every entry both
///   serves `model` and supports streaming. Callers should try them in order and can
///   safely fail over between them (all serve the same model).
/// - `Err(ModelNotFound)` — no registered provider serves `model` at all.
/// - `Err(ProviderError)` — at least one provider serves `model`, but none of them
///   support streaming (e.g. a model that only exists on an Ollama backend). This is
///   a real, distinct edge — surfaced clearly rather than silently misrouted.
async fn resolve_streaming_providers(
    model: &str,
    registry: &ProviderRegistry,
) -> Result<Vec<Arc<ProviderEntry>>, GaussMeridianError> {
    let mut owning = Vec::new();
    for entry in registry.all() {
        if entry.provider.supports_model(model).await {
            owning.push(entry);
        }
    }

    if owning.is_empty() {
        return Err(GaussMeridianError::ModelNotFound(model.to_string()));
    }

    let streaming: Vec<_> = owning
        .into_iter()
        .filter(|entry| entry.provider.capabilities().supports_streaming)
        .collect();

    if streaming.is_empty() {
        return Err(GaussMeridianError::ProviderError(format!(
            "Model '{}' is served by a registered provider but that provider does not support streaming",
            model
        )));
    }

    Ok(streaming)
}

#[async_trait]
pub trait Router: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn route_chat_completion(
        &self,
        request: ChatCompletionRequest,
        auth_context: AuthContext,
    ) -> Result<ChatCompletionResponse, Self::Error>;

    async fn route_chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
        auth_context: AuthContext,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    >;

    async fn route_completion(
        &self,
        request: CompletionRequest,
        auth_context: AuthContext,
    ) -> Result<CompletionResponse, Self::Error>;

    async fn route_completion_stream(
        &self,
        request: CompletionRequest,
        auth_context: AuthContext,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>, Self::Error>;

    async fn route_embedding(
        &self,
        request: EmbeddingRequest,
        auth_context: AuthContext,
    ) -> Result<EmbeddingResponse, Self::Error>;

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error>;

    async fn get_model_info(&self, model_id: &str) -> Result<ModelInfo, Self::Error>;

    async fn get_usage(&self, request_id: &str) -> Result<UsageInfo, Self::Error>;

    async fn get_balance(&self, auth_context: &AuthContext) -> Result<BalanceInfo, Self::Error>;
}

pub struct EnterpriseGaussMeridian {
    pub registry: Arc<ProviderRegistry>,
    pub cache: Arc<dyn Cache<CacheKey, CacheValue, Error = std::convert::Infallible>>,
    pub metrics: Option<Arc<MainMetricsCollector>>,
    pub load_balancer: Arc<dyn AdvancedLoadBalancer>,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub connection_pool: Arc<ConnectionPool>,
    pub rate_limiter: Arc<RateLimiter>,
    pub request_batcher: Arc<RequestBatcher>,
    pub shutdown_signal: Arc<tokio::sync::broadcast::Sender<()>>,
    /// Optional SurrealDB client for usage/billing persistence
    #[cfg(feature = "db")]
    pub db_client: Option<Arc<DatabaseClient>>,
}

impl EnterpriseGaussMeridian {
    pub fn new(
        cache: Arc<dyn Cache<CacheKey, CacheValue, Error = std::convert::Infallible>>,
        metrics: Option<Arc<MainMetricsCollector>>,
        load_balancer: Arc<dyn AdvancedLoadBalancer>,
        #[cfg(feature = "db")] db_client: Option<Arc<DatabaseClient>>,
    ) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(5, Duration::from_secs(30)));
        let connection_pool = Arc::new(ConnectionPool::new(100));
        let rate_limiter = Arc::new(RateLimiter::new());
        let registry = Arc::new(ProviderRegistry::new());
        let request_batcher = Arc::new(RequestBatcher::new(
            10,
            Duration::from_millis(100),
            registry.clone(),
        ));
        let (shutdown_signal, _) = tokio::sync::broadcast::channel(1);

        Self {
            registry,
            cache,
            metrics,
            load_balancer,
            circuit_breaker,
            connection_pool,
            rate_limiter,
            request_batcher,
            shutdown_signal: Arc::new(shutdown_signal),
            #[cfg(feature = "db")]
            db_client,
        }
    }

    pub async fn register_provider(
        &self,
        name: &str,
        provider: Arc<dyn crate::traits::LLMProvider<Error = gaussmeridian_models::ProviderError>>,
    ) -> Result<(), GaussMeridianError> {
        self.registry.register(name.to_string(), provider);
        Ok(())
    }

    pub async fn generate_cache_key(&self, request: &ChatCompletionRequest) -> CacheKey {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        // Serialize the request to JSON and hash the string
        let json_string =
            serde_json::to_string(request).unwrap_or_else(|_| format!("{:?}", request));
        json_string.hash(&mut hasher);

        CacheKey {
            provider: "default".to_string(),
            model: request.model.clone(),
            request_hash: format!("{:x}", hasher.finish()),
        }
    }

    fn record_metrics(&self, operation: &str, duration: std::time::Duration, success: bool) {
        if let Some(metrics) = &self.metrics {
            metrics.record_request(operation, duration, success);
        }
    }

    pub async fn route_with_fallback(
        &self,
        request: ChatCompletionRequest,
        _auth: AuthContext,
    ) -> Result<ChatCompletionResponse, GaussMeridianError> {
        let start = std::time::Instant::now();

        // Try primary provider
        let providers = self.registry.all();
        if let Some(primary) = providers.first() {
            match primary.provider.chat_completion(request.clone()).await {
                Ok(response) => {
                    self.record_metrics("chat_completion", start.elapsed(), true);
                    return Ok(response);
                }
                Err(e) => {
                    error!("Primary provider failed: {}", e);
                }
            }
        }

        // Try fallback providers
        for provider in providers.iter().skip(1) {
            match provider.provider.chat_completion(request.clone()).await {
                Ok(response) => {
                    self.record_metrics("chat_completion_fallback", start.elapsed(), true);
                    return Ok(response);
                }
                Err(e) => {
                    error!("Fallback provider failed: {}", e);
                }
            }
        }

        self.record_metrics("chat_completion", start.elapsed(), false);
        Err(GaussMeridianError::ProviderError(
            "All providers failed".to_string(),
        ))
    }

    pub async fn graceful_shutdown(&self) {
        info!("Starting graceful shutdown...");
        let _ = self.shutdown_signal.send(());

        // Wait for active requests to complete
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!("Graceful shutdown completed");
    }

    pub async fn health_check(&self) -> Result<(), GaussMeridianError> {
        let providers = self.registry.all();
        if providers.is_empty() {
            return Err(GaussMeridianError::ProviderError(
                "No providers registered".to_string(),
            ));
        }

        for provider in providers {
            if let Err(e) = provider.provider.health_check().await {
                error!("Provider health check failed: {}", e);
                return Err(GaussMeridianError::ProviderError(format!(
                    "Provider unhealthy: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    /// True when at least one registered provider answers its health check. `/ready` uses
    /// this — a fresh install configures several providers and usually one key, and requiring
    /// EVERY provider to be healthy made that first boot report itself unhealthy. `health_check`
    /// above is left untouched and keeps its ALL-must-pass semantics for callers that genuinely
    /// need every provider up; per-provider detail for debugging lives at `GET /health/providers`.
    pub async fn any_provider_callable(&self) -> bool {
        let providers = self.registry.all();
        if providers.is_empty() {
            return false;
        }
        for provider in providers {
            if provider.provider.health_check().await.is_ok() {
                return true;
            }
        }
        false
    }

    pub async fn route_with_enterprise_features(
        &self,
        request: ChatCompletionRequest,
        auth: AuthContext,
    ) -> Result<ChatCompletionResponse, GaussMeridianError> {
        let start = std::time::Instant::now();

        // Check rate limits
        self.rate_limiter
            .check_rate_limit("default", 100)
            .await
            .map_err(|_| GaussMeridianError::ProviderError("Rate limit exceeded".to_string()))?;

        // Acquire connection
        let _connection = self.connection_pool.acquire().await.map_err(|_| {
            GaussMeridianError::ProviderError("No available connections".to_string())
        })?;

        // Check cache first
        let cache_key = self.generate_cache_key(&request).await;
        if let Ok(Some(cached)) = self.cache.get(&cache_key).await {
            if let CacheValue::ChatCompletion(response) = cached {
                self.record_metrics("chat_completion_cached", start.elapsed(), true);
                return Ok(response);
            }
        }

        // Route with fallback
        let result = self
            .route_with_fallback(request.clone(), auth.clone())
            .await;

        // Track usage and cost for successful responses
        if let Ok(ref response) = result {
            // Cache successful responses
            let _ = self
                .cache
                .set(
                    cache_key,
                    CacheValue::ChatCompletion(response.clone()),
                    Some(Duration::from_secs(3600)),
                )
                .await;

            // Track usage (async, don't block response)
            let response_clone = response.clone();
            let model_id = request.model.clone();
            // Extract identifiers from auth context for per-key/tenant billing
            #[cfg(feature = "db")]
            let user_id = auth.user_id.clone();
            #[cfg(feature = "db")]
            let tenant_id = auth.tenant_id.clone();
            #[cfg(feature = "db")]
            let api_key_id = auth
                .metadata
                .get("api_key_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            // Capture optional DB client for persistence
            #[cfg(feature = "db")]
            let db_client = self.db_client.clone();
            tokio::spawn(async move {
                if let Some(usage) = &response_clone.usage {
                    // Persist request/response usage for analytics and billing when DB is available
                    #[cfg(feature = "db")]
                    if let Some(db) = db_client {
                        let request_repo = RequestRepository::new((*db).clone());
                        let response_repo = ResponseRepository::new((*db).clone());

                        let request_record = gaussmeridian_db::schema::Request {
                            id: None,
                            request_id: uuid::Uuid::new_v4().to_string(),
                            user_id,
                            api_key_id,
                            tenant_id,
                            model: model_id.clone(),
                            provider: "default".to_string(),
                            endpoint: "chat.completions".to_string(),
                            prompt_tokens: Some(usage.prompt_tokens as u32),
                            completion_tokens: Some(usage.completion_tokens as u32),
                            total_tokens: Some(usage.total_tokens as u32),
                            cost: None,
                            currency: None,
                            status: "success".to_string(),
                            error_message: None,
                            latency_ms: None,
                            created_at: chrono::Utc::now(),
                        };

                        if let Ok(request_db_id) = request_repo.create(request_record).await {
                            let response_record = gaussmeridian_db::schema::Response {
                                id: None,
                                request_id: request_db_id,
                                response_id: uuid::Uuid::new_v4().to_string(),
                                model: model_id.clone(),
                                provider: "default".to_string(),
                                prompt_tokens: usage.prompt_tokens as u32,
                                completion_tokens: usage.completion_tokens as u32,
                                total_tokens: usage.total_tokens as u32,
                                cost: 0.0,
                                currency: "USD".to_string(),
                                quality_score: None,
                                cached: false,
                                created_at: chrono::Utc::now(),
                            };
                            if let Err(e) = response_repo.create(response_record).await {
                                error!("Failed to persist response usage: {}", e);
                            }
                        }
                    } else {
                        info!(
                            "Usage tracked (in-memory only) - Request model: {}, Prompt tokens: {}, Completion tokens: {}, Total: {}",
                            model_id,
                            usage.prompt_tokens,
                            usage.completion_tokens,
                            usage.total_tokens
                        );
                    }

                    // Without the `db` feature there is no persistence path at all.
                    #[cfg(not(feature = "db"))]
                    info!(
                        "Usage tracked (in-memory only) - Request model: {}, Prompt tokens: {}, Completion tokens: {}, Total: {}",
                        model_id,
                        usage.prompt_tokens,
                        usage.completion_tokens,
                        usage.total_tokens
                    );
                }
            });
        }

        result
    }
}

#[async_trait]
impl Router for EnterpriseGaussMeridian {
    type Error = GaussMeridianError;

    async fn route_chat_completion(
        &self,
        request: ChatCompletionRequest,
        auth_context: AuthContext,
    ) -> Result<ChatCompletionResponse, Self::Error> {
        self.route_with_enterprise_features(request, auth_context)
            .await
    }

    async fn route_chat_completion_stream(
        &self,
        request: ChatCompletionRequest,
        auth_context: AuthContext,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, Self::Error>> + Send>>,
        Self::Error,
    > {
        let start = std::time::Instant::now();

        // Check rate limits
        self.rate_limiter
            .check_rate_limit(&auth_context.api_key, 100)
            .await
            .map_err(|_| GaussMeridianError::ProviderError("Rate limit exceeded".to_string()))?;

        // Acquire connection
        let _connection = self.connection_pool.acquire().await.map_err(|_| {
            GaussMeridianError::ProviderError("No available connections".to_string())
        })?;

        // Resolve only the provider(s) that actually serve `request.model` (and support
        // streaming) — see `resolve_streaming_providers` for why blind iteration over every
        // streaming-capable provider was the bug (nondeterministic 404s from a provider that
        // doesn't own the requested model).
        let candidates = resolve_streaming_providers(&request.model, &self.registry).await?;

        // Try model-owning candidates in order; a genuine failure still fails over, but only
        // across providers that actually serve this model — never a provider that doesn't.
        for entry in candidates {
            match entry.provider.chat_completion_stream(request.clone()).await {
                Ok(stream) => {
                    self.record_metrics("chat_completion_stream", start.elapsed(), true);

                    // Wrap the stream to convert errors
                    let wrapped_stream = stream.map(|result| {
                        result.map_err(|e| GaussMeridianError::ProviderError(e.to_string()))
                    });

                    return Ok(Box::pin(wrapped_stream));
                }
                Err(e) => {
                    error!(
                        "Streaming provider '{}' failed for model '{}': {}",
                        entry.name, request.model, e
                    );
                    continue;
                }
            }
        }

        self.record_metrics("chat_completion_stream", start.elapsed(), false);
        Err(GaussMeridianError::ProviderError(format!(
            "All streaming-capable providers for model '{}' failed",
            request.model
        )))
    }

    async fn route_completion(
        &self,
        request: CompletionRequest,
        auth_context: AuthContext,
    ) -> Result<CompletionResponse, Self::Error> {
        let start = std::time::Instant::now();

        // Check rate limits
        self.rate_limiter
            .check_rate_limit(&auth_context.api_key, 100)
            .await
            .map_err(|_| GaussMeridianError::ProviderError("Rate limit exceeded".to_string()))?;

        // Acquire connection
        let _connection = self.connection_pool.acquire().await.map_err(|_| {
            GaussMeridianError::ProviderError("No available connections".to_string())
        })?;

        // Route to providers with fallback
        let providers = self.registry.all();

        for entry in providers {
            match entry.provider.completion(request.clone()).await {
                Ok(response) => {
                    self.record_metrics("completion", start.elapsed(), true);

                    // Track usage if available
                    if let Some(ref usage) = response.usage {
                        let model_id = request.model.clone();
                        info!(
                            "Completion usage - Model: {}, Prompt tokens: {}, Completion tokens: {}",
                            model_id, usage.prompt_tokens, usage.completion_tokens
                        );
                    }

                    return Ok(response);
                }
                Err(e) => {
                    error!("Completion provider failed: {}", e);
                    continue;
                }
            }
        }

        self.record_metrics("completion", start.elapsed(), false);
        Err(GaussMeridianError::ProviderError(
            "All providers failed for completion".to_string(),
        ))
    }

    async fn route_completion_stream(
        &self,
        request: CompletionRequest,
        auth_context: AuthContext,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<CompletionChunk, Self::Error>> + Send>>, Self::Error>
    {
        let start = std::time::Instant::now();

        // Check rate limits
        self.rate_limiter
            .check_rate_limit(&auth_context.api_key, 100)
            .await
            .map_err(|_| GaussMeridianError::ProviderError("Rate limit exceeded".to_string()))?;

        // Get provider for streaming
        let providers = self.registry.all();

        for entry in providers {
            if !entry.provider.capabilities().supports_streaming {
                continue;
            }

            match entry.provider.completion_stream(request.clone()).await {
                Ok(stream) => {
                    self.record_metrics("completion_stream", start.elapsed(), true);

                    let wrapped_stream = stream.map(|result| {
                        result.map_err(|e| GaussMeridianError::ProviderError(e.to_string()))
                    });

                    return Ok(Box::pin(wrapped_stream));
                }
                Err(e) => {
                    error!("Completion streaming failed: {}", e);
                    continue;
                }
            }
        }

        self.record_metrics("completion_stream", start.elapsed(), false);
        Err(GaussMeridianError::ProviderError(
            "No provider available for completion streaming".to_string(),
        ))
    }

    async fn route_embedding(
        &self,
        request: EmbeddingRequest,
        auth_context: AuthContext,
    ) -> Result<EmbeddingResponse, Self::Error> {
        let start = std::time::Instant::now();

        // Check rate limits
        self.rate_limiter
            .check_rate_limit(&auth_context.api_key, 50)
            .await
            .map_err(|_| GaussMeridianError::ProviderError("Rate limit exceeded".to_string()))?;

        // Acquire connection
        let _connection = self.connection_pool.acquire().await.map_err(|_| {
            GaussMeridianError::ProviderError("No available connections".to_string())
        })?;

        // Route to providers with fallback
        let providers = self.registry.all();

        for entry in providers {
            if !entry.provider.capabilities().supports_embeddings {
                continue;
            }

            match entry.provider.embedding(request.clone()).await {
                Ok(response) => {
                    self.record_metrics("embedding", start.elapsed(), true);

                    // Track usage if available
                    if let Some(ref usage) = response.usage {
                        let model_id = request.model.clone();
                        info!(
                            "Embedding usage - Model: {}, Tokens: {}",
                            model_id, usage.total_tokens
                        );
                    }

                    return Ok(response);
                }
                Err(e) => {
                    error!("Embedding provider failed: {}", e);
                    continue;
                }
            }
        }

        self.record_metrics("embedding", start.elapsed(), false);
        Err(GaussMeridianError::ProviderError(
            "No provider available for embeddings".to_string(),
        ))
    }

    async fn list_models(&self) -> Result<Vec<Model>, Self::Error> {
        Ok(self.registry.all_models().await)
    }

    async fn get_model_info(&self, model_id: &str) -> Result<ModelInfo, Self::Error> {
        // Try to get model info from registry
        // Note: This requires access to model_registry which should be added to EnterpriseGaussMeridian
        // For now, try to get from provider registry
        let providers = self.registry.all();
        for entry in providers {
            if entry.provider.supports_model(model_id).await {
                // Get cost info
                let cost_info = entry
                    .provider
                    .get_cost_info(model_id)
                    .await
                    .unwrap_or_else(|_| CostInfo {
                        input_cost_per_1k_tokens: 0.0,
                        output_cost_per_1k_tokens: 0.0,
                        currency: "USD".to_string(),
                        model: model_id.to_string(),
                    });

                // Get capabilities
                let capabilities = entry.provider.capabilities();
                let model_capabilities = ModelCapabilities {
                    supports_streaming: capabilities.supports_streaming,
                    supports_functions: capabilities.supports_functions,
                    supports_vision: capabilities.supports_vision,
                    supports_embeddings: capabilities.supports_embeddings,
                };

                return Ok(ModelInfo {
                    id: model_id.to_string(),
                    name: model_id.to_string(),
                    context_length: capabilities.max_context_length.unwrap_or(4096),
                    pricing: cost_info,
                    capabilities: model_capabilities,
                });
            }
        }

        Err(GaussMeridianError::ModelNotFound(model_id.to_string()))
    }

    async fn get_usage(&self, request_id: &str) -> Result<UsageInfo, Self::Error> {
        // Prefer DB-backed usage lookup when SurrealDB is configured
        #[cfg(feature = "db")]
        if let Some(db) = &self.db_client {
            let request_repo = RequestRepository::new((**db).clone());
            if let Ok(Some(record)) = request_repo.get_by_request_id(request_id).await {
                // Map SurrealDB `Request` record into core `UsageInfo`
                let usage = UsageInfo {
                    request_id: record.request_id,
                    model: record.model,
                    provider: record.provider,
                    prompt_tokens: record.prompt_tokens.unwrap_or(0),
                    completion_tokens: record.completion_tokens.unwrap_or(0),
                    total_tokens: record.total_tokens.unwrap_or(0),
                    cost: record.cost.unwrap_or(0.0),
                    currency: record.currency.unwrap_or_else(|| "USD".to_string()),
                };
                return Ok(usage);
            }
        }

        Err(GaussMeridianError::ProviderError(format!(
            "Usage tracking for request {} not found",
            request_id
        )))
    }

    async fn get_balance(&self, auth_context: &AuthContext) -> Result<BalanceInfo, Self::Error> {
        use chrono::Utc;

        // Tenant balance is only ever DB-backed; without the `db` feature the context is unused.
        #[cfg(not(feature = "db"))]
        let _ = auth_context;

        // For authenticated tenants, pull balance from the SurrealDB `tenants` table
        #[cfg(feature = "db")]
        if let (Some(tenant_id), Some(db)) = (&auth_context.tenant_id, &self.db_client) {
            let tenant_repo = TenantRepository::new((**db).clone());
            if let Ok(Some(tenant)) = tenant_repo.get_by_id(tenant_id).await {
                return Ok(BalanceInfo {
                    balance: tenant.balance,
                    currency: tenant.currency,
                    last_updated: tenant.updated_at,
                });
            }
        }

        // Fallback: default zero balance when no tenant context or DB is configured
        Ok(BalanceInfo {
            balance: 0.0,
            currency: "USD".to_string(),
            last_updated: Utc::now(),
        })
    }
}

pub type GaussMeridian = EnterpriseGaussMeridian;

#[cfg(test)]
mod streaming_provider_resolution_tests {
    //! Regression coverage for the playground's intermittent streaming 404: prior to this
    //! fix, `route_chat_completion_stream` iterated every streaming-capable provider without
    //! checking which one actually serves `request.model`, so with openai + anthropic + google
    //! all registered and streaming-capable, it could hand a `gemini-2.5-flash` request to
    //! OpenAI or Anthropic — both build their stream lazily and only discover the wrong-model
    //! 404 once the stream is polled, by which point the router had already committed to it.
    //! `resolve_streaming_providers` must always resolve to the provider that owns the model.

    use super::*;
    use crate::traits::LLMProvider;
    use gaussmeridian_models::{
        ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, CompletionChunk,
        CompletionRequest, CompletionResponse, CostInfo, EmbeddingRequest, EmbeddingResponse,
        Model, ProviderCapabilities, ProviderConfig, ProviderError, ProviderMetadata,
    };

    /// Minimal `LLMProvider` stand-in. Only `supports_model` and `capabilities` are exercised
    /// by `resolve_streaming_providers`; every other method is unreachable from these tests.
    struct MockProvider {
        model_prefix: &'static str,
        supports_streaming: bool,
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        type Error = ProviderError;

        async fn chat_completion(
            &self,
            _request: ChatCompletionRequest,
        ) -> Result<ChatCompletionResponse, ProviderError> {
            unreachable!("not exercised by provider-resolution tests")
        }

        async fn chat_completion_stream(
            &self,
            _request: ChatCompletionRequest,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk, ProviderError>> + Send>>,
            ProviderError,
        > {
            unreachable!("not exercised by provider-resolution tests")
        }

        async fn completion(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse, ProviderError> {
            unreachable!("not exercised by provider-resolution tests")
        }

        async fn completion_stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<CompletionChunk, ProviderError>> + Send>>,
            ProviderError,
        > {
            unreachable!("not exercised by provider-resolution tests")
        }

        async fn embedding(
            &self,
            _request: EmbeddingRequest,
        ) -> Result<EmbeddingResponse, ProviderError> {
            unreachable!("not exercised by provider-resolution tests")
        }

        async fn list_models(&self) -> Result<Vec<Model>, ProviderError> {
            Ok(vec![])
        }

        fn metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                name: self.model_prefix.to_string(),
                version: "test".to_string(),
                supported_features: vec![],
                rate_limits: None,
                pricing: None,
                models: vec![],
            }
        }

        async fn health_check(&self) -> Result<(), ProviderError> {
            Ok(())
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                supports_streaming: self.supports_streaming,
                supports_functions: false,
                supports_vision: false,
                supports_embeddings: false,
                max_context_length: None,
                max_tokens_per_request: None,
                supported_models: vec![],
            }
        }

        async fn get_cost_info(&self, model: &str) -> Result<CostInfo, ProviderError> {
            Ok(CostInfo {
                input_cost_per_1k_tokens: 0.0,
                output_cost_per_1k_tokens: 0.0,
                currency: "USD".to_string(),
                model: model.to_string(),
            })
        }

        async fn supports_model(&self, model: &str) -> bool {
            model.starts_with(self.model_prefix)
        }

        fn get_config(&self) -> ProviderConfig {
            ProviderConfig {
                base_url: "http://mock.invalid".to_string(),
                api_key: None,
                timeout: 30,
                max_retries: 0,
                custom_headers: std::collections::HashMap::new(),
            }
        }
    }

    /// Registry with openai (`gpt-`), anthropic (`claude-`), and google (`gemini-`) — the
    /// exact three-provider shape that made the bug intermittent in production.
    fn registry_with_openai_anthropic_google() -> ProviderRegistry {
        let registry = ProviderRegistry::new();
        registry.register(
            "openai".to_string(),
            Arc::new(MockProvider {
                model_prefix: "gpt-",
                supports_streaming: true,
            }),
        );
        registry.register(
            "anthropic".to_string(),
            Arc::new(MockProvider {
                model_prefix: "claude-",
                supports_streaming: true,
            }),
        );
        registry.register(
            "google".to_string(),
            Arc::new(MockProvider {
                model_prefix: "gemini-",
                supports_streaming: true,
            }),
        );
        registry
    }

    #[tokio::test]
    async fn resolves_gemini_model_to_google_provider() {
        let registry = registry_with_openai_anthropic_google();
        let candidates = resolve_streaming_providers("gemini-2.5-flash", &registry)
            .await
            .expect("gemini-2.5-flash should resolve to a streaming provider");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "google");
    }

    #[tokio::test]
    async fn resolves_openai_model_to_openai_provider() {
        let registry = registry_with_openai_anthropic_google();
        let candidates = resolve_streaming_providers("gpt-4o-mini", &registry)
            .await
            .expect("gpt-4o-mini should resolve to a streaming provider");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "openai");
    }

    #[tokio::test]
    async fn resolution_is_deterministic_across_repeated_calls() {
        // Guards against the exact failure mode reported: the same request resolving to a
        // different (wrong) provider on different attempts.
        let registry = registry_with_openai_anthropic_google();
        for _ in 0..25 {
            let candidates = resolve_streaming_providers("gemini-2.5-flash", &registry)
                .await
                .expect("gemini-2.5-flash should resolve every time");
            assert_eq!(candidates[0].name, "google");
        }
    }

    #[tokio::test]
    async fn unknown_model_returns_model_not_found() {
        let registry = registry_with_openai_anthropic_google();
        // `ProviderEntry` (the `Ok` payload) doesn't implement `Debug`, so `expect_err`/
        // `unwrap_err` aren't available here — match explicitly instead.
        let err = match resolve_streaming_providers("totally-unknown-model", &registry).await {
            Err(e) => e,
            Ok(_) => panic!("no provider owns this model"),
        };
        assert!(matches!(err, GaussMeridianError::ModelNotFound(_)));
    }

    #[tokio::test]
    async fn model_owned_only_by_non_streaming_provider_returns_clear_error() {
        // e.g. a model that only exists behind an Ollama backend with streaming disabled —
        // must not silently misroute to some other provider that doesn't serve the model.
        let registry = ProviderRegistry::new();
        registry.register(
            "ollama".to_string(),
            Arc::new(MockProvider {
                model_prefix: "llama",
                supports_streaming: false,
            }),
        );

        let err = match resolve_streaming_providers("llama3", &registry).await {
            Err(e) => e,
            Ok(_) => panic!("model is owned but not streaming-capable"),
        };
        match err {
            GaussMeridianError::ProviderError(msg) => {
                assert!(msg.contains("llama3"));
                assert!(msg.contains("streaming"));
            }
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }
}
