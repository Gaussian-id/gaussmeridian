# GaussMeridian

**A self-hosted LLM gateway.** One OpenAI-compatible endpoint in front of OpenAI,
Anthropic, Google and local models — with a web console for keys, projects, budgets
and usage.

Rust gateway · Next.js console · SurrealDB · Redis · AGPL-3.0

> **Technical preview.** It runs, and the API is stable enough to build against.
> Expect breaking changes before 1.0.

---

## Run it

You need Docker with Compose v2. Nothing else.

```bash
git clone https://github.com/Gaussian-id/gaussmeridian
cd gaussmeridian
cp .env.example .env
docker compose up -d
```

The gateway and the mock provider are **pulled prebuilt from Docker Hub** — nothing
Rust compiles on your machine. The console builds from source on the first run, because
its source is in this repository; give it a few minutes.

```bash
docker compose ps                    # all services healthy
curl http://localhost:8000/health    # {"status":"healthy",...}
```

Copied verbatim, `.env.example` boots a working gateway with **zero provider keys and
zero spend** — requests route to a bundled mock. Its secrets are development-only;
replace every one in the "Required" block before this touches a network you care about.

Stop with `docker compose down`, or `down -v` to delete the database and cache too.

---

## Your first request

Everything except sign-up needs a key, and generation needs one scoped to a project.
The console does that in about a minute.

1. Open **http://localhost:3001** and create an account.
2. Create an organisation, then a project inside it.
3. In the project, create an API key and copy it.

```bash
curl http://localhost:8000/v1/chat/completions \
  -H "x-api-key: $GAUSSMERIDIAN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o-mini",
    "messages": [{"role": "user", "content": "Say hello in five words."}]
  }'
```

**The header is `x-api-key`.** `Authorization: Bearer` is parsed as a console session
JWT, so an API key sent that way returns 401 and looks like a bad key.

Two errors that mean your key, not your request, is wrong:

| Response | Meaning |
| --- | --- |
| `400 project_scope_required` | The key is not scoped to a project. Create it from inside a project. |
| `402 budget_exceeded` | The project's monthly budget is `0`. Raise it in project settings. |

---

## The API

OpenAI-compatible, so existing SDKs work by changing the base URL to
`http://localhost:8000/v1`.

| Endpoint | |
| --- | --- |
| `POST /v1/chat/completions` | Chat, streaming or not |
| `POST /v1/completions` | Legacy completions |
| `POST /v1/embeddings` | Embeddings |
| `GET /v1/models` | Every model the router can reach |
| `GET /health`, `/ready` | Liveness and readiness |
| `GET /metrics` | Prometheus |

`x-gaussmeridian-provider-selected` on the response tells you who actually served it.

---

## Bring your own models

Set one key in `.env` and restart — `docker compose up -d`:

```bash
GEMINI_API_KEY=...        # verified working end to end
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
```

Leave them all blank and the bundled mock answers instead, which is the point of the
zero-key path: the stack is provably up before any spend.

Per-customer keys (BYOK) are stored AES-256-encrypted under `BYOK_MASTER_KEY` and are
managed per project in the console.

---

## Images

Published on Docker Hub. `docker compose up` pulls them for you; these are here for
when you want to pin a version, run the gateway without this repository, or put it
behind your own orchestration.

| Image | What it is |
| --- | --- |
| [`gaussianid/gaussmeridian`](https://hub.docker.com/r/gaussianid/gaussmeridian) | The gateway |
| [`gaussianid/gaussmeridian-webui`](https://hub.docker.com/r/gaussianid/gaussmeridian-webui) | The console |
| [`gaussianid/gaussmeridian-mock`](https://hub.docker.com/r/gaussianid/gaussmeridian-mock) | The zero-key mock provider |

Each carries `latest` and a version tag. **Pin the version in anything you care
about** — `latest` moves.

```bash
docker pull gaussianid/gaussmeridian:3.0.0
```

Compose reads the tag from `TAG`, so the whole stack pins together:

```bash
TAG=3.0.0 docker compose up -d
```

The gateway needs SurrealDB and Redis reachable, and it reads its configuration from the
environment — `docker-compose.yml` in this repository is the reference for what it
expects. Running it bare:

```bash
docker run --rm -p 8000:8000 \
  -e GAUSSMERIDIAN_DB_URL=ws://your-surrealdb:8000 \
  -e GAUSSMERIDIAN_DB_USERNAME=root -e GAUSSMERIDIAN_DB_PASSWORD=... \
  -e REDIS_URL=redis://:password@your-redis:6379 \
  -e JWT_SECRET=... -e GAUSSMERIDIAN_API_KEY=... \
  gaussianid/gaussmeridian:3.0.0
```

If you modify the gateway and serve it over a network, set `SOURCE_OFFER_URL` to a URL
carrying your source. That is the whole of what AGPL Section 13 asks of you, and the
image advertises whatever you set in the `x-source-offer` header on every response.

---

## Ports

| | Port | |
| --- | --- | --- |
| Gateway API | 8000 | |
| Console | 3001 | |
| Grafana | 3000 | `--profile observability` |
| Prometheus | 9091 | `--profile observability` |
| SurrealDB UI | 8001 | loopback only |
| Redis | 6379 | loopback only |

The console and observability sit behind Compose profiles:

```bash
docker compose --profile webui --profile observability up -d
```

---

## What's in this repository

| | |
| --- | --- |
| `gaussmeridian/crates/` | The routing, provider, auth, cache and multi-agent libraries |
| `gaussmeridian/services/tui/` | Terminal client |
| `webui/` | The console — Next.js 16, React 19, Tailwind v4 |
| `docker-compose.yml`, `monitoring/` | The deployment you just ran |
| `docs/` | Everything else. There is no external docs site. |

**The gateway binary is distributed as a container image, not as source in this
repository.** `docker compose` pulls `gaussianid/gaussmeridian`; the library crates it
is built on are here and are AGPL-3.0 like the rest.

---

## Contributing

Issues and pull requests are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) for the
DCO sign-off and the checks CI runs. Security reports go through
[SECURITY.md](SECURITY.md), never a public issue.

## License

[GNU AGPL v3.0](LICENSE). If you run a modified version as a network service, you owe
its users the corresponding source — see [NOTICE](NOTICE) for the Section 13 offer and
the Section 7 permissions covering two dependencies. Third-party attributions are in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

"GaussMeridian" and the Meridian mark are trademarks — [TRADEMARKS.md](TRADEMARKS.md).
