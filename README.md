# GaussMeridian Ecosystem

<p align="center">
  <strong>Self-hosted, OpenAI-compatible LLM API gateway with multi-provider routing</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-3.0.0-blue.svg" alt="Version">
  <img src="https://img.shields.io/badge/rust-1.75+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/license-AGPL--3.0--only-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/status-active--MVP--build-yellow.svg" alt="Status">
</p>

---

Built in **Rust/Axum/Tokio** (backend) and **Next.js** (web UI), GaussMeridian is a
self-hostable, OpenAI-compatible API gateway for multiple LLM providers: BYOK
(bring-your-own-key) provider credentials, per-project API keys, a request ledger, and
multi-agent orchestration (GaussMoA). It exposes the same `/v1/chat/completions` shape
most OpenAI SDKs already speak, so existing client code can point at it with a base-URL
change.

**On routing efficacy:** PRD-26 closed at P0–P6 functional scope on 2026-08-16 — Meridian
P1, CARROT P2, BELLA P3, R2 P4, and xRouter P5 are all implemented, wired into the runtime,
and accepted at their recorded controlled-mechanism boundaries in isolated, zero-spend
testing. That is a **functional** closure and nothing more. None of it has been
independently qualified for efficacy; every path still fails closed at runtime with
`production_promotion_blocked`; and this project makes no representative-benchmark,
market-comparison, or production-readiness claim anywhere. See
[Routing Intelligence Phase Map](#-routing-intelligence-phase-map) below for the per-boundary
state and for what genuinely remains open.

---

## 📖 There is no website — this repository is the whole thing

GaussMeridian has **no marketing site, no docs portal, no hosted SaaS, and no signup**.
There is nothing to create an account for and nowhere else to read. This repository is
the entire distribution: the source, the documentation, the deployment files, and the
issue tracker.

That is a deliberate consequence of how the project is licensed. GaussMeridian is
[AGPL-3.0-only](#-license) — you are meant to run your own instance, and everything
required to do that ships in the clone you already have.

Practically, that means:

- **Every link in this README is a relative path into this repository**, or a GitHub
  URL for this repository. If you find a link to `docs.gaussmeridian.com` or any other
  host, it is stale — please open an issue.
- **You never need credentials to get started.** The default stack boots with zero
  provider API keys and serves deterministic responses from a bundled mock provider.
- **Everything below runs offline** after the initial `git clone` and dependency pull.

---

## 🆕 New here? Read in this order

1. **This `README.md`** — setup, environment, ports, first request, troubleshooting.
2. [`CONTRIBUTING.md`](CONTRIBUTING.md) — contribution licensing, the supported native
   setup, and the pull-request loop. Read the licensing section before writing code.
3. [`docs/operations/native-community-preview.md`](docs/operations/native-community-preview.md)
   — the exact stack, qualification contract, and troubleshooting for contributors.
4. [`docs/codebase-report/00-INDEX.md`](docs/codebase-report/00-INDEX.md) — full
   codebase report index.
5. [`docs/codebase-report/09-MVP-HUMAN-STATUS.md`](docs/codebase-report/09-MVP-HUMAN-STATUS.md)
   — historical M1 status versus the build plan.

---

## ⚠️ Current Build Truth

> Read this before running anything. It is deliberately unflattering — the project
> would rather you know what is unfinished than discover it at runtime.

| Item                         | Status                                                                                       |
| ---------------------------- | -------------------------------------------------------------------------------------------- |
| Workspace root               | `gaussmeridian/Cargo.toml` — all `cargo` commands must be run from `gaussmeridian/`              |
| Workspace version            | `3.0.0`                                                                                      |
| License                      | `AGPL-3.0-only` (relicensed from Apache-2.0; legal review gate G3 not yet cleared)            |
| Compile status               | Required suites are qualified at the accepted boundary; rerun the relevant suite after changes |
| PRD-26 Routing Intelligence  | **CLOSED at P0–P6 functional scope** (2026-08-16). Functional only — no efficacy, promotion, or rollout authority |
| Native Community Preview     | **Qualified, not published** — 2 isolated runs, 25/25 cases, byte-identical report SHA |
| Active milestone             | PRD-29 brand-logo rollout (gated, not started). Billing/launch-bridge work lives on a separate lane |
| Historical M1 reality check  | [`docs/codebase-report/09-MVP-HUMAN-STATUS.md`](docs/codebase-report/09-MVP-HUMAN-STATUS.md) |
| Streaming                    | Chat/text SSE routes are live and covered by the exact-image qualification gate       |
| Auth / rate-limit middleware | **Layered and active** (`services/server/src/routes.rs`) — auth, rate limiting, resource control, budget pre-check, and cache all sit in the documented pre-provider order |
| Redis                        | **Connected at startup** as the sliding-window rate limiter (`app.rs`). Failure is non-fatal — the server falls back to the in-memory limiter |
| Moka L1 cache                | **Initialized at startup** and held in `AppState.l1_cache`; serves the L1 exact-match tier ahead of the L3 HNSW semantic tier |
| SurrealDB version            | Pinned at `2.0` — do not upgrade to v3 without reading open Decision 7                       |
| Provider catalog             | Only **Google (Gemini)** routes out of the box — see [Provider catalog](#-provider-catalog)  |

---

## 🧰 Prerequisites

You need Docker for every path. Rust and Node are only needed if you intend to build
or modify the source rather than just run it.

| Tool                | Minimum         | Needed for                             | Check with                 |
| ------------------- | --------------- | -------------------------------------- | -------------------------- |
| **Git**             | any recent      | everything                             | `git --version`            |
| **Docker**          | 24+             | every path                             | `docker --version`         |
| **Docker Compose**  | v2 (the plugin) | every path                             | `docker compose version`   |
| **Rust (stable)**   | 1.75+           | building/testing the Rust workspace    | `rustc --version`          |
| **Cargo**           | ships with Rust | building/testing the Rust workspace    | `cargo --version`          |
| **Node.js**         | 20+             | working on the WebUI outside Docker    | `node --version`           |
| **pnpm**            | 9+              | WebUI dependencies (`pnpm-lock.yaml`)  | `pnpm --version`           |
| **Python**          | 3.11+           | the qualification/evidence scripts     | `python --version`         |
| **jq**              | any recent      | the shell snippets in Your first API call (parsing JSON responses) | `jq --version` |

Notes that will save you an hour:

- **`docker compose` (space), not `docker-compose` (hyphen).** The hyphenated v1
  binary is not supported here; every command in this repository assumes the v2 plugin.
- **Rust is not pinned.** There is no `rust-toolchain.toml`; a current stable toolchain
  from [rustup](https://rustup.rs) is what CI and contributors use. The workspace is
  edition 2021.
- **Node is not pinned either.** `webui/package.json` declares no `engines` field, but
  the WebUI is on Next.js 16, which requires Node 20.9 or newer.
- **pnpm is the package manager of record** for the WebUI — `pnpm-lock.yaml` and
  `pnpm-workspace.yaml` are committed. Installing with npm or yarn will resolve a
  different tree than CI does.
- **Windows works**, and is what much of this project is developed on. Use PowerShell
  or Git Bash; both are used in the examples below where the syntax differs.
- **Disk and time.** No prebuilt images are published yet, so the first
  `docker compose up` compiles the entire Rust workspace (600+ crates) and the Next.js
  app inside Docker. Budget several GB and a long first build. Subsequent runs are
  cached and fast.

---

## 🚀 Setup

### Step 0 — Pick your path

| Path                                        | You want to…                                              | Requires            |
| ------------------------------------------- | --------------------------------------------------------- | ------------------- |
| **[A — Docker quickstart](#path-a--docker-quickstart)** | Run the gateway and see it work. Start here. | Docker only         |
| **[B — Local development loop](#path-b--local-development-loop)** | Change Rust or WebUI code and test it. | Docker + Rust + Node |
| **[C — Contributor preview](#path-c--contributor-qualification-preview)** | Open a pull request against this repo. | Docker + Rust + Python |

If this is your first contact with the project, do Path A first even if you intend to
contribute. It takes one command and tells you whether your machine is set up.

---

### Path A — Docker quickstart

The clone-and-run public stack, defined by `docker-compose.yml` at the repository root.
It builds every service from source.

**1. Clone and enter the repository.**

```bash
git clone https://github.com/Gaussian-id/gaussmeridian
cd gauss-meridian
```

**2. Create your environment file.**

```bash
cp .env.example .env
```

Copied verbatim, this boots a working gateway with **zero provider API keys and zero
spend**. It contains development-only secrets that you must replace before exposing the
stack to anyone — see [Environment variables](#-environment-variables).

**3. Bring the stack up.** The first run compiles everything; expect it to take a
while and print a lot.

```bash
docker compose up -d
```

This starts the gateway and its datastores — **four containers**. The WebUI, Prometheus
and Grafana sit behind Compose profiles and are not included; see [Ports](#-ports) for
how to add them.

**4. Wait for health, then verify.**

```bash
docker compose ps                    # every service should read healthy/running
curl http://localhost:8000/health    # {"status":"healthy",...}
curl http://localhost:8000/ready     # 200 once a provider is callable
curl http://localhost:8000/          # API info, including the AGPL source offer
```

If `docker compose ps` shows a service restarting, jump to
[Troubleshooting](#-troubleshooting) — do not keep re-running `up`.

**5. Make a request.** See [Your first API call](#-your-first-api-call).

> **Known limitation — the zero-key path cannot complete an inference.**
> The zero-key path is wired to a bundled mock provider registered as the `openai`
> adapter, but `gaussmeridian/gaussmeridian.toml`'s openai `models` allowlist and the
> seeded routing catalog currently have no models in common, so a
> `POST /v1/chat/completions` against the mock returns 503
> (`no_hard_eligible_models`). This is a known, escalated provider-catalog gap awaiting
> a decision from the project owner — it is not a Docker or networking problem on your
> machine. Tracked in [`docs/evidence/report.md`](docs/evidence/report.md) under "Known
> blocker". To complete a real inference today, add a Gemini key —
> see [Provider catalog](#-provider-catalog).

**Stopping and cleaning up:**

```bash
docker compose down                  # stop, keep data volumes
docker compose down -v               # stop and DELETE the database and cache volumes
docker compose logs -f gaussmeridian # follow the gateway's logs
```

---

### Path B — Local development loop

Run the datastores in Docker and the code on your machine. This is the fast
edit-compile-test loop.

**Rust workspace.** Every `cargo` command runs from `gaussmeridian/`, not the
repository root — the workspace `Cargo.toml` lives there.

```bash
cd gaussmeridian
cargo build                                                    # compile
cargo test --workspace --locked                                # full test suite
cargo fmt --all -- --check                                     # formatting gate
cargo clippy --workspace --all-targets --all-features -- -D warnings   # lint gate
```

Those last three are exactly what a pull request is checked against, so run them before
you push. `cargo watch -x run` gives you auto-reload if you have `cargo-watch`
installed.

The gateway needs SurrealDB and Redis. You can start only those from the compose file
and leave the gateway to your local build:

```bash
cd ..                                # back to the repository root
docker compose up -d surrealdb redis
```

> **A note on verification.** This project's convention is to live-verify the backend
> through `docker compose`, not by launching a locally-built executable. A native
> binary picks up your shell's environment rather than the compose environment, and the
> two diverge in ways that produce confusing results. Use the native loop for
> `cargo test`/`cargo clippy`; use Docker when you need to confirm real runtime
> behavior.

**WebUI.** A separate pnpm project under `webui/`.

```bash
cd webui
pnpm install
cp .env.example .env                 # points at http://127.0.0.1:8000 by default
pnpm dev                             # http://127.0.0.1:3000
```

Other WebUI commands:

```bash
pnpm typecheck      # tsc --noEmit
pnpm lint           # eslint
pnpm test           # vitest
pnpm test:ux        # playwright
pnpm build          # production build
```

Use `127.0.0.1` rather than `localhost` in WebUI URLs. The session cookie is host-only,
and Node resolves `localhost` to IPv6 (`::1`) first while the backend binds IPv4 — mixing
the two silently breaks auth.

If you have no backend running, `NEXT_PUBLIC_USE_MOCKS=1` serves the console from
in-memory fixtures instead. It is refused when `NODE_ENV=production`.

---

### Path C — Contributor qualification preview

The isolated, deterministic native preview used for development and qualification. It
needs **no commercial provider credentials** — it runs against the repository's own
deterministic provider simulator, on port **8020**, under its own compose project name
so it never collides with the Path A stack.

Start with [`CONTRIBUTING.md`](CONTRIBUTING.md) for the exact first-start commands
(they generate and pin a database password you must keep), then use
[`docs/operations/native-community-preview.md`](docs/operations/native-community-preview.md)
for the lifecycle commands, architecture ownership, stability matrix, and
troubleshooting contract.

---

## 🔑 Environment variables

Two environment files, and they are not interchangeable:

| File               | Consumed by                    | Created from                     |
| ------------------ | ------------------------------ | -------------------------------- |
| `.env` (root)      | `docker-compose.yml` — gateway, database, cache | `cp .env.example .env`    |
| `webui/.env`       | the Next.js app                | `cp webui/.env.example webui/.env` |

`.env.example` is the authoritative, commented source for the root file — read it, it
explains each value in place. The tables here are a map, not a replacement.

### Required — the stack refuses to start without these

`docker-compose.yml` declares each with `:?`, so a missing value fails `docker compose
up` loudly rather than falling back to a silent default. The values shipped in
`.env.example` are **development-only**; regenerate every one with
`openssl rand -hex 32` before any real deployment.

| Variable                | Purpose                                                   |
| ----------------------- | --------------------------------------------------------- |
| `SURREALDB_PASSWORD`    | SurrealDB `root` password, shared by the database and gateway |
| `REDIS_PASSWORD`        | Redis password, folded into the gateway's `REDIS_URL`     |
| `JWT_SECRET`            | Signs session tokens. Minimum 32 characters               |
| `GAUSSMERIDIAN_API_KEY` | **Read only by the TUI client**, which sends it as its own credential. The gateway does not accept it — see [Your first API call](#-your-first-api-call). Compose still declares it `:?`, so a value must be present for the stack to start |
| `GRAFANA_PASSWORD`      | Must stay **uncommented even if you never enable Grafana** — Compose interpolates all services' variables before filtering by profile |

### Provider keys — all optional

Leave every one blank and the gateway routes to the bundled mock provider. Set one and
re-run `docker compose up -d`. The `x-gaussmeridian-provider-selected` response header
tells you which provider actually served a request.

| Variable            | Status today                                                    |
| ------------------- | ---------------------------------------------------------------- |
| `GEMINI_API_KEY`    | ✅ **Works.** The only provider that routes out of the box       |
| `OPENAI_API_KEY`    | ⚠️ Stale allowlist — cannot route even with a valid key          |
| `ANTHROPIC_API_KEY` | ⚠️ Stale allowlist — cannot route even with a valid key          |
| `COHERE_API_KEY`    | Declared; not covered by the current evidence pack               |
| `OPENAI_BASE_URL`   | Point the `openai` adapter elsewhere (defaults to the mock)      |

See [Provider catalog](#-provider-catalog) for why, and what to do about it.

### AGPL Section 13 — source offer

GaussMeridian is AGPL-3.0-only, and Section 13 requires that network users be offered
the Corresponding Source. Both variables default to this repository, which is correct
for an unmodified build. **If you deploy a modified build, set both to a URL serving
your own source** — that is the whole compliance action.

| Variable                        | Applies to | Default                                          |
| ------------------------------- | ---------- | ------------------------------------------------ |
| `SOURCE_OFFER_URL`              | gateway    | `https://github.com/Gaussian-id/gaussmeridian`  |
| `NEXT_PUBLIC_SOURCE_OFFER_URL`  | WebUI      | `https://github.com/Gaussian-id/gaussmeridian`  |

The gateway publishes the offer as the `x-source-offer` header on every response and in
the `GET /` body; the WebUI links it from the footer of every page. Details in
[`NOTICE`](NOTICE).

### Frequently useful optional variables

| Variable                    | Effect                                                                 |
| --------------------------- | ---------------------------------------------------------------------- |
| `SUPERADMIN_EMAILS`         | Comma-separated allowlist. **Unset, every `/v1/admin/*` route returns 404** — deliberately, so the surface is never advertised |
| `BYOK_MASTER_KEY`           | AES-256 key (base64, 32 bytes) encrypting customer provider keys. Compose injects a dev-only default; generate yours with `openssl rand -base64 32` |
| `BYOK_ADMIN_EMAILS`         | Gates BYOK key registration; blank allows the first registered account  |
| `REDIS_URL`                 | Only if you run Redis outside this stack. The name is unprefixed on purpose — the server reads `REDIS_URL` and nothing else. `GAUSSMERIDIAN_REDIS_URL` is read by no code in the workspace |
| `GAUSSMERIDIAN_CASCADE`     | Cascade routing + confidence calibration (default on)                   |
| `GAUSSMERIDIAN_MOA`         | Multi-agent orchestration on complex queries (default on)               |
| `GAUSSMERIDIAN_GUARDRAIL_PII` / `_INJECTION` | Response scanning for PII and prompt injection (default on) |

WebUI-side (`webui/.env`): `NEXT_PUBLIC_API_BASE_URL` is inlined into the browser bundle
at build time and must be reachable **from your browser**, not from inside the Docker
network; `GAUSSMERIDIAN_API_URL` is the server-only override used by the proxy and auth
route handlers.

---

## 🔌 Ports

Anything bound to `127.0.0.1` is loopback-only by design and is not part of the public
API surface.

| Port   | Service                | Binding     | Notes                                        |
| ------ | ---------------------- | ----------- | -------------------------------------------- |
| `8000` | Gateway HTTP API       | all         | The one you actually call                    |
| `9090` | Prometheus metrics     | `127.0.0.1` | Gateway's own metrics endpoint               |
| `8001` | SurrealDB web UI       | `127.0.0.1` | Container-internal port is 8000              |
| `6379` | Redis                  | `127.0.0.1` |                                              |
| `3001` | WebUI (`--profile webui`) | all      | Container-internal 3000; host 3001 because Grafana holds 3000 |
| `3000` | Grafana (`--profile observability`) | all | Also the port `pnpm dev` uses locally  |
| `9091` | Prometheus (`--profile observability`) | `127.0.0.1` | Container-internal 9090          |
| `8020` | Native contributor preview (Path C) | `127.0.0.1` | Separate compose project, container-internal 8000 |

Optional profiles:

```bash
docker compose --profile webui up -d           # + WebUI on :3001
docker compose --profile observability up -d   # + Prometheus and Grafana
```

---

## 🧪 Your first API call

Every `/v1/*` route needs a credential, and there are two kinds. They are not
interchangeable:

| Credential | Header | How you get it |
| ---------- | ------ | -------------- |
| **Session token** (JWT) | `Authorization: Bearer <token>` | `POST /v1/auth/register` or `/v1/auth/login` |
| **Project API key** | `x-api-key: <key>` | `POST /v1/api/keys` with a `project_id` |

> **`GAUSSMERIDIAN_API_KEY` from `.env` is neither of them.** It is the TUI client's own
> credential. The gateway never reads it and answers `401 invalid credentials`. Compose
> still requires it to be set, which is why it appears in the required table above.

**Step 1 — get a session token.** Registration is the only unauthenticated way in:

```bash
TOKEN=$(curl -s http://localhost:8000/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com","username":"you","password":"YourPassword123!"}' \
  | jq -r .token)
```

That token works as a bearer immediately:

```bash
curl -H "Authorization: Bearer $TOKEN" http://localhost:8000/v1/models
```

**Step 2 — create an organization, a project, and a key scoped to it.** An unscoped key
is refused on inference with `project_scope_required`:

```bash
ORG=$(curl -s -X POST http://localhost:8000/v1/orgs \
  -H "Content-Type: application/json" -H "Authorization: Bearer $TOKEN" \
  -d '{"name":"My Org"}' | jq -r .id)

PROJECT=$(curl -s -X POST http://localhost:8000/v1/orgs/$ORG/projects \
  -H "Content-Type: application/json" -H "Authorization: Bearer $TOKEN" \
  -d '{"name":"My Project"}' | jq -r .id)

KEY=$(curl -s -X POST http://localhost:8000/v1/api/keys \
  -H "Content-Type: application/json" -H "Authorization: Bearer $TOKEN" \
  -d "{\"name\":\"my key\",\"project_id\":\"$PROJECT\"}" | jq -r .api_key)
```

The raw secret is returned **once**. Store it now.

**Step 3 — call the gateway with `x-api-key`:**

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-api-key: $KEY" \
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Say hello in five words."}]
  }'
```

> **On a brand-new install this will not return a completion yet.** In order, you will
> hit `payment_required` (the project has no budget), and — once funded, with no provider
> key set — `503 no_hard_eligible_models`. Both are expected; see
> [Troubleshooting](#-troubleshooting) and [Provider catalog](#-provider-catalog).
> Everything up to this point is verifiable today.

PowerShell:

```powershell
$body = @{ model = "gemini-2.5-flash"; messages = @(@{ role = "user"; content = "Say hello in five words." }) } | ConvertTo-Json -Depth 5
Invoke-RestMethod -Uri http://localhost:8000/v1/chat/completions -Method Post -Body $body -ContentType "application/json" -Headers @{ "x-api-key" = $env:MERIDIAN_PROJECT_KEY }
```

**Because the API is OpenAI-compatible, existing SDKs work with a base-URL change** —
with one adjustment. The SDKs send their `api_key` as `Authorization: Bearer`, which the
gateway rejects for a project key, so pass the key as a default header instead:

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8000/v1",
    api_key="unused",  # the gateway reads x-api-key, not the Authorization header
    default_headers={"x-api-key": "your-project-key"},
)
print(client.chat.completions.create(
    model="gemini-2.5-flash",
    messages=[{"role": "user", "content": "Say hello in five words."}],
).choices[0].message.content)
```

```javascript
import OpenAI from "openai";

const client = new OpenAI({
  baseURL: "http://localhost:8000/v1",
  apiKey: "unused", // the gateway reads x-api-key, not the Authorization header
  defaultHeaders: { "x-api-key": "your-project-key" },
});
const res = await client.chat.completions.create({
  model: "gemini-2.5-flash",
  messages: [{ role: "user", content: "Say hello in five words." }],
});
console.log(res.choices[0].message.content);
```

A successful response carries `x-gaussmeridian-provider-selected: google` — that header
is how you confirm routing actually happened rather than being served from cache or a
mock. Verified end-to-end in
[`docs/evidence/gemini/report.md`](docs/evidence/gemini/report.md).

Streaming works the same way: add `"stream": true` and read the SSE response.

Every registered route is live-executed and captured in
[`docs/evidence/report.md`](docs/evidence/report.md) — that pack, not this README, is
the authoritative list of what the API does.

---

## 🗄️ Provider catalog

**Only Google (Gemini) routes out of the box.** This is the single most likely thing to
confuse a first-time user, so it is stated plainly:

| Provider  | With a valid key | Why                                                                    |
| --------- | ---------------- | ----------------------------------------------------------------------- |
| Google    | ✅ Routes         | Its `gaussmeridian.toml` entry matches the seeded routing catalog        |
| OpenAI    | ❌ 503            | The `models` allowlist in `gaussmeridian.toml` shares no model ids with the seeded catalog |
| Anthropic | ❌ 503            | Allowlist lists `claude-3-opus`/`-sonnet`/`-haiku`; the adapter reports dated ids like `claude-3-haiku-20240307` |
| Mock      | ❌ 503            | Registered as the `openai` adapter, so it inherits OpenAI's stale allowlist |

The root cause is shared: the allowlists in `gaussmeridian/gaussmeridian.toml` are stale
relative to the real provider catalogs. The failure surfaces as HTTP 503 with
`no_hard_eligible_models`.

This is an **escalated, known gap awaiting a project-owner decision** — the provider
catalog is a governed surface under [`GOVERNANCE.md`](GOVERNANCE.md), so it is not
something a contributor can simply patch. Tracked in
[`docs/evidence/report.md`](docs/evidence/report.md) under "Known blocker".

To get a working inference today:

```bash
echo 'GEMINI_API_KEY=your-key' >> .env
docker compose up -d
```

Get a Gemini key from Google AI Studio. No GaussMeridian account is involved — the key
is yours, stays in your `.env`, and is never sent anywhere but Google.

---

## ✅ Verify your install

Run these in order. Each one tells you something the previous one did not.

| # | Command                                             | Expected                                     |
| - | --------------------------------------------------- | -------------------------------------------- |
| 1 | `docker compose ps`                                 | Four services `healthy` — gateway, SurrealDB, Redis, mock provider. The WebUI, Prometheus and Grafana appear only with their profiles |
| 2 | `curl http://localhost:8000/health`                 | `{"status":"healthy","version":"3.0.0",...}` |
| 3 | `curl http://localhost:8000/ready`                  | 200 — at least one provider is callable      |
| 4 | `curl -H "x-api-key: $KEY" http://localhost:8000/health/providers` | Per-provider up/down detail. This route needs a credential; without one it returns `401` |
| 5 | `curl -i http://localhost:8000/ \| grep -i x-source-offer` | The AGPL §13 source offer header       |
| 6 | `curl http://localhost:8000/v1/models -H "x-api-key: $KEY"` | The routable model list. `$KEY` is a project key from [Your first API call](#-your-first-api-call) — **not** `GAUSSMERIDIAN_API_KEY` |
| 7 | A chat completion — see [above](#-your-first-api-call) | 200 with `x-gaussmeridian-provider-selected` |

For the source tree rather than the running stack:

```bash
cd gaussmeridian && cargo test --workspace --locked
cd ../webui && pnpm typecheck && pnpm test
```

---

## 🧯 Troubleshooting

| Symptom | Cause | Fix |
| ------- | ----- | --- |
| `docker compose up` fails complaining about an unset variable | A `:?` variable is missing from `.env` | `cp .env.example .env`. Note `GRAFANA_PASSWORD` must stay uncommented even without the observability profile |
| `docker-compose: command not found` | Compose v1 | Use `docker compose` (space). Install the v2 plugin |
| `401 invalid credentials` when using `GAUSSMERIDIAN_API_KEY` | That variable is the TUI client's credential; the gateway never reads it | Register a user and create a project key — see [Your first API call](#-your-first-api-call) |
| `project_scope_required` on a chat completion | The API key is not scoped to a project | Recreate the key with a `project_id` |
| `payment_required` / `budget_exceeded` on a chat completion | A brand-new project has no budget, so routing is refused before a provider is contacted | Expected on a fresh install. Fund the project, or use the WebUI's billing flow |
| `503` with `no_hard_eligible_models` on a chat completion | The stale provider allowlist | Not your setup. See [Provider catalog](#-provider-catalog); use a Gemini key |
| Every `/v1/admin/*` route returns `404` | `SUPERADMIN_EMAILS` is unset | Set it to your operator email in `.env` and restart. 404 rather than 403 is deliberate |
| `/ready` returns non-200 while `/health` is fine | No provider is callable yet | Expected on a cold start; check `docker compose logs gaussmeridian` and `/health/providers` |
| Gateway starts but cannot reach Redis | The credential was passed as `GAUSSMERIDIAN_REDIS_URL` | The binary reads `REDIS_URL` and nothing else. Rename it |
| WebUI loads but you are bounced to login forever | `localhost` versus `127.0.0.1` mismatch | Use `127.0.0.1` for both WebUI and backend URLs — the session cookie is host-only |
| WebUI port 3000 already in use | Grafana's observability profile holds host 3000 | The Dockerized WebUI is on **3001**; local `pnpm dev` uses 3000 |
| First build takes an extremely long time | No prebuilt images; Docker compiles 600+ crates and the Next.js app | Expected once. Do not interrupt it — a partial build cache makes the retry slower |
| `pnpm install` resolves a different tree than CI | Installed with npm or yarn | Use pnpm; `pnpm-lock.yaml` is the lockfile of record |
| Database is in a bad state after schema changes | Retained volume from an earlier run | `docker compose down -v` deletes the volumes. **This destroys your local data** |

Still stuck? [`SUPPORT.md`](SUPPORT.md) explains where to file. Include what you ran,
what you expected, `docker compose logs gaussmeridian` with keys redacted, and your OS
and Docker versions.

---

## 📦 Project Structure

```text
GaussMeridian/
├── gaussmeridian/                 # Main Rust workspace (Cargo.toml is here)
│   ├── crates/                  # Core library crates
│   │   ├── gaussmeridian-core/    # Routing engine, load balancing
│   │   ├── gaussmeridian-providers/ # LLM provider adapters
│   │   ├── gaussmeridian-moa/     # Multi-agent orchestration (8 strategies)
│   │   ├── gaussmeridian-cache/   # Caching (custom MemoryCache active; Moka/Redis declared)
│   │   ├── gaussmeridian-auth/    # JWT, OAuth2, RBAC authentication
│   │   ├── gaussmeridian-metrics/ # Prometheus metrics, observability
│   │   ├── gaussmeridian-plugins/ # Plugin system
│   │   ├── gaussmeridian-config/  # Configuration management
│   │   └── gaussmeridian-db/      # SurrealDB integration
│   ├── services/
│   │   ├── server/              # HTTP API gateway (Axum)
│   │   └── tui/                 # Terminal admin interface (Ratatui)
│   ├── gaussmeridian.toml       # Provider + routing configuration
│   └── tests/                   # Integration & E2E tests
├── webui/                       # Web console (Next.js) — separate pnpm project
├── docs/                        # Documentation — the only docs that exist
│   ├── operations/              # Runbooks: native preview, qualification, startup
│   ├── codebase-report/         # Onboarding and current status reports
│   ├── knowledge/               # Shared knowledge base (AI agents + humans)
│   ├── research-documents/      # SOTA research papers and HTML briefs
│   ├── evidence/                # Endpoint evidence pack (docs/evidence/report.md)
│   ├── ADMIN_GUIDE.md           # System administration
│   ├── USER_GUIDE.md            # User documentation
│   └── PERFORMANCE_PROFILING.md # Performance tuning
├── docker-compose.yml           # Path A — clone-and-run stack
├── docker-compose.native-preview.yml # Path C — contributor qualification preview
├── .env.example                 # Environment variable template (read the comments)
├── LICENSE                      # GNU AGPL v3.0, full text
├── NOTICE                       # Copyright, §7(e) trademark term, §13 source offer
├── THIRD_PARTY_NOTICES.md       # Dependency inventory + AGPL-compatibility assessment
└── README.md                    # This file
```

---

## 🛠️ Core Components

### Services

| Service                | Description                          | Technology         |
| ---------------------- | ------------------------------------ | ------------------ |
| **gaussmeridian-server** | HTTP API gateway (OpenAI-compatible) | Axum, Tokio        |
| **gaussmeridian-tui**    | Terminal management interface        | Ratatui, Crossterm |
| **gaussmeridian-webui**  | Web-based admin console              | Next.js            |

### Core Crates

| Crate                   | Purpose                                                                                                         |
| ----------------------- | --------------------------------------------------------------------------------------------------------------- |
| `gaussmeridian-core`      | Router engine, load balancing, circuit breakers                                                                 |
| `gaussmeridian-providers` | OpenAI, Anthropic, Cohere, Ollama, vLLM adapters                                                                |
| `gaussmeridian-moa`       | Multi-agent orchestration with 8 strategies                                                                     |
| `gaussmeridian-cache`     | Moka L1 exact-match cache (active, wired into `AppState`), SurrealDB L3 semantic tier, and a Redis-backed sliding-window rate limiter |
| `gaussmeridian-auth`      | JWT, API keys, OAuth2, RBAC                                                                                     |
| `gaussmeridian-metrics`   | Prometheus metrics, health checks                                                                               |
| `gaussmeridian-db`        | SurrealDB integration                                                                                           |

Every crate is licensed `AGPL-3.0-only`, inherited from `[workspace.package]`. There is
no permissively-licensed subset.

---

## 🎯 Features

### Intelligent Routing

- ⚡ **Cost-optimized** - Route to cheapest provider
- 🏎️ **Latency-aware** - Prioritize the lowest-latency provider observed
- ⚖️ **Load-balanced** - Distribute across providers
- 🔄 **Automatic failover** - Circuit breakers + fallback
- 🧪 **A/B testing** - Test routing strategies

### Multi-Agent Orchestration (MoA)

Eight strategies for combining multiple agent calls into one response:

1. **Standard** - Confidence-based selection
2. **Attention** - Multi-head attention weighting
3. **Debate** - Multi-round consensus
4. **Roles** - Specialized agent roles
5. **Sparse** - Top-K filtering
6. **Self-MoA** - Iterative refinement
7. **Collaborative** - Shared context
8. **Adaptive** - Learning from history

### Enterprise Features

- 🔐 **Authentication**: JWT, API keys, OAuth2
- 👥 **Multi-tenancy**: Complete tenant isolation
- 📊 **Resource quotas**: Per-tenant limits
- 🚦 **Rate limiting**: Sliding window algorithm
- 📝 **Audit logging**: Complete request tracking
- 🛡️ **RBAC**: Role-based access control

### Observability

- 📈 **Prometheus metrics**: Request/latency/error tracking
- 📋 **Structured logging**: JSON logs with tracing
- 🔍 **Distributed tracing**: Request flow visualization
- ❤️ **Health checks**: Liveness and readiness probes

---

## 🧠 Routing Intelligence Phase Map

Paper names are used only for the data object, objective, and evidence boundary actually implemented.
Legacy complexity, skill-vector, routing-score, and billing identifiers do not establish paper fidelity.

**PRD-26 closed at P0–P6 functional scope on 2026-08-16.** All six acceptance criteria hold.
That closes the **functional** requirement only — it does not close P6 market qualification,
and no reader may infer an efficacy or market claim from it.

| Boundary | Role | Current state |
| --- | --- | --- |
| **Meridian P1** | Versioned deterministic complexity evidence and immutable eligible ballot | Accepted at its controlled-mechanism boundary |
| **CARROT P2** | Conditional per-model outcome and cost prediction | Accepted at its controlled-mechanism boundary; production promotion blocked |
| **BELLA P3** | Skill decomposition, proficiency, critic evidence, uncertainty, and attribution | Accepted at its controlled-mechanism boundary; production promotion blocked |
| **R2-Router P4** | Governed budget-aware degradation policy | Accepted at its controlled-mechanism boundary; production promotion blocked |
| **xRouter P5** | Compound routing action/scoring policy; not customer-billing authority | Accepted at its controlled-mechanism boundary; production promotion blocked |
| **Bandit/optimizer P6** | Governed online learning and release controls | Functional scope closed (isolated, zero-spend). **Market qualification open** |
| **GaussMoA** | Separate multi-agent service repair and product track | Deferred from PRD-26 scope |

"Accepted at its controlled-mechanism boundary" means the mechanism is implemented, wired into
the runtime, and passed its recorded acceptance in isolated testing. It is **not** representative
efficacy, production promotion, or market superiority — none of which has been established or
authorized for any boundary above. Every one of these paths still fails closed at runtime with
`production_promotion_blocked` unless a qualification-state identity is explicitly supplied.

What remains genuinely open is P6 **market** qualification — representative efficacy, independent
reproduction, and rollout authority — tracked as MG-RI-BL-008, MG-RI-BL-009, and MG-RI-BL-011 and
descoped to a future mission. That is a governance gate, not unwritten code.

Naming caveat carried from the engineering spec: historical terminology does not establish present
CARROT, BELLA, xRouter, or source-unverified R2 fidelity to the published papers. The `R2*`
identifiers name a Meridian P4 compatibility surface only.

Research sources: [`docs/research-documents/`](docs/research-documents/)

---

## 🖥️ Terminal User Interface (TUI)

GaussMeridian includes a terminal-based admin interface. Run it from `gaussmeridian/`
against a running gateway:

```bash
cd gaussmeridian
GAUSSMERIDIAN_API_URL=http://localhost:8000 cargo run --bin gaussmeridian-tui
```

**Features:**

- Real-time dashboard with system metrics
- Provider health monitoring
- Request tracking and analysis
- Log viewer with filtering
- Multi-tenant administration

See [Admin Guide - TUI Section](docs/ADMIN_GUIDE.md#terminal-user-interface-tui) for details.

---

## 📚 Documentation

All of it is in this repository. There is no external docs site.

| Document                                                                                   | Description                                            |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------ |
| [CONTRIBUTING.md](CONTRIBUTING.md)                                                         | Contribution licensing and the supported native workflow |
| [docs/operations/native-community-preview.md](docs/operations/native-community-preview.md) | Native preview operations and qualification          |
| [docs/operations/service-startup-runbook.md](docs/operations/service-startup-runbook.md)   | Service startup runbook                                |
| [docs/evidence/report.md](docs/evidence/report.md)                                         | Endpoint evidence pack — every registered route, live-executed and captured |
| [ROADMAP.md](ROADMAP.md)                                                                   | Current status, what's shipped, and what's explicitly out of scope |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md)                                                   | Complete user documentation                            |
| [docs/ADMIN_GUIDE.md](docs/ADMIN_GUIDE.md)                                                 | Administration and operations                          |
| [docs/codebase-report/00-INDEX.md](docs/codebase-report/00-INDEX.md)                       | Full codebase report index                             |
| [docs/codebase-report/09-MVP-HUMAN-STATUS.md](docs/codebase-report/09-MVP-HUMAN-STATUS.md) | Historical M1 status vs build plan                     |
| [docs/research-documents/](docs/research-documents/)                                       | Research HTML documents (CARROT, BELLA, xRouter, etc.) |
| [LICENSE](LICENSE) · [NOTICE](NOTICE) · [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)   | License text, copyright/§13 offer, dependency inventory |

> `QUICK_START.md`, `ASSESSMENT.md`, and `ARCHITECTURE.md` are pre-existing project
> documents intentionally not linked above pending a separate cleanup or removal pass:
> `QUICK_START.md` and `ASSESSMENT.md` are stale (wrong ports, a "Production Ready"
> self-grade that predates and contradicts this README's own build-truth table,
> and — in `ASSESSMENT.md` — a named-competitor comparison this launch explicitly
> retracts); `ARCHITECTURE.md` opens with the same unsupported superiority claim
> against the same named competitor. None of the three is rewritten here — which
> documents ship is a separate, product-owner-owned decision. **Prefer this README
> over all three** where they disagree.

---

## 🤝 Contributing

Contributions are welcome. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) first — in
particular its licensing section, because **contributing to this repository licenses
your work under AGPL-3.0-only** and there is no CLA to sign or copyright to assign.

```bash
# The gates a pull request is checked against
cd gaussmeridian
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Then follow the
[`docs/operations/native-community-preview.md`](docs/operations/native-community-preview.md)
qualification contract.

---

## 👥 Community & Governance

- [`SECURITY.md`](SECURITY.md) — how to report a vulnerability
- [`SUPPORT.md`](SUPPORT.md) — where to ask questions and file issues
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) — community standards
- [`GOVERNANCE.md`](GOVERNANCE.md) — decision-making and maintainer roles
- [`ROADMAP.md`](ROADMAP.md) — what's shipped, in progress, and explicitly out of scope
- [`TRADEMARKS.md`](TRADEMARKS.md) — name and mark usage (an AGPL §7(e) term)

---

## 📄 License

Copyright (C) 2026 Gaussian.

Licensed under the **GNU Affero General Public License v3.0 only**
(`AGPL-3.0-only`) — see [`LICENSE`](LICENSE) for the full text. GaussMeridian was
previously distributed under Apache-2.0; AGPL-3.0-only supersedes that for this and
every later revision.

### What AGPL means for you here

GaussMeridian is a gateway — you almost certainly reach it over a network rather than
by running a copy on your own desk. That is exactly the case AGPL Section 13 covers,
so read this part rather than assuming it behaves like a permissive license:

- **Self-hosting it unmodified, for yourself or your company** — nothing is asked of
  you. Internal use is not distribution and triggers no obligation.
- **Modifying it and letting anyone reach that instance over a network** — including
  your own users, and including an internal-facing deployment reachable by people
  other than you — you must offer those users the Corresponding Source of your
  modified build, free of charge. Point `SOURCE_OFFER_URL` (gateway) and
  `NEXT_PUBLIC_SOURCE_OFFER_URL` (WebUI) at it; both are already wired to publish the
  offer for you. Details in [`NOTICE`](NOTICE).
- **Building your own software on top of it** — code that links GaussMeridian's
  crates into your binary forms a combined work and must itself be AGPL-3.0. Calling
  the gateway over HTTP as a separate program does not, and carries no such
  obligation. If you need it under other terms, ask — see [`SUPPORT.md`](SUPPORT.md).

Every crate in the `gaussmeridian/` workspace and the `webui/` application is
AGPL-3.0-only. There is no permissively-licensed subset.

### Section 7 linking exception

Two dependencies are not GPL-compatible — GSAP (proprietary, free of charge) and
the SurrealDB Rust client (Business Source License 1.1). An AGPL Section 7
additional permission in [`NOTICE`](NOTICE) grants you permission to convey a work
combining this Program with either of them. You still have to obtain and comply
with those two libraries' own licenses separately; the permission only removes the
AGPL-side obstacle. Like the rest of the relicense, it has not cleared legal review
(governance gate G3).

[`NOTICE`](NOTICE) and [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) describe
this codebase's actual Rust and JavaScript dependency trees, generated with `cargo
tree` and `pnpm licenses list`, and assess each against AGPL-3.0 compatibility. Both
are marked pending legal review (governance gate G3) — treat them as an accurate
inventory, not yet as a legally-cleared attribution document. The relicense itself is
part of what G3 must clear.

---

## 💬 Support

There is no paid support tier and no staffed support team. See [`SUPPORT.md`](SUPPORT.md)
for the full policy. Short version:

- 🐛 **Issues**: [https://github.com/Gaussian-id/gaussmeridian/issues](https://github.com/Gaussian-id/gaussmeridian/issues)
- 💬 **Discussions**: [https://github.com/Gaussian-id/gaussmeridian/discussions](https://github.com/Gaussian-id/gaussmeridian/discussions)
- 🔒 **Security reports**: see [`SECURITY.md`](SECURITY.md) — do not open a public issue

---

<p align="center">
  <strong>GaussMeridian</strong> — a self-hostable, OpenAI-compatible LLM gateway
</p>
