//! PRD-25 Phase 1 — refresh-token endpoint + BUG-05 request-path integration tests.
//!
//! Clones `tests/prd21_wave_b_onboarding_endpoints.rs`'s no-external-deps harness (`build_test_
//! server`) — no SurrealDB or Redis required. The DB-backed rotation algorithm itself is proven
//! against a real `Mem` engine in `gaussmeridian-db`'s `refresh_token_repository` tests; this file
//! owns the HTTP-boundary contracts that need no DB:
//!   - `/v1/auth/refresh` is PUBLIC (reachable with no Authorization header) — proven by the
//!     no-DB 503 (it reached its handler, distinct from the middleware's own 401);
//!   - the `AuthResponse` wire contract omits `refresh_token` when absent, includes it when set;
//!   - **BUG-05 regression:** an EXPIRED access JWT on a protected route returns 401 with the
//!     `x-gr-token-expired` marker preserved through the auth middleware (the fix: `Expired` is
//!     no longer flattened to `Invalid`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::header::HeaderName;
use axum::http::HeaderValue;
use axum_test::TestServer;
use serde_json::json;

use gaussmeridian_auth::rate_limit::RateLimitConfig;
use gaussmeridian_auth::{ApiKeyManager, JwtManager, RBACManager, RateLimiter};
use gaussmeridian_cache::{Cache, MemoryCache, MokaL1Cache};
use gaussmeridian_core::{GaussMeridian, LeastConnectionsLoadBalancer};
use gaussmeridian_server::handlers::{AuthResponse, PublicUser};
use gaussmeridian_server::routes::create_app;
use gaussmeridian_server::state::{AppState, RoutingConfig, RoutingMetricsData};

const BUG05_JWT_SECRET: &str = "prd25-bug05-fixed-secret";

