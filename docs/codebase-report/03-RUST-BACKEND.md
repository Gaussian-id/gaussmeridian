# 03 — Rust Backend Deep Dive

## Workspace Layout

The Rust code lives under `gaussmeridian/` as a Cargo workspace with 12 members:

| Crate | Type | Purpose |
|-------|------|---------|
| `gaussmeridian-core` | lib | Router engine, provider registry, load balancing, circuit breakers |
| `gaussmeridian-providers` | lib | OpenAI, Anthropic, Ollama implementations |
| `gaussmeridian-models` | lib | Shared API types (ChatCompletion, Embedding, etc.) |
| `gaussmeridian-cache` | lib | In-memory (Moka) + optional Redis caching |
| `gaussmeridian-auth` | lib | JWT, API keys, RBAC, rate limiting |
| `gaussmeridian-metrics` | lib | Prometheus metrics collection |
| `gaussmeridian-plugins` | lib | Dynamic plugin loading (optional feature) |
| `gaussmeridian-config` | lib | Typed configuration + env/file loading |
| `gaussmeridian-utils` | lib | Validation and security helpers |
| `gaussmeridian-db` | lib | SurrealDB client, schema, repositories |
| `gaussmeridian-moa` | lib | Mixture of Agents orchestration |
| `services/server` | bin | **Main binary** — HTTP server |
| `services/tui` | bin | Terminal UI monitoring dashboard |

## Key Dependencies

| Category | Crate(s) | Version |
|----------|----------|---------|
| HTTP server | `axum` 0.7, `tower` 0.4, `tower-http` 0.5, `hyper` 1.0 | |
| HTTP client | `reqwest` 0.11, `reqwest-eventsource` 0.4 | |
| Async | `tokio` 1.35 (full), `futures` 0.3, `async-stream` 0.3 | |
| Serialization | `serde` 1.0, `serde_json` 1.0 | |
| Database | `surrealdb` 2.0 (kv-mem, kv-rocksdb, protocol-ws, protocol-http) | |
| Cache | `moka` 0.12, `redis` 0.24, `deadpool-redis` 0.14 | |
| Auth | `jsonwebtoken` 9.2, `sha2` 0.10, `hmac` 0.12 | |
| Metrics | `prometheus` 0.13, `metrics` 0.22 | |
| Config | `config` 0.13, `dotenvy` 0.15, `clap` 4.4 | |
| TUI | `ratatui` 0.26, `crossterm` 0.27 | |
| Concurrency | `dashmap` 5.5, `parking_lot` 0.12, `rayon` 1.8 | |

---

## Application Bootstrap (`services/server/src/app.rs`)

The server starts via `main.rs` → `run_server()` → `create_application()`:

```
1.  Load AppConfig from env/files
2.  Create in-memory cache (Moka — pre-M1 MemoryCache, now MokaL1Cache)
3.  Initialize metrics collector (if feature enabled)
4.  Connect to SurrealDB (if GAUSSMERIDIAN_DB_* env vars set)
    - Run schema initialization + M1 additive migrations
5.  Build EnterpriseGaussMeridian (core router) with config, cache, metrics, load balancer, optional DB
6.  Register LLM providers from config.providers
7.  Create AuthManager with JWT secret + optional DB
8.  [M1] Initialize shared RateLimiter (in-process sliding window)
9.  [M1] Initialize ByokVault from BYOK_MASTER_KEY — non-fatal if absent
10. [M1] Initialize MokaL1Cache with capacity + TTL from config.cache
11. [M1] Probe Redis at REDIS_URL — store redis_connected: bool in AppState
12. Assemble AppState and pass to route builder
```

### AppState fields (post-M1)

