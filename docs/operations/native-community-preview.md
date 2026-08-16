# Native Community Preview Operations

This is the operational authority for building, testing, and qualifying the native
GaussMeridian community preview. It is designed to give contributors one reproducible
local stack, deterministic provider behavior, durable failure evidence, and scoped
cleanup.

## Evidence boundary

The preview qualifies the current controlled P0A-P5 mechanisms and their integration
at one exact Git commit. It checks startup, four OpenAI-compatible generation
transports, typed refusals, dependency and provider faults, interrupted streams,
durable evidence, graceful shutdown, and persistent restart behavior.

A passing report does not establish P6 online learning, learned-policy promotion,
model-quality or routing efficacy, production readiness, or universal crash freedom.
Those claims require separate evidence and release gates.

## Fixed local identities

The compose contract binds only loopback ports and uses dedicated names:

| Resource | Fixed identity |
| --- | --- |
| Compose project and network | `gaussmeridian-native-preview` |
| API container | `gaussmeridian-api` at `127.0.0.1:8020` |
| SurrealDB container | `gaussmeridian-native-surrealdb` at `127.0.0.1:8002` |
| Redis container | `gaussmeridian-native-redis` (network-only) |
| Provider fixture | `gaussmeridian-native-provider-simulator` at `127.0.0.1:18082` |
| Compose file | `docker-compose.native-preview.yml` |

Named volumes use the same `gaussmeridian-native-preview-*` prefix; the database volume is
`gaussmeridian-native-preview-surrealdb-data-v2`. Normal shutdown retains them so a restart
can prove persistence and a failed run remains diagnosable.

## Prerequisites and source identity

Run from the repository root unless a command explicitly changes directory:

```text
git --version
docker --version
docker compose version
rustc --version
cargo --version
python --version
```

Docker must be running. Use a current stable Rust toolchain and Python 3.11 or newer.
The preview does not need a paid provider key: its only enabled provider points to the
local deterministic fixture.

Every built image and container is labeled with the native source commit. Set it in
the same shell before invoking Compose.

PowerShell:

```powershell
$env:NATIVE_PREVIEW_SOURCE_COMMIT = git rev-parse HEAD
$env:NATIVE_PREVIEW_DB_PASSWORD = python -c "import secrets; print(secrets.token_urlsafe(32))"
$env:NATIVE_PREVIEW_JWT_SECRET = python -c "import secrets; print(secrets.token_urlsafe(48))"
$env:NATIVE_PREVIEW_PROVIDER_TOKEN = python -c "import secrets; print(secrets.token_urlsafe(32))"
```

POSIX shell:

```bash
export NATIVE_PREVIEW_SOURCE_COMMIT="$(git rev-parse HEAD)"
export NATIVE_PREVIEW_DB_PASSWORD="$(python -c 'import secrets; print(secrets.token_urlsafe(32))')"
export NATIVE_PREVIEW_JWT_SECRET="$(python -c 'import secrets; print(secrets.token_urlsafe(48))')"
export NATIVE_PREVIEW_PROVIDER_TOKEN="$(python -c 'import secrets; print(secrets.token_urlsafe(32))')"
```

The qualifier generates the JWT and fixture token in memory. It generates the database
password once at `.runtime/native-preview-credentials.json` and reuses it so the retained
database volume remains accessible across separate qualifier invocations. `.runtime/` is
Git-ignored; do not commit, copy, or hand-edit this local state. For manual Compose use,
keep the values only in the current shell. They are local fixture credentials, not paid-provider
credentials, and must never be reused outside the preview.

## Build, start, request, and stop

Build and start the exact source:

```text
docker compose --project-name gaussmeridian-native-preview --file docker-compose.native-preview.yml up --detach --build --wait --remove-orphans
```

`--wait` returns only after all four services are healthy. Verify the public API and
the two local fixtures.

PowerShell:

```powershell
Invoke-RestMethod http://127.0.0.1:8020/health
Invoke-RestMethod http://127.0.0.1:8020/ready
Invoke-RestMethod http://127.0.0.1:8002/health
Invoke-RestMethod http://127.0.0.1:18082/health
```

POSIX shell:

```bash
curl --fail --silent http://127.0.0.1:8020/health
curl --fail --silent http://127.0.0.1:8020/ready
curl --fail --silent http://127.0.0.1:8002/health
curl --fail --silent http://127.0.0.1:18082/health
```

Health and readiness are the credential-free smoke requests. Authenticated generation
requires durable seeded identity and budget records, which the qualification runner
creates and validates automatically.

