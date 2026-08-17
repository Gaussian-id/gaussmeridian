# Roadmap

This roadmap describes what is shipped, what is in progress, and — just as
importantly — what is explicitly **not** claimed or promised yet. GaussMeridian
is pre-1.0. Nothing here is a commitment to a date.

## Shipped (this closure pack — PRD-26 P6, "Native Community Preview")

- An OpenAI-compatible gateway (`/v1/chat/completions` and related routes)
  that starts cleanly from a fresh database — no 500s on first boot.
- Corrected `/health` and `/ready` semantics (`/ready` requires at least one
  callable provider, not all of them).
- A bundled, publicly-shippable mock provider for a zero-key clone-and-run
  path.
- A clone-and-run `docker compose up` stack (`docker-compose.yml`) — gateway,
  SurrealDB, Redis, and an optional WebUI profile.
- A 96-case endpoint evidence harness (`scripts/evidence/`) that exercises
  every registered route against a live stack and publishes a redacted,
  reproducible report (`docs/evidence/report.md`).
- CI that runs the Rust workspace tests, the WebUI type-check, and the
  evidence harness against a live compose stack on every push and pull
  request; CodeQL scanning; and a release workflow that builds and (once
  package permissions are granted — see "Explicitly out of scope" below)
  publishes container images.
- The public-repository documentation set this pack introduces: this file,
  `SECURITY.md`, `SUPPORT.md`, `CODE_OF_CONDUCT.md`, `GOVERNANCE.md`,
  `TRADEMARKS.md`, `LICENSE`, `NOTICE`, `THIRD_PARTY_NOTICES.md`, and the
  `.github/` templates and workflows.

## Routing Intelligence — current state

| Boundary | Role | State |
| --- | --- | --- |
| Meridian P1 | Versioned deterministic complexity evidence and immutable eligible ballot | Accepted at its controlled-mechanism boundary |
| CARROT P2 | Conditional per-model outcome and cost prediction | Accepted at its controlled-mechanism boundary; **production promotion blocked** |
| BELLA P3 | Skill decomposition, proficiency, critic evidence, uncertainty, and attribution | Accepted at its controlled-mechanism boundary; **production promotion blocked** |
| R2-Router P4 | Governed budget-aware degradation policy | Accepted at its controlled-mechanism boundary; **production promotion blocked** |
| xRouter P5 | Compound routing action/scoring policy | Accepted at its controlled-mechanism boundary; **production promotion blocked** |
| Bandit/optimizer P6 | Governed online learning and release controls | This closure pack qualifies the isolated development mechanism only — see below |
| GaussMoA | Multi-agent orchestration service | Present as a feature; separate repair/product track, deferred from PRD-26 scope |

The qualification delivered as part of this closure pack is an isolated,
zero-spend development qualification for the P6 boundary. It proves
mechanism, transport, persistence, and crash behavior only. It establishes
**no** efficacy, market comparison, promotion, publication, or rollout
authority. P6 stays open.

## Known, currently-open gaps

- **Zero-key quickstart cannot complete an inference.** The bundled mock
  provider is registered as the `openai` adapter, but
  `gaussmeridian/gaussmeridian.toml`'s openai `models` allowlist has no
  overlap with the seeded routing catalog, so every request that routes to
  an openai-catalog model 503s. This is escalated to the project owner for a
  provider-catalog decision; a real provider key (e.g. `GEMINI_API_KEY`)
  works today. See `README.md` and `docs/evidence/report.md`.
- **Auth / rate-limit middleware** is defined but not fully layered into the
  server; **Redis / Moka caching** is declared in dependencies but not fully
  wired at runtime. See the "Current Build Truth" table in `README.md`.
- **33 pre-existing WebUI vitest specs fail**, unrelated to this closure
  pack. The 4 rename-touched spec files were checked at pre-rename HEAD and
  showed the same 27-failed/11-passed split there; the remaining files were
  not individually re-run at that commit. CI marks the vitest step
  accordingly (`continue-on-error`, with the count and scope documented
  inline) rather than hiding it or claiming a green suite.

## Explicitly out of scope (not implemented, not claimed)

- **Representative efficacy, market comparison, promotion, or rollout** for
  the routing layer. Descoped to a future mission — not achieved by this
  closure pack, and not claimed anywhere in this repository.
- **Public repository visibility.** A separate, project-owner-approved gate
  (governance gate G6, see `GOVERNANCE.md`) — unrelated to code readiness.
- **Fixing the 33 pre-existing WebUI test failures.** Tracked as known debt,
  not fixed here.
- **Xendit billing integration and the associated payment/credit surfaces.**
  A separate initiative; not part of this repository's current scope.
- **Any comparison against a named competitor**, and any claim of superior
  performance, features, or enterprise-readiness relative to another
  product. This project does not make those claims.

### Aspirational internal design targets (not measured, not benchmarked)

These numbers were internal design targets during earlier development. They
are **not** measured results, no independent or representative benchmark has
ever been published for this codebase, and they must not be read as a
capability claim — they are recorded here, out of the launch README, purely
as historical design intent:

| Metric | Target | Notes |
| --- | --- | --- |
| Throughput | 10,000+ req/s | Per instance |
| Latency (p50) | <10ms | Cached responses |
| Latency (p95) | <50ms | Non-cached |
| Memory | <500MB | Per instance |
| Availability | 99.99% | Annual uptime |
| Cache hit rate | >80% | For repeated queries |
