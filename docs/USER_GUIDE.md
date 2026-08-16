# GaussMeridian User Guide

**Version:** 3.1.0  
**Last Updated:** 2026-01-16

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Authentication](#authentication)
4. [API Reference](#api-reference)
5. [Rate Limiting](#rate-limiting)
6. [Usage Tracking & Billing](#usage-tracking--billing)
7. [Multi-Tenancy](#multi-tenancy)
8. [Terminal User Interface (TUI)](#terminal-user-interface-tui)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

---

## Introduction

GaussMeridian is an enterprise-grade AI model routing and orchestration platform that provides:

- **Unified API** - OpenAI-compatible API for all major LLM providers
- **Smart Routing** - Cost-optimized, latency-aware routing with automatic fallback
- **Rate Limiting** - Distributed rate limiting across multiple instances
- **Usage Tracking** - Comprehensive analytics and cost tracking
- **Multi-Tenancy** - Complete tenant isolation with RBAC
- **High Availability** - Circuit breakers, health checks, automatic failover

### Key Features

- 🚀 **High Performance** - 10,000+ req/s throughput
- 💰 **Cost Optimization** - Automatic routing to lowest-cost providers
- 🔒 **Enterprise Security** - JWT, API keys, OAuth2, RBAC
- 📊 **Observability** - Prometheus metrics, distributed tracing
- 🎯 **Smart Caching** - Multi-level caching with semantic similarity
- 🔄 **GaussMoA** - 8 advanced multi-agent orchestration strategies

---

## Getting Started

### Prerequisites

- Docker & Docker Compose (for local development)
- OR Rust 1.70+ (for building from source)
- SurrealDB 1.0+ (automatically started by Docker Compose)

### Quick Start with Docker

1. **Clone the repository:**
```bash
git clone https://github.com/gaussmeridian/gaussmeridian.git
cd gaussmeridian
```

2. **Start all services:**
```bash
docker-compose up -d
```

This starts:
- GaussMeridian API Server (port 3000)
- SurrealDB (port 8000)
- Prometheus (port 9090)
- Grafana (port 3001)

3. **Verify installation:**
```bash
curl http://localhost:3000/health
```

Expected response:
```json
{
  "status": "healthy",
  "version": "3.0.0"
}
```

### Building from Source

1. **Install Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. **Build the server:**
```bash
cd gaussmeridian
cargo build --release
```

3. **Run the server:**
```bash
./target/release/gaussmeridian-server
```

---

## Authentication

GaussMeridian supports three authentication methods:

### 1. API Keys (Recommended for Production)

API keys are the simplest and most secure method for production use.

#### Creating an API Key

**Step 1: Register a user account**

```bash
curl -X POST http://localhost:3000/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "username": "myusername",
    "password": "SecurePassword123!"
  }'
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "user_123",
    "email": "user@example.com",
    "username": "myusername"
  }
}
```

**Step 2: Create an API key**

```bash
curl -X POST http://localhost:3000/v1/api/keys \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "name": "Production API Key",
    "rate_limit_per_minute": 1000,
    "rate_limit_per_day": 100000,
    "expires_in_days": 365
  }'
```

Response:
```json
{
  "key_id": "key_abc123",
  "api_key": "sk-gaussmeridian-<your-key>",
  "key_prefix": "sk-gauss",
  "message": "API key created successfully. Store this key securely - it will not be shown again."
}
```

⚠️ **Important:** Save the API key immediately. It cannot be retrieved later.

#### Using an API Key

Include the API key in the `x-api-key` header:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-api-key: sk-gaussmeridian-<your-key>" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {"role": "user", "content": "Hello!"}
    ]
  }'
```

### 2. JWT Tokens (For User Sessions)

JWT tokens are ideal for web applications with user sessions.

**Login to get a token:**

```bash
curl -X POST http://localhost:3000/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePassword123!"
  }'
```

**Use the token:**

```bash
curl -X GET http://localhost:3000/v1/auth/me \
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
```

### 3. OAuth2 (Enterprise)

OAuth2 integration with providers like Google, GitHub, Azure AD.

See [OAuth2 Configuration Guide](./oauth2.md) for setup instructions.

---

## API Reference

### Base URL

```
http://localhost:3000/v1
```

### Chat Completions

OpenAI-compatible chat completions endpoint.

**Endpoint:** `POST /v1/chat/completions`

**Request:**
```json
{
  "model": "gpt-3.5-turbo",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "What is the capital of France?"}
  ],
  "temperature": 0.7,
  "max_tokens": 150,
  "stream": false
}
```

**Response:**
```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-3.5-turbo",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "The capital of France is Paris."
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 8,
    "total_tokens": 28
  }
}
```

### Streaming

Enable streaming for real-time responses:

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-api-key: YOUR_API_KEY" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Tell me a story"}],
    "stream": true
  }'
```

