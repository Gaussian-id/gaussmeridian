# Production Deployment Guide

This guide provides comprehensive instructions for deploying GaussMeridian in production environments.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Configuration](#configuration)
3. [Docker Deployment](#docker-deployment)
4. [Kubernetes Deployment](#kubernetes-deployment)
5. [Security Hardening](#security-hardening)
6. [Monitoring & Observability](#monitoring--observability)
7. [Scaling](#scaling)
8. [Backup & Recovery](#backup--recovery)
9. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **OS**: Linux (Ubuntu 20.04+, RHEL 8+, or similar)
- **CPU**: 4+ cores recommended
- **Memory**: 8GB+ RAM recommended
- **Storage**: 50GB+ SSD recommended
- **Network**: Stable network connection

### Software Requirements

- Docker 24+ (for containerized deployment)
- Kubernetes 1.28+ (for orchestrated deployment)
- SurrealDB 2.0+ (for persistent storage)
- Redis 7+ (for caching, optional but recommended)
- Prometheus (for metrics, optional)
- Grafana (for dashboards, optional)

## Configuration

### Environment Variables

Create a `.env` file or set environment variables:

```bash
# Server Configuration
GAUSSMERIDIAN_HOST=0.0.0.0
GAUSSMERIDIAN_PORT=8000
GAUSSMERIDIAN_LOG_LEVEL=info
GAUSSMERIDIAN_LOG_FORMAT=json

# Database
SURREALDB_URL=http://surrealdb:8000
SURREALDB_USER=root
SURREALDB_PASS=root
SURREALDB_NS=production
SURREALDB_DB=gaussmeridian

# Redis (optional)
REDIS_URL=redis://redis:6379

# Security
JWT_SECRET=your-secret-key-change-in-production
API_KEY_SECRET=your-api-key-secret

# OAuth2 (optional)
OAUTH2_CLIENT_ID=your-client-id
OAUTH2_CLIENT_SECRET=your-client-secret
OAUTH2_REDIRECT_URI=https://your-domain.com/auth/callback

# Monitoring
PROMETHEUS_ENABLED=true
JAEGER_ENDPOINT=http://jaeger:14268/api/traces
ZIPKIN_ENDPOINT=http://zipkin:9411/api/v2/spans
```

### Configuration File

Edit `gaussmeridian.toml` for production settings:

```toml
[server]
host = "0.0.0.0"
port = 8000
max_connections = 1000
request_timeout = 300
enable_websocket = true
enable_cors = true
max_request_size = 16777216  # 16MB
graceful_shutdown_timeout = 30
worker_threads = 8

[security]
rate_limiting.requests_per_minute = 1000
rate_limiting.tokens_per_minute = 100000
cors_origins = ["https://your-domain.com"]
enable_https = true

[logging]
level = "info"
format = "json"
output = "stdout"
structured_logging = true

[metrics]
enabled = true
prometheus_enabled = true
```

## Docker Deployment

### Build Image

```bash
docker build -t gaussmeridian:latest .
```

### Run with Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  gaussmeridian:
    image: gaussmeridian:latest
    ports:
      - "8000:8000"
    environment:
      - GAUSSMERIDIAN_HOST=0.0.0.0
      - GAUSSMERIDIAN_PORT=8000
      - SURREALDB_URL=http://surrealdb:8000
      - REDIS_URL=redis://redis:6379
    depends_on:
      - surrealdb
      - redis
    volumes:
      - ./gaussmeridian.toml:/app/gaussmeridian.toml
    restart: unless-stopped

  surrealdb:
    image: surrealdb/surrealdb:latest
    ports:
      - "8000:8000"
    command: start --log trace --user root --pass root memory
    volumes:
      - surrealdb_data:/data

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data

volumes:
  surrealdb_data:
  redis_data:
```

Start services:

```bash
docker-compose up -d
```

### Health Checks

```bash
# Check server health
curl http://localhost:8000/health

# Check readiness
curl http://localhost:8000/ready

# Check metrics
curl http://localhost:8000/metrics
```

## Kubernetes Deployment

### Namespace

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: gaussmeridian
```

### ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: gaussmeridian-config
  namespace: gaussmeridian
data:
  gaussmeridian.toml: |
    [server]
    host = "0.0.0.0"
    port = 8000
    # ... configuration ...
```

### Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: gaussmeridian-secrets
  namespace: gaussmeridian
type: Opaque
stringData:
  JWT_SECRET: your-secret-key
  API_KEY_SECRET: your-api-key-secret
```

### Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: gaussmeridian
  namespace: gaussmeridian
spec:
  replicas: 3
  selector:
    matchLabels:
      app: gaussmeridian
  template:
    metadata:
      labels:
        app: gaussmeridian
    spec:
      containers:
      - name: gaussmeridian
        image: gaussmeridian:latest
        ports:
        - containerPort: 8000
        env:
        - name: GAUSSMERIDIAN_PORT
          value: "8000"
        envFrom:
        - secretRef:
            name: gaussmeridian-secrets
        volumeMounts:
        - name: config
          mountPath: /app/gaussmeridian.toml
          subPath: gaussmeridian.toml
        livenessProbe:
          httpGet:
            path: /health
            port: 8000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8000
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
      volumes:
      - name: config
        configMap:
          name: gaussmeridian-config
```

### Service

```yaml
apiVersion: v1
kind: Service
metadata:
  name: gaussmeridian
  namespace: gaussmeridian
spec:
  selector:
    app: gaussmeridian
  ports:
  - port: 8000
    targetPort: 8000
  type: LoadBalancer
```

### Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: gaussmeridian-hpa
  namespace: gaussmeridian
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: gaussmeridian
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

## Security Hardening

### 1. TLS/HTTPS

Use a reverse proxy (nginx/traefik) or configure TLS directly:

```toml
[tls]
cert_path = "/etc/ssl/certs/gaussmeridian.crt"
key_path = "/etc/ssl/private/gaussmeridian.key"
min_tls_version = "1.3"
```

### 2. API Key Management

- Use strong, randomly generated API keys
- Store keys securely (use secrets management)
- Rotate keys regularly
- Implement key expiration

### 3. Rate Limiting

Configure appropriate rate limits:

```toml
[security]
rate_limiting.requests_per_minute = 1000
rate_limiting.tokens_per_minute = 100000
rate_limiting.burst_size = 100
```

### 4. CORS Configuration

Restrict CORS origins:

```toml
[security]
cors_origins = ["https://your-domain.com"]
```

### 5. Input Validation

Enable comprehensive input validation:

- Use `InputValidator` from `gaussmeridian-utils`
- Sanitize all user inputs
- Validate API keys, URLs, and other inputs
- Prevent SQL injection, XSS, SSRF attacks

### 6. Audit Logging

Enable audit logging for security events:

```rust
use gaussmeridian_utils::security::AuditLogger;

let audit_logger = AuditLogger::new(true);
audit_logger.log_access(
    Some(user_id),
    "resource",
    "action",
    AuditStatus::Success,
    Some(ip_address),
);
```

## Monitoring & Observability

### Prometheus Metrics

Access metrics endpoint:

```bash
curl http://localhost:8000/metrics
```

Configure Prometheus to scrape:

```yaml
scrape_configs:
  - job_name: 'gaussmeridian'
    static_configs:
      - targets: ['gaussmeridian:8000']
```

The admin consoles consume these metrics as follows:

- **TUI**: Parses `/metrics` directly to render system metrics in the dashboard view.
- **WebUI**: The `lib/api-client.ts` client fetches `/metrics`, parses the Prometheus text, and aggregates it into a typed `Metrics` object used by the landing dashboard, console overview, and analytics pages.

This keeps the server surface area aligned with `SPECS.md` while still providing rich, structured metrics to all admin experiences.

### Grafana Dashboards

Import dashboard JSON from `docs/grafana_dashboard.json` or create custom dashboards.

### Distributed Tracing

Configure tracing endpoints:

```toml
[monitoring.tracing]
enabled = true
sampling_rate = 0.1
jaeger_endpoint = "http://jaeger:14268/api/traces"
zipkin_endpoint = "http://zipkin:9411/api/v2/spans"
```

### Log Aggregation

Use structured JSON logging:

```toml
[logging]
format = "json"
structured_logging = true
```

Forward logs to:
- ELK Stack (Elasticsearch, Logstash, Kibana)
- Loki
- CloudWatch
- Datadog

## Scaling

### Horizontal Scaling

- Use Kubernetes HPA or Docker Swarm
- Deploy multiple replicas behind a load balancer
- Ensure stateless architecture (use Redis/SurrealDB for state)

### Vertical Scaling

- Increase CPU/memory limits
- Optimize worker threads: `worker_threads = cpu_cores * 2`
- Monitor resource usage

### Database Scaling

- Configure SurrealDB clustering
- Use connection pooling
- Implement read replicas for read-heavy workloads

## Backup & Recovery

### Database Backups

```bash
# Backup SurrealDB
surreal export --conn http://localhost:8000 --user root --pass root --ns production --db gaussmeridian backup.sql
```

### Configuration Backups

```bash
# Backup configuration
tar -czf gaussmeridian-config-backup.tar.gz gaussmeridian.toml .env
```

### Recovery Procedures

1. Stop the service
2. Restore database backup
3. Restore configuration
4. Verify health checks
5. Restart service

## Troubleshooting

### Common Issues

1. **High Memory Usage**
   - Check for memory leaks
   - Reduce cache sizes
   - Increase available memory

2. **High CPU Usage**
   - Check for busy loops
   - Optimize hot paths
   - Increase worker threads if CPU-bound

3. **Connection Errors**
   - Check network connectivity
   - Verify firewall rules
   - Check SurrealDB/Redis availability

4. **Authentication Errors**
   - Verify API keys
   - Check JWT secret configuration
   - Review audit logs

### Debug Mode

Enable debug logging:

```toml
[logging]
level = "debug"
```

### Health Check Failures

```bash
# Check detailed health
curl http://localhost:8000/health

# Check component status
curl http://localhost:8000/metrics
```

## Performance Tuning

### Optimize for High Throughput

```toml
[server]
worker_threads = 16
max_connections = 5000
request_timeout = 600

[cache]
max_size = 50000
ttl = 7200
```

### Connection Pooling

```toml
[database]
max_connections = 50
min_connections = 10
connection_timeout = 30
```

## Production Checklist

- [ ] TLS/HTTPS configured
- [ ] Strong API keys generated
- [ ] Rate limiting configured
- [ ] CORS properly restricted
- [ ] Audit logging enabled
- [ ] Monitoring configured
- [ ] Backups scheduled
- [ ] Health checks verified
- [ ] Load balancing configured
- [ ] Auto-scaling configured
- [ ] Documentation updated
- [ ] Security audit completed

---

For additional support, see:
- [Architecture Documentation](ARCHITECTURE.md)
- [API Documentation](api/)
- [Security Guide](SECURITY.md)
- [Observability Guide](OBSERVABILITY.md)