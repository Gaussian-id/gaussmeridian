# 06 — Setup Guide

> **Warning:** This project has NOT been verified to compile or run by the current maintainer. These instructions are derived from code analysis. Expect issues on first run.

---

## Prerequisites

### Required Software

| Tool                            | Version               | Purpose                  |
| ------------------------------- | --------------------- | ------------------------ |
| **Rust**                        | 1.75+ (edition 2021)  | Backend compilation      |
| **Node.js**                     | 18+ (recommended 20+) | Frontend (Next.js)       |
| **npm**                         | 9+                    | Frontend package manager |
| **Docker** + **Docker Compose** | Latest                | Full stack orchestration |
| **Git**                         | Any                   | Version control          |

### Optional Software

| Tool              | Purpose                         |
| ----------------- | ------------------------------- |
| **k6**            | Load testing                    |
| **SurrealDB CLI** | Direct DB access outside Docker |
| **Redis CLI**     | Direct cache inspection         |

### Windows-Specific Notes

- The management script (`gaussmeridian-manage.sh`) is **bash-only** — use Git Bash, WSL, or Docker instead
- Rust compilation on Windows may need Visual Studio Build Tools (MSVC) or use WSL
- Docker Desktop for Windows is recommended for the Docker path

---

## Option A: Docker Compose (Recommended)

This is the simplest path — runs everything in containers.

### Step 1: Configure Environment

```bash
# From the project root (where docker-compose.yml lives)
cp .env.example .env
```

Edit `.env` and set **at minimum**:

```bash
# Database
SURREALDB_PASSWORD=choose-a-secure-password

# Security
JWT_SECRET=generate-a-random-string-at-least-32-chars-long

# Bootstrap API key (prefix with gr-)
GAUSSMERIDIAN_API_KEY=gr-my-development-key

# LLM Providers (at least one)
OPENAI_API_KEY=sk-your-actual-openai-key
ANTHROPIC_API_KEY=sk-ant-<your-anthropic-key>

# Monitoring
GRAFANA_USER=admin
GRAFANA_PASSWORD=choose-a-grafana-password
```

### Step 2: Build and Start

```bash
docker compose up -d
```

First run will build the Rust binary (can take 5-15 minutes depending on hardware).

### Step 3: Verify

```bash
# Health check
curl http://localhost:8000/health

# List available models
curl -H "x-api-key: gr-my-development-key" http://localhost:8000/v1/models

# Test chat completion
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-api-key: gr-my-development-key" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Service URLs (Docker)

| Service         | URL                   |
| --------------- | --------------------- |
| GaussMeridian API | http://localhost:8000 |
| SurrealDB       | http://localhost:8001 |
| Redis           | localhost:6379        |
| Prometheus      | http://localhost:9091 |
| Grafana         | http://localhost:3000 |

---

## Option B: Local Development (Without Docker)

### Step 1: Install Rust

```bash
# Install rustup (Rust toolchain manager)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify
rustc --version  # Should be 1.75+
cargo --version
```

### Step 2: Install Node.js

```bash
# Using nvm (recommended)
nvm install 20
nvm use 20

# Or download from https://nodejs.org/
node --version   # Should be 18+
npm --version    # Should be 9+
```

### Step 3: Install SurrealDB (Optional but Recommended)

```bash
# macOS
brew install surrealdb/tap/surreal

# Linux
curl -sSf https://install.surrealdb.com | sh

# Windows — download from https://surrealdb.com/install

# Start SurrealDB
surreal start --user root --pass your-password file:./data/gaussmeridian.db
```

### Step 4: Install Redis (Optional)

```bash
# macOS
brew install redis && redis-server

# Linux
sudo apt install redis-server && sudo systemctl start redis

# Windows — use Docker: docker run -d -p 6379:6379 redis:7-alpine
```

### Step 5: Configure Environment

```bash
# From the project root
cp .env.example .env
# Edit .env with your settings (same variables as Docker section)
```

### Step 6: Build and Run the Rust Backend

```bash
cd gaussmeridian

# Check if it compiles (do this first!)
cargo check

