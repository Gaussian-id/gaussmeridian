# 07 — Known Issues, Tech Debt & Risks

> Last updated: 2026-05-11 (post-Sprint M1)

## ✅ Resolved in Sprint M1

| Issue | Resolution |
|-------|-----------|
| Build health unknown | `cargo check` exit 0, warnings only across full workspace including gaussmoa |
| Streaming endpoint not exposed | `POST /v1/chat/completions/stream` wired in `routes.rs` (Task 1) |
| Middleware defined but not applied | Full Tower stack now active: `request_logging → request_validation → auth → rate_limiting → cache → handler` (Task 2) |
| Redis not started | Redis probed at startup; `redis_connected: bool` in `AppState` for graceful fallback (Task 3) |
| SurrealDB schema old design | M1 additive schema applied: `org`, `team`, `api_key`, `project`, `provider_model`, `ledger_entry`, `cache_entry` + HNSW index (Task 4) |
| BYOK vault missing | `gaussmeridian-auth/src/byok.rs` — AES-256-GCM, `BYOK_MASTER_KEY` from env, graceful degradation (Task 5) |
| Moka declared but unused | `MokaL1Cache` implemented and wired into `AppState.l1_cache`; used by `cache_middleware_with_state` (Task 6) |
| CacheMiddleware missing | L1 exact-match active; L3 HNSW gated until embedding provider wired (Task 7) |

---

## Critical Issues

### 1. ~~Build Health Unknown~~ ✅ RESOLVED

`cargo check` exits 0, warnings only (full workspace including gaussmoa).

### 2. ~~Streaming Endpoint Not Exposed~~ ✅ RESOLVED

`POST /v1/chat/completions/stream` wired in Sprint M1 Task 1.

### 3. Permissive Auth Without Database

When SurrealDB is not connected, API key validation falls through to a **permissive placeholder** `AuthContext` — effectively allowing anonymous access. This is dangerous in any deployment where the DB connection fails.

### 4. TypeScript Errors Hidden

`next.config.mjs` sets `typescript.ignoreBuildErrors: true`. The frontend builds "successfully" but may have runtime errors from uncaught type issues.

---

## Documentation Contradictions

| Topic | Source A | Source B | Reality (from code) |
| --- | --- | --- | --- |
| **Web UI framework** | Deno/Fresh (README, ARCHITECTURE, TODO) | Next.js 16 (ASSESSMENT, IMPLEMENTATION) | **Next.js** — `gaussmeridian/services/webui/routes/` is live frontend routing code, not dead code |
| **Completion %** | ~97% (TODO.md) | ~85% (ASSESSMENT.md) | Unknown without running |
| **Build status** | Clean build (REVIEW.md) | 23 errors in MoA (IMPLEMENTATION.md) | Unknown — `cargo check` needed |
| **API port** | 3000 (QUICK_START, manage script) | 8000 (docker-compose) | **8000** in Docker; configurable otherwise |
| **MoA endpoint** | `/v1/moa/process` (QUICK_START) | `/api/v1/moa/process` (ASSESSMENT) | Neither may be wired in current `routes.rs` |
| **Version** | 3.0.0 (workspace Cargo.toml) | 3.1.0 (README, QUICK_START) | **3.0.0** in actual code |

---

## Architectural Issues

### Provider Routing Doesn't Use Model Names

The core router iterates **all registered providers sequentially** on fallback. It does NOT check which provider actually supports the requested model. A `gpt-4o` request could be sent to Anthropic if it's registered first, wasting an API call and returning an error.

**Expected behavior:** Map model prefixes (e.g., `gpt-*` → OpenAI, `claude-*` → Anthropic) to specific providers.

### ~~Middleware Defined but Not Applied~~ ✅ RESOLVED

Full Tower stack is now wired in `create_app()`. See Sprint M1 Task 2.

### Inconsistent Error Handling

The codebase has a well-designed `ApiError` type in `services/server/src/error.rs` with structured error codes and `IntoResponse`. But most handlers use ad-hoc `(StatusCode, Json(json!(...)))` instead. The two patterns should be unified.

### RBAC Role Mismatch

