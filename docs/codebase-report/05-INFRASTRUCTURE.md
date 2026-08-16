# 05 — Infrastructure & DevOps

## Docker Compose Architecture

**File:** `docker-compose.yml` (Compose v3.8)  
**Network:** Single bridge `gaussmeridian-network` (subnet `172.28.0.0/16`)

### Services

| Service | Image | Host Port | Purpose |
|---------|-------|-----------|---------|
| `gaussmeridian` | Build from `./gaussmeridian` (target: `runtime`) | 8000 (HTTP), 9090 (metrics) | Rust API server |
| `surrealdb` | `surrealdb/surrealdb:v2.0.0` | 8001 → 8000 | Database (file-backed) |
| `redis` | `redis:7-alpine` | 6379 | Cache (AOF, 256MB, allkeys-lru) |
| `prometheus` | `prom/prometheus:v2.48.0` | 9091 → 9090 | Metrics collection (15d retention) |
| `grafana` | `grafana/grafana:10.2.0` | 3000 | Dashboards |

### Service Dependencies

```
gaussmeridian
├── depends_on: surrealdb (healthy)
└── depends_on: redis (started)

prometheus
└── depends_on: gaussmeridian

grafana
└── depends_on: prometheus
```

### Volumes (Named, Persistent)

| Volume | Mount Target | Service |
|--------|-------------|---------|
| `surrealdb-data` | `/data` | surrealdb |
| `redis-data` | `/data` | redis |
| `prometheus-data` | `/prometheus` | prometheus |
| `grafana-data` | `/var/lib/grafana` | grafana |
| `gaussmeridian-logs` | `/app/logs` | gaussmeridian |

### Network Topology

```
  Host Machine
  ┌────────────────────────────────────────────────┐
  │                                                │
  │  :8000 ──► gaussmeridian ──ws──► surrealdb:8000  │
  │  :9090 ──►      │                              │
  │                  └─────────►  redis:6379       │
  │                                                │
  │  :8001 ──► surrealdb:8000                      │
  │  :6379 ──► redis:6379                          │
  │                                                │
  │  :9091 ──► prometheus:9090 ◄── scrape ──►      │
  │                    gaussmeridian:8000/metrics     │
  │                                                │
  │  :3000 ──► grafana:3000 ──► prometheus:9090    │
  │                                                │
  └────────────────────────────────────────────────┘
  All on bridge: gaussmeridian-network (172.28.0.0/16)
```

---

## Dockerfile (`gaussmeridian/Dockerfile`)

Multi-stage build:

### Stage 1: `builder` (from `rust:1.75-bookworm`)

- Installs system deps: `pkg-config`, `libssl-dev`, `cmake`, `clang`, `llvm`, `libclang-dev`
- Copies full workspace (Cargo.toml, crates/, services/, etc.)
- Runs `cargo build --release --bin gaussmeridian`

### Stage 2: `runtime` (from `debian:bookworm-slim`)

- Installs: `ca-certificates`, `libssl3`
- Creates non-root user `gaussmeridian`
- Copies binary and `gaussmeridian.toml` from builder
- Exposes ports: **8000** (HTTP), **9090** (metrics)
- Healthcheck: `curl http://localhost:8000/health`
- ENV: `RUST_LOG=info`, `GAUSSMERIDIAN_HOST=0.0.0.0`, `GAUSSMERIDIAN_PORT=8000`
- CMD: `["./gaussmeridian"]`

### Stage 3: `development` (from `runtime`)

- Adds: `curl`, `htop`
- Sets: `RUST_LOG=debug`, `RUST_BACKTRACE=1`

Compose uses **`target: runtime`** (production stage).

---

## Environment Variables (`.env.example`)

### Required

