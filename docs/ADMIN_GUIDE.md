# GaussMeridian Admin Guide

**Version:** 3.1.0  
**For:** System Administrators and DevOps Engineers  
**Last Updated:** 2026-01-16

---

## Table of Contents

1. [Installation & Deployment](#installation--deployment)
2. [Configuration](#configuration)
3. [Database Management](#database-management)
4. [Terminal User Interface (TUI)](#terminal-user-interface-tui)
5. [Monitoring & Observability](#monitoring--observability)
6. [Security](#security)
7. [Backup & Recovery](#backup--recovery)
8. [Performance Tuning](#performance-tuning)
9. [Troubleshooting](#troubleshooting)

---

## Installation & Deployment

### System Requirements

**Minimum:**
- CPU: 2 cores
- RAM: 4GB
- Disk: 20GB SSD
- OS: Linux (Ubuntu 20.04+, RHEL 8+, Debian 11+)

**Recommended (Production):**
- CPU: 8+ cores
- RAM: 16GB+
- Disk: 100GB+ NVMe SSD
- OS: Linux (Ubuntu 22.04 LTS)

### Docker Deployment (Recommended)

**1. Clone repository:**
```bash
git clone https://github.com/gaussmeridian/gaussmeridian.git
cd gaussmeridian
```

**2. Configure environment:**
```bash
cp .env.example .env
nano .env  # Edit configuration
```

**3. Start services:**
```bash
docker-compose up -d
```

**4. Verify deployment:**
```bash
curl http://localhost:3000/health
docker-compose logs -f gaussmeridian
```

### Kubernetes Deployment

**1. Apply manifests:**
```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/ingress.yaml
```

**2. Verify deployment:**
```bash
kubectl get pods -n gaussmeridian
kubectl logs -f deployment/gaussmeridian -n gaussmeridian
```

**3. Scale deployment:**
```bash
kubectl scale deployment gaussmeridian --replicas=3 -n gaussmeridian
```

### Binary Deployment

**1. Download binary:**
```bash
wget https://github.com/gaussmeridian/gaussmeridian/releases/download/v3.0.0/gaussmeridian-server-linux-amd64
chmod +x gaussmeridian-server-linux-amd64
```

**2. Create systemd service:**
```bash
sudo nano /etc/systemd/system/gaussmeridian.service
```

```ini
[Unit]
Description=GaussMeridian API Server
After=network.target

[Service]
Type=simple
User=gaussmeridian
WorkingDirectory=/opt/gaussmeridian
ExecStart=/opt/gaussmeridian/gaussmeridian-server
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

**3. Enable and start:**
```bash
sudo systemctl daemon-reload
sudo systemctl enable gaussmeridian
sudo systemctl start gaussmeridian
```

---

## Configuration

### Environment Variables

#### Server Configuration

```bash
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
SERVER_WORKERS=4  # Number of worker threads

# Database
DATABASE_URL=ws://localhost:8000
DATABASE_NAMESPACE=gaussmeridian
DATABASE_DATABASE=gaussmeridian
DATABASE_USERNAME=root
DATABASE_PASSWORD=root

# Authentication
JWT_SECRET=your-super-secret-jwt-key-change-me
JWT_EXPIRATION=86400  # 24 hours in seconds

# Rate Limiting
RATE_LIMIT_ENABLED=true
RATE_LIMIT_REQUESTS_PER_MINUTE=1000
RATE_LIMIT_TOKENS_PER_MINUTE=100000

# Caching
CACHE_ENABLED=true
CACHE_TTL=3600  # 1 hour
REDIS_URL=redis://localhost:6379

# Observability
METRICS_ENABLED=true
METRICS_PORT=9090
TRACING_ENABLED=true
TRACING_ENDPOINT=http://localhost:4317
LOG_LEVEL=info  # trace, debug, info, warn, error

# CORS
CORS_ENABLED=true
CORS_ALLOWED_ORIGINS=*  # Use specific origins in production
```

#### Provider Configuration

```bash
# OpenAI
OPENAI_API_KEY=sk-...
OPENAI_ORGANIZATION=org-...

# Anthropic
ANTHROPIC_API_KEY=sk-ant-...

# Cohere
COHERE_API_KEY=...

# Custom Provider
CUSTOM_PROVIDER_URL=https://api.custom.com
CUSTOM_PROVIDER_API_KEY=...
```

### Configuration File (config.toml)

```toml
[server]
host = "0.0.0.0"
port = 3000
workers = 4
enable_cors = true
cors_origins = ["*"]

[database]
url = "ws://localhost:8000"
namespace = "gaussmeridian"
database = "gaussmeridian"
username = "root"
password = "root"
connection_pool_size = 100

[auth]
jwt_secret = "your-super-secret-jwt-key"
jwt_expiration = 86400
api_key_rotation_days = 90

[rate_limiting]
enabled = true
requests_per_minute = 1000
tokens_per_minute = 100000
window_size_seconds = 60

[caching]
enabled = true
ttl = 3600
max_size_mb = 1024

[metrics]
enabled = true
port = 9090
endpoint = "/metrics"

[tracing]
enabled = true
endpoint = "http://localhost:4317"
sample_rate = 0.1  # 10% sampling

[logging]
level = "info"
format = "json"
output = "stdout"

[[providers]]
name = "openai"
enabled = true
priority = 1
max_retries = 3
timeout_seconds = 30

[[providers]]
name = "anthropic"
enabled = true
priority = 2
max_retries = 3
timeout_seconds = 30
```

---

## Database Management

### SurrealDB Setup

**Starting SurrealDB:**
```bash
# Docker
docker run -d --name surrealdb \
  -p 8000:8000 \
  -v surrealdb_data:/data \
  surrealdb/surrealdb:latest \
  start --log trace --user root --pass root file://data/database.db

# Binary
surreal start --log trace --user root --pass root file://database.db
```

### Database Initialization

```bash
# Initialize schema
gaussmeridian-cli db init \
  --url ws://localhost:8000 \
  --namespace gaussmeridian \
  --database gaussmeridian \
  --username root \
  --password root
```

### Running Migrations

```bash
# Apply all pending migrations
gaussmeridian-cli db migrate \
  --url ws://localhost:8000 \
  --namespace gaussmeridian \
  --database gaussmeridian
```

### Database Backup

**Manual backup:**
```bash
# Export data
surreal export \
  --conn ws://localhost:8000 \
  --user root \
  --pass root \
  --ns gaussmeridian \
  --db gaussmeridian \
  backup-$(date +%Y%m%d).surql

# Compress backup
gzip backup-$(date +%Y%m%d).surql
```

**Automated backup script:**
```bash
#!/bin/bash
# /opt/gaussmeridian/scripts/backup.sh

BACKUP_DIR="/var/backups/gaussmeridian"
DATE=$(date +%Y%m%d-%H%M%S)
RETENTION_DAYS=30

# Create backup
surreal export \
  --conn ws://localhost:8000 \
  --user root \
  --pass root \
  --ns gaussmeridian \
  --db gaussmeridian \
  "$BACKUP_DIR/backup-$DATE.surql"

# Compress
gzip "$BACKUP_DIR/backup-$DATE.surql"

# Upload to S3 (optional)
aws s3 cp "$BACKUP_DIR/backup-$DATE.surql.gz" \
  s3://my-bucket/backups/gaussmeridian/

# Clean up old backups
find "$BACKUP_DIR" -name "backup-*.surql.gz" \
  -mtime +$RETENTION_DAYS -delete

echo "Backup completed: backup-$DATE.surql.gz"
```

**Cron schedule:**
```bash
# Daily backup at 2 AM
0 2 * * * /opt/gaussmeridian/scripts/backup.sh >> /var/log/gaussmeridian-backup.log 2>&1
```

### Database Restore

```bash
# Import from backup
surreal import \
  --conn ws://localhost:8000 \
  --user root \
  --pass root \
  --ns gaussmeridian \
  --db gaussmeridian \
  backup-20251230.surql
```

---

## Terminal User Interface (TUI)

GaussMeridian includes a professional Terminal User Interface (TUI) for server administration, providing real-time monitoring, management, and diagnostics directly from the terminal.

### Starting the TUI

**Prerequisites:**
```bash
# Set environment variables
export GAUSSMERIDIAN_API_URL="http://localhost:3000"
export GAUSSMERIDIAN_API_KEY="your-api-key"
```

**Run the TUI:**
```bash
# Using cargo
cargo run --bin gaussmeridian-tui --release

# Or using the binary
./target/release/gaussmeridian-tui
```

### TUI Features

The GaussMeridian TUI provides a comprehensive administration interface with the following views:

#### 1. Dashboard View (Default)
Real-time overview of system health and performance:
- **System Metrics**: CPU usage, memory consumption, uptime
- **Request Statistics**: Total requests, requests per second, success rate
- **Latency Metrics**: Average, P95, and P99 latency with visual gauges
- **Provider Health**: Live status of all configured LLM providers
- **Cache Performance**: Hit rate and efficiency metrics

#### 2. Providers View
Manage and monitor LLM provider connections:
- **Provider Status**: Real-time health indicators (●/○)
- **Response Times**: Average latency per provider
- **Request Volume**: Total and active requests
- **Circuit Breaker State**: Open/Closed/Half-Open status
- **Enable/Disable**: Toggle providers on/off with `Space`

#### 3. Models View
View and manage available AI models:
- **Model Registry**: All available models with metadata
- **Provider Mapping**: Which provider serves each model
- **Token Pricing**: Input/output cost per 1K tokens
- **Context Window**: Maximum context length support
- **Usage Statistics**: Request counts per model

#### 4. Request Monitor
Real-time request tracking and analysis:
- **Live Request Feed**: Streaming request log
- **Request Details**: Method, model, tokens, latency, status
- **Error Tracking**: Failed requests with error details
- **Filtering**: Filter by status, model, or provider

#### 5. Agents View (MoA)
Monitor Multi-Agent Orchestration:
- **Agent Status**: Active agents and their current state
- **Strategy Distribution**: Which strategies are in use
- **Performance Metrics**: Agent response quality scores
- **Configuration**: View agent settings and roles

#### 6. Log Viewer
Comprehensive logging interface:
- **Log Levels**: Filter by DEBUG, INFO, WARN, ERROR
- **Real-time Streaming**: Live log tail functionality
- **Search**: Filter logs by pattern
- **Export**: Copy or export log entries

#### 7. Tenants Admin
Multi-tenant administration:
- **Tenant List**: All configured tenants with quotas
- **Usage Tracking**: Per-tenant resource consumption
- **Quota Management**: View and monitor limits
- **API Key Management**: Keys per tenant

#### 8. Configuration View
System configuration overview:
- **Server Settings**: Host, port, workers
- **Database Connection**: SurrealDB status
- **Cache Configuration**: TTL, max size
- **Rate Limiting**: Current limits and window size

### TUI Keyboard Shortcuts

| Shortcut       | Action                           |
| -------------- | -------------------------------- |
| `Tab`          | Cycle forward through views      |
| `Shift+Tab`    | Cycle backward through views     |
| `↑/↓` or `j/k` | Navigate list items              |
| `Enter`        | Select/expand item               |
| `Space`        | Toggle provider/feature          |
| `r`            | Refresh data                     |
| `q`            | Quit application                 |
| `?` or `F1`    | Show help                        |
| `/`            | Open search (in supported views) |
| `Esc`          | Close modal/dialog               |

### TUI Color Scheme

The TUI uses a professional dark theme optimized for terminal readability:

- **Healthy/Success**: Bright Green (`#00FF87`)
- **Warning**: Bright Yellow (`#FFD700`)  
- **Error/Critical**: Bright Red (`#FF6B6B`)
- **Information**: Cyan (`#00D4FF`)
- **Primary Accent**: Bright Magenta (`#FF00FF`)
- **Background**: Dark gray for optimal contrast

### TUI Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│ ╔══════════════════════════════════════════════════════════════════════╗│
│ ║  GAUSSMERIDIAN SERVER ADMIN                    v3.1.0 │ ● Connected   ║│
│ ╚══════════════════════════════════════════════════════════════════════╝│
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  [ Dashboard ] [ Providers ] [ Models ] [ Requests ] [ Agents ] [Logs] │
│                                                                         │
│  ┌─ System Metrics ─────────────────┐ ┌─ Request Statistics ───────────┐│
│  │ CPU Usage    ████████░░░░░  65%  │ │ Total Requests    1,234,567   ││
│  │ Memory       ██████░░░░░░░  45%  │ │ Requests/sec             1,245 ││
│  │ Active Conns ███░░░░░░░░░░  150  │ │ Success Rate          99.87%  ││
│  └──────────────────────────────────┘ │ Error Rate              0.13%  ││
│                                       └─────────────────────────────────┘│
│  ┌─ Provider Health ──────────────────────────────────────────────────┐ │
│  │ ● OpenAI        Healthy     23ms    ████████████░░  85%             │ │
│  │ ● Anthropic     Healthy     45ms    █████████░░░░░  65%             │ │
│  │ ● Cohere        Degraded   120ms    ████░░░░░░░░░░  30%             │ │
│  │ ○ Ollama        Offline      --     ░░░░░░░░░░░░░░   0%             │ │
│  └──────────────────────────────────────────────────────────────────────┘│
│                                                                         │
│  ┌─ Recent Activity ────────────────────────────────────────────────────┐│
│  │ 14:23:45  POST /v1/chat/completions  gpt-4       245 tok   23ms  ✓ ││
│  │ 14:23:44  POST /v1/chat/completions  claude-3    189 tok   45ms  ✓ ││
│  │ 14:23:43  POST /v1/embeddings        ada-002     512 tok   12ms  ✓ ││
│  └──────────────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────────┤
│ Tab: Switch View │ ↑↓: Navigate │ r: Refresh │ q: Quit │ ?: Help        │
└─────────────────────────────────────────────────────────────────────────┘
```

### TUI Configuration

The TUI can be configured via environment variables:

```bash
# Required
GAUSSMERIDIAN_API_URL=http://localhost:3000

# Optional
GAUSSMERIDIAN_API_KEY=sk-your-api-key      # For authenticated endpoints
GAUSSMERIDIAN_TUI_REFRESH_RATE=1000        # Refresh interval in ms (default: 1000)
GAUSSMERIDIAN_TUI_LOG_LEVEL=info           # Log level: debug, info, warn, error
```

### Running TUI in Production

For production environments, run the TUI with minimal logging:

```bash
# Production mode
RUST_LOG=warn GAUSSMERIDIAN_API_URL=http://localhost:3000 \
  ./target/release/gaussmeridian-tui

# With authentication
GAUSSMERIDIAN_API_KEY="$API_KEY" GAUSSMERIDIAN_API_URL="$API_URL" \
  ./target/release/gaussmeridian-tui
```

### TUI Troubleshooting

**Issue: TUI not connecting to server**
```bash
# Verify server is running
curl $GAUSSMERIDIAN_API_URL/health

# Check API key validity
curl -H "Authorization: Bearer $GAUSSMERIDIAN_API_KEY" $GAUSSMERIDIAN_API_URL/v1/models
```

**Issue: Display corruption**
```bash
# Reset terminal
reset

# Ensure terminal supports Unicode
echo $TERM  # Should be xterm-256color or similar
```

**Issue: Slow refresh**
```bash
# Increase refresh interval
GAUSSMERIDIAN_TUI_REFRESH_RATE=2000 ./target/release/gaussmeridian-tui
```

---

## Monitoring & Observability

### Prometheus Metrics

**Available metrics:**
- `gaussmeridian_requests_total` - Total requests
- `gaussmeridian_request_duration_seconds` - Request latency
- `gaussmeridian_errors_total` - Total errors
- `gaussmeridian_rate_limit_hits_total` - Rate limit hits
- `gaussmeridian_cache_hits_total` - Cache hits
- `gaussmeridian_cache_misses_total` - Cache misses
- `gaussmeridian_active_connections` - Active connections
- `gaussmeridian_database_queries_total` - Database queries
- `gaussmeridian_token_usage_total` - Token usage

**Scraping configuration (prometheus.yml):**
```yaml
scrape_configs:
  - job_name: 'gaussmeridian'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```

### Grafana Dashboards

**Import pre-built dashboard:**
```bash
# Download dashboard
wget https://raw.githubusercontent.com/gaussmeridian/gaussmeridian/main/grafana/dashboard.json

# Import via Grafana UI or API
curl -X POST http://localhost:3001/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d @dashboard.json
```

### Health Checks

**Basic health check:**
```bash
curl http://localhost:3000/health
```

**Detailed readiness check:**
```bash
curl http://localhost:3000/ready
```

Expected response:
```json
{
  "status": "ready",
  "checks": {
    "database": "ok",
    "cache": "ok",
    "providers": {
      "openai": "ok",
      "anthropic": "ok"
    }
  }
}
```

### Log Aggregation

**ELK Stack (Elasticsearch, Logstash, Kibana):**

```yaml
# docker-compose.yml
version: '3.8'
services:
  elasticsearch:
    image: elasticsearch:8.11.0
    environment:
      - discovery.type=single-node
    ports:
      - "9200:9200"
  
  logstash:
    image: logstash:8.11.0
    volumes:
      - ./logstash.conf:/usr/share/logstash/pipeline/logstash.conf
    ports:
      - "5044:5044"
  
  kibana:
    image: kibana:8.11.0
    ports:
      - "5601:5601"
```

**Filebeat configuration:**
```yaml
# filebeat.yml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/gaussmeridian/*.log
    json.keys_under_root: true

output.logstash:
  hosts: ["localhost:5044"]
```

---

## Security

### SSL/TLS Configuration

**Using Let's Encrypt with nginx:**

```nginx
server {
    listen 443 ssl http2;
    server_name api.gaussmeridian.com;

    ssl_certificate /etc/letsencrypt/live/api.gaussmeridian.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.gaussmeridian.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Firewall Rules

```bash
# UFW (Ubuntu)
sudo ufw allow 22/tcp    # SSH
sudo ufw allow 443/tcp   # HTTPS
sudo ufw enable

# iptables
sudo iptables -A INPUT -p tcp --dport 22 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 443 -j ACCEPT
sudo iptables -A INPUT -j DROP
sudo service iptables save
```

### Secrets Management

**Using Vault:**

```bash
# Store secrets in Vault
vault kv put secret/gaussmeridian \
  jwt_secret="your-secret" \
  openai_api_key="sk-..." \
  database_password="secure-password"

# Read secrets in application
export JWT_SECRET=$(vault kv get -field=jwt_secret secret/gaussmeridian)
```

---

## Backup & Recovery

### Disaster Recovery Plan

1. **Regular Backups** - Daily automated backups to S3
2. **Replication** - Multi-region database replication
3. **Monitoring** - 24/7 monitoring with alerting
4. **Documentation** - Updated runbooks
5. **Testing** - Quarterly DR drills

### Recovery Procedures

**Complete system recovery:**

1. Provision new infrastructure
2. Restore database from backup
3. Deploy application
4. Verify functionality
5. Update DNS

**Estimated RTO:** 4 hours  
**Estimated RPO:** 24 hours

---

## Performance Tuning

### Application Tuning

```toml
[server]
workers = 16  # 2x CPU cores
max_connections = 10000
keepalive_timeout = 75

[database]
connection_pool_size = 200
query_timeout = 5000

[caching]
enabled = true
max_size_mb = 4096
ttl = 7200
```

### Database Tuning

```sql
-- Create indexes for common queries
DEFINE INDEX idx_user_email ON users FIELDS email;
DEFINE INDEX idx_api_key_hash ON api_keys FIELDS key_hash;
DEFINE INDEX idx_request_user_id ON requests FIELDS user_id;
DEFINE INDEX idx_request_created ON requests FIELDS created_at;
```

### System Tuning

```bash
# /etc/sysctl.conf
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.ip_local_port_range = 1024 65535
fs.file-max = 2097152

# Apply changes
sudo sysctl -p
```

---

## Troubleshooting

### Common Issues

**1. High Latency**
- Check database query performance
- Review cache hit rate
- Monitor provider response times
- Check network latency

**2. Memory Leaks**
- Monitor RSS memory over time
- Check for connection leaks
- Review cache sizes
- Profile with valgrind

**3. Database Connection Errors**
- Check connection pool settings
- Verify database is running
- Review firewall rules
- Check authentication credentials

### Debug Mode

Enable debug logging:
```bash
LOG_LEVEL=debug gaussmeridian-server
```

### Support

For production issues:
- Email: support@gaussmeridian.ai
- Slack: #gaussmeridian-support
- Phone: +1-xxx-xxx-xxxx (Enterprise only)

---

**© 2025 GaussMeridian. All rights reserved.**

