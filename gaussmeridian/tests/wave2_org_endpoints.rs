//! Wave 2 / DR-009 — Org / Project / Member / Role endpoint integration tests.
//!
//! Endpoint-level coverage through the **real** Tower stack (`create_app`), mirroring the
//! `tests/m1_integration.rs` / `tests/m3_integration.rs` harness (no external SurrealDB or
//! Redis dependency). This file owns the **401 (no auth)** case for the new `/v1/orgs`,
//! `/v1/orgs/:id/...`, and `/v1/roles` routes — proving the routes are wired into the app
//! and sit behind the auth boundary.
//!
//! The 403 (wrong role) and 200 (allowed) cases require real org/membership/role rows, which
//! this harness deliberately does not provision (same no-external-deps constraint as
//! m1/m3). Those are covered directly against the RBAC decision function
//! (`require_org_permission_with`) with in-memory fakes in
//! `gaussmeridian_server::handlers::org_rbac_tests` — see that module's doc comment.
//!
//! Note on `build_test_state`: `AppState::new` (the hand constructor these tests use)
//! defaults `org_repo`/`membership_repo`/`role_repo` to `None` — matching a server booted
//! without SurrealDB. So authenticated requests to these endpoints return 503
//! (SERVICE_UNAVAILABLE), which is exactly the documented `state.<repo>.ok_or(503)` behavior
//! and still proves the route + auth wiring. Unauthenticated requests are rejected at the
//! auth boundary with 401 before any repo is touched.

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
use gaussmeridian_server::routes::create_app;
use gaussmeridian_server::state::{AppState, RoutingConfig, RoutingMetricsData};

const X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");
const TEST_KEY: &str = "test-api-key-wave2-12345678";

async fn build_test_state() -> Arc<AppState> {
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

    // AuthManager without DB: any non-empty API key validates (fallback path) — same as the
    // m1/m3 harnesses. So `x-api-key` calls authenticate but carry no `user_id`.
    let auth_manager = Arc::new(gaussmeridian_auth::AuthManager::new(
        JwtManager::new("test-wave2-secret"),
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
    let state = build_test_state().await;
    let app = create_app((*state).clone());
    TestServer::new(app).expect("Failed to create TestServer")
}

// ─── 401: unauthenticated requests are rejected at the auth boundary ─────────────

#[tokio::test]
async fn list_orgs_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/orgs").await;
    assert_eq!(
        response.status_code(),
        401,
        "GET /v1/orgs without auth must be 401"
    );
}

#[tokio::test]
async fn create_org_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server
        .post("/v1/orgs")
        .content_type("application/json")
        .json(&json!({ "name": "Acme", "slug": "acme" }))
        .await;
    assert_eq!(
        response.status_code(),
        401,
        "POST /v1/orgs without auth must be 401"
    );
}

#[tokio::test]
async fn get_org_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/orgs/org123").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn delete_org_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.delete("/v1/orgs/org123").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn list_projects_for_org_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/orgs/org123/projects").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn invite_member_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server
        .post("/v1/orgs/org123/members")
        .content_type("application/json")
        .json(&json!({ "email": "x@example.com", "role_id": "role_1" }))
        .await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn get_roles_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/roles").await;
    assert_eq!(
        response.status_code(),
        401,
        "GET /v1/roles without auth must be 401"
    );
}

// ─── Routes are wired: authenticated-but-no-DB requests reach the handler (503) ──
//
// Proves each route is nested into the app and passes the auth boundary — with no
// SurrealDB configured (org/role repos = None), the handler returns 503, NOT 404 (which is
// what an unrouted path would give). This is the documented `.ok_or(SERVICE_UNAVAILABLE)`
// behavior; it confirms wiring without needing a live DB.

#[tokio::test]
async fn get_roles_authenticated_without_db_is_503_not_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/roles")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(
        response.status_code(),
        503,
        "GET /v1/roles is routed + authenticated; with no DB it must be 503 (not 404)"
    );
}

#[tokio::test]
async fn list_orgs_authenticated_without_user_id_is_401() {
    // The API-key fallback path authenticates but produces no `user_id`. `list_orgs`
    // requires a `user_id` (org membership is per-user), so it rejects with 401 — distinct
    // from the unrouted-404 and the no-DB-503 cases.
    let server = build_test_server().await;
    let response = server
        .get("/v1/orgs")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(
        response.status_code(),
        401,
        "list_orgs needs a user_id; an x-api-key caller without one is 401"
    );
}

// ─── DR-012 / DR-011 — the `list_tenants` admin endpoint is removed ──────────────
//
// `GET /v1/admin/db/tenants` was deleted (tenants hard-retired). The global auth layer wraps
// the whole `/v1` tree, so an UNAUTHENTICATED request to any `/v1/...` path (routed or not)
// returns 401 before routing resolves — meaning 404-vs-401 is only observable AFTER passing
// auth. So these tests authenticate (X_API_KEY), then: the removed path resolves to no route
// → 404; the sibling `/v1/admin/db/api-keys` still resolves to its handler (which then rejects
// the non-admin test key with 403) → NOT 404. Together they prove a surgical route removal.

#[tokio::test]
async fn list_tenants_route_is_gone_404_when_authenticated() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/db/tenants")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(
        response.status_code(),
        404,
        "GET /v1/admin/db/tenants must be unrouted (404) past auth — the endpoint was removed"
    );
}

#[tokio::test]
async fn list_api_keys_admin_route_still_exists_not_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/db/api-keys")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_ne!(
        response.status_code(),
        404,
        "GET /v1/admin/db/api-keys is still routed — must not be 404 (the test key gets 403)"
    );
}
