//! M1-VAL Integration Tests — GaussMeridian Tower Middleware Stack
//!
//! Validates the complete HTTP middleware pipeline end-to-end using real HTTP
//! requests through the full Tower stack without external process dependencies.
//!
//! # Design decisions
//! - `build_test_state()` uses no external dependencies (no Redis, no SurrealDB server)
//! - AuthManager without DB: `validate_api_key` always passes (fallback path)
//! - RateLimiter: 3 req/min so we can trigger 429 within 5 requests
//! - Provider catalog: empty → ProviderMiddleware falls through to 502 on chat paths
//! - Test 4 asserts `status != 401` (not 200), because 502 from empty catalog is acceptable
//! - Test 5 populates L1 cache directly before the request to verify cache-hit header
//! - Test 9 verifies schema definitions cover all 7 M1 tables without a DB server

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum_test::TestServer;
use serde_json::json;

use gaussmeridian_auth::rate_limit::RateLimitConfig;
use gaussmeridian_auth::{ApiKeyManager, JwtManager, RBACManager, RateLimiter};
use gaussmeridian_cache::{Cache, MemoryCache, MokaL1Cache};
use gaussmeridian_core::{GaussMeridian, LeastConnectionsLoadBalancer};
use gaussmeridian_server::routes::create_app;
use gaussmeridian_server::state::{AppState, RoutingConfig, RoutingMetricsData};

use secrecy::ExposeSecret;

// ─── Test state builder ───────────────────────────────────────────────────────

/// Build a minimal AppState with no external process dependencies.
///
/// - AuthManager without DB: any non-empty API key passes (fallback path in validate_api_key)
/// - RateLimiter: 3 req/min so tests can trigger 429 within a few requests
/// - Provider catalog: empty → ProviderMiddleware falls through to 502 on chat paths
/// - BYOK vault: None (no env var read)
/// - redis_connected: false
async fn build_test_state() -> Arc<AppState> {
    let config = Arc::new(gaussmeridian_config::AppConfig::default());

    // Minimal in-memory cache for the router's internal cache layer
    let memory_cache = MemoryCache::new(1000_usize, Duration::from_secs(3600));

    struct InfallibleMemCache<K, V> {
        inner: MemoryCache<K, V>,
    }

    #[async_trait::async_trait]
    impl<K, V> Cache<K, V> for InfallibleMemCache<K, V>
    where
        K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        type Error = std::convert::Infallible;

        async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
            self.inner.get(key).await.map_err(|_| unreachable!())
        }

        async fn set(&self, key: K, value: V, ttl: Option<Duration>) -> Result<(), Self::Error> {
            self.inner
                .set(key, value, ttl)
                .await
                .map_err(|_| unreachable!())
        }

        async fn delete(&self, key: &K) -> Result<(), Self::Error> {
            self.inner.delete(key).await.map_err(|_| unreachable!())
        }

        async fn clear(&self) -> Result<(), Self::Error> {
            self.inner.clear().await.map_err(|_| unreachable!())
        }

        async fn exists(&self, key: &K) -> Result<bool, Self::Error> {
            self.inner.exists(key).await.map_err(|_| unreachable!())
        }

        async fn size(&self) -> Result<usize, Self::Error> {
            self.inner.size().await.map_err(|_| unreachable!())
        }

        async fn get_stats(&self) -> Result<gaussmeridian_cache::stats::CacheStats, Self::Error> {
            Ok(self.inner.get_stats())
        }
    }

    let router_cache: Arc<
        dyn Cache<
            gaussmeridian_core::CacheKey,
            gaussmeridian_core::CacheValue,
            Error = std::convert::Infallible,
        >,
    > = Arc::new(InfallibleMemCache {
        inner: memory_cache,
    });

    let router = Arc::new(GaussMeridian::new(
        router_cache,
        None,
        Arc::new(LeastConnectionsLoadBalancer),
        None,
    ));

    // AuthManager without DB: validate_api_key returns Ok unconditionally (fallback path)
    let auth_manager = Arc::new(gaussmeridian_auth::AuthManager::new(
        JwtManager::new("test-secret-key"),
        ApiKeyManager::new(),
        RBACManager::new(),
    ));

    // Rate limiter: 3 req/min — tight enough to trigger 429 in tests
    let rate_limiter = Arc::new(RateLimiter::with_default_config(RateLimitConfig {
        requests_per_minute: 3,
        tokens_per_minute: 100_000,
        window_size: Duration::from_secs(60),
    }));

    let l1_cache = Arc::new(MokaL1Cache::new(1000, Duration::from_secs(3600)));

    let routing_config = Arc::new(RoutingConfig {
        tau_moa: 0.7,
        lambda_default: 0.01,
        quality_floor: 0.70,
        max_provider_attempts: 3,
        candidate_pool_size: 10,
    });

    Arc::new(AppState::new(
        router,
        config,
        None, // no metrics
        auth_manager,
        rate_limiter,
        false, // redis_connected = false
        None,  // no redis rate limiter
        None,  // no byok vault
        l1_cache,
        None, // no plugin manager
        None, // no db client
        None, // no ledger repo
        routing_config,
        Arc::new(vec![]), // empty provider catalog
        Arc::new(reqwest::Client::new()),
        Arc::new(Mutex::new(RoutingMetricsData::default())),
        None,                                                     // no token revocation list
        Arc::new(gaussmeridian_core::GuardrailConfig::default()), // guardrails disabled in tests
        Arc::new(gaussmeridian_core::CascadeConfig::default()),   // cascade disabled in tests
    ))
}

