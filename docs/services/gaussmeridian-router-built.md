# GaussMeridian Router — What's Built

> Capability inventory for the **router / gateway service** (repo `gauss-meridian`, Rust/Axum crate
> workspace; server binary `gaussmeridian`). Companion doc for the WebUI service lives in
> `gauss-boilerplate/docs/gaussmeridian-webui-built.md`. For *how to run* see
> `docs/operations/service-startup-runbook.md`; for *how a request flows* see
> `docs/architecture/business-technical-workflow-v1-v2.md`.

Everything below is implemented and, where noted, live-verified this program. V1 = base router
(default); V2 = the same binary with Meridian feature flags on.

> **Claim boundary:** the deterministic Meridian estimator and legacy advisory skill features are
> product heuristics, not CARROT or BELLA implementations. Formal P2 is mechanism-qualified, but
> production predictor promotion is blocked pending governed authority. BELLA remains P3, and
> compound xRouter policy remains P5.

---

## 1. What the router is
A self-hosted, **OpenAI-compatible** LLM gateway: one endpoint that classifies each request, picks
the best model for the caller's quality/cost preference, serves it with cross-provider fallback,
and bills only for outcomes that pass the caller's validator. Written as a Cargo workspace
(`gaussmeridian-core`, `-providers`, `-models`, `-db`, `-auth`, `-cache`, `gaussmoa`, `services/server`).

---

## 2. Built capabilities

### Auth, tenancy & project resolution
- `POST /v1/auth/register` · `login` · `logout` (JWT + **Redis token revocation**) · `me`.
- **Password recovery:** `forgot-password` / `reset-password` — single-use hashed tokens, `lettre`
  SMTP (log-only until SMTP creds set).
- **API keys:** create / list / revoke (revoke enforces ownership).
- **Auto-provisioning:** every registration creates a **default project** and links it
  (`user.default_project_id`). All per-project config resolves via **`user_id → default project`**,
  so it works identically for session (JWT) and x-api-key callers. *(OD-011/OD-012, live-verified.)*

### Routing pipeline (V1 core)
- **Meridian complexity estimator** → versioned `complexity_score ∈ [0,1]`, sets `moa_flagged` at `≥ τ_moa`.
- **Legacy advisory skill features** → 12 keyword-derived dimensions; formal BELLA evidence is P3.
- **Meridian deterministic routing policy** (`quality`, `cost`, health, hard eligibility) → immutable ranked ballot.
- **P2 CARROT mechanism boundary** → conditional predictor objects and fallback are qualified;
  production promotion currently returns `production_promotion_blocked`.
- **Provider-aware selection (Phase 0):** candidates filtered to **registered** providers +
  cross-provider **diversity guarantee** → a provider outage falls back to another provider instead
  of 502. *(Live-verified: OpenAI 503 → Anthropic 200.)*
- **Fallback chain:** attempts up to `GAUSSMERIDIAN_MAX_PROVIDER_ATTEMPTS` registered candidates,
  skipping open circuit breakers.

### Provider adapters (`gaussmeridian-providers`)
- **OpenAI** — live-verified (gpt-4o, gpt-4o-mini, o4-mini); **o-series param fix**
  (`max_completion_tokens`); **env-overridable** base URL (`OPENAI_API_BASE`).
- **Anthropic** — adapter complete + **env-overridable** base URL (`ANTHROPIC_API_BASE`); the
  `base_url` was formerly hardcoded (now honored). *Live run mock-proven (account has no credits).*
- Scaffolded but **not wired to the catalog:** cohere, huggingface, ollama, lmstudio, vllm, custom.

### Meridian V2 features (env-gated, off by default)
- **Guardrails** (`GAUSSMERIDIAN_GUARDRAIL_PII/INJECTION/BLOCKED_TERMS`) — inspect the response and
  return `403 guardrail_violation` on PII / prompt-injection / blocklist hits.
- **Cascade routing + confidence calibration** (`GAUSSMERIDIAN_CASCADE*`) — cheapest-first ordering;
  calibrate raw confidence `σ(logit(raw)/T)`; escalate to a stronger model below threshold.
