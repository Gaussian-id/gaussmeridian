# GaussMeridian — Service Startup Runbook (V1 base router & V2 Meridian)

> How to start the router in **V1 (base)** and **V2 (Meridian)** modes, the WebUI, and the
> dependencies — plus the exact env flags that separate the two. **V1 and V2 are the same binary;
> V2 is V1 with the Meridian feature flags turned on** (the code path is identical when the flags
> are unset — see `services/server/src/app.rs`).

---

## 0. The key idea: one binary, two modes

Every Meridian V2 capability is **off by default** and gated behind an environment flag. Running
`gaussmeridian` with no V2 flags **is** the V1 base router. Setting the flags upgrades it to V2 in
place — no separate build, branch, or deploy.

|                                           | V1 (base router)          | V2 (Meridian)                   |
| ----------------------------------------- | ------------------------- | ------------------------------- |
| How to run                                | default env (no V2 flags) | V1 env **+** the V2 flags below |
| Guardrails (PII / injection / blocklist)  | off                       | on                              |
| Cascade routing + confidence calibration  | off (score-descending)    | on (cheapest-first + escalate)  |
| GaussMoA (multi-agent on complex queries) | off (single-model)        | on                              |

Base-router improvements that are **always on in both** (they are correctness fixes, not V2
features): provider-aware candidate filtering, cross-provider fallback, and the outcome-gate
billing ledger.

---

## 1. Prerequisites

- Rust toolchain (stable), `cargo`.
- Docker (for SurrealDB + Redis) — or local installs.
- `gaussmeridian/.env` present (DB creds, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `JWT_SECRET`).
- `gaussmeridian/gaussmeridian.toml` present (server + provider config).
- Node 24 + pnpm 10 for the WebUI.

---

## 2. Start the dependencies (SurrealDB + Redis)

```bash
# SurrealDB — host port 8001 → container 8000, ns=gaussmeridian db=main
docker start gaussmeridian-surrealdb      # or `docker compose up -d surrealdb redis`
docker start gaussmeridian-redis
# verify
docker ps --filter name=gaussmeridian- --format '{{.Names}}\t{{.Status}}'
```

The backend reads DB connection from `.env`:
`GAUSSMERIDIAN_DB_URL=ws://127.0.0.1:8001`, `GAUSSMERIDIAN_DB_NAMESPACE=gaussmeridian`,
`GAUSSMERIDIAN_DB_DATABASE=main`, `GAUSSMERIDIAN_DB_USERNAME/PASSWORD`.

---

## 3. Build

```bash
cd gaussmeridian
cargo build --bin gaussmeridian          # add --release for production
```

---

## 4. Start the router — V1 (base)

Run with **no** Meridian flags (the default). This is the pre-V2 behavior.

```bash
cd gaussmeridian
./target/debug/gaussmeridian        # Windows: ./target/debug/gaussmeridian.exe
```

Optional base-router tuning (safe in both modes):

| Env var                             | Default | Meaning                                                |
| ----------------------------------- | ------- | ------------------------------------------------------ |
| `GAUSSMERIDIAN_LAMBDA`                | `0.01`  | cost-sensitivity λ (0 = quality-first, 1 = cost-first) |
| `GAUSSMERIDIAN_QUALITY_FLOOR`         | `0.70`  | drop candidates below this EWMA quality                |
| `GAUSSMERIDIAN_TAU_MOA`               | `0.7`   | Meridian complexity threshold that sets `moa_flagged`  |
| `GAUSSMERIDIAN_MAX_PROVIDER_ATTEMPTS` | `3`     | provider fallback attempts (also the diversity window) |
| `GAUSSMERIDIAN_CANDIDATE_POOL_SIZE`   | `10`    | ranked candidates kept for the fallback chain          |
| `GAUSSMERIDIAN_DB_SEED`               | —       | seed the `provider_models` catalog at boot             |

Server listens on `0.0.0.0:8000` (`gaussmeridian.toml [server]`).

---

## 5. Start the router — V2 (Meridian)

V1 command **plus** the feature flags you want. Enable all three for full V2:

```bash
cd gaussmeridian
GAUSSMERIDIAN_GUARDRAIL_PII=1 \
GAUSSMERIDIAN_GUARDRAIL_INJECTION=1 \
GAUSSMERIDIAN_GUARDRAIL_BLOCKED_TERMS="term1,term2" \
GAUSSMERIDIAN_CASCADE=1 \
GAUSSMERIDIAN_CASCADE_THRESHOLD=0.7 \
GAUSSMERIDIAN_CASCADE_TEMPERATURE=1.0 \
GAUSSMERIDIAN_MOA=1 \
GAUSSMERIDIAN_MOA_AGENTS="gpt-4o-mini,gpt-4o" \
  ./target/debug/gaussmeridian
```

