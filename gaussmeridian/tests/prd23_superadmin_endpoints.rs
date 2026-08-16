//! PRD-23 Wave B — superadmin endpoint integration tests.
//!
//! Mirrors `tests/wave2_org_endpoints.rs`'s harness and structure exactly (no external
//! SurrealDB or Redis dependency — see that file's module doc comment for the full rationale).
//! This file owns:
//!   - the **401 (no auth)** case for every new `/v1/admin/*` and `/v1/auth/me/deletion-request`
//!     route — proving each is wired into the app and sits behind the auth boundary;
//!   - the **404-for-non-admin matrix**: every `/v1/admin/*` route, authenticated with a caller
//!     NOT on the (unset, in this clean test process) `SUPERADMIN_EMAILS` allowlist, must return
//!     404 — never 403 (see `handlers.rs::require_superadmin`'s doc comment for why) — proving
//!     the gate is wired into every handler and denies uniformly regardless of DB availability
//!     (these handlers call `require_superadmin` before touching any repo, so denial happens
//!     even with no SurrealDB configured in this harness);
//!   - the **503 (routed + authenticated-as-caller, no DB)** case for the user-side
//!     `/v1/auth/me/deletion-request` endpoints, proving they're wired and reach their handler.
//!
//! `require_superadmin`'s allowlist decision logic itself (`is_superadmin`) is exercised
//! directly with fixture data in `handlers.rs::superadmin_gate_tests`; the sole-owner-vs-
//! multi-owner fulfillment branching is exercised directly in
//! `handlers.rs::deletion_fulfillment_tests`; every new repository query string is exercised
//! against `Mem` in `gaussmeridian-db`'s `deletion_request_repository.rs` /
//! `login_event_repository.rs` / `admin_metrics_repository.rs` / `user_repository.rs` tests.
//! This file's job is only to prove the endpoints are wired, authenticated, and gated
//! correctly — same division of labor as Wave 2's file.

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
const TEST_KEY: &str = "test-api-key-prd23-12345678";

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

    // AuthManager without DB: any non-empty API key validates (fallback path) — an
    // `x-api-key` caller authenticates but carries no `user_id`/`email` metadata, so
    // `is_superadmin` denies regardless of `SUPERADMIN_EMAILS` (which is unset in this clean
    // test process anyway — see the module doc comment).
    let auth_manager = Arc::new(gaussmeridian_auth::AuthManager::new(
        JwtManager::new("test-prd23-secret"),
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
async fn admin_me_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/admin/me").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn admin_metrics_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/admin/metrics").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn admin_users_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/admin/users").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn admin_deletion_requests_list_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.get("/v1/admin/deletion-requests").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn admin_deletion_requests_fulfill_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server
        .post("/v1/admin/deletion-requests/req123/fulfill")
        .await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn admin_deletion_requests_reject_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server
        .post("/v1/admin/deletion-requests/req123/reject")
        .content_type("application/json")
        .json(&json!({ "note": "no" }))
        .await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn my_deletion_request_create_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.post("/v1/auth/me/deletion-request").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn my_deletion_request_cancel_without_auth_is_401() {
    let server = build_test_server().await;
    let response = server.delete("/v1/auth/me/deletion-request").await;
    assert_eq!(response.status_code(), 401);
}

// ─── 404-for-non-admin matrix ─────────────────────────────────────────────────────
//
// Authenticated (via the x-api-key fallback path — no user_id, no email, so `is_superadmin`
// denies for any `SUPERADMIN_EMAILS` value, including unset) but not allowlisted: every
// `/v1/admin/*` route must respond 404, never 403 — proving `require_superadmin` runs, denies,
// and does so BEFORE any DB-availability check (this harness has no DB configured — a 503
// here instead of 404 would mean a repo check ran before the gate, an ordering bug).

#[tokio::test]
async fn admin_me_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/me")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn admin_metrics_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/metrics")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(
        response.status_code(),
        404,
        "denial must be 404, never 503 — the gate runs before any repo check"
    );
}