| Variable | Example | Purpose |
|----------|---------|---------|
| `SURREALDB_PASSWORD` | `your-secure-surrealdb-password` | SurrealDB root password |
| `JWT_SECRET` | `your-super-secret-jwt-key-minimum-32-characters` | JWT signing secret |
| `GAUSSMERIDIAN_API_KEY` | `gr-your-initial-api-key` | Bootstrap API key |
| `OPENAI_API_KEY` | `sk-your-openai-api-key` | OpenAI provider key |
| `ANTHROPIC_API_KEY` | `sk-ant-<your-anthropic-key>` | Anthropic provider key |

### Monitoring

| Variable | Example | Purpose |
|----------|---------|---------|
| `GRAFANA_USER` | `admin` | Grafana admin username |
| `GRAFANA_PASSWORD` | `your-secure-grafana-password` | Grafana admin password |

### Optional

| Variable | Purpose |
|----------|---------|
| `COHERE_API_KEY` | Cohere provider (commented out) |
| `REDIS_URL` | External Redis override. Deliberately unprefixed — `services/server/src/app.rs:815` reads `REDIS_URL` and nothing else. `GAUSSMERIDIAN_REDIS_URL` is read by no code in the workspace; setting it leaves the gateway on its built-in `redis://localhost:6379` default. |
| `SENTRY_DSN` | Error tracking |
| `AWS_REGION` / `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` | Cloud deployment |

### Compose-Injected Variables (gaussmeridian service)

These are set in `docker-compose.yml` for the gaussmeridian service:

```yaml
GAUSSMERIDIAN_HOST: "0.0.0.0"
GAUSSMERIDIAN_PORT: "8000"
GAUSSMERIDIAN_DB_URL: "ws://surrealdb:8000"
GAUSSMERIDIAN_DB_NAMESPACE: "gaussmeridian"
GAUSSMERIDIAN_DB_DATABASE: "main"
GAUSSMERIDIAN_DB_USERNAME: "root"
GAUSSMERIDIAN_DB_PASSWORD: "${SURREALDB_PASSWORD}"
REDIS_URL: "redis://:${REDIS_PASSWORD}@redis:6379"
JWT_SECRET: "${JWT_SECRET}"
GAUSSMERIDIAN_API_KEY: "${GAUSSMERIDIAN_API_KEY}"
OPENAI_API_KEY: "${OPENAI_API_KEY}"
ANTHROPIC_API_KEY: "${ANTHROPIC_API_KEY}"
RUST_LOG: "info,gaussmeridian=debug"
```

---

## CI/CD (GitHub Actions)

### `ci.yml` — Push/PR to `main` and `develop`

| Job | Steps |
|-----|-------|
| `lint` | `cargo fmt --check` + `cargo clippy -D warnings` |
| `test` | Build all features, run lib tests + integration tests (Ubuntu + macOS matrix) |
| `security` | `cargo audit` for vulnerability scanning |
| `docker` | Buildx build (no push), tagged `gaussmeridian:<sha>` — only on push to `main` |
| `docs` | `cargo doc --no-deps --all-features`, uploads as artifact |

### `release.yml` — On tag push `v*`

| Job | Steps |
|-----|-------|
| `build` | Cross-compile: Linux x86_64/ARM64, macOS x86_64/ARM64; tar.gz artifacts |
| `docker` | Push to GHCR (`ghcr.io/<repo>`), multi-arch, semver tags |
| `release` | GitHub Release with artifacts, auto release notes, prerelease for `-rc`/`-beta`/`-alpha` |

---

## Monitoring Stack

### Prometheus (`monitoring/prometheus/`)

**`prometheus.yml`:**
- Global scrape interval: 15s
- Self-scrape: `localhost:9090`
- GaussMeridian scrape: `gaussmeridian:8000` on `/metrics` (10s interval)
- Redis and SurrealDB exporters: commented out

**Potential issue:** Compose exposes metrics on host port 9090 from the `gaussmeridian` service, but `prometheus.yml` scrapes `gaussmeridian:8000/metrics`. If the Rust server serves metrics on port 9090 (separate from the API on 8000), the scrape target may need to be `gaussmeridian:9090` instead.

