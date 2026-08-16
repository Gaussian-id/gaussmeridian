# 02 — Directory Structure

## Root Level

```
GaussMeridian/0. Code/
├── .github/workflows/           # CI/CD pipelines
│   ├── ci.yml                   # Lint, test, security audit, Docker build
│   └── release.yml              # Multi-platform release + GHCR push
├── .runtime/                    # Runtime state (PIDs, logs) — gitignored in practice
│   ├── logs/
│   └── pids/
├── docs/                        # User-facing documentation
│   ├── ADMIN_GUIDE.md
│   ├── USER_GUIDE.md
│   ├── PLUGIN_MARKETPLACE.md
│   ├── PERFORMANCE_PROFILING.md
│   └── codebase-report/         # THIS REPORT
├── gaussmeridian/                 # ★ Main Rust workspace + frontend
├── load_tests/                  # k6 load testing
│   ├── k6_load_test.js
│   └── README.md
├── monitoring/                  # Prometheus + Grafana configs
│   ├── prometheus/
│   │   ├── prometheus.yml       # Scrape config
│   │   └── alerts.yml           # Alert rules
│   └── grafana/
│       └── provisioning/
│           ├── datasources/datasources.yml
│           └── dashboards/dashboards.yml
├── .env.example                 # Environment variables template
├── docker-compose.yml           # Full stack orchestration
├── gaussmeridian-manage.sh        # Bash management script (start/stop/status)
├── ARCHITECTURE.md              # Detailed system architecture (~1200 lines)
├── ASSESSMENT.md                # Completeness assessment (Jan 2026)
├── EXECUTION_PLAN.md            # 30-hour implementation plan
├── IMPLEMENTATION.md            # Implementation status notes
├── QUICK_START.md               # Quick start guide
├── README.md                    # Project README
├── REVIEW.md                    # Production readiness review (Dec 2025)
├── TODO.md                      # Phase-based TODO tracker
├── ARCHITECTURE.tex             # LaTeX version of architecture doc
├── SPECIFICATION.tex            # LaTeX specification doc
└── GaussMeridian.pdf           # PDF documentation
```

## Rust Workspace (`gaussmeridian/`)