| Field | Type | Purpose |
|-------|------|---------|
| `router` | `Arc<EnterpriseGaussMeridian>` | Core router engine |
| `config` | `Arc<AppConfig>` | Application config |
| `metrics` | `Option<Arc<MainMetricsCollector>>` | Prometheus metrics |
| `auth_manager` | `Arc<AuthManager>` | JWT + API key + RBAC |
| `rate_limiter` | `Arc<RateLimiter>` | Shared in-process sliding window |
| `redis_connected` | `bool` | Redis probe result at startup |
| `byok_vault` | `Option<Arc<ByokVault>>` | AES-256-GCM per-org key encryption |
| `l1_cache` | `Arc<MokaL1Cache>` | Moka exact-match L1 cache |
| `plugin_manager` | `Option<Arc<PluginManager>>` | Plugin system |
| `db_client` | `Option<Arc<DatabaseClient>>` | SurrealDB client |

---

## HTTP API Endpoints

All endpoints are defined in `services/server/src/routes.rs`:

### Authentication

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| POST | `/v1/auth/register` | `register_user` | No |
| POST | `/v1/auth/login` | `login_user` | No |
| GET | `/v1/auth/me` | `get_current_user` | Yes (JWT) |

### API Key Management

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| GET | `/v1/api/keys` | `list_my_api_keys` | Yes |
| POST | `/v1/api/keys` | `create_api_key` | Yes |
| POST | `/v1/api/keys/revoke` | `revoke_api_key` | Yes |

### LLM Operations (OpenAI-Compatible)

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| POST | `/v1/chat/completions` | `chat_completions` | Yes |
| POST | `/v1/chat/completions/stream` | `stream_chat_completions` | Yes |
| POST | `/v1/chat/completions/batch` | `batch_completions` | Yes |
| POST | `/v1/completions` | `completions` | Yes |
| POST | `/v1/embeddings` | `embeddings` | Yes |
| GET | `/v1/models` | `list_models` | Yes |
| GET | `/v1/models/:model` | `get_model` | Yes |

### Usage & Analytics

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| GET | `/v1/usage/:request_id` | `get_usage` | Yes |
| GET | `/v1/balance` | `get_balance` | Yes |
| GET | `/v1/analytics/cost` | `get_cost_analytics` | Yes |
| GET | `/v1/analytics/usage` | `get_usage_analytics` | Yes |
| GET | `/v1/logs` | `get_request_logs` | Yes |

### Admin

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| GET | `/v1/admin/db/tenants` | `list_tenants` | Yes (admin RBAC) |
| GET | `/v1/admin/db/api-keys` | `list_api_keys` | Yes (admin RBAC) |

### Billing

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| GET | `/v1/billing/models` | `get_model_pricing` | Yes |
| GET | `/v1/billing/summary` | `get_billing_summary` | Yes |
| GET | `/v1/billing/budget` | `get_budget_status` | Yes |

### Cache Management

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| GET | `/v1/cache/stats` | `get_cache_stats` | Yes |
| POST | `/v1/cache/clear` | `clear_cache` | Yes (admin) |

### Routing Info

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| GET | `/v1/routing/stats` | `get_routing_stats` | Yes |
| GET | `/v1/routing/config` | `get_routing_config` | Yes |

### Infrastructure

| Method | Path | Handler | Auth Required |
|--------|------|---------|---------------|
| GET | `/` | `root` | No |
| GET | `/health` | `health_check` | No |
| GET | `/ready` | `readiness_check` | No |
| GET | `/health/providers` | `provider_health` | Yes |
| GET | `/metrics` | `metrics_handler` | No |

`/health/providers` requires auth unlike its `/health`/`/ready` siblings: it fans out a live
credentialed `health_check()` call to every registered provider on every request (no cache, no
short-circuit), so left public it would let an anonymous caller burn the operator's provider-side
API quota and probe how many providers are configured.

---

## Authentication Flow

### How Auth Is Extracted (in `handlers.rs`)

```
fn extract_auth_context(state, headers):
    1. Check for "x-api-key" header
       → auth_manager.validate_api_key(key)
       → Returns AuthContext with user_id, tenant_id, limits
    
    2. Else check for "Authorization: Bearer <token>"
       → auth_manager.validate_jwt_token(token)
       → Returns AuthContext with user roles and permissions
    
    3. Else → AuthError::Invalid
```

### API Key Validation (with DB)

- Hash the raw key with SHA-256
- Look up hash in SurrealDB `api_keys` table
- Check: active status, expiration, rate limits
- Update `last_used` timestamp
- Build AuthContext from associated user/tenant

