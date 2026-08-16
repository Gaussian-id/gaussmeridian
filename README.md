# GaussMeridian

> **Technical Preview.** Expect rough edges and breaking changes. See [Known issues](#known-issues).

GaussMeridian is a self-hosted LLM gateway with an OpenAI-compatible API and a web console. It is being prepared as a fully open-source project: the gateway, console, deployment definitions, and public documentation are intended to ship together after the publication gates pass.

## Intended capabilities

- Route compatible chat and model requests across configured providers.
- Manage providers, credentials, models, routing policy, and usage from a web console.
- Stream responses through an OpenAI-compatible HTTP surface.
- Use Redis for cache and coordination.
- Use SurrealDB for durable application data.
- Run the API and console with a reviewed container-compose deployment.

These are release targets, not performance or compatibility guarantees. The verified feature matrix and runnable instructions will be added from the frozen release candidate.

## Deployment model

The planned self-hosted stack has four explicit parts:

1. GaussMeridian API
2. GaussMeridian web console
3. Redis
4. SurrealDB

Operators may run Redis and SurrealDB locally or provide compatible managed endpoints. PostgreSQL-based services are not a drop-in replacement for SurrealDB; a separate adapter and migration contract would be required.

## Release status

No supported release exists yet. Do not deploy this preparation template. Before the first Technical Preview, maintainers must:

- freeze and export a reviewed source commit;
- complete secret, provenance, dependency, and license scans;
- approve the source and web-console boundary;
- publish license and third-party notices;
- validate clean-machine installation and upgrade paths;
- approve the protected release environment.

## Project documents

- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)
- [Support](SUPPORT.md)
- [Governance](GOVERNANCE.md)
- [Roadmap](ROADMAP.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Trademark guidance](TRADEMARKS.md)

## License

The intended source license is Apache License 2.0, subject to the final legal and third-party review. The license files are deliberately absent while that gate remains closed.