```
gaussmeridian/
├── Cargo.toml                   # Workspace root (12 members)
├── Dockerfile                   # Multi-stage: builder → runtime → development
├── gaussmeridian.toml             # Default app config file
├── tests/
│   └── api_tests.rs             # Integration tests
├── docs/                        # Developer docs
│   ├── SPECS.md
│   ├── SECURITY.md
│   ├── OBSERVABILITY.md
│   ├── PROD-DEPLOYMENT.md
│   ├── PROVIDER-DEV.md
│   ├── PLUGIN-DEV.md
│   └── development/
│       ├── SETUP.md
│       └── DEVROADMAP.md
│
├── crates/                      # ★ Library crates
│   ├── gaussmeridian-core/        # Router engine, provider registry, load balancing
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── router.rs        # Core routing logic + streaming + fallback
│   │       ├── provider_registry.rs  # DashMap-based provider storage
│   │       ├── traits.rs        # LLMProvider trait definition
│   │       ├── billing.rs       # Cost calculation
│   │       ├── circuit_breaker.rs
│   │       ├── connection_pool.rs
│   │       ├── cost_tracker.rs
│   │       ├── load_balancer.rs
│   │       └── types.rs
│   │
│   ├── gaussmeridian-providers/   # LLM provider implementations
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── openai.rs        # OpenAI + SSE streaming
│   │       ├── anthropic.rs
│   │       └── ollama.rs
│   │       # Feature-gated stubs: huggingface, vllm, cohere, custom, lmstudio
│   │
│   ├── gaussmeridian-models/      # Shared API types
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chat.rs          # ChatCompletionRequest/Response/Chunk
│   │       ├── completion.rs
│   │       ├── embedding.rs
│   │       ├── provider.rs
│   │       ├── usage.rs
│   │       └── error.rs
│   │
│   ├── gaussmeridian-auth/        # Authentication + authorization
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── auth_manager.rs  # JWT validation, API key validation, user CRUD
│   │       ├── api_key.rs       # Key generation, hashing
│   │       ├── jwt.rs           # Token creation/validation
│   │       ├── oauth2.rs        # OAuth2 scaffolding (partial)
│   │       ├── rbac.rs          # Role-based access control
│   │       ├── rate_limit.rs    # Token bucket rate limiter
│   │       └── error.rs
│   │
│   ├── gaussmeridian-db/          # SurrealDB integration
│   │   ├── migrations/          # SQL migration files
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs        # DB connection + initialization
│   │       ├── init.rs          # Schema + migration runner
│   │       ├── schema.rs        # Table definitions (User, ApiKey, Tenant, etc.)
│   │       └── repositories/    # CRUD for each entity
│   │           ├── mod.rs
│   │           ├── users.rs
│   │           ├── api_keys.rs
│   │           ├── tenants.rs
│   │           ├── requests.rs
│   │           ├── responses.rs
│   │           ├── models.rs
│   │           ├── agents.rs
│   │           ├── strategies.rs
│   │           ├── cache.rs
│   │           └── metrics.rs
│   │
│   ├── gaussmeridian-cache/       # Caching layer
│   │   └── src/
│   │       ├── lib.rs           # CacheProvider trait + MemoryCache + optional Redis
│   │       └── ...
│   │
│   ├── gaussmeridian-config/      # Configuration types and loading
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs         # AppConfig, ServerConfig, ProviderConfig, etc.
│   │       ├── validator.rs
│   │       └── error.rs
│   │
│   ├── gaussmeridian-metrics/     # Prometheus metrics collection
│   │   └── src/lib.rs
│   │
│   ├── gaussmeridian-plugins/     # Dynamic plugin loading (optional)
│   │   └── src/lib.rs
│   │
│   ├── gaussmeridian-utils/       # Validation and security helpers
│   │   └── src/lib.rs
│   │
│   └── gaussmeridian-moa/         # Mixture of Agents
│       ├── docs/
│       │   ├── strategies.md
│       │   └── user_guide.md
│       └── src/
│           └── lib.rs           # MoA strategies, agent types, orchestration
│
└── services/
    ├── server/                  # ★ Main binary
    │   ├── Cargo.toml
    │   └── src/
    │       ├── main.rs          # Entry point: CLI parse → setup logging → run server
    │       ├── lib.rs           # Re-exports
    │       ├── app.rs           # Application bootstrap (cache, DB, auth, providers)
    │       ├── routes.rs        # All HTTP route definitions
    │       ├── handlers.rs      # All request handlers
    │       ├── state.rs         # AppState struct
    │       ├── server.rs        # TCP bind + axum::serve + graceful shutdown
    │       ├── middleware.rs     # Middleware definitions (mostly unused)
    │       ├── error.rs         # ApiError types
    │       ├── config.rs        # Config re-exports
    │       ├── cli.rs           # Clap CLI definition
    │       └── openapi.rs       # OpenAPI spec generation (if present)
    │
    ├── tui/                     # Terminal UI service
    │   ├── Cargo.toml
    │   └── src/
    │       └── ...              # Ratatui-based monitoring dashboard
    │
    └── webui/                   # ★ Next.js frontend
        ├── package.json
        ├── next.config.mjs
        ├── tsconfig.json
        ├── components.json      # shadcn/ui config
        ├── postcss.config.mjs
        ├── app/                 # Next.js App Router pages
        ├── components/          # React components (ui/ = shadcn primitives)
        ├── hooks/               # Custom React hooks
        ├── islands/             # Client-side island components
        ├── lib/                 # API client, auth context, utilities
        ├── public/              # Static assets
        └── routes/              # ★ LEGACY Deno/Fresh routes (orphaned)
```

## Key Observation: Legacy Deno/Fresh Code

The `webui/` directory contains both:
- `app/` — Active **Next.js App Router** pages
- `routes/` — **Orphaned Deno/Fresh** routes from a previous iteration
- `deno.json`, `import_map.json` — Deno config files (no longer used with Next.js)

The `routes/` folder and Deno configs are dead code. The active frontend is entirely in `app/`.
