# Security Policy

## Supported versions

GaussMeridian is currently pre-1.0 and under active development on a single
active line (`main`). There is no long-term-support branch and no published
release yet — the workspace version (`3.0.0`, `gaussmeridian/Cargo.toml`) tracks
internal build milestones, not a public compatibility promise. Security fixes
land on `main`; there is nothing older to backport to.

## Reporting a vulnerability

**Do not open a public GitHub issue for a security report.** Public issues are
readable by anyone before a fix ships.

Instead, use GitHub's private vulnerability reporting for this repository
(Security tab → "Report a vulnerability"), or, if that is unavailable, open a
draft security advisory. Include:

- A description of the issue and its impact.
- Steps to reproduce (a minimal request, config, or `docker compose` setup is
  ideal — this project's `scripts/evidence/` harness and `docker-compose.yml`
  are good starting points if the issue is reachable through the HTTP API).
- The affected commit or version.
- Any proof-of-concept code, redacted of real credentials.

We will acknowledge receipt and work with you on a fix and disclosure timeline.
Because this project does not yet have a dedicated security team or a
published SLA, response times are best-effort — if you have not heard back in
a reasonable window, it is fair to follow up.

## Scope

In scope: the GaussMeridian gateway (`gaussmeridian/`), the WebUI (`webui/`),
the bundled mock provider (`gaussmeridian/scripts/mock-provider/`), and the
Docker Compose stacks in this repository.

Out of scope: vulnerabilities in third-party LLM providers themselves,
vulnerabilities that require an attacker to already have `BYOK_MASTER_KEY`,
`JWT_SECRET`, or database-root credentials (i.e. a fully compromised
deployment), and findings against the development-only defaults documented in
`.env.example` / `docker-compose.yml` (e.g. the placeholder
`BYOK_MASTER_KEY` and `JWT_SECRET` values are intentionally insecure
defaults for a fresh local clone — the compose file and `.env.example` say so
explicitly, and operators are expected to replace them before any real
deployment).

## Known, already-tracked issues

Do not report the following as new findings — they are known, deliberately
scoped, and tracked elsewhere in this repository:

- The bundled zero-key quickstart mock provider cannot currently complete an
  inference due to a provider-catalog / model-allowlist mismatch. See
  `README.md` ("Known limitation") and `docs/evidence/report.md` ("Known
  blocker").
- Routing-layer efficacy (CARROT P2 and later boundaries) is explicitly not
  qualified and production promotion is blocked by design — see `ROADMAP.md`
  and the Routing Intelligence Phase Map in `README.md`.
