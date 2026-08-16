# GaussMeridian API Specification

## Overview

GaussMeridian provides a unified API for accessing multiple LLM providers with advanced routing, load balancing, and enterprise features. The API is compatible with OpenAI's API format while extending it with additional capabilities.

## Base URL

```
http://localhost:3000/v1
```

## Authentication

### API Key Authentication

```http
Authorization: Bearer your-api-key
```

### JWT Authentication

```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

## Endpoints

### Chat Completions

#### POST /v1/chat/completions

Creates a completion for the chat message.

**Request Body:**

```json
{
  "model": "gpt-4",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant."
    },
    {
      "role": "user",
      "content": "Hello, world!"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 1000,
  "stream": false,
  "top_p": 1.0,
  "frequency_penalty": 0.0,
  "presence_penalty": 0.0,
  "stop": null,
  "n": 1,
  "logprobs": null,
  "echo": false,
  "suffix": null,
  "best_of": 1,
  "logit_bias": {},
  "user": "user123"
}
```

**Response:**

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop",
      "logprobs": null
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 10,
    "total_tokens": 30
  },
  "system_fingerprint": "fp_44709d6fcb"
}
```

### Completions

#### POST /v1/completions

Creates a completion for the provided prompt.

**Request Body:**

```json
{
  "model": "gpt-3.5-turbo",
  "prompt": "Hello, world!",
  "max_tokens": 100,
  "temperature": 0.7,
  "top_p": 1.0,
  "n": 1,
  "stream": false,
  "logprobs": null,
  "echo": false,
  "stop": null,
  "presence_penalty": 0.0,
  "frequency_penalty": 0.0,
  "best_of": 1,
  "logit_bias": {},
  "user": "user123"
}
```

**Response:**

```json
{
  "id": "cmpl-123",
  "object": "text_completion",
  "created": 1677652288,
  "model": "gpt-3.5-turbo",
  "choices": [
    {
      "text": "Hello! How are you doing today?",
      "index": 0,
      "logprobs": null,
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 5,
    "completion_tokens": 8,
    "total_tokens": 13
  }
}
```

### Embeddings

#### POST /v1/embeddings

Creates an embedding vector representing the input text.

**Request Body:**

```json
{
  "model": "text-embedding-ada-002",
  "input": "Hello, world!",
  "user": "user123"
}
```

**Response:**

```json
{
  "object": "list",
  "data": [
    {
      "object": "embedding",
      "embedding": [0.1, 0.2, 0.3, ...],
      "index": 0
    }
  ],
  "model": "text-embedding-ada-002",
  "usage": {
    "prompt_tokens": 5,
    "total_tokens": 5
  }
}
```

### Models

#### GET /v1/models

Lists the currently available models.

**Response:**

```json
{
  "data": [
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1677610602,
      "owned_by": "openai"
    },
    {
      "id": "gpt-3.5-turbo",
      "object": "model",
      "created": 1677610602,
      "owned_by": "openai"
    }
  ]
}
```

#### GET /v1/models/{model}

Retrieves a model instance.

**Response:**

```json
{
  "id": "gpt-4",
  "object": "model",
  "created": 1677610602,
  "owned_by": "openai"
}
```

### Health Checks

#### GET /health

Basic health check endpoint.

**Response:**

```json
{
  "status": "healthy",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "0.1.0"
}
```

#### GET /ready

Readiness check endpoint.

**Response:**

```json
{
  "status": "ready",
  "timestamp": "2024-01-01T00:00:00Z",
  "version": "0.1.0"
}
```

### Metrics

#### GET /metrics

Prometheus metrics endpoint.

**Response:**

```
# HELP gaussmeridian_requests_total Total number of requests
# TYPE gaussmeridian_requests_total counter
gaussmeridian_requests_total{endpoint="/v1/chat/completions",method="POST"} 1234

# HELP gaussmeridian_request_duration_seconds Request duration in seconds
# TYPE gaussmeridian_request_duration_seconds histogram
gaussmeridian_request_duration_seconds_bucket{endpoint="/v1/chat/completions",le="0.1"} 1000
gaussmeridian_request_duration_seconds_bucket{endpoint="/v1/chat/completions",le="0.5"} 1200
gaussmeridian_request_duration_seconds_bucket{endpoint="/v1/chat/completions",le="1.0"} 1234
```

## Data Types

### Message

```json
{
  "role": "system|user|assistant",
  "content": "string",
  "name": "string (optional)",
  "function_call": "object (optional)",
  "tool_calls": "array (optional)",
  "tool_call_id": "string (optional)"
}
```

### Choice

```json
{
  "index": 0,
  "message": "Message object",
  "finish_reason": "stop|length|content_filter|function_call|tool_calls",
  "logprobs": "object (optional)"
}
```

### Usage

```json
{
  "prompt_tokens": 0,
  "completion_tokens": 0,
  "total_tokens": 0
}
```

### Model

```json
{
  "id": "string",
  "object": "model",
  "created": 0,
  "owned_by": "string"
}
```

## Error Responses

### Standard Error Format

