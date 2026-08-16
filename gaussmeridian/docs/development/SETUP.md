# Development Setup Guide

This guide will help you set up a local development environment for GaussMeridian.

## Prerequisites

### Required Tools

1. **Rust 1.75+**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup toolchain install stable
   rustup default stable
   ```

2. **Deno 2.0+** (for webui)
   ```bash
   curl -fsSL https://deno.land/install.sh | sh
   ```

3. **SurrealDB 2.0+**
   ```bash
   # Using Docker (recommended)
   docker pull surrealdb/surrealdb:latest
   
   # Or install locally
   curl -sSf https://install.surrealdb.com | sh
   ```

4. **Git** (for version control)

### Optional Tools

- **Docker** and **Docker Compose** (for containerized development)
- **PostgreSQL** or **Redis** (for advanced caching/testing)
- **Grafana** and **Prometheus** (for metrics visualization)

## Initial Setup

### 1. Clone the Repository

```bash
git clone https://github.com/gaussmeridian/gaussmeridian.git
cd gaussmeridian/gaussmeridian
```

### 2. Install Rust Dependencies

```bash
cargo fetch
```

### 3. Start SurrealDB

**Option A: Using Docker (Recommended)**
```bash
docker run -d \
  --name surrealdb \
  -p 8000:8000 \
  surrealdb/surrealdb:latest \
  start --log trace --user root --pass root memory
```

**Option B: Local Installation**
```bash
surreal start --log trace --user root --pass root file://./data.db
```

### 4. Verify Setup

```bash
# Build the workspace
cargo build

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --all-targets --all-features
```

## Development Workflow

### Running Services Locally

**Terminal 1: API Server**
```bash
cargo run --bin gaussmeridian --release
# Server will start on http://localhost:3000
```

**Terminal 2: TUI**
```bash
cargo run --bin gaussmeridian-tui --release
```

**Terminal 3: WebUI**
```bash
cd services/webui
deno task dev
# WebUI will start on http://localhost:8080
```

### Environment Variables

Create a `.env` file in the workspace root:

```env
# SurrealDB Configuration
SURREALDB_URL=ws://localhost:8000
SURREALDB_USER=root
SURREALDB_PASS=root
SURREALDB_NS=gaussmeridian
SURREALDB_DB=development

# Server Configuration
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
RUST_LOG=debug

# API Keys (for testing)
OPENAI_API_KEY=your-key-here
ANTHROPIC_API_KEY=your-key-here
```

### Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p gaussmeridian-core

# With output
cargo test -- --nocapture

# Integration tests
cargo test --test integration

# Benchmarks
cargo bench
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Lint with clippy
cargo clippy --all-targets --all-features -- -D warnings

# Check documentation
cargo doc --no-deps --open
```

## IDE Setup

### VS Code

Recommended extensions:
- `rust-lang.rust-analyzer` - Rust language support
- `vadimcn.vscode-lldb` - Debugging
- `serayuzgur.crates` - Cargo.toml support
- `denoland.vscode-deno` - Deno support (for webui)

### IntelliJ / CLion

- Install the Rust plugin
- Configure Rust toolchain in Settings → Languages & Frameworks → Rust

## Debugging

### VS Code Launch Configuration

Create `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug gaussmeridian-server",
      "cargo": {
        "args": ["build", "--bin", "gaussmeridian"],
        "filter": {
          "name": "gaussmeridian",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}/gaussmeridian"
    }
  ]
}
```

### Logging

Set `RUST_LOG` environment variable:

```bash
# Debug level
RUST_LOG=debug cargo run --bin gaussmeridian

# Specific module
RUST_LOG=gaussmeridian_core=debug cargo run --bin gaussmeridian

# Trace level
RUST_LOG=trace cargo run --bin gaussmeridian
```

## Common Issues

### Build Errors

**Issue**: `error: linker 'cc' not found`
**Solution**: Install build essentials:
```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

**Issue**: `error: failed to run custom build command for 'ring'`
**Solution**: Ensure you have the latest Rust toolchain:
```bash
rustup update stable
```

### SurrealDB Connection Issues

**Issue**: Cannot connect to SurrealDB
**Solution**: 
1. Verify SurrealDB is running: `docker ps` or check process
2. Check connection string in `.env`
3. Verify network/firewall settings

### Port Conflicts

**Issue**: Port already in use
**Solution**: Change port in configuration or kill existing process:
```bash
# Find process using port
lsof -i :3000

# Kill process
kill -9 <PID>
```

## Next Steps

- Read the [Architecture Documentation](../../../ARCHITECTURE.md)
- Review [API Documentation](../api/)
- Check [Contributing Guidelines](../../../CONTRIBUTING.md)
- Explore example code in `examples/` directory

