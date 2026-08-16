# GaussMeridian Performance Profiling Guide

**Version:** 3.0.0  
**Last Updated:** 2025-12-30

---

## Overview

This guide covers profiling and performance optimization for GaussMeridian.

## Profiling Tools

### 1. Cargo Flamegraph

Generate CPU flame graphs to identify hot paths.

**Installation:**
```bash
cargo install flamegraph
```

**Usage:**
```bash
# Profile the server
cd gaussmeridian
cargo flamegraph --bin gaussmeridian-server

# Profile specific tests
cargo flamegraph --test database_tests

# Profile benchmarks
cargo flamegraph --bench performance_bench
```

**Output:** `flamegraph.svg` - Interactive flame graph

### 2. Perf (Linux)

Detailed CPU profiling.

```bash
# Record performance data
cargo build --release
perf record -g ./target/release/gaussmeridian-server

# Generate report
perf report

# Generate flame graph from perf data
perf script | stackcollapse-perf.pl | flamegraph.pl > perf-flamegraph.svg
```

### 3. Valgrind (Memory Profiling)

Find memory leaks and inefficiencies.

```bash
# Install valgrind
sudo apt-get install valgrind

# Run with valgrind
cargo build
valgrind --leak-check=full --show-leak-kinds=all \
  --track-origins=yes \
  ./target/debug/gaussmeridian-server

# Memory profiling with massif
valgrind --tool=massif ./target/debug/gaussmeridian-server
ms_print massif.out.<pid>
```

### 4. Heaptrack (Memory Profiling)

Modern memory profiler with GUI.

```bash
# Install
sudo apt-get install heaptrack heaptrack-gui

# Profile
heaptrack ./target/release/gaussmeridian-server

# Analyze
heaptrack_gui heaptrack.gaussmeridian-server.<pid>.gz
```

### 5. Tokio Console

Real-time async runtime monitoring.

**Add to Cargo.toml:**
```toml
[dependencies]
console-subscriber = "0.2"

[features]
tokio-console = ["tokio/tracing"]
```

**Usage:**
```bash
# Run with tokio-console
RUSTFLAGS="--cfg tokio_unstable" cargo run --features tokio-console

# In another terminal, run console
tokio-console
```

---

## Critical Paths to Profile

### 1. Request Handling Path

**Profile:**
- HTTP request parsing
- Authentication/authorization
- Rate limit checking
- Request validation
- Response serialization

**Command:**
```bash
cargo flamegraph --bin gaussmeridian-server -- --duration 60
# Then make requests with: ab -n 10000 -c 100 http://localhost:3000/v1/models
```

### 2. Provider Integration Path

**Profile:**
- Provider selection
- Request transformation
- HTTP client operations
- Response parsing
- Error handling

### 3. Database Operations

**Profile:**
- Connection pool acquisition
- Query execution
- Result parsing
- Transaction handling

**Command:**
```bash
cargo flamegraph --test database_tests
```

### 4. Cache Operations

**Profile:**
- Cache key generation
- Cache lookup (Redis/memory)
- Cache write
- Cache eviction

### 5. MoA Processing

**Profile:**
- Agent orchestration
- Strategy execution
- Response aggregation
- Confidence scoring

---

## Optimization Checklist

### High-Priority Optimizations

- [ ] **Reduce allocations** - Use `&str` instead of `String` where possible
- [ ] **Optimize serialization** - Use `serde` efficiently, consider binary formats
- [ ] **Pool connections** - Reuse HTTP connections, database connections
- [ ] **Cache aggressively** - Cache parsed models, provider configs
- [ ] **Async efficiently** - Avoid blocking operations in async code
- [ ] **Batch operations** - Batch database writes, batch provider requests

### Database Optimizations

- [ ] **Add indexes** - Index frequently queried columns
- [ ] **Optimize queries** - Use EXPLAIN to analyze queries
- [ ] **Connection pooling** - Tune pool size (recommended: 2x CPU cores)
- [ ] **Query caching** - Cache frequently executed queries
- [ ] **Batch operations** - Use batch inserts/updates

### Cache Optimizations

