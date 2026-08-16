# 01 — Project Overview

## What Is GaussMeridian?

GaussMeridian is a **Rust-based LLM API gateway** — essentially an OpenRouter clone built with Rust for the core backend instead of TypeScript. It provides:

- **OpenAI-compatible API** — Drop-in replacement; clients using OpenAI SDKs can point their `base_url` to GaussMeridian
- **Multi-provider routing** — Routes requests to OpenAI, Anthropic, Ollama, and others via a unified interface
- **Mixture of Agents (MoA)** — A multi-agent orchestration system with 8 strategies (debate, voting, layered, etc.)
- **Enterprise features** — Multi-tenancy, RBAC, billing/usage tracking, caching, rate limiting, circuit breakers
- **Web UI** — A Next.js dashboard for managing keys, viewing logs, monitoring usage
- **TUI** — A terminal UI built with Ratatui for server monitoring

## Tech Stack

### Backend (Rust)

| Layer | Technology |
| --- | --- |
| HTTP framework | Axum 0.7 + Tower middleware |
| Async runtime | Tokio (full features) |
| Database | SurrealDB 2.0 (WebSocket/HTTP protocol) |
| Cache | Custom `MemoryCache` currently active; Moka declared but unused; Redis implementation exists but is not wired at server startup |
| Auth | JWT (jsonwebtoken) + API keys + RBAC |
| Metrics | Prometheus + custom metrics |
| Streaming | reqwest-eventsource for SSE, async-stream |
| Config | config crate + dotenvy + clap CLI |
| Plugins | libloading + dynamic_reload (optional feature) |

### Frontend (Next.js)

| Layer | Technology |
| --- | --- |
| Framework | Next.js 16.x (App Router) |
| UI components | shadcn/ui (Radix primitives + Tailwind) |
| Styling | Tailwind CSS v4 (CSS-based config, OKLCH tokens) |
| Forms | React Hook Form + Zod |
| Charts | Recharts |
| Auth | React Context + localStorage (JWT) |
| Theming | next-themes (dark mode default) |

### Infrastructure

| Component | Technology |
| --- | --- |
| Database | SurrealDB v2.0.0 |
| Cache | Redis 7 Alpine |
| Monitoring | Prometheus 2.48 + Grafana 10.2 |
| Container | Docker + Docker Compose |
| CI/CD | GitHub Actions (lint, test, security audit, Docker, release) |

## Architecture (High Level)

```text
                    ┌─────────────┐
                    │   Web UI    │  (Next.js, port 3000 dev)
                    │  /console   │
                    └──────┬──────┘
                           │ HTTP (fetch)
                           ▼
┌──────────┐     ┌─────────────────┐     ┌───────────────┐
│  Clients │────►│  GaussMeridian    │────►│  LLM Providers│
│ (OpenAI  │     │  (Rust/Axum)    │     │  - OpenAI     │
│  SDK)    │◄────│  Port 8000      │◄────│  - Anthropic  │
└──────────┘     └──┬──────┬───┬──┘     │  - Ollama     │
                    │      │   │         └───────────────┘
              ┌─────┘      │   └─────┐
              ▼            ▼         ▼
        ┌──────────┐ ┌─────────┐ ┌───────┐
        │ SurrealDB│ │  Redis  │ │Metrics│
        │ Port 8001│ │Port 6379│ │ /metrics
        └──────────┘ └─────────┘ └───┬───┘
                                     ▼
                              ┌────────────┐
                              │ Prometheus  │──► Grafana
                              │ Port 9091   │    Port 3000
                              └────────────┘
```

## Request Flow (Chat Completion)

```text
1. Client sends POST /v1/chat/completions
2. Auth: Extract x-api-key or Bearer JWT → validate → AuthContext
3. Rate limiting check (in-core, not middleware)
4. Cache lookup (semantic hash of request)
5. If cache miss → Provider routing:
   a. Iterate registered providers (sequential fallback)
   b. First provider that succeeds returns the response
   c. Circuit breaker tracks provider health
6. Response processing: token counting, cost calculation
7. Cache write (if enabled)
8. Usage/billing recorded to SurrealDB (if connected)
9. Return OpenAI-compatible JSON response
```

### Current M1 Gap Snapshot (2026-05-10)

This flow is the intended shape, but several foundation pieces are not yet active:

- Auth, rate limiting, and request validation middleware functions exist but are not layered on the Axum app.
- Redis cache implementation exists, but `create_application` starts only `MemoryCache`.
- Streaming handler exists but has no registered route.
- BYOK key storage is missing; provider keys still come from environment variables.
- MVP SurrealDB tables (`org`, `team`, `project`, `provider_model`, `ledger_entry`, `cache_entry`) are not present yet.
- Docker Compose, `/health`, `/metrics`, and HTTP `TraceLayer` logging are already present.

For the human-readable M1 status, see `09-MVP-HUMAN-STATUS.md`.

## Crate Dependency Graph

```text
services/server ──► gaussmeridian-core
                ──► gaussmeridian-auth
                ──► gaussmeridian-config
                ──► gaussmeridian-db
                ──► gaussmeridian-providers
                ──► gaussmeridian-cache
                ──► gaussmeridian-metrics
                ──► gaussmeridian-models
                ──► gaussmeridian-plugins (optional)

gaussmeridian-core ──► gaussmeridian-models
                 ──► gaussmeridian-cache
                 ──► gaussmeridian-providers (trait)
                 ──► gaussmeridian-metrics

gaussmeridian-providers ──► gaussmeridian-models

gaussmeridian-auth ──► gaussmeridian-db (optional)

gaussmeridian-db ──► gaussmeridian-models
```

## What Makes This Different from OpenRouter

According to the codebase goals:

1. **Rust performance** — Targets <5ms routing overhead, 10K+ concurrent connections
2. **MoA (Mixture of Agents)** — Built-in multi-agent orchestration (not in OpenRouter)
3. **Self-hosted** — Designed for on-premise deployment vs. OpenRouter's SaaS model
4. **SurrealDB** — Graph-capable database instead of traditional PostgreSQL
5. **TUI** — Terminal-based monitoring interface