**`alerts.yml`:**

Comprehensive alert rules covering:
- Error rate > threshold
- Latency percentiles (p95, p99)
- Service down
- Database connection pool exhaustion
- Memory / CPU usage
- Cache hit rate drops
- Rate limit breaches
- Provider health / latency / errors
- Cost per request anomalies
- Traffic spikes
- Disk usage (requires node-exporter, not deployed)

No Alertmanager service is deployed — alerts are defined but have nowhere to route.

### Grafana (`monitoring/grafana/`)

**Datasource:** Prometheus at `http://prometheus:9090` (auto-provisioned)  
**Dashboards:** Provisioning config points to `/etc/grafana/provisioning/dashboards/`, but **no JSON dashboard files exist in the repo**. Grafana will show an empty folder until dashboards are created.

---

## Load Testing (`load_tests/`)

**`k6_load_test.js`:**
- Tool: [k6](https://k6.io/) (Grafana's load testing tool)
- Stages: Ramp 10 → 50 → 100 → 200 → 0 VUs
- Thresholds: p95 < 500ms, error rate < 1%
- Endpoints tested: `/health`, `/v1/models`, `/v1/chat/completions`, `/v1/balance`
- Auth: Optional JWT from `/v1/auth/register` in setup phase

**Default `BASE_URL`:** `http://localhost:3000` — **This is wrong for Docker** (API is on 8000, 3000 is Grafana). Override with `BASE_URL=http://localhost:8000`.

---

## Management Script (`gaussmeridian-manage.sh`)

Bash script for local (non-Docker) development:

| Command | What It Does |
|---------|-------------|
| `./gaussmeridian-manage.sh start` | Starts backend + frontend |
| `./gaussmeridian-manage.sh stop` | Stops all services |
| `./gaussmeridian-manage.sh restart` | Restarts services |
| `./gaussmeridian-manage.sh status` | Shows running status |
| `./gaussmeridian-manage.sh logs` | Tails log files |

**Targets:** `backend`, `frontend` (alias: `webui`, `front`), `all` (default)

**Backend:** `cargo run --bin gaussmeridian --release` in `gaussmeridian/`  
**Frontend:** `deno task start` in `gaussmeridian/services/webui/`

**Platform note:** Written for bash/macOS. Will not work natively on Windows — requires Git Bash, WSL, or similar.

**Issue:** Frontend command uses `deno task start`, but the active frontend is **Next.js** (should be `npm run dev` or `npx next dev`).

---

## Existing Documentation (`docs/`)

### Root `docs/`

| File | Content |
|------|---------|
| `ADMIN_GUIDE.md` | Installation (Docker, K8s, binary), configuration, DB, TUI, monitoring, security, backup |
| `USER_GUIDE.md` | Product intro, authentication, API usage, rate limits, billing |
| `PLUGIN_MARKETPLACE.md` | Plugin architecture, metadata format, marketplace flows |
| `PERFORMANCE_PROFILING.md` | Flamegraphs, perf tools, valgrind, heaptrack |

### `gaussmeridian/docs/`

| File | Content |
|------|---------|
| `SPECS.md` | Technical specifications |
| `SECURITY.md` | Security architecture and practices |
| `OBSERVABILITY.md` | Monitoring and observability guide |
| `PROD-DEPLOYMENT.md` | Production deployment instructions |
| `PROVIDER-DEV.md` | How to add new LLM providers |
| `PLUGIN-DEV.md` | Plugin development guide |
| `development/SETUP.md` | Dev environment setup |
| `development/DEVROADMAP.md` | Development roadmap |

### `gaussmeridian/crates/gaussmeridian-moa/docs/`

| File | Content |
|------|---------|
| `strategies.md` | MoA strategy descriptions |
| `user_guide.md` | MoA usage guide |