- DB users get role `"user"` on registration
- RBAC system expects role IDs like `"administrator"`, `"developer"`, `"viewer"`
- Admin endpoint checks use `Permission::Custom("db.tenants.read")` etc.
- Without manual role assignment to one of the standard RBAC roles, **admin endpoints will reject all users**

---

## Frontend Issues

### JWT Not Connected to API Client

`GaussMeridianClient.setJwtToken()` exists but is **never called** after login. Console pages using `getGaussMeridianClient()` fall back to the API key from environment variables. If no API key is configured, many admin/list calls will fail or show mock data.

### Duplicate Auth Implementations

Three overlapping auth stacks:

1. `lib/auth-context.tsx` — React Context with login/logout/register
2. `lib/auth.ts` — localStorage helpers + direct fetch
3. `lib/api-client.ts` — GaussMeridianClient with its own auth header logic

The team page (`/console/team`) uses a **fourth** pattern: `useAuth().token` passed to raw `fetch()`.

### No Route Protection

There is no Next.js `middleware.ts`. Console pages are accessible without authentication at the routing level. Protection depends on individual components checking `useAuth()`.

### Frontend Legacy Warning

| Item | Status |
| --- | --- |
| `gaussmeridian/services/webui/routes/` directory | Live frontend routing code. Do not treat it as dead code. |
| `deno.json` | Deno config — orphaned |
| `import_map.json` | Deno import map — orphaned |
| `/dashboard` page | Overlaps with `/console` |
| `/settings` page | Overlaps with `/console/settings` |
| `package.json` name | Still `my-v0-project` |

### Placeholder Data in Console

`app/console/page.tsx` renders hardcoded metrics (e.g., "12.5M requests", "Admin" user) while `islands/console-content.tsx` fetches real data from the API. The two can show conflicting information.

---

## Infrastructure Issues

### Prometheus Scrape Target May Be Wrong

- Dockerfile exposes port **9090** for metrics (separate from 8000 for API)
- Docker Compose maps host 9090 to container 9090 for gaussmeridian
- But `prometheus.yml` scrapes `gaussmeridian:8000` on `/metrics`
- If metrics are served on port 9090, Prometheus won't collect them

**Needs verification:** Check if the Rust server serves `/metrics` on port 8000 or 9090.

### No Grafana Dashboards

The provisioning config is set up, but **no JSON dashboard files exist** in the repo. Grafana will show an empty dashboard folder.

### No Alertmanager

`alerts.yml` defines comprehensive alert rules, but no Alertmanager service is deployed. Alerts fire but have nowhere to go.

### Load Test Default Port Wrong

`k6_load_test.js` defaults to `BASE_URL=http://localhost:3000` (Grafana's port in Docker). Should be `:8000` for the API.

### Management Script Uses Deno

`gaussmeridian-manage.sh` runs the frontend with `deno task start`, but the active frontend is **Next.js** (should be `npm run dev` or `npx next dev`).

---

## Security Concerns

| Issue | Severity | Description |
| --- | --- | --- |
| No DB = open access | **High** | Without SurrealDB, API keys are validated permissively |
| JWT in localStorage | Medium | Vulnerable to XSS; consider httpOnly cookies |
| No HTTPS enforced | Medium | All examples use HTTP; no TLS termination config |
| Secrets in `.env` | Low | Standard practice, but no vault integration |
| `ignoreBuildErrors` | Medium | Type errors may cause runtime security issues |

---

## Performance Unknowns

The docs claim ambitious targets:

- <5ms routing overhead
- 10K+ concurrent connections
- <100ms p99 latency

**None of these have been verified.** The load test infrastructure exists (k6) but hasn't been run against the current codebase. The Grafana dashboards needed to visualize performance don't exist in the repo.

---

## Recommended First Steps

1. ~~**Run `cargo check`** in `gaussmeridian/`~~ ✅ Done — exits 0, full workspace clean
2. **Run `npm install && npm run build`** in `services/webui/` — verify frontend builds
3. **Try `docker compose up`** — see if the full stack starts
4. ~~**Fix streaming route**~~ ✅ Done — wired in Sprint M1
5. **Add model-to-provider mapping** — prevent wasted API calls
6. **Connect JWT to API client** — fix the frontend auth gap
7. **Add Next.js middleware** — protect `/console/*` routes
8. **Clean up legacy frontend leftovers** — verify before deleting anything