### API Key Validation (without DB)

- Falls through to a **permissive placeholder** AuthContext
- **SECURITY RISK**: Effectively anonymous access without SurrealDB

### JWT Validation

- Decode with HS256 using `JWT_SECRET`
- Load user from DB (if available)
- Build AuthContext with roles from user record
- Roles map to permissions via RBAC system

### RBAC System

Built-in roles (in `rbac.rs`):
- `administrator` — `Permission::All`
- `developer` — Read/write on models, requests, analytics
- `viewer` — Read-only access

Admin endpoints check permissions via `RBACManager::check_permission()`.

---

## LLM Provider System

### Provider Registration (`app.rs`)

Reads `config.providers` HashMap and registers each:

```rust
for (name, provider_config) in &config.providers {
    match provider_config.provider_type.as_str() {
        "openai"    => register OpenAIProvider
        "anthropic" => register AnthropicProvider
        "ollama"    => register OllamaProvider
        _ => skip
    }
}
```

Provider API keys support `${ENV_VAR}` interpolation from environment.

### Provider Trait (`gaussmeridian-core/traits.rs`)

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn supports_streaming(&self) -> bool;
    fn supports_embeddings(&self) -> bool;
    
    async fn chat_completion(&self, request) -> Result<ChatCompletionResponse>;
    async fn chat_completion_stream(&self, request) -> Result<Stream<ChatCompletionChunk>>;
    async fn completion(&self, request) -> Result<CompletionResponse>;
    async fn embedding(&self, request) -> Result<EmbeddingResponse>;
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
}
```

### Routing Logic (`gaussmeridian-core/router.rs`)

**Non-streaming**: `route_chat_completion()` → `route_with_enterprise_features()`:
1. Rate limit check
2. Connection pool checkout
3. Cache lookup (semantic hash of request)
4. On miss: `route_with_fallback()` — tries providers **sequentially** in registration order
5. First successful response wins; failures trigger next provider
6. Response: cache write, usage tracking

**Streaming**: `route_chat_completion_stream()`:
- Loops through providers with `supports_streaming`
- Returns first successful SSE stream
- **Not routed** — `stream_chat_completions` handler exists but is NOT registered in `routes.rs`

### Provider Selection

**Current behavior**: Sequential fallback over ALL registered providers. The router does **NOT** map `request.model` to a specific provider. This means a request for `gpt-4o` could theoretically be sent to Anthropic first if it's registered first.

---

## Database (SurrealDB)

### Connection

Requires environment variables:
- `GAUSSMERIDIAN_DB_URL` — e.g., `ws://surrealdb:8000`
- `GAUSSMERIDIAN_DB_NAMESPACE`
- `GAUSSMERIDIAN_DB_DATABASE`
- `GAUSSMERIDIAN_DB_USERNAME`
- `GAUSSMERIDIAN_DB_PASSWORD`

### Schema (`gaussmeridian-db/schema.rs`)

| Table | Key Fields |
|-------|-----------|
| `users` | id, email, name, password_hash, roles, tenant_id, active |
| `api_keys` | id, user_id, tenant_id, name, key_hash, prefix, active, rate_limit, expires_at |
| `tenants` | id, name, tier, rate_limit, active |
| `models` | id, provider, name, capabilities, pricing |
| `requests` | id, user_id, tenant_id, model, provider, tokens, cost, duration, cached |
| `responses` | id, request_id, status, tokens, content |
| `agents` | id, name, model, system_prompt, capabilities |
| `strategies` | id, name, type, config |
| `cache_entries` | key, value, ttl, created_at |
| `metrics` | timestamp, name, value, labels |

### Repositories

CRUD operations for each entity under `gaussmeridian-db/src/repositories/`:
- `users.rs` — create, find by email, find by id, update, list
- `api_keys.rs` — create, find by hash, update last_used, revoke
- `tenants.rs` — create, find by id, list
- `requests.rs` / `responses.rs` — insert, query by user/time
- etc.

---

## Configuration System

### Loading Order