async fn build_test_state_with_secret(jwt_secret: &str) -> Arc<AppState> {
    let config = Arc::new(gaussmeridian_config::AppConfig::default());

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

    // AuthManager WITHOUT a DB (no refresh store) — but with a KNOWN JWT secret so this test can
    // mint a token the middleware's validator will accept-then-reject-as-expired. `with_access_ttl`
    // is exercised here too (the BUG-05 token overrides `exp` explicitly — see the test).
    let auth_manager = Arc::new(gaussmeridian_auth::AuthManager::new(
        JwtManager::new(jwt_secret).with_access_ttl(1),
        ApiKeyManager::new(),
        RBACManager::new(),
    ));

    let rate_limiter = Arc::new(RateLimiter::with_default_config(RateLimitConfig {
        requests_per_minute: 100,
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
        None,
        auth_manager,
        rate_limiter,
        false,
        None,
        None,
        l1_cache,
        None,
        None,
        None,
        routing_config,
        Arc::new(vec![]),
        Arc::new(reqwest::Client::new()),
        Arc::new(Mutex::new(RoutingMetricsData::default())),
        None,
        Arc::new(gaussmeridian_core::GuardrailConfig::default()),
        Arc::new(gaussmeridian_core::CascadeConfig::default()),
    ))
}

async fn build_test_server() -> TestServer {
    let state = build_test_state_with_secret(BUG05_JWT_SECRET).await;
    let app = create_app((*state).clone());
    TestServer::new(app).expect("Failed to create TestServer")
}

/// Mint a JWT with an explicit far-past `exp` so it reads as expired regardless of
/// jsonwebtoken's default 60 s validation leeway (a `ttl=1` + sleep-2s token would still fall
/// inside that leeway — see the report's deviation note). `create_token` honors an explicit
/// `exp` claim over the configured default TTL.
fn mint_expired_jwt(secret: &str, sub: &str) -> String {
    let jwt = JwtManager::new(secret).with_access_ttl(1);
    let past = (chrono::Utc::now().timestamp() - 3600) as u64;
    let mut claims = std::collections::HashMap::new();
    claims.insert("sub".to_string(), json!(sub));
    claims.insert("email".to_string(), json!("bug05@example.com"));
    claims.insert("exp".to_string(), json!(past));
    jwt.create_token(&claims).expect("mint expired jwt")
}

// ─── /v1/auth/refresh is PUBLIC and reaches its handler ──────────────────────────

#[tokio::test]
async fn refresh_is_reachable_without_auth_header() {
    let server = build_test_server().await;
    // No Authorization / x-api-key header at all. If the route were behind the auth boundary the
    // middleware would answer 401 "API key or Bearer token required" BEFORE the handler ran.
    let response = server
        .post("/v1/auth/refresh")
        .content_type("application/json")
        .json(&json!({ "refresh_token": "deadbeef".repeat(8) }))
        .await;
    // With no DB configured the handler returns 503 (infrastructure-unavailable) — crucially NOT
    // a 401, which proves the request passed the public-path check and reached the handler.
    assert_eq!(
        response.status_code(),
        503,
        "POST /v1/auth/refresh is public; with no DB it reaches the handler and returns 503, not a middleware 401"
    );
}

#[tokio::test]
async fn refresh_with_empty_body_is_unprocessable_not_a_middleware_401() {
    let server = build_test_server().await;
    // A malformed/empty JSON body fails the `Json<RefreshRequest>` extractor (422/400) — still
    // proves the route is public (the middleware never rejected it for missing credentials).
    let response = server.post("/v1/auth/refresh").await;
    assert_ne!(
        response.status_code(),
        401,
        "a public route must not 401 for missing credentials"
    );
}

// ─── AuthResponse wire contract (refresh_token presence) ─────────────────────────

fn sample_public_user() -> PublicUser {
    PublicUser {
        id: "user_1".to_string(),
        email: "u@example.com".to_string(),
        username: "u".to_string(),
        tenant_id: None,
        roles: vec!["user".to_string()],
        created_at: chrono::Utc::now(),
        active: true,
        onboarding_completed: false,
        full_name: None,
        display_name: None,
        company: None,
        timezone: None,
        deletion_requested: false,
    }
}

#[test]
fn auth_response_omits_refresh_token_when_absent() {
    let resp = AuthResponse {
        token: "access".to_string(),
        refresh_token: None,
        user: sample_public_user(),
    };
    let v = serde_json::to_value(&resp).expect("serialize");
    assert!(v.get("token").is_some(), "access token is always present");
    assert!(
        v.get("refresh_token").is_none(),
        "refresh_token must be omitted (skip_serializing_if) when None"
    );
}

#[test]
fn auth_response_includes_refresh_token_when_present() {
    let resp = AuthResponse {
        token: "access".to_string(),
        refresh_token: Some("r3fr3sh".to_string()),
        user: sample_public_user(),
    };
    let v = serde_json::to_value(&resp).expect("serialize");
    assert_eq!(
        v.get("refresh_token").and_then(|r| r.as_str()),
        Some("r3fr3sh"),
        "refresh_token must be present on the wire when Some"
    );
}

// ─── BUG-05 regression: expired access JWT → 401 + x-gr-token-expired marker ──────

#[tokio::test]
async fn expired_access_jwt_on_protected_route_is_401_with_expired_marker() {
    let server = build_test_server().await;
    let expired = mint_expired_jwt(BUG05_JWT_SECRET, "user_1");

    // A protected route (GET /v1/models sits behind the auth middleware).
    let response = server
        .get("/v1/models")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {expired}")).unwrap(),
        )
        .await;

    assert_eq!(
        response.status_code(),
        401,
        "an expired access JWT on a protected route must be 401"
    );
    // The fix: `validate_jwt_token` preserves `Expired` (was flattened to `Invalid`), so the
    // middleware can emit the refreshable-401 marker the BFF keys on.
    let marker = response.headers().get("x-gr-token-expired");
    assert_eq!(
        marker.and_then(|v| v.to_str().ok()),
        Some("1"),
        "the x-gr-token-expired marker must survive through the auth middleware"
    );
}

#[tokio::test]
async fn invalid_signature_jwt_is_401_without_the_expired_marker() {
    let server = build_test_server().await;
    // A token signed with the WRONG secret is Invalid (not Expired) — 401, but NO marker (it is
    // not refreshable). Proves the marker is specific to expiry, not blanket-applied to any 401.
    let wrong = mint_expired_jwt("some-other-secret", "user_1");
    let response = server
        .get("/v1/models")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {wrong}")).unwrap(),
        )
        .await;
    assert_eq!(response.status_code(), 401);
    assert!(
        response.headers().get("x-gr-token-expired").is_none(),
        "an invalid-signature token is not refreshable — no expired marker"
    );
}