Stop this project without discarding evidence:

```text
docker compose --project-name gaussmeridian-native-preview --file docker-compose.native-preview.yml down --remove-orphans
```

That command removes this preview's containers and network while retaining its named
volumes. Volume deletion is not part of routine development or troubleshooting.

## Architecture ownership

Keep changes inside the deepest module that owns the behavior:

| Concern | Owning seam |
| --- | --- |
| Deterministic routing policy and P0A-P5 evidence objects | `gaussmeridian/crates/gaussmeridian-core/src/routing_policy/` (`gaussmeridian-core`) |
| Request orchestration, selection, budget, streaming, and reconciliation | `gaussmeridian/services/server/src/routing/` |
| Durable requests, attempts, reservations, ledger, and evidence | `gaussmeridian/crates/gaussmeridian-db/src/repositories/` (`gaussmeridian-db`) |
| Provider adapters, retry, error, and streaming parsing | `gaussmeridian/crates/gaussmeridian-providers/src/` (`gaussmeridian-providers`) |
| Controlled provider faults | `gaussmeridian/scripts/fixtures/native_preview_provider.py` |
| End-to-end invariant oracle | `gaussmeridian/scripts/qualify_native_preview.py` |

Policy code should not perform HTTP or database orchestration. Provider adapters should
not decide identity, budget, or billing state. Request orchestration should use the
repositories for durable transitions instead of inventing parallel in-memory truth.

## Development loop

1. Create a narrow `feat/*` branch.
2. Add a failing regression or contract test.
3. Implement the smallest owning-module change.
4. Run the focused test, formatting, and lint checks.
5. Commit the candidate so the source identity is immutable.
6. Rebuild and exercise the native preview stack by hand for high-risk changes.

Rust workspace gate, from `gaussmeridian/`:

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --keep-going --workspace --all-targets --all-features -- -D warnings
```

Do not disable tests or weaken a durable invariant to make the suite pass. A change to
an error contract, commit point, shutdown rule, or evidence cardinality needs an
explicit regression test.

The optional MOA jemalloc dependency remains available on supported non-Windows targets.
It is target-gated out on Windows so `--all-features` does not invoke an upstream Unix
allocator build that requires `make` and rejects workspace paths containing spaces. This
does not change the Native API allocator or runtime behavior.

For a high-risk change (routing, persistence, provider, streaming, shutdown, or Docker),
exercise the full preview matrix by hand against the running stack: clean startup; chat
and text JSON/SSE; invalid startup configuration; database/Redis unavailable; malformed
or unauthenticated requests; provider timeout, `429`, `5xx`, malformed and empty
responses; stream client disconnect before and after the first byte; and graceful
shutdown plus a persistent restart. Confirm zero panics/aborts and zero unexpected
restarts in the container logs for each case.

## Troubleshooting

### Docker is unavailable

```text
docker version
docker compose version
docker info
```

Start the Docker engine if `docker info` cannot reach it. Do not work around a daemon
failure by changing the compose contract.

### Port collision

The preview requires host ports 8020, 8002, and 18082. On PowerShell:

```powershell
Get-NetTCPConnection -State Listen -LocalPort 8020,8002,18082 -ErrorAction SilentlyContinue
```

On Linux:

```bash
ss -ltn '( sport = :8020 or sport = :8002 or sport = :18082 )'
```

Stop the process you own or choose a different development time. Do not silently edit
the fixed ports: other tooling and documentation assume them.

### A service is unhealthy

```text
docker compose --project-name gaussmeridian-native-preview --file docker-compose.native-preview.yml ps
docker compose --project-name gaussmeridian-native-preview --file docker-compose.native-preview.yml logs --no-color --tail 200 api surrealdb redis provider
```

Check SurrealDB and Redis health before diagnosing the API. Preserve the logs. After
correcting the cause, rerun `up --detach --build --wait --remove-orphans` with
`NATIVE_PREVIEW_SOURCE_COMMIT` set.

### The image does not match HEAD

PowerShell:

```powershell
git rev-parse HEAD
docker inspect --format '{{ index .Config.Labels "org.gaussmeridian.source" }}' gaussmeridian-api
```

If the values differ, stop this project, set `NATIVE_PREVIEW_SOURCE_COMMIT` again, and
rebuild.

## Reporting a failure

Include the source commit, OS, Docker/Compose/Rust versions, the failing case, scoped
service logs, and exact commands run. Remove secrets and personal data. A controlled
failure is useful evidence; do not relabel it as a pass.
