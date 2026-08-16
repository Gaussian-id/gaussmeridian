# Security Guide

GaussMeridian is designed for secure, multi-tenant, production use.

## API Keys
- All requests require a valid API key
- Keys can be scoped to users/tenants
- Rotate and revoke keys regularly

## Rate Limiting
- Per-user and per-tenant rate limits
- Configurable burst and sustained limits
- Returns 429 on limit exceeded

## Tenant Isolation
- Data and quotas are isolated per tenant
- No cross-tenant data leakage

## TLS/HTTPS
- Use a reverse proxy (nginx, Traefik) or native TLS
- Never expose plain HTTP in production

## Secret Management
- Use environment variables or secret managers (Vault, AWS Secrets)
- Never hardcode secrets in code or configs

## RBAC (Role-Based Access Control)
- Optional: restrict actions by user/role
- Use least privilege for all accounts

## Logging & Auditing
- Structured logs, trace IDs, and audit trails
- Never log sensitive data (API keys, PII)

## Best Practices
- Keep dependencies up to date
- Run as non-root in containers
- Use network policies and firewalls
- Monitor for suspicious activity 