1. Optional config file: `gaussmeridian.toml` or `.yaml` in working directory
2. Environment variables with prefix `GAUSSMERIDIAN__` (double underscore for nesting)
3. CLI arguments override port, host, log level, feature flags

### Config Structure (`gaussmeridian-config/types.rs`)

```
AppConfig
├── server: ServerConfig          # host, port, enable_cors, max_body_size
├── providers: HashMap<String, ProviderConfig>  # name → type, api_key, base_url, models
├── cache: CacheConfig            # enabled, ttl, max_size
├── security: SecurityConfig      # jwt_secret, cors_origins, rate_limits
├── logging: LoggingConfig        # level, format, file
├── metrics: MetricsConfig        # enabled, port
└── deployment: DeploymentConfig  # environment, region
```

### Database Config

Intentionally **NOT** in `AppConfig` — loaded directly from environment variables in `app.rs`. This separates infrastructure config from application config.

---

## Error Handling

### Unified Error Type (partially used)

`services/server/src/error.rs` defines `ApiError` with:
- `ErrorType` enum (Authentication, Authorization, NotFound, etc.)
- `ErrorCode` for structured error codes
- `IntoResponse` implementation for consistent JSON errors

### Actual Usage

Most handlers use **ad-hoc** `StatusCode` + `serde_json::json!()` instead of the unified `ApiError`. The structured error system exists but isn't consistently adopted.

---

## Middleware Stack (post-M1, fully wired)

`services/server/src/middleware.rs` defines and `routes.rs` applies the full Tower stack:

```
CorsLayer                          ← outermost, runs first
TraceLayer                         ← tower-http distributed tracing
request_logging                    ← structured JSON request/response log
request_validation                 ← content-type + body size checks
auth_middleware_with_state         ← API key / Bearer token enforcement
rate_limiting_with_state           ← shared RateLimiter from AppState
cache_middleware_with_state        ← L1 Moka exact-match; L3 HNSW gated
handler                            ← innermost, runs last
```

**Public paths** (auth bypass): `/`, `/health`, `/ready`, `/metrics`, `/v1/auth/register`, `/v1/auth/login`

`/health/providers` is deliberately NOT public — see the Infrastructure table note above.

**Rate limiting note:** M1 uses in-memory `RateLimiter` from `gaussmeridian-auth`. Redis sliding-window (per-key, per-second) is wired in M2.

**Cache middleware note:** Only activates on `POST /v1/chat/completions`. Streaming endpoint bypassed. L1 key = SHA-256(path | body_bytes). L3 HNSW gated until embedding provider wired (M2).

---

## Streaming Implementation

### OpenAI Provider (`openai.rs`)

Uses `reqwest` + `reqwest_eventsource`:
1. Sends request with `stream: true` to OpenAI API
2. Opens SSE connection via `reqwest_eventsource`
3. Parses each `data:` event into `ChatCompletionChunk`
4. Maps to `async_stream` that yields chunks
5. Detects `[DONE]` sentinel to end stream

### Handler (`stream_chat_completions` in `handlers.rs`)

- Extracts auth, builds request, calls `router.route_chat_completion_stream()`
- Converts stream to `Sse<Event>` response
- **Registered at `POST /v1/chat/completions/stream`** (wired in Sprint M1 Task 1)

---

## Notable Patterns

1. **Trait-based provider abstraction** — Clean `LLMProvider` trait enables easy provider additions
2. **Optional DB** — System works (partially) without SurrealDB connected
3. **DashMap for registry** — Lock-free concurrent provider lookup
4. **Workspace separation** — Clean crate boundaries (core, auth, db, providers)

## Notable Anti-Patterns

1. **No model-to-provider mapping** — Routing doesn't check which provider supports the requested model
2. ~~**Streaming not exposed**~~ ✅ Fixed in Sprint M1
3. ~~**Middleware not applied**~~ ✅ Fixed in Sprint M1
4. **Inconsistent error handling** — Mix of unified `ApiError` and ad-hoc JSON responses
5. **Permissive auth without DB** — Anonymous-like access when SurrealDB is not connected
6. **Many placeholder responses** — Billing summary, budget status, cache stats return static/zero data