```json
{
  "error": {
    "message": "string",
    "type": "string",
    "code": "string",
    "param": "string (optional)",
    "request_id": "string (optional)"
  }
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `invalid_request_error` | The request was invalid |
| `authentication_error` | Authentication failed |
| `rate_limit_error` | Rate limit exceeded |
| `quota_exceeded` | Quota exceeded |
| `server_error` | Internal server error |
| `service_unavailable` | Service temporarily unavailable |

### HTTP Status Codes

| Status | Description |
|--------|-------------|
| 200 | Success |
| 400 | Bad Request |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 429 | Too Many Requests |
| 500 | Internal Server Error |
| 503 | Service Unavailable |

## Rate Limiting

### Rate Limit Headers

```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640995200
Retry-After: 60
```

### Rate Limit Response

```json
{
  "error": {
    "message": "Rate limit exceeded",
    "type": "rate_limit_error",
    "code": "rate_limit_exceeded",
    "retry_after": 60
  }
}
```

## Streaming

### Server-Sent Events Format

For streaming responses, the server sends Server-Sent Events (SSE):

```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
```

## Configuration

### Server Configuration

```toml
[server]
host = "0.0.0.0"
port = 3000
workers = 4
max_connections = 1000
request_timeout = 300

[server.cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]
allowed_headers = ["*"]
max_age = 86400

[server.tls]
enabled = false
cert_file = "cert.pem"
key_file = "key.pem"
```

### Provider Configuration

```toml
[[providers]]
name = "openai"
type = "openai"
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
models = ["gpt-4", "gpt-3.5-turbo"]
weight = 1.0
timeout = 30
max_retries = 3
health_check_interval = 60

[[providers]]
name = "anthropic"
type = "anthropic"
api_key = "${ANTHROPIC_API_KEY}"
base_url = "https://api.anthropic.com"
models = ["claude-3-opus", "claude-3-sonnet"]
weight = 1.0
timeout = 30
max_retries = 3
health_check_interval = 60
```

### Cache Configuration

```toml
[cache]
type = "redis"
url = "redis://localhost:6379"
ttl = 3600
max_size = 10000
compression = true

[cache.memory]
max_size = 1000
eviction_strategy = "lru"
```

### Authentication Configuration

```toml
[auth]
jwt_secret = "${JWT_SECRET}"
jwt_expiration = 3600
api_key_header = "x-api-key"
rate_limit_requests_per_minute = 60
rate_limit_tokens_per_minute = 10000
```

### Metrics Configuration

```toml
[metrics]
enabled = true
prometheus_enabled = true
sentry_dsn = "${SENTRY_DSN}"
datadog_api_key = "${DATADOG_API_KEY}"
```

## Load Balancing

### Strategies

1. **Round Robin**: Simple rotation through providers
2. **Weighted Round Robin**: Rotation with provider weights
3. **Least Connections**: Route to provider with fewest active connections
4. **Response Time**: Route to fastest responding provider
5. **Cost Optimization**: Route to most cost-effective provider

### Configuration

```toml
[load_balancer]
strategy = "weighted_round_robin"
health_check_interval = 30
circuit_breaker_threshold = 5
circuit_breaker_timeout = 60
```

## Caching

### Cache Keys

Cache keys are generated based on:
- Request hash (model, messages, parameters)
- User ID
- Tenant ID

### Cache Invalidation

- TTL-based expiration
- Manual invalidation via admin API
- Cache warming for frequently requested data

## Security

### Authentication Methods

1. **API Key**: Simple key-based authentication
2. **JWT**: Token-based authentication with expiration
3. **OAuth2**: Enterprise SSO integration

### Request Signing

For additional security, requests can be signed:

```http
X-Signature: sha256=abc123...
X-Timestamp: 1640995200
```

### Audit Logging

All requests are logged with:
- Request ID
- User ID
- Tenant ID
- Timestamp
- Request details
- Response status
- Performance metrics

## Monitoring

### Health Checks

- **Liveness**: Basic service health
- **Readiness**: Service ready to handle requests
- **Startup**: Service startup complete

### Metrics

- Request rate and latency
- Provider health and availability
- Cache hit/miss ratios
- Error rates by provider
- Circuit breaker state
- Resource usage (CPU, memory, disk)

### Alerts

- High error rates
- Provider unavailability
- High latency
- Resource exhaustion
- Rate limit violations

## Performance

### Benchmarks

| Metric | Value |
|--------|-------|
| **Throughput** | 10,000+ req/s |
| **Latency (p95)** | <50ms |
| **Memory Usage** | <100MB |
| **CPU Usage** | <50% under load |

### Optimization Features

- Async I/O throughout
- Connection pooling
- Request batching
- Zero-copy serialization
- Lock-free data structures

## Extensions

### Custom Headers

```http
X-Provider: openai
X-Model: gpt-4
X-Tenant: tenant123
X-User: user123
```

### Custom Parameters

```json
{
  "model": "gpt-4",
  "messages": [...],
  "provider": "openai",
  "tenant": "tenant123",
  "cache_ttl": 3600,
  "priority": "high"
}
```

## Versioning

API versioning is handled through the URL path:

- Current version: `/v1/`
- Future versions: `/v2/`, `/v3/`, etc.

## Deprecation Policy

- Deprecated features are announced 6 months in advance
- Deprecated features remain functional for 12 months
- Breaking changes are only made in major version releases

## Support

For API support:
- Documentation: [docs.gaussmeridian.com](https://docs.gaussmeridian.com)
- Issues: [GitHub Issues](https://github.com/gaussmeridian/gaussmeridian/issues)
- Email: api@gaussmeridian.com