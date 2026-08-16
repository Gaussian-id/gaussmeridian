# Load Testing Guide for GaussMeridian

This directory contains load testing scripts and benchmarks for GaussMeridian.

## Prerequisites

### For k6 Load Tests
Install k6: https://k6.io/docs/get-started/installation/

```bash
# macOS
brew install k6

# Linux
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# Windows
choco install k6
```

### For Rust Benchmarks
Criterion benchmarks are built into the workspace. No additional installation needed.

## Running Load Tests

### k6 Load Tests

Basic load test:
```bash
cd load_tests
k6 run k6_load_test.js
```

With custom base URL and API key:
```bash
k6 run --env BASE_URL=http://localhost:3000 --env API_KEY=your-api-key k6_load_test.js
```

Smoke test (quick validation):
```bash
k6 run --vus 1 --duration 30s k6_load_test.js
```

Stress test (higher load):
```bash
k6 run --vus 500 --duration 5m k6_load_test.js
```

With output to InfluxDB:
```bash
k6 run --out influxdb=http://localhost:8086/k6 k6_load_test.js
```

### Rust Benchmarks

Run all benchmarks:
```bash
cd gaussmeridian
cargo bench
```

Run specific benchmark:
```bash
cargo bench --bench performance_bench
```

Run with baseline comparison:
```bash
# Create baseline
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

## Load Test Scenarios

### 1. Smoke Test
**Purpose:** Verify the system works under minimal load
- **Users:** 1-5
- **Duration:** 30s
- **Expected:** All requests succeed, low latency

```bash
k6 run --vus 5 --duration 30s k6_load_test.js
```

### 2. Load Test
**Purpose:** Test typical production load
- **Users:** 50-100
- **Duration:** 5m
- **Expected:** < 1% errors, p95 < 500ms

```bash
k6 run --vus 100 --duration 5m k6_load_test.js
```

### 3. Stress Test
**Purpose:** Find the breaking point
- **Users:** Ramp from 0 to 500+
- **Duration:** 10m
- **Expected:** Identify max throughput

```bash
k6 run --vus 500 --duration 10m k6_load_test.js
```

### 4. Spike Test
**Purpose:** Test sudden traffic spikes
- **Pattern:** 10 → 200 → 10 users
- **Duration:** 5m
- **Expected:** System recovers gracefully

```bash
k6 run --stage 1m:10,30s:200,1m:200,30s:10,1m:10 k6_load_test.js
```

### 5. Soak Test
**Purpose:** Test stability over time (memory leaks, etc.)
- **Users:** 50 constant
- **Duration:** 1h+
- **Expected:** Performance remains stable

```bash
k6 run --vus 50 --duration 1h k6_load_test.js
```

## Performance Targets

### Latency Targets
- **p50:** < 100ms
- **p95:** < 500ms
- **p99:** < 1000ms

### Throughput Targets
- **Minimum:** 1000 req/s
- **Target:** 5000 req/s
- **Stretch:** 10000 req/s

### Error Rate Targets
- **Maximum:** < 0.1% (99.9% success rate)
- **Rate Limiting:** Graceful 429 responses

### Resource Usage Targets
- **CPU:** < 80% under typical load
- **Memory:** < 2GB per instance
- **Database:** < 100 connections per instance

## Analyzing Results

### k6 Output

k6 provides detailed metrics:
- **http_req_duration:** Request duration percentiles
- **http_req_failed:** Error rate
- **http_reqs:** Total requests per second
- **vus:** Virtual users (concurrent connections)

### Criterion Output

Criterion provides:
- **Time:** Mean execution time with confidence intervals
- **Throughput:** Operations per second
- **Comparison:** Change from baseline (if available)

### What to Look For

**Good Signs:**
- ✅ Linear scaling with user count
- ✅ Stable latency under load
- ✅ Graceful degradation (429s instead of 500s)
- ✅ Quick recovery after spikes

**Warning Signs:**
- ⚠️ Increasing latency over time (memory leak?)
- ⚠️ High error rates (> 1%)
- ⚠️ Timeouts or connection errors
- ⚠️ Database connection pool exhaustion

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Load Tests

on:
  push:
    branches: [main]
  pull_request:

jobs:
  load-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Start services
        run: docker-compose up -d
      
      - name: Install k6
        run: |
          sudo gpg -k
          sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
          echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
          sudo apt-get update
          sudo apt-get install k6
      
      - name: Run smoke test
        run: k6 run --vus 10 --duration 30s load_tests/k6_load_test.js
      
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: load-test-results
          path: results/
```

## Monitoring During Load Tests

### Metrics to Monitor
1. **Application Metrics** (Prometheus)
   - Request rate
   - Error rate
   - Latency percentiles
   - Active connections

2. **System Metrics**
   - CPU usage
   - Memory usage
   - Disk I/O
   - Network I/O

3. **Database Metrics**
   - Query latency
   - Connection pool usage
   - Transaction rate

### Tools
- **Grafana** - Visualization
- **Prometheus** - Metrics collection
- **k6 Cloud** - Advanced k6 metrics
- **DataDog/New Relic** - APM

## Troubleshooting

### High Latency
- Check database query performance
- Review cache hit rates
- Look for N+1 queries
- Check network latency

### High Error Rates
- Review application logs
- Check rate limiting configuration
- Verify database connection limits
- Check for deadlocks

### Memory Issues
- Profile with `valgrind` or `heaptrack`
- Check for connection leaks
- Review cache sizes
- Monitor goroutine/task counts

## Best Practices

1. **Always run on production-like infrastructure**
2. **Use separate test environments (don't test on production!)**
3. **Monitor during tests, not just after**
4. **Test incrementally (smoke → load → stress)**
5. **Document baselines and regressions**
6. **Include load tests in CI/CD**
7. **Test failure scenarios (circuit breakers, retries)**

## Resources

- [k6 Documentation](https://k6.io/docs/)
- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Performance Testing Best Practices](https://k6.io/docs/test-types/introduction/)

