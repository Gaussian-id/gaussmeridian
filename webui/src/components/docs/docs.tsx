"use client";

import { useEffect, useState } from "react";

import { cn } from "@core/lib/utils";

import {
  ENDPOINT_GROUPS,
  ENV_ACCESS,
  ENV_CORE,
  ENV_INFRA,
  ENV_PROVIDERS,
  ENV_ROUTING,
  ERROR_ROWS,
  HEADER_FAMILIES,
} from "./docs-data";

import type { EnvVar } from "./docs-data";
import type { ReactNode } from "react";

const nav: { group: string; items: [string, string][] }[] = [
  {
    group: "Start here",
    items: [
      ["overview", "What it is"],
      ["install", "Install"],
      ["first-request", "First request"],
      ["sdks", "Use your SDK"],
    ],
  },
  {
    group: "Concepts",
    items: [
      ["tenancy", "Orgs and projects"],
      ["auth", "Authentication"],
      ["keys", "API keys"],
      ["byok", "Bring your own key"],
      ["routing", "How routing works"],
    ],
  },
  {
    group: "API",
    items: [
      ["endpoints", "Endpoint reference"],
      ["completions", "Chat completions"],
      ["streaming", "Streaming"],
      ["headers", "Response headers"],
      ["errors", "Errors"],
    ],
  },
  {
    group: "Operating",
    items: [
      ["configuration", "Configuration"],
      ["services", "Services and ports"],
      ["production", "Going to production"],
      ["troubleshooting", "Troubleshooting"],
    ],
  },
];

function DocSection({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <section id={id} data-doc className="border-border scroll-mt-24 border-t py-11">
      <h2 className="font-display text-2xl font-semibold tracking-tight">{title}</h2>
      <div className="mt-4">{children}</div>
    </section>
  );
}

function DocCode({ children, label }: { children: ReactNode; label?: string }) {
  return (
    <div className="my-3.5">
      {label ? (
        <div className="text-muted-foreground mb-1 font-mono text-[10.5px] tracking-[0.14em] uppercase">
          {label}
        </div>
      ) : null}
      <pre className="border-border bg-muted/40 overflow-x-auto rounded-[12px] border p-4 font-mono text-[13px] leading-[1.75]">
        <code>{children}</code>
      </pre>
    </div>
  );
}

function Callout({ tone = "note", children }: { tone?: "note" | "warn"; children: ReactNode }) {
  return (
    <div
      className={cn(
        "my-4 rounded-[10px] border-l-[3px] py-3 pr-4 pl-4 text-[14.5px] leading-relaxed",
        tone === "warn"
          ? "border-l-[var(--gauss-500)] bg-[color-mix(in_srgb,var(--gauss-500)_7%,transparent)]"
          : "border-l-border bg-muted/40",
      )}
    >
      {children}
    </div>
  );
}