On boot you'll see `Guardrails enabled (Meridian V2)`, `Cascade routing enabled (Meridian V2)`,
and `GaussMoA engine initialized (Meridian V2)` in the logs — those three lines confirm V2 mode.

### V2 env reference

**Guardrails** (output filtering — inspects the response before it leaves the gateway):

| Env var                               | Default | Meaning                                  |
| ------------------------------------- | ------- | ---------------------------------------- |
| `GAUSSMERIDIAN_GUARDRAIL_PII`           | off     | block responses leaking PII (SSN, cards) |
| `GAUSSMERIDIAN_GUARDRAIL_INJECTION`     | off     | block prompt-injection patterns          |
| `GAUSSMERIDIAN_GUARDRAIL_BLOCKED_TERMS` | —       | comma-separated blocklist                |

**Cascade routing + confidence calibration:**

| Env var                           | Default | Meaning                                                  |
| --------------------------------- | ------- | -------------------------------------------------------- |
| `GAUSSMERIDIAN_CASCADE`             | off     | try cheapest candidate first, escalate on low confidence |
| `GAUSSMERIDIAN_CASCADE_THRESHOLD`   | `0.7`   | calibrated-confidence floor below which to escalate      |
| `GAUSSMERIDIAN_CASCADE_TEMPERATURE` | `1.0`   | temperature `T` for `σ(logit(raw)/T)` calibration        |

**GaussMoA (multi-agent on Meridian-flagged complex queries):**

| Env var                              | Default              | Meaning                                             |
| ------------------------------------ | -------------------- | --------------------------------------------------- |
| `GAUSSMERIDIAN_MOA`                    | off                  | enable in-process multi-agent dispatch              |
| `GAUSSMERIDIAN_MOA_AGENTS`             | `gpt-4o-mini,gpt-4o` | comma-separated models, one agent each              |
| `GAUSSMERIDIAN_MOA_TIMEOUT_SECS`       | `30`                 | latency budget; on breach fall back to single-model |
| `GAUSSMERIDIAN_MOA_TEMPERATURE`        | `0.7`                | per-agent temperature                               |
| `GAUSSMERIDIAN_MOA_MAX_TOKENS`         | `1024`               | per-agent max tokens                                |
| `GAUSSMERIDIAN_MOA_AGENT_TIMEOUT_SECS` | `60`                 | per-agent request timeout                           |

**BYOK (customer provider keys)** — needed for the WebUI BYOK feature:

| Env var             | Meaning                                        |
| ------------------- | ---------------------------------------------- |
| `BYOK_MASTER_KEY`   | base64 32-byte AES key; BYOK is 503 until set  |
| `BYOK_ADMIN_EMAILS` | comma-separated allowlist; empty = BYOK closed |

**Forgot-password email (optional; log-only until set):**
`SMTP_HOST/PORT/USERNAME/PASSWORD/FROM`, `WEBUI_BASE_URL` (reset-link base, default
`http://localhost:3000`).

---

## 6. Start the WebUI (gauss-boilerplate)

Dev:
```bash
cd "0. WebUI Boilerplate"
pnpm install
pnpm dev            # http://localhost:3000, proxies to the backend on :8000
```

Docker (self-contained, no public hosting):
```bash
cd "0. WebUI Boilerplate"
docker compose up --build     # WebUI on :3000 → backend via host.docker.internal:8000
```
Repoint the backend at runtime with `GAUSSMERIDIAN_API_URL` (the container reads it server-side;
`NEXT_PUBLIC_API_BASE_URL` is build-time only).

---

## 7. Smoke test

```bash
curl -s localhost:8000/health                         # {"status":"ok"...}
# register → get a key → chat
TOKEN=$(curl -s -X POST localhost:8000/v1/auth/register -H 'Content-Type: application/json' \
  -d '{"email":"me@x.com","username":"me","password":"SecurePass123!"}' | jq -r .token)
KEY=$(curl -s -X POST localhost:8000/v1/api/keys -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"name":"test"}' | jq -r .api_key)
curl -s -D- -X POST localhost:8000/v1/chat/completions -H "x-api-key: $KEY" \
  -H 'Content-Type: application/json' \
  -d '{"model":"auto","messages":[{"role":"user","content":"hi"}],"max_tokens":16}' | grep -i x-gaussmeridian
```

Response headers tell you what happened: `x-gaussmeridian-model` / `-provider` (who served),
`x-gaussmeridian-moa: true` (V2 MoA path), `x-gaussmeridian-r-binary` (charged vs $0),
`x-gaussmeridian-candidates` (the fallback chain).

---

## 8. Stop

```bash
# Windows
taskkill /F /IM gaussmeridian.exe
# WebUI container
docker compose down
```