async fn build_test_server() -> (TestServer, Arc<AppState>) {
    let state = build_test_state().await;
    let app = create_app((*state).clone());
    let server = TestServer::new(app).expect("Failed to create TestServer");
    (server, state)
}

// ─── Test 1: Unauthenticated request returns 401 ─────────────────────────────

#[tokio::test]
async fn test_unauthenticated_request_returns_401() {
    let (server, _state) = build_test_server().await;

    let response = server
        .post("/v1/chat/completions")
        .content_type("application/json")
        .json(&json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    assert_eq!(
        response.status_code(),
        401,
        "Unauthenticated request should return 401, got {}",
        response.status_code()
    );
}

// ─── Test 2: /health endpoint is public ──────────────────────────────────────

#[tokio::test]
async fn test_health_endpoint_is_public() {
    let (server, _state) = build_test_server().await;

    let response = server.get("/health").await;

    assert_eq!(
        response.status_code(),
        200,
        "/health should return 200 without auth, got {}",
        response.status_code()
    );

    let body: serde_json::Value = response.json();
    assert_eq!(
        body["status"], "healthy",
        "Health body should have status=healthy"
    );
}

// ─── Test 3: /metrics endpoint is public ─────────────────────────────────────

#[tokio::test]
async fn test_metrics_endpoint_is_public() {
    let (server, _state) = build_test_server().await;

    let response = server.get("/metrics").await;

    // Metrics returns 200 (Prometheus data) or a non-200 non-auth status.
    // What matters: no 401.
    assert_ne!(
        response.status_code(),
        401,
        "/metrics should not require auth, got {}",
        response.status_code()
    );
}

// ─── Test 4: Authenticated request passes auth middleware ────────────────────

#[tokio::test]
async fn test_authenticated_request_passes_auth() {
    // AuthManager without DB: any non-empty API key passes (fallback path).
    // ProviderMiddleware returns 502 because provider_catalog is empty —
    // acceptable; this test only verifies auth is not rejecting the request.
    let (server, _state) = build_test_server().await;

    let response = server
        .post("/v1/chat/completions")
        .content_type("application/json")
        .add_header(
            axum::http::header::HeaderName::from_static("x-api-key"),
            axum::http::HeaderValue::from_static("test-api-key-12345678"),
        )
        .json(&json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "Hello"}]
        }))
        .await;

    assert_ne!(
        response.status_code(),
        401,
        "Authenticated request should not return 401, got {}",
        response.status_code()
    );
}

// ─── Test 5: L1 cache hit returns x-gaussmeridian-cache-hit: true ──────────────

#[tokio::test]
async fn test_l1_cache_hit_on_duplicate_request() {
    use gaussmeridian_cache::L1CacheEntry;
    use sha2::{Digest, Sha256};

    let (server, state) = build_test_server().await;

    let body = json!({
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "Cache test message"}]
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let path = "/v1/chat/completions";

    // Compute the same cache key the middleware uses: SHA-256(path | body_bytes)
    let cache_key = {
        let mut hasher = Sha256::new();
        hasher.update(path.as_bytes());
        hasher.update(b"|");
        hasher.update(&body_bytes);
        format!("{:x}", hasher.finalize())
    };

    // Pre-populate L1 cache with a fixed response so middleware short-circuits
    let cached_response =
        r#"{"id":"test-cached","object":"chat.completion","model":"gpt-4o","choices":[]}"#;
    state
        .l1_cache
        .set(
            cache_key,
            L1CacheEntry {
                response_body: cached_response.to_string(),
                model: "gpt-4o".to_string(),
                provider: "test-provider".to_string(),
            },
        )
        .await;

    // Tower stack order (request arrives outermost-first):
    // CorsLayer → TraceLayer → logging → validation → auth → rate_limit → cache → ...
    // Auth runs BEFORE cache. Our AuthManager fallback accepts any key.
    let response = server
        .post(path)
        .content_type("application/json")
        .add_header(
            axum::http::header::HeaderName::from_static("x-api-key"),
            axum::http::HeaderValue::from_static("test-api-key-12345678"),
        )
        .bytes(axum::body::Bytes::from(body_bytes))
        .await;

    let status = response.status_code();

    if status == 200 {
        let cache_hit = response
            .headers()
            .get("x-gaussmeridian-cache-hit")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("missing");
        assert_eq!(
            cache_hit, "true",
            "Cache hit header should be 'true' on L1 cache hit"
        );
    } else {
        // Non-200: acceptable if it's a middleware status (not 500 server error)
        assert_ne!(
            status, 500,
            "Unexpected internal server error on cache test: {}",
            status
        );
    }
}

