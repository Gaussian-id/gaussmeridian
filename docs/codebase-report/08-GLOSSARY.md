# 08 — Glossary

## Project-Specific Terms

| Term | Definition |
|------|-----------|
| **GaussMeridian** | This project — a Rust-based LLM API gateway, aiming to be an OpenRouter alternative |
| **OpenRouter** | Third-party SaaS LLM gateway (openrouter.ai) that this project is modeled after |
| **MoA** | Mixture of Agents — multi-agent orchestration where multiple LLM calls are combined using strategies like debate, voting, or layered processing |
| **GaussMoA** | The MoA subsystem within GaussMeridian (crate: `gaussmeridian-moa`) |
| **Provider** | An LLM service backend (OpenAI, Anthropic, Ollama, etc.) that GaussMeridian routes requests to |
| **Provider Registry** | `DashMap`-based in-memory store of registered LLM providers |
| **Circuit Breaker** | Pattern that stops sending requests to a failing provider after N errors, with auto-recovery |
| **Fallback Routing** | Current routing strategy: try providers sequentially until one succeeds |
| **AuthContext** | The authenticated identity extracted from a request (user ID, tenant ID, permissions) |
| **Tenant** | Multi-tenancy concept — an organization or team that groups users and has its own rate limits/billing |
| **AppState** | Axum's shared state containing the router, auth manager, config, and optional DB client |
| **Island** | Frontend pattern — a client-side interactive component (`islands/` directory) that fetches its own data |

## Architecture Terms

| Term | Definition |
|------|-----------|
| **Cargo Workspace** | Rust's monorepo pattern — multiple crates (`gaussmeridian-core`, `gaussmeridian-auth`, etc.) sharing a root `Cargo.toml` |
| **Crate** | A Rust package (library or binary) — the unit of compilation |
| **Axum** | The Rust web framework used for the HTTP server (built on Tokio + Tower + Hyper) |
| **Tower** | Rust middleware framework that Axum uses for layers (CORS, tracing, rate limiting) |
| **DashMap** | A concurrent HashMap implementation used for the provider registry |
| **Moka** | An in-memory caching library for Rust (used for response caching) |
| **SurrealDB** | A multi-model database (document + graph + relational) used as the primary datastore |
| **Ratatui** | Rust TUI framework used for the terminal monitoring dashboard |

## Frontend Terms

| Term | Definition |
|------|-----------|
| **App Router** | Next.js routing system based on the `app/` directory (as opposed to the older `pages/` directory) |
| **shadcn/ui** | Component collection using Radix primitives styled with Tailwind — the UI kit used throughout |
| **Radix** | Headless UI component library (provides behavior, no default styling) — foundation for shadcn |
| **OKLCH** | A perceptual color space used for design tokens in `globals.css` |
| **Deno/Fresh** | A JavaScript runtime + web framework — the **previous** frontend stack (now dead code in `routes/`) |

## API Terms

| Term | Definition |
|------|-----------|
| **OpenAI-Compatible** | API follows OpenAI's request/response format so existing SDKs work by changing `base_url` |
| **Chat Completion** | `POST /v1/chat/completions` — the primary LLM endpoint (messages in, response out) |
| **SSE** | Server-Sent Events — protocol for streaming LLM responses token by token |
| **Bearer Token** | JWT passed in `Authorization: Bearer <token>` header |
| **API Key** | Alternative auth via `x-api-key` header, prefixed with `gr-` |

## Infrastructure Terms

| Term | Definition |
|------|-----------|
| **Docker Compose** | Multi-container orchestration — defines the full stack (Rust API, SurrealDB, Redis, Prometheus, Grafana) |
| **Multi-stage Build** | Dockerfile pattern: `builder` stage compiles, `runtime` stage only contains the binary |
| **GHCR** | GitHub Container Registry — where Docker images are pushed on release |
| **k6** | Grafana's open-source load testing tool (used in `load_tests/`) |
| **Prometheus** | Time-series database for metrics collection (scrapes `/metrics` endpoint) |
| **Grafana** | Visualization platform for metrics dashboards |

## MoA Strategies

The MoA system supports 8 orchestration strategies (defined in `gaussmeridian-moa`):

| Strategy | How It Works |
|----------|-------------|
| **Debate** | Multiple agents argue and refine through rounds |
| **Voting** | Multiple agents answer, majority/best wins |
| **Layered** | Agents process sequentially, each refining the previous output |
| **Mixture** | Combines outputs from multiple agents |
| **Expert Panel** | Specialized agents for different aspects of a query |
| **Chain of Thought** | Sequential reasoning pipeline |
| **Consensus** | Agents iterate until they agree |
| **Tournament** | Bracket-style elimination of responses |

## File Conventions

| Pattern | Meaning |
|---------|---------|
| `*.rs` | Rust source file |
| `*.tsx` | TypeScript React component |
| `*.ts` | TypeScript module (non-React) |
| `Cargo.toml` | Rust package manifest |
| `package.json` | Node.js package manifest |
| `mod.rs` | Rust module root (re-exports) |
| `lib.rs` | Rust library crate entry point |
| `main.rs` | Rust binary entry point |
| `page.tsx` | Next.js App Router page component |
| `layout.tsx` | Next.js App Router layout wrapper |
| `loading.tsx` | Next.js App Router loading skeleton |
| `error.tsx` | Next.js App Router error boundary |
