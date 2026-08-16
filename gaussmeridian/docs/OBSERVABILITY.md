# Observability Guide

GaussMeridian provides deep observability for production operations.

## Metrics
- `/metrics` endpoint (Prometheus format)
- Tracks requests, latency, errors, cache, tokens, etc.
- Enable with `[metrics]` config

## Logging
- Structured logs (JSON or pretty)
- Correlation IDs for tracing requests
- Log levels: info, warn, error, debug

## Tracing
- Distributed tracing with `tracing` crate
- Spans for each request, provider call, plugin, etc.
- Integrate with Jaeger/Tempo via OpenTelemetry (future)

## Dashboards
- Use Grafana with Prometheus data source
- Example dashboard: requests/sec, latency, error rate, cache hits, active users

## Alerting
- Set up Prometheus alert rules (e.g. high error rate, slow requests)
- Integrate with PagerDuty, Slack, etc.

## Example Prometheus Config
```yaml
scrape_configs:
  - job_name: 'gaussmeridian'
    static_configs:
      - targets: ['gaussmeridian:8000']
```

## Example Grafana Panel
- Import Prometheus dashboard JSON from `docs/grafana_dashboard.json` 