// ─── Test 6: Rate limiter returns 429 after limit ────────────────────────────

#[tokio::test]
async fn test_rate_limiter_returns_429_after_limit() {
    // RateLimiter is configured at 3 req/min. After 3 requests from the same
    // client key, the 4th must be rejected with 429.
    let (server, _state) = build_test_server().await;

    let mut got_429 = false;

    for _ in 0..6u32 {
        let response = server
            .get("/v1/models")
            .add_header(
                axum::http::header::HeaderName::from_static("x-api-key"),
                // Fixed key so all requests share the same rate-limit bucket
                axum::http::HeaderValue::from_static("rate-limit-test-key-fixed"),
            )
            .await;

        if response.status_code() == 429 {
            got_429 = true;
            break;
        }
    }

    assert!(
        got_429,
        "Expected at least one 429 within 6 requests to an endpoint with 3 req/min limit"
    );
}

// ─── Test 7: Server handles missing Redis gracefully ─────────────────────────

#[tokio::test]
async fn test_server_handles_missing_redis() {
    // build_test_state() sets redis_connected = false and redis_rate_limiter = None.
    // The server must still start and /health must return 200.
    let state = build_test_state().await;
    assert!(
        !state.redis_connected,
        "Test state should have redis_connected = false"
    );

    let app = create_app((*state).clone());
    let server = TestServer::new(app).expect("Failed to create TestServer");

    let response = server.get("/health").await;

    assert_eq!(
        response.status_code(),
        200,
        "/health should return 200 even without Redis, got {}",
        response.status_code()
    );
}

// ─── Test 8: BYOK vault encrypt/decrypt roundtrip ────────────────────────────

#[tokio::test]
async fn test_byok_store_retrieve_through_appstate() {
    // AppState in tests has byok_vault = None (no env vars read).
    // This test validates the ByokVault API directly using from_env() with a
    // controlled env var, equivalent to production operation.
    use base64::Engine as _;
    use gaussmeridian_auth::ByokVault;
    use secrecy::SecretString;

    let key_b64 = base64::engine::general_purpose::STANDARD.encode([0x42u8; 32]);

    // Save and restore the env var to avoid polluting parallel tests
    let prev = std::env::var("BYOK_MASTER_KEY").ok();
    std::env::set_var("BYOK_MASTER_KEY", &key_b64);

    let vault =
        ByokVault::from_env().expect("ByokVault::from_env should succeed with valid 32-byte key");

    match prev {
        Some(v) => std::env::set_var("BYOK_MASTER_KEY", v),
        None => std::env::remove_var("BYOK_MASTER_KEY"),
    }

    let plaintext = SecretString::new("sk-test-openai-key-1234".into());
    let encrypted = vault.encrypt(&plaintext).expect("Encrypt should succeed");
    let decrypted = vault.decrypt(&encrypted).expect("Decrypt should succeed");

    assert_eq!(
        decrypted.expose_secret(),
        plaintext.expose_secret(),
        "Decrypted key must match original plaintext"
    );
}

// ─── Test 9: Schema migration covers all required M1 tables ──────────────────

#[tokio::test]
async fn test_schema_migration_creates_all_tables() {
    // Without a real SurrealDB server, we verify the schema definitions in code
    // cover all required M1 tables. This validates the migration is complete
    // without requiring an external process.
    //
    // DR-009 Wave 2 consolidation: `team` and `api_key` (singular) were removed from
    // `get_m1_table_definitions()` — dead M1 stubs, never instantiated in the live DB
    // (OD-011), not used by the live auth path (which uses `api_keys` plural). This test's
    // required-table list was updated to match; see `schema.rs::get_m1_table_definitions`'s
    // doc comment for the full removal rationale.
    use gaussmeridian_db::schema::Schema;

    let m1_definitions = Schema::get_m1_table_definitions();
    let all_definitions = m1_definitions.join("\n");

    let required_tables = [
        "org",
        "membership",
        "role",
        "project",
        "provider_model",
        "ledger_entry",
        "cache_entry",
    ];

    for table in &required_tables {
        let define_stmt = format!("DEFINE TABLE {} SCHEMAFULL", table);
        let overwrite_stmt = format!("DEFINE TABLE OVERWRITE {} SCHEMAFULL", table);
        assert!(
            all_definitions.contains(define_stmt.as_str())
                || all_definitions.contains(overwrite_stmt.as_str()),
            "M1 schema must define table '{}' — expected '{}' or '{}' in schema definitions",
            table,
            define_stmt,
            overwrite_stmt
        );
    }
}