# If cargo check succeeds, build in release mode
cargo build --release

# Run the server
./target/release/gaussmeridian
# Or with debug logging:
RUST_LOG=debug cargo run --bin gaussmeridian
```

The API should be available at `http://localhost:8000` (or the configured port).

### Step 7: Install and Run the Frontend

```bash
cd gaussmeridian/services/webui

# Install dependencies
npm install

# Create frontend env (optional, defaults to localhost:8000)
# Create a .env.local file:
echo "NEXT_PUBLIC_GAUSSMERIDIAN_API_URL=http://localhost:8000" > .env.local

# Run in development mode
npm run dev
```

The Web UI should be available at `http://localhost:3000`.

---

## First-Time Troubleshooting

### Rust Build Fails

**Missing system dependencies (Linux/WSL):**
```bash
sudo apt install pkg-config libssl-dev cmake clang llvm libclang-dev
```

**`gaussmeridian-moa` compilation errors:**
Previous assessments mention ~23 compile errors in the MoA crate. If the workspace doesn't build:
```bash
# Try building without MoA
cargo build --release --bin gaussmeridian
# Or check specific crate errors
cargo check -p gaussmeridian-moa 2>&1 | head -50
```

**SurrealDB crate build issues:**
The `surrealdb` crate (v2.0) with `kv-rocksdb` feature may need cmake, clang, and llvm. On Windows, this can be particularly troublesome.

### Frontend Build Issues

**`typescript.ignoreBuildErrors: true`** is set in `next.config.mjs`, so `npm run build` should succeed even with type errors. But `npm run dev` may still show warnings.

**CORS errors in browser:**
If the frontend can't reach the backend, ensure:
1. Backend is running on port 8000
2. `NEXT_PUBLIC_GAUSSMERIDIAN_API_URL=http://localhost:8000` is set
3. CORS is enabled in the Rust config (it's permissive in non-production by default)

### Docker Build Slow

First-time Rust compilation in Docker downloads all crates and compiles from scratch. Subsequent builds use Docker layer cache. The GHA cache (`docker/build-push-action` with `cache-from: gha`) helps in CI.

### SurrealDB Connection Fails

If running locally without SurrealDB, the backend will skip DB initialization and run with **reduced functionality** (no persistent users, API keys validated permissively, no usage tracking).

---

## Verifying the Full Stack

Once everything is running, test the complete flow:

```bash
# 1. Register a user
curl -X POST http://localhost:8000/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "securepassword123", "name": "Test User"}'

# 2. Login (get JWT)
curl -X POST http://localhost:8000/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "securepassword123"}'
# → Returns { "token": "eyJ...", "user": { ... } }

# 3. Use JWT for authenticated requests
curl -H "Authorization: Bearer <token-from-step-2>" \
  http://localhost:8000/v1/auth/me

# 4. Create a personal API key
curl -X POST http://localhost:8000/v1/api/keys \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-dev-key"}'
# → Returns the API key

# 5. Use the API key for LLM requests
curl -X POST http://localhost:8000/v1/chat/completions \
  -H "x-api-key: <key-from-step-4>" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "What is GaussMeridian?"}]
  }'
```

---

## Configuration Reference

### `gaussmeridian.toml` (Rust Backend)

Located at `gaussmeridian/gaussmeridian.toml`. This is the file-based config that gets merged with environment variables.

Key sections:
- `[server]` — host, port, cors settings
- `[providers.openai]` — provider type, api_key (supports `${ENV_VAR}`), models
- `[providers.anthropic]` — same pattern
- `[cache]` — enabled, ttl, max_size
- `[security]` — jwt_secret, cors_origins
- `[logging]` — level, format
- `[metrics]` — enabled
- `[deployment]` — environment name

### Environment Variable Prefix

All Rust config can be overridden via environment with `GAUSSMERIDIAN__` prefix (double underscore for nesting):

```bash
GAUSSMERIDIAN__SERVER__PORT=9000
GAUSSMERIDIAN__CACHE__ENABLED=true
GAUSSMERIDIAN__LOGGING__LEVEL=debug
```