function Table({ head, children }: { head: string[]; children: ReactNode }) {
  return (
    <div className="border-border my-4 overflow-x-auto rounded-[10px] border">
      <table className="w-full border-collapse text-left text-[14px]">
        <thead>
          <tr className="bg-muted/50">
            {head.map((h) => (
              <th
                key={h}
                className="text-muted-foreground border-border border-b px-3 py-2 font-mono text-[10.5px] font-semibold tracking-[0.12em] uppercase"
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}

const AUTH_LABEL: Record<string, string> = {
  key: "API key",
  session: "Session",
  admin: "Admin",
  none: "Public",
};

function AuthPill({ kind }: { kind: string }) {
  return (
    <span
      className={cn(
        "rounded px-1.5 py-0.5 font-mono text-[10px] whitespace-nowrap",
        kind === "none" && "bg-muted text-muted-foreground",
        kind === "key" && "bg-[color-mix(in_srgb,var(--accent)_12%,transparent)] text-primary",
        kind === "session" && "bg-muted text-foreground",
        kind === "admin" &&
          "bg-[color-mix(in_srgb,var(--gauss-500)_14%,transparent)] text-[var(--gauss-600)]",
      )}
    >
      {AUTH_LABEL[kind]}
    </span>
  );
}

const p = "text-foreground mt-2.5 text-[15.5px] leading-relaxed";
const pm = "text-muted-foreground mt-2.5 text-[15px] leading-relaxed";
const h3 = "text-foreground mt-7 mb-1 text-[15.5px] font-semibold";
const ic =
  "text-primary rounded-md bg-[color-mix(in_srgb,var(--accent)_10%,transparent)] px-1.5 py-0.5 font-mono text-[0.9em]";
const td = "border-border border-b px-3 py-2 align-top";
const tdMono = cn(td, "font-mono text-[12.5px]");

function EnvTable({ rows }: { rows: EnvVar[] }) {
  return (
    <Table head={["Variable", "Default", "Purpose"]}>
      {rows.map((r) => (
        <tr key={r.name}>
          <td className={cn(tdMono, "whitespace-nowrap")}>
            {r.name}
            {r.required ? (
              <span className="text-[var(--gauss-600)]" title="required">
                {" "}
                *
              </span>
            ) : null}
          </td>
          <td className={cn(tdMono, "text-muted-foreground")}>{r.def ?? (r.required ? "—" : "off")}</td>
          <td className={td}>{r.purpose}</td>
        </tr>
      ))}
    </Table>
  );
}

/** Reference documentation: sticky sidebar, scrollspy, no background animation. */
export function Docs() {
  const [active, setActive] = useState("overview");

  useEffect(() => {
    if (typeof IntersectionObserver === "undefined") return;
    const io = new IntersectionObserver(
      (entries) =>
        entries.forEach((e) => e.isIntersecting && setActive((e.target as HTMLElement).id)),
      { rootMargin: "-40% 0px -55% 0px" },
    );
    document.querySelectorAll<HTMLElement>("section[data-doc]").forEach((s) => io.observe(s));
    return () => io.disconnect();
  }, []);

  const sideLink = (id: string) =>
    cn(
      "block rounded-md border-l-2 px-3 py-1.5 transition-colors",
      active === id
        ? "text-primary border-[var(--gauss-500)] bg-[color-mix(in_srgb,var(--accent)_8%,transparent)]"
        : "text-muted-foreground border-transparent hover:text-foreground",
    );

  return (
    <div className="mx-auto grid max-w-[86rem] grid-cols-1 gap-11 px-6 pt-24 pb-24 lg:grid-cols-[240px_minmax(0,1fr)]">
      <aside className="top-24 hidden self-start text-sm lg:sticky lg:block">
        {nav.map((g) => (
          <div key={g.group} className="mb-5">
            <div className="text-muted-foreground mb-2 font-mono text-[10.5px] tracking-[0.16em] uppercase">
              {g.group}
            </div>
            {g.items.map(([id, label]) => (
              <a key={id} href={`#${id}`} className={sideLink(id)}>
                {label}
              </a>
            ))}
          </div>
        ))}
      </aside>

      <main className="max-w-3xl min-w-0">
        <div className="mb-2">
          <div className="text-muted-foreground font-mono text-[11px] tracking-[0.24em] uppercase">
            Documentation
          </div>
          <h1 className="font-display mt-3 text-4xl font-semibold tracking-tight sm:text-[44px]">
            Run it, call it, ship it.
          </h1>
          <p className={cn(pm, "max-w-[54ch]")}>
            Everything needed to stand GaussMeridian up, issue a key, and make a real completion —
            with the configuration, the full endpoint surface, and the failure modes you will
            actually meet.
          </p>
        </div>

        {/* ------------------------------------------------------------ */}
        <DocSection id="overview" title="What it is">
          <p className={p}>
            GaussMeridian is a self-hosted gateway that speaks the OpenAI API. Point an existing SDK
            at it instead of a provider, and it chooses a model per request, enforces your budgets
            and guardrails, and records what happened in enough detail to reconstruct the decision
            later.
          </p>
          <p className={pm}>The parts you interact with:</p>
          <ul className={cn(pm, "mt-2 list-disc space-y-1 pl-5")}>
            <li>
              <strong>Gateway</strong> — one OpenAI-compatible endpoint in front of OpenAI,
              Anthropic, Google, and local Ollama models.
            </li>
            <li>
              <strong>Console</strong> — a web UI for orgs, projects, keys, and routing traces.
            </li>
            <li>
              <strong>Router</strong> — scores candidate models per request against cost, quality
              floor, and capability band.
            </li>
            <li>
              <strong>Ledger</strong> — per-project budgets and per-request cost attribution.
            </li>
          </ul>
          <Callout>
            Everything runs on your infrastructure. There is no hosted service to sign up for — the
            install below is the product.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="install" title="Install">
          <p className={pm}>
            Docker with Compose is the only prerequisite. The database, cache, and a mock model
            provider all come up with the stack.
          </p>

          <div className={h3}>1 · Clone and configure</div>
          <DocCode>{`git clone https://github.com/Gaussian-id/gaussmeridian.git
cd gaussmeridian
cp .env.example .env`}</DocCode>
          <p className={pm}>
            <code className={ic}>.env.example</code> contains every required variable. Compose
            refuses to start if one is missing rather than silently defaulting, so a copied file is
            enough to boot.
          </p>

          <div className={h3}>2 · Start</div>
          <DocCode>{`docker compose --profile webui up -d`}</DocCode>
          <p className={pm}>
            The first run compiles the gateway from source and takes several minutes; later runs are
            seconds. Omit <code className={ic}>--profile webui</code> for the API alone, add{" "}
            <code className={ic}>--profile observability</code> for Prometheus and Grafana.
          </p>

          <div className={h3}>3 · Verify</div>
          <DocCode>{`curl http://localhost:8000/health
# {"status":"healthy","timestamp":"...","version":"3.0.0"}

curl http://localhost:8000/ready
# {"status":"ready",...}`}</DocCode>
          <p className={pm}>
            <code className={ic}>/ready</code> reports ready when at least one configured provider
            is callable — not when every one is, so a single working credential is enough to serve.
          </p>

          <Callout>
            With no provider credentials set, the gateway routes to a bundled mock so you can
            exercise the entire flow without spending anything. Add{" "}
            <code className={ic}>GEMINI_API_KEY</code>, <code className={ic}>OPENAI_API_KEY</code>,
            or <code className={ic}>ANTHROPIC_API_KEY</code> to <code className={ic}>.env</code> and
            restart to use real models.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="first-request" title="First request">
          <p className={pm}>
            Four conditions must hold before a completion succeeds. Nearly every first-time failure
            is one of them.
          </p>
          <Table head={["#", "Condition", "If missing"]}>
            <tr>
              <td className={tdMono}>1</td>
              <td className={td}>An organisation and a project exist.</td>
              <td className={td}>Nothing to scope a key to.</td>
            </tr>
            <tr>
              <td className={tdMono}>2</td>
              <td className={td}>
                The project&apos;s <code className={ic}>budget_monthly</code> is above zero.
              </td>
              <td className={tdMono}>402 payment_required</td>
            </tr>
            <tr>
              <td className={tdMono}>3</td>
              <td className={td}>The API key is scoped to that project.</td>
              <td className={tdMono}>400 project_scope_required</td>
            </tr>
            <tr>
              <td className={tdMono}>4</td>
              <td className={td}>
                The key is sent as <code className={ic}>x-api-key</code>.
              </td>
              <td className={tdMono}>401 unauthorized</td>
            </tr>
          </Table>
          <p className={pm}>
            Projects are created with a zero budget, so step 2 is a real step and not a formality.
            Set it in project settings, or over the API:
          </p>
          <DocCode label="set a budget">{`curl -X PATCH http://localhost:8000/v1/orgs/$ORG/projects/$PROJECT \\
  -H "authorization: Bearer $SESSION" \\
  -H "content-type: application/json" \\
  -d '{"budget_monthly": 100.0}'`}</DocCode>

          <div className={h3}>Then call it</div>
          <DocCode label="request">{`curl http://localhost:8000/v1/chat/completions \\
  -H "content-type: application/json" \\
  -H "x-api-key: $GAUSSMERIDIAN_KEY" \\
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "What is 2+2?"}],
    "max_tokens": 64
  }'`}</DocCode>
          <DocCode label="response">{`{
  "id": "gemini-96d72d5c-07c2-4bd7-920f-75412c4fcdc7",
  "object": "chat.completion",
  "created": 1786851159,
  "model": "gemini-2.5-flash",
  "choices": [{
    "index": 0,
    "message": { "role": "assistant", "content": "4" },
    "finish_reason": "stop"
  }],
  "usage": { "prompt_tokens": 14, "completion_tokens": 1, "total_tokens": 15 }
}`}</DocCode>
          <Callout tone="warn">
            Set <code className={ic}>max_tokens</code> generously. Reasoning models spend tokens
            before emitting visible text, so a tight ceiling can return{" "}
            <code className={ic}>finish_reason: &quot;length&quot;</code> with empty content — a
            successful request with nothing in it.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="sdks" title="Use your SDK">
          <p className={pm}>
            The API is OpenAI-shaped, so official SDKs work once the base URL points at the gateway.
            The one adjustment is the credential header.
          </p>
          <DocCode label="python">{`from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8000/v1",
    api_key="unused",                                  # the SDK requires a value
    default_headers={"x-api-key": "YOUR_MERIDIAN_KEY"}, # this is what authenticates
)

resp = client.chat.completions.create(
    model="gemini-2.5-flash",
    messages=[{"role": "user", "content": "Hello"}],
)
print(resp.choices[0].message.content)`}</DocCode>
          <DocCode label="typescript">{`import OpenAI from "openai";

const client = new OpenAI({
  baseURL: "http://localhost:8000/v1",
  apiKey: "unused",
  defaultHeaders: { "x-api-key": process.env.GAUSSMERIDIAN_KEY! },
});

const resp = await client.chat.completions.create({
  model: "gemini-2.5-flash",
  messages: [{ role: "user", content: "Hello" }],
});`}</DocCode>
          <p className={pm}>
            The SDK will also send <code className={ic}>Authorization: Bearer unused</code>. That is
            harmless: the gateway checks <code className={ic}>x-api-key</code> first and only falls
            back to the bearer token when no API key header is present.
          </p>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="tenancy" title="Orgs and projects">
          <p className={p}>Three levels, each with a distinct job.</p>
          <div className={h3}>Organisation</div>
          <p className={pm}>
            The membership and plan boundary. People are invited to an org and hold a role there.
            Returned fields include <code className={ic}>plan</code>,{" "}
            <code className={ic}>balance</code>, <code className={ic}>member_count</code>, and{" "}
            <code className={ic}>project_count</code>.
          </p>
          <div className={h3}>Project</div>
          <p className={pm}>
            Where budget and routing behaviour live — map one to an application or an environment.
            Spend is attributed per project.
          </p>
          <Table head={["Field", "Meaning"]}>
            <tr>
              <td className={tdMono}>budget_monthly</td>
              <td className={td}>Monthly ceiling. Zero blocks generation entirely.</td>
            </tr>
            <tr>
              <td className={tdMono}>hard_limit</td>
              <td className={td}>Whether exceeding the budget rejects rather than warns.</td>
            </tr>
            <tr>
              <td className={tdMono}>quality_floor</td>
              <td className={td}>Minimum acceptable candidate score, 0–1.</td>
            </tr>
            <tr>
              <td className={tdMono}>lambda</td>
              <td className={td}>Cost/quality trade-off weight used when ranking candidates.</td>
            </tr>
            <tr>
              <td className={tdMono}>tau_moa</td>
              <td className={td}>Per-project complexity threshold for mixture-of-agents.</td>
            </tr>
            <tr>
              <td className={tdMono}>alert_webhook_url</td>
              <td className={td}>Notified on budget events. Validated against SSRF on save.</td>
            </tr>
          </Table>
          <div className={h3}>API key</div>
          <p className={pm}>
            Belongs to a project, inherits its budget, carries its own rate limits, and is revoked
            independently of the others.
          </p>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="auth" title="Authentication">
          <p className={p}>
            Two credential types that are not interchangeable. Confusing them is the most common
            setup mistake, and the failure looks like a broken key rather than a wrong header.
          </p>
          <Table head={["Credential", "Header", "Used by", "Obtained from"]}>
            <tr>
              <td className={td}>API key</td>
              <td className={tdMono}>x-api-key</td>
              <td className={td}>Your application</td>
              <td className={tdMono}>POST /v1/api/keys</td>
            </tr>
            <tr>
              <td className={td}>Session token</td>
              <td className={tdMono}>Authorization: Bearer</td>
              <td className={td}>The console</td>
              <td className={tdMono}>POST /v1/auth/login</td>
            </tr>
          </Table>
          <Callout tone="warn">
            A bearer token is validated as a <em>session</em> token, never as an API key. Send an API
            key that way and the answer is{" "}
            <code className={ic}>401 invalid credentials</code> — technically correct, and
            indistinguishable from a revoked key. If a key you just created is rejected, check the
            header name before anything else.
          </Callout>
          <p className={pm}>
            In the browser the console does not use a header at all: the session is an http-only
            cookie, and the front end additionally rejects any state-changing request whose{" "}
            <code className={ic}>Origin</code> does not match its own host. Tooling pointed at the
            console&apos;s API routes must send an <code className={ic}>Origin</code> header or it
            will receive <code className={ic}>403</code>.
          </p>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="keys" title="API keys">
          <DocCode label="create">{`curl -X POST http://localhost:8000/v1/api/keys \\
  -H "authorization: Bearer $SESSION" \\
  -H "content-type: application/json" \\
  -d '{
    "name": "production",
    "project_id": "pblriuae4eyfke39q2af",
    "rate_limit_per_minute": 60
  }'`}</DocCode>
          <DocCode label="response — the secret appears once">{`{
  "key_id": "287zatv5bi24ma6obc2b",
  "api_key": "6e3e6dbca450c82f7237cb1a6a9fa47f...",
  "key_prefix": "6e3e6dbc",
  "message": "API key created successfully. Store this key securely - it will not be shown again."
}`}</DocCode>
          <p className={pm}>
            Only the prefix and a hash are stored. Listing keys returns metadata and never the
            secret. Revoke by id:
          </p>
          <DocCode label="revoke">{`curl -X POST http://localhost:8000/v1/api/keys/revoke \\
  -H "authorization: Bearer $SESSION" \\
  -H "content-type: application/json" \\
  -d '{"key_id": "287zatv5bi24ma6obc2b"}'`}</DocCode>
          <Callout tone="warn">
            Omitting <code className={ic}>project_id</code> creates an <em>unscoped</em> key. It
            authenticates fine and passes <code className={ic}>/v1/models</code>, but generation
            returns <code className={ic}>400 project_scope_required</code>, because there is no
            project to bill.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="byok" title="Bring your own key">
          <p className={p}>
            BYOK lets a project call a provider with its own credential instead of the gateway&apos;s.
            Keys are encrypted with <code className={ic}>BYOK_MASTER_KEY</code> before storage and
            are never returned by any endpoint afterwards.
          </p>
          <DocCode label="register">{`curl -X POST http://localhost:8000/v1/byok/keys \\
  -H "authorization: Bearer $SESSION" \\
  -H "x-project-id: $PROJECT" \\
  -H "content-type: application/json" \\
  -d '{"provider": "google", "api_key": "..."}'
# {"message":"Provider key registered","provider":"google"}`}</DocCode>
          <p className={pm}>
            Writes are gated on <code className={ic}>BYOK_ADMIN_EMAILS</code>. With that variable
            unset every BYOK write returns <code className={ic}>403</code> — the default is closed,
            not open.
          </p>
          <Callout tone="warn">
            <strong>Known issue.</strong> Registration stores the key and logs the correct project,
            but <code className={ic}>GET /v1/byok/keys</code> returns an empty list and{" "}
            <code className={ic}>DELETE /v1/byok/keys/:provider</code> returns{" "}
            <code className={ic}>404</code>. A registered key therefore cannot be listed, rotated,
            or revoked through the API. Treat BYOK as write-only until this is fixed.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="routing" title="How routing works">
          <p className={p}>
            Each request is scored against the catalog rather than sent straight to the named model.
            The model you ask for is the starting point, not necessarily the one that serves.
          </p>
          <ol className={cn(pm, "mt-3 list-decimal space-y-1.5 pl-5")}>
            <li>
              <strong>Complexity.</strong> The prompt is scored 0–1, surfaced as{" "}
              <code className={ic}>x-gaussmeridian-complexity</code>.
            </li>
            <li>
              <strong>Candidates.</strong> Models in the required capability band are ranked on
              score and cost, weighted by the project&apos;s <code className={ic}>lambda</code>.
            </li>
            <li>
              <strong>Band adjustment.</strong> With nothing available in the desired band, the
              router moves to the nearest one and reports why in{" "}
              <code className={ic}>x-gaussmeridian-band-reason</code>.
            </li>
            <li>
              <strong>Budget check.</strong> The projected cost is reserved against the project
              budget; insufficient budget fails the request before any provider is called.
            </li>
            <li>
              <strong>Dispatch and guardrails.</strong> The response is scanned if guardrails are
              on, and retried against the next candidate on provider failure.
            </li>
          </ol>
          <div className={h3}>Cascade</div>
          <p className={pm}>
            Tries a cheaper model first and escalates when confidence falls below{" "}
            <code className={ic}>GAUSSMERIDIAN_CASCADE_THRESHOLD</code>.
          </p>
          <div className={h3}>Mixture of agents</div>
          <p className={pm}>
            Runs several models on complex prompts and reconciles the answers. Gated on complexity
            exceeding <code className={ic}>GAUSSMERIDIAN_TAU_MOA</code> — 0.7 means high-complexity
            prompts only, 0.0 every request, 1.0 never.
          </p>
          <Callout>
            Every decision is recoverable after the fact. Take{" "}
            <code className={ic}>x-gaussmeridian-ballot-id</code> from a response and read the full
            candidate set and policy version from{" "}
            <code className={ic}>GET /v1/route-decisions/:request_id</code>.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="endpoints" title="Endpoint reference">
          <p className={pm}>
            The complete surface, grouped by what it is for. &ldquo;API key&rdquo; means{" "}
            <code className={ic}>x-api-key</code>; &ldquo;Session&rdquo; means a bearer token from
            login; &ldquo;Admin&rdquo; additionally requires membership of{" "}
            <code className={ic}>SUPERADMIN_EMAILS</code>.
          </p>
          {ENDPOINT_GROUPS.map((g) => (
            <div key={g.id} className="mt-7">
              <div className={h3}>{g.title}</div>
              <p className={cn(pm, "mt-0")}>{g.blurb}</p>
              <Table head={["Method", "Path", "Auth", "Summary"]}>
                {g.endpoints.map((e) => (
                  <tr key={e.method + e.path}>
                    <td className={cn(tdMono, "text-muted-foreground whitespace-nowrap")}>
                      {e.method}
                    </td>
                    <td className={cn(tdMono, "whitespace-nowrap")}>{e.path}</td>
                    <td className={td}>
                      <AuthPill kind={e.auth} />
                    </td>
                    <td className={td}>{e.summary}</td>
                  </tr>
                ))}
              </Table>
            </div>
          ))}
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="completions" title="Chat completions">
          <p className={pm}>
            <code className={ic}>POST /v1/chat/completions</code> — request and response bodies match
            the OpenAI schema.
          </p>
          <Table head={["Field", "Required", "Notes"]}>
            <tr>
              <td className={tdMono}>model</td>
              <td className={td}>yes</td>
              <td className={td}>
                Must appear in <code className={ic}>GET /v1/models</code>, or it will not route.
              </td>
            </tr>
            <tr>
              <td className={tdMono}>messages</td>
              <td className={td}>yes</td>
              <td className={td}>
                Non-empty. An empty array is <code className={ic}>400 empty_messages</code>.
              </td>
            </tr>
            <tr>
              <td className={tdMono}>max_tokens</td>
              <td className={td}>no</td>
              <td className={td}>Also caps the output budget the router reserves.</td>
            </tr>
            <tr>
              <td className={tdMono}>temperature</td>
              <td className={td}>no</td>
              <td className={td}>Passed to the provider unchanged.</td>
            </tr>
            <tr>
              <td className={tdMono}>stream</td>
              <td className={td}>no</td>
              <td className={td}>
                Prefer the dedicated <code className={ic}>/stream</code> endpoint.
              </td>
            </tr>
          </Table>
          <p className={pm}>
            <code className={ic}>GET /v1/usage/:request_id</code> returns the token and cost
            accounting for a completed request, using the <code className={ic}>id</code> from the
            response body.
          </p>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="streaming" title="Streaming">
          <p className={pm}>
            <code className={ic}>POST /v1/chat/completions/stream</code> returns server-sent events
            in the OpenAI chunk format, terminated by <code className={ic}>data: [DONE]</code>.
          </p>
          <DocCode>{`curl -N http://localhost:8000/v1/chat/completions/stream \\
  -H "content-type: application/json" \\
  -H "x-api-key: $GAUSSMERIDIAN_KEY" \\
  -d '{"model":"gemini-2.5-flash","messages":[{"role":"user","content":"Count to three"}]}'

data: {"id":"...","object":"chat.completion.chunk","choices":[{"delta":{"content":"One"}}]}
data: {"id":"...","object":"chat.completion.chunk","choices":[{"delta":{"content":", two"}}]}
data: [DONE]`}</DocCode>
          <Callout>
            Routing headers are sent with the response head, before the first chunk — so a streaming
            client can read the selected model and cost estimate immediately, without waiting for
            the body.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="headers" title="Response headers">
          <p className={pm}>
            A completion carries around 46 <code className={ic}>x-gaussmeridian-*</code> headers
            describing the decision. They are grouped below by what question they answer.
          </p>
          {HEADER_FAMILIES.map((f) => (
            <div key={f.id} className="mt-7">
              <div className={h3}>{f.title}</div>
              <p className={cn(pm, "mt-0")}>{f.blurb}</p>
              <Table head={["Header", "Meaning"]}>
                {f.rows.map(([name, meaning]) => (
                  <tr key={name}>
                    <td className={cn(tdMono, "whitespace-nowrap")}>{name}</td>
                    <td className={td}>{meaning}</td>
                  </tr>
                ))}
              </Table>
            </div>
          ))}
          <Callout tone="warn">
            <strong>Header size.</strong> <code className={ic}>catalog-version</code> and{" "}
            <code className={ic}>price-version</code> enumerate every model in the catalog on every
            response — together roughly 10&nbsp;KB, against a typical 400-byte body. Reverse proxies
            commonly cap response headers at 4–8&nbsp;KB (nginx&apos;s{" "}
            <code className={ic}>proxy_buffer_size</code> defaults to 4&nbsp;KB), and will return
            502 rather than truncate. If you put a proxy in front of the gateway, raise its header
            buffers.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="errors" title="Errors">
          <p className={pm}>Errors are JSON and carry a machine-readable type and code.</p>
          <DocCode>{`{
  "error": {
    "message": "Messages must contain at least one item",
    "type": "invalid_request_error",
    "code": "empty_messages",
    "param": "messages"
  }
}`}</DocCode>
          <Table head={["Status", "Code", "Meaning"]}>
            {ERROR_ROWS.map(([status, code, meaning], i) => (
              <tr key={i}>
                <td className={cn(tdMono, "whitespace-nowrap")}>{status}</td>
                <td className={cn(tdMono, "whitespace-nowrap")}>{code}</td>
                <td className={td}>{meaning}</td>
              </tr>
            ))}
          </Table>
          <Callout>
            The admin surface answers <code className={ic}>404</code> rather than{" "}
            <code className={ic}>403</code> for non-superadmins. That is deliberate: it makes those
            routes indistinguishable from routes that do not exist, so probing reveals nothing.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="configuration" title="Configuration">
          <p className={pm}>
            All configuration is environment variables read at startup. Variables marked{" "}
            <span className="text-[var(--gauss-600)]">*</span> are required — Compose refuses to
            start without them rather than falling back to a default.
          </p>
          <div className={h3}>Core</div>
          <EnvTable rows={ENV_CORE} />
          <div className={h3}>Providers</div>
          <EnvTable rows={ENV_PROVIDERS} />
          <div className={h3}>Routing behaviour</div>
          <EnvTable rows={ENV_ROUTING} />
          <div className={h3}>Access control</div>
          <EnvTable rows={ENV_ACCESS} />
          <div className={h3}>Infrastructure</div>
          <EnvTable rows={ENV_INFRA} />
          <Callout tone="warn">
            <code className={ic}>REDIS_URL</code> is deliberately unprefixed. The server reads that
            exact name and nothing else — setting{" "}
            <code className={ic}>GAUSSMERIDIAN_REDIS_URL</code> instead leaves the gateway on its
            built-in localhost default, pointing at the wrong host and unauthenticated, while Redis
            itself demands a password.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="services" title="Services and ports">
          <p className={pm}>
            The Compose project is named <code className={ic}>gaussmeridian</code> regardless of the
            directory you cloned into. Anything bound to <code className={ic}>127.0.0.1</code> is
            deliberately not reachable from the network.
          </p>
          <Table head={["Service", "Address", "Profile", "Purpose"]}>
            <tr>
              <td className={td}>gaussmeridian</td>
              <td className={tdMono}>:8000</td>
              <td className={td}>default</td>
              <td className={td}>The API. Metrics on 127.0.0.1:9090.</td>
            </tr>
            <tr>
              <td className={td}>webui</td>
              <td className={tdMono}>:3001</td>
              <td className={tdMono}>webui</td>
              <td className={td}>The console. Container listens on 3000.</td>
            </tr>
            <tr>
              <td className={td}>surrealdb</td>
              <td className={tdMono}>127.0.0.1:8001</td>
              <td className={td}>default</td>
              <td className={td}>Database, file-backed volume.</td>
            </tr>
            <tr>
              <td className={td}>redis</td>
              <td className={tdMono}>internal</td>
              <td className={td}>default</td>
              <td className={td}>Cache and rate-limit state. Password required.</td>
            </tr>
            <tr>
              <td className={td}>mock-provider</td>
              <td className={tdMono}>internal</td>
              <td className={td}>default</td>
              <td className={td}>Deterministic provider so the stack runs with no credentials.</td>
            </tr>
            <tr>
              <td className={td}>prometheus</td>
              <td className={tdMono}>127.0.0.1:9091</td>
              <td className={tdMono}>observability</td>
              <td className={td}>Scrapes the gateway.</td>
            </tr>
            <tr>
              <td className={td}>grafana</td>
              <td className={tdMono}>:3000</td>
              <td className={tdMono}>observability</td>
              <td className={td}>Dashboards.</td>
            </tr>
          </Table>
          <Callout>
            Grafana takes host port 3000, which is why the console is published on 3001. Both can be
            run together.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="production" title="Going to production">
          <p className={pm}>
            The defaults are tuned for a clone-and-run first experience. Before exposing the stack
            to anyone else:
          </p>
          <ul className={cn(pm, "mt-2 list-disc space-y-1.5 pl-5")}>
            <li>
              <strong>Replace every secret.</strong> The shipped{" "}
              <code className={ic}>.env.example</code> values are placeholders, and{" "}
              <code className={ic}>BYOK_MASTER_KEY</code> falls back to a development-only default.
              Generate one with <code className={ic}>openssl rand -base64 32</code>.
            </li>
            <li>
              <strong>Set the access lists.</strong> Both{" "}
              <code className={ic}>SUPERADMIN_EMAILS</code> and{" "}
              <code className={ic}>BYOK_ADMIN_EMAILS</code> default to empty, which closes those
              surfaces to everyone including you.
            </li>
            <li>
              <strong>Raise proxy header buffers.</strong> See the note under response headers — the
              defaults on most proxies are smaller than what the gateway emits.
            </li>
            <li>
              <strong>Terminate TLS in front.</strong> The gateway serves plain HTTP.
            </li>
            <li>
              <strong>Persist the volumes.</strong>{" "}
              <code className={ic}>surrealdb-data</code> holds every account, key, and ledger entry.{" "}
              <code className={ic}>docker compose down -v</code> destroys it.
            </li>
            <li>
              <strong>Set per-key rate limits.</strong>{" "}
              <code className={ic}>rate_limit_per_minute</code> is optional at creation and unlimited
              when omitted.
            </li>
          </ul>
          <Callout tone="warn">
            <code className={ic}>docker compose down -v</code> removes the named volumes and with
            them every account in the database. Use <code className={ic}>down</code> without{" "}
            <code className={ic}>-v</code> unless you intend a clean slate.
          </Callout>
        </DocSection>

        {/* ------------------------------------------------------------ */}
        <DocSection id="troubleshooting" title="Troubleshooting">
          <div className={h3}>Compose will not start</div>
          <p className={pm}>
            A required variable is missing; the error names it. An{" "}
            <code className={ic}>.env</code> carried over from an older checkout is the usual cause —
            re-copy <code className={ic}>.env.example</code> and merge your values in.
          </p>

          <div className={h3}>A valid key returns 401</div>
          <p className={pm}>
            Check the header name. API keys go in <code className={ic}>x-api-key</code>;{" "}
            <code className={ic}>Authorization: Bearer</code> is parsed as a session token and will
            reject a perfectly good key.
          </p>

          <div className={h3}>Completions return 402</div>
          <p className={pm}>
            The project&apos;s budget is zero, which is how projects are created. Set{" "}
            <code className={ic}>budget_monthly</code> before generating.
          </p>

          <div className={h3}>Completions return 400 project_scope_required</div>
          <p className={pm}>
            The key was created without <code className={ic}>project_id</code>. Unscoped keys
            authenticate but cannot generate. Issue a new one from the project.
          </p>

          <div className={h3}>A model will not route</div>
          <p className={pm}>
            Confirm it is listed by <code className={ic}>GET /v1/models</code>. If not, the provider
            credential is absent or the model is outside the seeded catalog. Compare against{" "}
            <code className={ic}>x-gaussmeridian-candidates</code> on a working request to see what
            the router considered.
          </p>

          <div className={h3}>Empty content with finish_reason: length</div>
          <p className={pm}>
            <code className={ic}>max_tokens</code> is too low. Reasoning models consume tokens before
            producing visible output, so the ceiling is reached before any text is emitted.
          </p>

          <div className={h3}>502 from a reverse proxy</div>
          <p className={pm}>
            Almost certainly response header size. Raise{" "}
            <code className={ic}>proxy_buffer_size</code> and{" "}
            <code className={ic}>proxy_buffers</code> to 16&nbsp;KB or more.
          </p>

          <div className={h3}>Everything 403s through the console&apos;s API routes</div>
          <p className={pm}>
            The console rejects state-changing requests whose <code className={ic}>Origin</code> does
            not match its host, and fails closed when the header is absent. Send one, or call the
            gateway on port 8000 directly.
          </p>
        </DocSection>
      </main>
    </div>
  );
}