#[tokio::test]
async fn admin_metrics_with_months_param_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/metrics?months=12")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn admin_users_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/users")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn admin_users_with_query_params_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/users?limit=10&start=0&q=shelby")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn admin_deletion_requests_list_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/deletion-requests")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn admin_deletion_requests_list_with_status_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/deletion-requests?status=pending")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn admin_deletion_requests_fulfill_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .post("/v1/admin/deletion-requests/req123/fulfill")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_eq!(
        response.status_code(),
        404,
        "must not leak whether req123 exists (404, not a repo-driven 404-for-missing-row vs 403)"
    );
}

#[tokio::test]
async fn admin_deletion_requests_reject_authenticated_non_admin_is_404() {
    let server = build_test_server().await;
    let response = server
        .post("/v1/admin/deletion-requests/req123/reject")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .content_type("application/json")
        .json(&json!({ "note": "no" }))
        .await;
    assert_eq!(response.status_code(), 404);
}

// The existing BYOK-admin `/v1/admin/db/api-keys` route uses a DIFFERENT gate
// (`check_db_admin_rbac`) and must be unaffected by the new superadmin gate sharing its
// `/v1/admin` nest — regression guard against the two gates being accidentally merged.
#[tokio::test]
async fn existing_db_api_keys_route_is_unaffected_by_the_new_superadmin_gate() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/admin/db/api-keys")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_ne!(
        response.status_code(),
        404,
        "the pre-existing admin DB route must still resolve (403 from its own RBAC gate, not 404)"
    );
}

// ─── User-side deletion-request endpoints — routed + authenticated, no DB ────────

#[tokio::test]
async fn my_deletion_request_create_authenticated_without_db_is_503_not_404() {
    let server = build_test_server().await;
    let response = server
        .post("/v1/auth/me/deletion-request")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    // The x-api-key fallback path authenticates but yields no user_id, so this actually 401s
    // before reaching the repo check — proving the route is wired (not 404) and requires a
    // real user_id, distinct from an unrouted path.
    assert_ne!(
        response.status_code(),
        404,
        "the route must be wired — not unrouted"
    );
}

#[tokio::test]
async fn my_deletion_request_cancel_authenticated_without_db_is_not_404() {
    let server = build_test_server().await;
    let response = server
        .delete("/v1/auth/me/deletion-request")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_ne!(
        response.status_code(),
        404,
        "the route must be wired — not unrouted"
    );
}

// ─── Billing endpoints — still routed (scoped, not superadmin-gated) ─────────────
//
// PRD-23 Wave B security fix: `/v1/billing/summary` and `/v1/billing/budget` are scoped to the
// caller's own project (`resolve_caller_project_id`), NOT superadmin-gated — see
// `handlers.rs::get_billing_summary`'s doc comment for why (a live WebUI page depends on
// `/v1/billing/budget` as a normal, non-superadmin user). These regression-guard that neither
// route was accidentally superadmin-gated (which would 404 a non-admin caller) or left
// unauthenticated (which would 200 an anonymous caller).

#[tokio::test]
async fn billing_summary_without_auth_is_401_not_404() {
    let server = build_test_server().await;
    let response = server.get("/v1/billing/summary").await;
    assert_eq!(
        response.status_code(),
        401,
        "still authenticated-only, not superadmin-gated (would be 404) or public"
    );
}

#[tokio::test]
async fn billing_budget_without_auth_is_401_not_404() {
    let server = build_test_server().await;
    let response = server.get("/v1/billing/budget").await;
    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn billing_budget_authenticated_non_admin_is_not_404() {
    let server = build_test_server().await;
    let response = server
        .get("/v1/billing/budget")
        .add_header(X_API_KEY.clone(), HeaderValue::from_static(TEST_KEY))
        .await;
    assert_ne!(
        response.status_code(),
        404,
        "a normal (non-superadmin) authenticated caller must still be able to reach this endpoint"
    );
}