- [ ] **Cache sizing** - Monitor hit rates, adjust sizes
- [ ] **Cache warming** - Pre-populate hot data
- [ ] **Cache keys** - Optimize key generation (avoid allocations)
- [ ] **TTL tuning** - Set appropriate TTLs for different data types
- [ ] **Eviction policy** - Use LRU for most cases

---

## Performance Targets

### Latency Targets

| Endpoint | p50 | p95 | p99 |
|----------|-----|-----|-----|
| `/health` | 5ms | 10ms | 20ms |
| `/v1/models` | 20ms | 50ms | 100ms |
| `/v1/chat/completions` | 100ms | 500ms | 1000ms |
| `/v1/embeddings` | 50ms | 200ms | 500ms |

### Throughput Targets

| Scenario | Target | Stretch |
|----------|--------|---------|
| Simple requests | 5,000 req/s | 10,000 req/s |
| With DB writes | 2,000 req/s | 5,000 req/s |
| Streaming | 1,000 streams/s | 2,000 streams/s |

### Resource Targets

| Resource | Idle | Under Load | Maximum |
|----------|------|------------|---------|
| CPU | <5% | 50-70% | 90% |
| Memory | 500MB | 1-2GB | 4GB |
| DB Connections | 10 | 50-100 | 200 |
| HTTP Connections | 0 | 500-1000 | 2000 |

---

## Profiling Workflow

### 1. Baseline Measurement

```bash
# Run load test to establish baseline
cd load_tests
k6 run --vus 100 --duration 5m k6_load_test.js | tee baseline.txt

# Run benchmarks
cd ../gaussmeridian
cargo bench --bench performance_bench | tee baseline-bench.txt
```

### 2. Profile Critical Paths

```bash
# Profile server under load
cargo flamegraph --bin gaussmeridian-server &
SERVER_PID=$!

# Generate load
ab -n 10000 -c 100 http://localhost:3000/v1/models

# Stop profiling
kill -INT $SERVER_PID
```

### 3. Analyze Results

- Open `flamegraph.svg` in browser
- Identify functions consuming >5% CPU time
- Look for unexpected allocations
- Check for lock contention
- Identify I/O bottlenecks

### 4. Optimize

- Apply targeted optimizations
- Run benchmarks again
- Compare before/after
- Iterate

### 5. Validate

```bash
# Re-run load test
k6 run --vus 100 --duration 5m k6_load_test.js | tee optimized.txt

# Compare results
diff baseline.txt optimized.txt
```

---

## Common Bottlenecks

### 1. Excessive Cloning

**Problem:** Cloning large structures unnecessarily

**Solution:**
```rust
// Before
fn process(data: MyStruct) { ... }
process(data.clone());

// After
fn process(data: &MyStruct) { ... }
process(&data);
```

### 2. String Allocations

**Problem:** Converting to String unnecessarily

**Solution:**
```rust
// Before
fn log_message(msg: String) { ... }
log_message(format!("Error: {}", err));

// After
fn log_message(msg: &str) { ... }
log_message(&format!("Error: {}", err));
```

### 3. Blocking in Async

**Problem:** Blocking operations in async functions

**Solution:**
```rust
// Before
async fn process() {
    let data = std::fs::read_to_string("file.txt").unwrap(); // BLOCKS!
}

// After
async fn process() {
    let data = tokio::fs::read_to_string("file.txt").await.unwrap();
}
```

### 4. Lock Contention

**Problem:** Multiple threads competing for locks

**Solution:**
```rust
// Before
let data = Arc::new(Mutex::new(HashMap::new()));

// After (if read-heavy)
let data = Arc::new(RwLock::new(HashMap::new()));

// Or (if rarely modified)
let data = Arc::new(DashMap::new()); // Lock-free concurrent map
```

---

## Continuous Monitoring

### Setup Continuous Profiling

```bash
# Run profiling in production (low overhead)
cargo build --release --features profiling

# Export to Prometheus
# Metrics automatically exposed at /metrics
```

### Dashboard Metrics

Key metrics to monitor:
- Request latency (p50, p95, p99)
- Request rate
- Error rate
- CPU usage
- Memory usage
- Database query time
- Cache hit rate
- Provider response time

---

## Resources

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [Tokio Console](https://github.com/tokio-rs/console)
- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)

---

**© 2025 GaussMeridian. All rights reserved.**