**Streaming Response Format:**
```
data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"Once"},"index":0}]}

data: {"id":"chatcmpl-123","choices":[{"delta":{"content":" upon"},"index":0}]}

data: {"id":"chatcmpl-123","choices":[{"delta":{"content":" a"},"index":0}]}

data: [DONE]
```

### List Models

**Endpoint:** `GET /v1/models`

**Response:**
```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1677652288,
      "owned_by": "openai"
    },
    {
      "id": "gpt-3.5-turbo",
      "object": "model",
      "created": 1677652288,
      "owned_by": "openai"
    }
  ]
}
```

### Usage & Balance

**Get current balance:**
```bash
curl -X GET http://localhost:3000/v1/balance \
  -H "x-api-key: YOUR_API_KEY"
```

**Response:**
```json
{
  "balance": 100.00,
  "currency": "USD",
  "last_updated": "2025-12-30T12:00:00Z"
}
```

---

## Rate Limiting

GaussMeridian implements distributed rate limiting with configurable limits per API key.

### Rate Limit Headers

Every response includes rate limit information:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 60
```

- `X-RateLimit-Limit`: Maximum requests per window
- `X-RateLimit-Remaining`: Remaining requests in current window
- `X-RateLimit-Reset`: Seconds until window resets

### Rate Limit Response

When rate limited, you'll receive a 429 status:

```json
{
  "error": {
    "message": "Rate limit exceeded. Please try again in 45 seconds.",
    "type": "rate_limit_error",
    "code": "rate_limit_exceeded"
  }
}
```

### Configuring Rate Limits

Rate limits are set per API key:

```bash
curl -X POST http://localhost:3000/v1/api/keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "name": "High Volume Key",
    "rate_limit_per_minute": 5000,
    "rate_limit_per_day": 500000
  }'
```

### Best Practices

1. **Implement exponential backoff** when receiving 429 responses
2. **Monitor rate limit headers** to avoid hitting limits
3. **Use multiple API keys** for different services/environments
4. **Cache responses** when possible to reduce API calls

---

## Usage Tracking & Billing

GaussMeridian tracks all API usage for analytics and billing.

### Viewing Usage

**Get usage summary:**
```bash
curl -X GET "http://localhost:3000/v1/analytics/usage?start_date=2025-12-01&end_date=2025-12-30" \
  -H "x-api-key: YOUR_API_KEY"
```

**Response:**
```json
{
  "total_requests": 15234,
  "total_tokens": 5234567,
  "total_cost": 15.67,
  "currency": "USD",
  "breakdown_by_model": [
    {
      "model": "gpt-4",
      "requests": 1234,
      "tokens": 456789,
      "cost": 12.34
    },
    {
      "model": "gpt-3.5-turbo",
      "requests": 14000,
      "tokens": 4777778,
      "cost": 3.33
    }
  ]
}
```

### Cost Calculation

Costs are calculated based on token usage:

| Model           | Prompt Cost (per 1K tokens) | Completion Cost (per 1K tokens) |
| --------------- | --------------------------- | ------------------------------- |
| GPT-4           | $0.03                       | $0.06                           |
| GPT-3.5 Turbo   | $0.0015                     | $0.002                          |
| Claude 3 Opus   | $0.015                      | $0.075                          |
| Claude 3 Sonnet | $0.003                      | $0.015                          |
| Claude 3 Haiku  | $0.00025                    | $0.00125                        |

---

## Multi-Tenancy

GaussMeridian supports complete tenant isolation for enterprise deployments.

### Creating a Tenant

```bash
curl -X POST http://localhost:3000/v1/admin/tenants \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN" \
  -d '{
    "name": "Acme Corp",
    "rate_limit_per_minute": 10000,
    "rate_limit_per_day": 1000000,
    "max_users": 100,
    "features": ["chat", "embeddings", "moa"]
  }'
