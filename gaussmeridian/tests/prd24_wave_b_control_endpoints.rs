//! PRD-24 Wave B1 — superadmin CONTROL endpoint integration tests.
//!
//! Mirrors `tests/prd23_superadmin_endpoints.rs`'s harness and structure exactly (no external
//! SurrealDB or Redis dependency). This file owns, for every new `/v1/admin/*` control / dry-run
//! / audit route:
//!   - the **401 (no auth)** case — proving each route is wired into the app and sits behind the
//!     auth boundary;
//!   - the **404-for-non-admin matrix** — authenticated via the `x-api-key` fallback (no email,
//!     so `is_superadmin` denies for any `SUPERADMIN_EMAILS`, including unset in this clean test
//!     process), every control route must return **404, never 403 and never 503** (see
//!     `handlers.rs::require_superadmin`) — proving the gate runs BEFORE any repo/DB check.
//!
//! The state-transition behavior itself (a suspend flips `status` + writes an `admin_action` row,
//! a reactivate clears the lock, a dry-run reads without mutating) is exercised against `Mem` in
//! `gaussmeridian-db`'s repo tests (`org_repository.rs` / `project_repository.rs` /
//! `api_key_repository.rs` `set_status`/`set_active` tests, `admin_action_repository.rs` insert +
//! list, `admin_observability_repository.rs` `recent_activity`) — the same division of labor as
//! PRD-23's endpoint file, since this harness has no DB and control mutations need one.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::header::HeaderName;
use axum::http::HeaderValue;
use axum_test::TestServer;

use gaussmeridian_auth::rate_limit::RateLimitConfig;
use gaussmeridian_auth::{ApiKeyManager, JwtManager, RBACManager, RateLimiter};
use gaussmeridian_cache::{Cache, MemoryCache, MokaL1Cache};
use gaussmeridian_core::{GaussMeridian, LeastConnectionsLoadBalancer};
use gaussmeridian_server::routes::create_app;
use gaussmeridian_server::state::{AppState, RoutingConfig, RoutingMetricsData};

const X_API_KEY: HeaderName = HeaderName::from_static("x-api-key");
const TEST_KEY: &str = "test-api-key-prd24-controls-12345678";

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

    // AuthManager without DB: any non-empty API key validates (fallback path) — an `x-api-key`
    // caller authenticates but carries no `user_id`/`email` metadata, so `is_superadmin` denies
    // regardless of `SUPERADMIN_EMAILS` (unset in this clean test process anyway).
    let auth_manager = Arc::new(gaussmeridian_auth::AuthManager::new(
        JwtManager::new("test-prd24-secret"),
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

/// Every new control / dry-run / audit route, with its HTTP method, for table-driven coverage.
fn control_routes() -> Vec<(&'static str, &'static str)> {
    vec![
        ("POST", "/v1/admin/orgs/acme/lock"),
        ("POST", "/v1/admin/orgs/acme/suspend"),
        ("POST", "/v1/admin/orgs/acme/reactivate"),
        ("GET", "/v1/admin/orgs/acme/impact"),
        ("POST", "/v1/admin/projects/p1/lock"),
        ("POST", "/v1/admin/projects/p1/suspend"),
        ("POST", "/v1/admin/projects/p1/reactivate"),
        ("GET", "/v1/admin/projects/p1/impact"),
        ("POST", "/v1/admin/users/u1/suspend"),
        ("POST", "/v1/admin/users/u1/reactivate"),
        ("POST", "/v1/admin/keys/k1/suspend"),
        ("POST", "/v1/admin/keys/k1/reactivate"),
        ("GET", "/v1/admin/audit"),
    ]
}

async fn send(server: &TestServer, method: &str, path: &str, admin_header: bool) -> u16 {
    let response = if admin_header {
        match method {
            "POST" => {
                server
                    .post(path)
                    .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
                    .await
            }
            _ => {
                server
                    .get(path)
                    .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
                    .await
            }
        }
    } else {
        match method {
            "POST" => server.post(path).await,
            _ => server.get(path).await,
        }
    };
    response.status_code().as_u16()
}

// ─── 401: unauthenticated requests are rejected at the auth boundary ─────────────

#[tokio::test]
async fn every_control_route_without_auth_is_401() {
    let server = build_test_server().await;
    for (method, path) in control_routes() {
        let status = send(&server, method, path, false).await;
        assert_eq!(
            status, 401,
            "{method} {path} without auth must be 401 (wired, behind auth boundary)"
        );
    }
}

// ─── 404-for-non-admin matrix ─────────────────────────────────────────────────────
//
// Authenticated (x-api-key fallback — no user_id/email, so `is_superadmin` denies) but not
// allowlisted: every control route must respond 404 — never 403, and never 503 (a 503 would mean
// a repo/DB check ran before the gate; this harness has no DB, so 404 proves the gate runs first).

#[tokio::test]
async fn every_control_route_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    for (method, path) in control_routes() {
        let status = send(&server, method, path, true).await;
        assert_eq!(
            status, 404,
            "{method} {path} as an authenticated non-admin must be 404 — the gate runs before any repo/DB check"
        );
    }
}

// ─── Wiring regression: the pre-existing read-side observability routes still resolve ─────────
//
// Guards against the new control routes (all sharing the `/orgs/:id` and `/projects/:id` param
// nodes) accidentally shadowing the Wave A read routes when they were added to the same nest.

#[tokio::test]
async fn wave_a_read_routes_are_unaffected_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    for path in [
        "/v1/admin/orgs/acme",
        "/v1/admin/projects/p1",
        "/v1/admin/overview",
        "/v1/admin/watchlist",
    ] {
        let status = send(&server, "GET", path, true).await;
        assert_eq!(
            status, 404,
            "GET {path} (Wave A) must still route + gate to 404, not 405/unrouted"
        );
    }
}