- **GaussMoA** — see §3.

### GaussMoA (multi-agent) — integrated this program
- In-process engine built from the shared provider stack (`MoaEngine::from_parts`), gated by
  `GAUSSMERIDIAN_MOA`; agents env-configured (`GAUSSMERIDIAN_MOA_AGENTS`, temperature/max_tokens/timeout).
- **Real parallel fan-out** (was single-agent) with **per-agent isolation**, best-of-N aggregation.
- Dispatched at the provider stage when the Meridian estimator sets `moa_flagged`; **single-model fallback on any
  error or latency-budget breach** (`GAUSSMERIDIAN_MOA_TIMEOUT_SECS`).
- Agents call the **one** provider stack (real keys, BYOK, o-series fix) — the old duplicate
  clients + `DUMMY_API_KEY` are deleted.
- *Live-verified: MoA across real OpenAI + mocked Anthropic → 200; forced failure → clean fallback.*

### Outcome-based billing
- One `ledger_entry` per request with **`r_binary`** (1 = validator passed → charged; 0 = failed →
  `cost_charged = 0`). MoA runs write one aggregate `moa_flagged` row.
- `provider_models` catalog seeded at boot (15 models, 4 tiers).

### Project settings, logs, BYOK
- `GET/PATCH /v1/project/settings` — λ, quality_floor, budget, hard limit, alert webhook (+ read-only
  τ_moa, validator_type). Session-reachable via `user_id → default project`.
- `GET /v1/logs` — project-scoped, ledger-sourced (carries `r_binary` + `cost_charged`).
- **BYOK** — AES-256-GCM-encrypted provider keys on the **project** (`byok_keys` map); admin-gated
  (`BYOK_ADMIN_EMAILS`); vault requires `BYOK_MASTER_KEY` (503 until set). Names-only listing.

### Platform
- **Config is env-driven** (all V2 flags, MoA params, provider base URLs, routing budgets); default
  values reproduce V1. `gaussmeridian.toml` for server + provider config; `.env` for secrets/DB.
- SurrealDB (persistence/ledger), Redis (rate limit + JWT revocation), Moka L1 cache, Prometheus
  `/metrics`, circuit breakers, distributed rate limiting.

---

## 3. Endpoint surface (implemented)
`/v1/auth/{register,login,logout,forgot-password,reset-password,me}` ·
`/v1/api/keys` (GET/POST) + `/keys/revoke` ·
`/v1/byok/keys` (GET/POST) + `/keys/:provider` (DELETE) ·
`/v1/chat/completions` (+ `/stream`, `/batch`) · `/v1/completions` · `/v1/embeddings` ·
`/v1/models` (+ `/:model`) · `/v1/usage/:id` · `/v1/balance` ·
`/v1/analytics/{cost,usage}` · `/v1/logs` ·
`/v1/billing/{models,summary,budget}` · `/v1/project/settings` (GET/PATCH) ·
`/v1/cache/{stats,clear}` · `/v1/routing/{stats,config,trace}` ·
`/v1/admin/db/{tenants,api-keys}` · `/health` · `/ready` · `/metrics`.

---

## 4. What's NOT built (router-side gaps)
- **Provider adapters** for google / deepseek / qwen / mistral — catalog-listed but **unservable**
  (10 of 15 models can't complete). Only OpenAI + Anthropic are wired.
- **Live Anthropic** — mock-proven only; account has no credits.
- **Per-agent MoA billing** — one aggregate ledger row with an **estimated** cost; per-agent token
  accounting not threaded.
- **Per-project MoA config** — MoA is server-level env, not per-project (τ_moa is per-project).
- **Streaming through MoA** — MoA returns a single aggregated completion (no stream).
- **Calibration recalibration job** (periodic re-fit) — not built.
- **Tech debt:** dead `gaussmoa/engine/` dir (safe to delete); two quarantined tests
  (`test_resource_pool` ~1h sleep, `is_response_diverse` pre-existing failure).
- Branch state: work lives on `feat/webui-launch-backend`, unmerged to `main`.