```

### Tenant Isolation

Each tenant has:
- **Isolated users** - Users belong to exactly one tenant
- **Isolated API keys** - Keys are scoped to tenant
- **Isolated rate limits** - Per-tenant rate limiting
- **Isolated usage tracking** - Separate billing per tenant
- **Custom features** - Enable/disable features per tenant

---

## Terminal User Interface (TUI)

GaussMeridian includes a professional Terminal User Interface for monitoring and management.

### Quick Start

```bash
# Set the API URL
export GAUSSMERIDIAN_API_URL="http://localhost:3000"

# Run the TUI
./target/release/gaussmeridian-tui
```

### Available Views

| View          | Description                         | Key          |
| ------------- | ----------------------------------- | ------------ |
| **Dashboard** | Real-time system metrics and health | `1` or `Tab` |
| **Providers** | LLM provider status and management  | `2`          |
| **Models**    | Available models and pricing        | `3`          |
| **Requests**  | Live request monitoring             | `4`          |
| **Agents**    | MoA agent orchestration             | `5`          |
| **Logs**      | Real-time log viewer                | `6`          |

### Keyboard Navigation

| Key                    | Action              |
| ---------------------- | ------------------- |
| `Tab` / `Shift+Tab`    | Cycle through views |
| `↑` / `↓` or `j` / `k` | Navigate items      |
| `Enter`                | Select/expand       |
| `Space`                | Toggle option       |
| `r`                    | Refresh data        |
| `q`                    | Quit                |
| `?`                    | Help                |

### Dashboard Metrics

The TUI dashboard displays:
- **CPU & Memory Usage**: Real-time resource monitoring
- **Request Statistics**: Total requests, req/sec, success rate
- **Latency Metrics**: Average, P95, P99 with visual gauges
- **Provider Health**: Status of all configured providers
- **Cache Performance**: Hit rate and efficiency

### Use Cases

1. **Production Monitoring**: Watch server health in real-time
2. **Debugging**: Monitor request flow and errors
3. **Performance Analysis**: Track latency and throughput
4. **Provider Management**: Enable/disable providers on the fly

For detailed TUI documentation, see the [Admin Guide](./ADMIN_GUIDE.md#terminal-user-interface-tui).

---

## Best Practices

### Security

1. **Never commit API keys** to version control
2. **Rotate API keys regularly** (every 90 days recommended)
3. **Use environment variables** for configuration
4. **Enable HTTPS** in production
5. **Implement IP whitelisting** for sensitive applications

### Performance

1. **Use streaming** for long responses
2. **Implement client-side caching** for repeated queries
3. **Use batch requests** for multiple independent queries
4. **Monitor rate limits** to avoid throttling
5. **Set appropriate timeouts** (30s recommended)

### Cost Optimization

1. **Choose the right model** (GPT-3.5 for simple tasks, GPT-4 for complex)
2. **Limit max_tokens** to avoid unnecessary generation
3. **Use caching** for identical or similar queries
4. **Monitor usage** regularly via analytics dashboard
5. **Set up billing alerts** to avoid surprises

---

## Troubleshooting

### Common Issues

#### 401 Unauthorized

**Cause:** Invalid or missing API key

**Solution:**
- Verify API key is correct
- Check that key hasn't expired
- Ensure key is active (not revoked)

#### 429 Rate Limit Exceeded

**Cause:** Too many requests in time window

**Solution:**
- Implement exponential backoff
- Check rate limit headers
- Consider requesting higher limits
- Use multiple API keys for different services

#### 503 Service Unavailable

**Cause:** All provider backends are unavailable

**Solution:**
- Check provider status pages
- Verify API keys for providers are valid
- Check network connectivity
- Review circuit breaker status

### Getting Help

- **Documentation:** https://docs.gaussmeridian.ai
- **GitHub Issues:** https://github.com/gaussmeridian/gaussmeridian/issues
- **Discord Community:** https://discord.gg/gaussmeridian
- **Email Support:** support@gaussmeridian.ai

---

## Next Steps

- [Admin Guide](./admin-guide.md) - System administration and operations
- [Developer Guide](./developer-guide.md) - Building with GaussMeridian
- [API Reference](./api-reference.md) - Complete API documentation
- [Architecture](../ARCHITECTURE.md) - System architecture deep dive

---

**© 2025 GaussMeridian. All rights reserved.**

