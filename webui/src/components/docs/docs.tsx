"use client";

import { useEffect, useState } from "react";

import { cn } from "@core/lib/utils";

import type { ReactNode } from "react";

const nav: { group: string; items: [string, string][] }[] = [
  {
    group: "Start here",
    items: [
      ["overview", "What it is"],
      ["quickstart", "Run it locally"],
      ["first-request", "Your first request"],
    ],
  },
  {
    group: "Core concepts",
    items: [
      ["concepts", "Orgs, projects, keys"],
      ["auth", "Authentication"],
      ["byok", "Bring your own key"],
    ],
  },
  {
    group: "API reference",
    items: [
      ["chat", "Chat completions"],
      ["models", "Models"],
      ["headers", "Response headers"],
      ["errors", "Errors"],
    ],
  },
  {
    group: "Operating it",
    items: [
      ["configuration", "Configuration"],
      ["routing", "Routing features"],
      ["services", "Services and ports"],
      ["troubleshooting", "Troubleshooting"],
    ],
  },
];

const headerRows: [string, ReactNode][] = [
  ["x-gaussmeridian-model-selected", <>The model that actually served the request.</>],
  ["x-gaussmeridian-provider-selected", <>The provider it was served from.</>],
  ["x-gaussmeridian-complexity", <>Estimated prompt complexity, 0–1. Drives cascade and MoA.</>],
  ["x-gaussmeridian-candidates", <>The models considered, with their scores.</>],
  ["x-gaussmeridian-score", <>Score of the selected candidate.</>],
  ["x-gaussmeridian-tier", <>Capability tier the request was routed into.</>],
  ["x-gaussmeridian-cost", <>Cost attributed to this request.</>],
  ["x-gaussmeridian-budget-used", <>Project budget consumed so far.</>],
  ["x-gaussmeridian-budget-limit", <>The project's configured monthly budget.</>],
  ["x-gaussmeridian-cache-hit", <>Whether the response came from cache.</>],
  ["x-gaussmeridian-cache-tier", <>Which cache tier answered.</>],
  ["x-gaussmeridian-guardrail", <>Set when a guardrail acted on the response.</>],
  ["x-gaussmeridian-retry-count", <>Provider retries before a response was returned.</>],
  ["x-gaussmeridian-r-binary", <>Outcome flag used by the billing ledger.</>],
];

const envRows: [string, string, ReactNode][] = [
  ["JWT_SECRET", "required", <>Signs console session tokens. Any sufficiently long random string.</>],
  [
    "GAUSSMERIDIAN_API_KEY",
    "required",
    <>Bootstrap API key baked in at startup, for calling the gateway before you create one.</>,
  ],
  ["SURREALDB_PASSWORD", "required", <>Root password for the bundled SurrealDB.</>],
  [
    "REDIS_PASSWORD",
    "required",
    <>Redis auth. Compose builds the gateway&apos;s connection string from it.</>,
  ],
  ["GRAFANA_PASSWORD", "required", <>Admin password for Grafana, under the observability profile.</>],
  [
    "GEMINI_API_KEY",
    "optional",
    <>Google provider credential. Also OPENAI_API_KEY and ANTHROPIC_API_KEY.</>,
  ],
  [
    "BYOK_MASTER_KEY",
    "optional",
    <>
      Base64 of 32 random bytes; encrypts stored provider keys. Compose injects a development-only
      default so BYOK works on a fresh clone — generate your own before storing anything real.
    </>,
  ],
  [
    "BYOK_ADMIN_EMAILS",
    "optional",
    <>Comma-separated emails allowed to register or delete BYOK keys. Empty means nobody can.</>,
  ],
  [
    "SUPERADMIN_EMAILS",
    "optional",
    <>
      Comma-separated emails with access to <code>/v1/admin/*</code>. Absent callers get 404, not
      403, so the admin surface is indistinguishable from routes that do not exist.
    </>,
  ],
];

const errorRows: [string, string, ReactNode][] = [
  [
    "401",
    "unauthorized",
    <>
      No credential, or one the gateway does not recognise. Check you used{" "}
      <code>x-api-key</code> and not <code>Authorization</code>.
    </>,
  ],
  [
    "400",
    "project_scope_required",
    <>The API key is not attached to a project. Recreate it from a project&apos;s Keys page.</>,
  ],
  [
    "402",
    "budget_exceeded",
    <>The project has no budget, or has spent it. Set a monthly budget in project settings.</>,
  ],
  ["403", "project_access_denied", <>The caller is not a member of that project&apos;s org.</>],
  ["404", "—", <>On <code>/v1/admin/*</code> this can mean &ldquo;not a superadmin&rdquo;.</>],
  ["429", "rate_limited", <>The key&apos;s per-minute or per-day limit was hit.</>],
];

function DocSection({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <section id={id} data-doc className="border-border scroll-mt-24 border-t py-11">
      <h2 className="font-display text-2xl font-semibold tracking-tight">{title}</h2>
      <div className="mt-4">{children}</div>
    </section>
  );
}

function DocCode({ children }: { children: ReactNode }) {
  return (
    <pre className="border-border bg-muted/40 my-3.5 overflow-x-auto rounded-[12px] border p-4 font-mono text-[13px] leading-[1.75]">
      <code>{children}</code>
    </pre>
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

const p = "text-foreground mt-2.5 text-[15.5px] leading-relaxed";
const pm = "text-muted-foreground mt-2.5 text-[15px] leading-relaxed";
const h3 = "text-foreground mt-7 mb-1 text-[15.5px] font-semibold";
const ic =
  "text-primary rounded-md bg-[color-mix(in_srgb,var(--accent)_10%,transparent)] px-1.5 py-0.5 font-mono text-[0.9em]";
const td = "border-border border-b px-3 py-2 align-top";
const tdMono = cn(td, "font-mono text-[12.5px] whitespace-nowrap");

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
    <div className="mx-auto grid max-w-[84rem] grid-cols-1 gap-11 px-6 pt-24 pb-24 lg:grid-cols-[230px_minmax(0,1fr)]">
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
          <p className={cn(pm, "max-w-[52ch]")}>
            Everything needed to stand GaussMeridian up on your own machine, issue a key, and make a
            real completion — plus the configuration and failure modes you will meet on the way.
          </p>
        </div>

        <DocSection id="overview" title="What it is">
          <p className={p}>
            GaussMeridian is a self-hosted gateway that speaks the OpenAI API. You point an existing
            SDK at it instead of a provider, and it routes each request to a model behind the
            scenes, applies your guardrails and budgets, and records what happened.
          </p>
          <p className={pm}>Concretely, it gives you:</p>
          <ul className={cn(pm, "mt-2 list-disc space-y-1 pl-5")}>
            <li>
              One OpenAI-compatible endpoint in front of OpenAI, Anthropic, and Google models.
            </li>
            <li>Per-project API keys, budgets, and rate limits.</li>
            <li>
              Customer-supplied provider credentials (BYOK), encrypted before they are stored.
            </li>
            <li>A web console for people, and the same REST API for machines.</li>
            <li>Response headers that tell you exactly what routed, what it cost, and why.</li>
          </ul>
          <Callout>
            Everything runs on your infrastructure. There is no hosted GaussMeridian to sign up for —
            the quickstart below is the product.
          </Callout>
        </DocSection>

        <DocSection id="quickstart" title="Run it locally">
          <p className={pm}>
            You need Docker with Compose. Nothing else — the database, cache, and a mock model
            provider all come up with the stack.
          </p>

          <div className={h3}>1 · Clone and configure</div>
          <DocCode>{`git clone https://github.com/Gaussian-id/gaussmeridian.git
cd gaussmeridian
cp .env.example .env`}</DocCode>
          <p className={pm}>
            <code className={ic}>.env.example</code> ships with every required variable already
            present. Fill in your own secrets before exposing the stack to anyone else.
          </p>

          <div className={h3}>2 · Bring the stack up</div>
          <DocCode>{`docker compose --profile webui up -d`}</DocCode>
          <p className={pm}>
            First run builds the gateway from source, which takes a while. Afterwards it is seconds.
            Drop <code className={ic}>--profile webui</code> if you only want the API.
          </p>

          <div className={h3}>3 · Check it is alive</div>
          <DocCode>{`curl http://localhost:8000/health
# {"status":"healthy","version":"3.0.0"}`}</DocCode>
          <p className={pm}>
            The console is on <code className={ic}>http://localhost:3001</code>. Register an account
            there — the first thing you create is an organisation, then a project inside it.
          </p>

          <Callout>
            With no provider keys set, the stack routes to a bundled mock provider, so you can
            exercise the whole flow without spending anything. Add{" "}
            <code className={ic}>GEMINI_API_KEY</code> (or OpenAI/Anthropic) to{" "}
            <code className={ic}>.env</code> and restart to use real models.
          </Callout>
        </DocSection>

        <DocSection id="first-request" title="Your first request">
          <p className={pm}>
            Four things have to be true before a completion will succeed. Most first-time failures
            are one of these missing.
          </p>
          <ol className={cn(pm, "mt-3 list-decimal space-y-1.5 pl-5")}>
            <li>You have an organisation and a project.</li>
            <li>
              The project has a monthly budget above zero — otherwise generation returns{" "}
              <code className={ic}>402</code>.
            </li>
            <li>
              Your API key is scoped to that project — otherwise you get{" "}
              <code className={ic}>400 project_scope_required</code>.
            </li>
            <li>
              You send the key as <code className={ic}>x-api-key</code>.
            </li>
          </ol>
          <p className={pm}>
            Create the key from the project&apos;s Keys page in the console. The secret is shown
            once, at creation, and is never retrievable again.
          </p>
          <DocCode>{`curl http://localhost:8000/v1/chat/completions \\
  -H "content-type: application/json" \\
  -H "x-api-key: $GAUSSMERIDIAN_KEY" \\
  -d '{
    "model": "gemini-2.5-flash",
    "messages": [{"role": "user", "content": "Say hello"}]
  }'`}</DocCode>
          <p className={pm}>
            The response is OpenAI-shaped, so an existing SDK works unchanged once you set the base
            URL and pass the key in the right header.
          </p>
        </DocSection>

        <DocSection id="concepts" title="Orgs, projects, keys">
          <p className={p}>
            The hierarchy is three levels deep, and each level exists for a reason.
          </p>
          <div className={h3}>Organisation</div>
          <p className={pm}>
            The billing and membership boundary. People are invited to an org and given a role
            there.
          </p>
          <div className={h3}>Project</div>
          <p className={pm}>
            Where budgets, model settings, and guardrail thresholds live. A project is the unit you
            would map to one application or environment. Spend is tracked per project.
          </p>
          <div className={h3}>API key</div>
          <p className={pm}>
            Belongs to a project and inherits its budget. Keys carry their own rate limits and can
            be revoked individually without touching the others.
          </p>
          <Callout tone="warn">
            A key created outside a project — from the onboarding wizard, or a direct API call with
            no <code className={ic}>project_id</code> — is stored unscoped. It authenticates, but it
            cannot generate, because generation requires a project to bill against.
          </Callout>
        </DocSection>

        <DocSection id="auth" title="Authentication">
          <p className={p}>
            There are two credential types and they are not interchangeable. Sending the wrong one
            is the single most common setup mistake.
          </p>

          <div className={h3}>API keys — for your code</div>
          <p className={pm}>
            Sent as <code className={ic}>x-api-key</code>. These are what your application uses.
          </p>
          <DocCode>{`curl http://localhost:8000/v1/models \\
  -H "x-api-key: $GAUSSMERIDIAN_KEY"`}</DocCode>

          <div className={h3}>Session tokens — for the console</div>
          <p className={pm}>
            Sent as <code className={ic}>Authorization: Bearer</code>, issued by{" "}
            <code className={ic}>POST /v1/auth/login</code>. The console uses these; in the browser
            they are carried in an http-only cookie rather than a header.
          </p>

          <Callout tone="warn">
            An <code className={ic}>Authorization: Bearer</code> header is validated as a session
            token, never as an API key. Put an API key there and you get{" "}
            <code className={ic}>401 invalid credentials</code> — a correct answer that looks exactly
            like a broken key. If a key you just created is rejected, check the header name first.
          </Callout>
        </DocSection>

        <DocSection id="byok" title="Bring your own key">
          <p className={p}>
            BYOK lets a project call a provider with its own credential instead of the one the
            gateway is configured with. Keys are encrypted with{" "}
            <code className={ic}>BYOK_MASTER_KEY</code> before storage and are never returned by any
            endpoint after registration.
          </p>
          <DocCode>{`curl -X POST http://localhost:8000/v1/byok/keys \\
  -H "content-type: application/json" \\
  -H "authorization: Bearer $SESSION_TOKEN" \\
  -H "x-project-id: $PROJECT_ID" \\
  -d '{"provider": "google", "api_key": "..."}'`}</DocCode>
          <p className={pm}>
            Registration and deletion are admin-gated: the caller&apos;s email must appear in{" "}
            <code className={ic}>BYOK_ADMIN_EMAILS</code>. If that variable is unset, every BYOK
            write returns <code className={ic}>403</code>, by design.
          </p>
          <Callout tone="warn">
            <strong>Known issue.</strong> Registration succeeds and stores the key, but the list and
            delete endpoints do not currently return it — so a registered key cannot be inspected or
            revoked through the API yet. Treat BYOK as write-only until that is fixed.
          </Callout>
        </DocSection>

        <DocSection id="chat" title="Chat completions">
          <p className={pm}>
            <code className={ic}>POST /v1/chat/completions</code> — OpenAI-compatible. Streaming is
            available at <code className={ic}>/v1/chat/completions/stream</code>, and{" "}
            <code className={ic}>/v1/chat/completions/batch</code> accepts an array.
          </p>
          <Table head={["Field", "Notes"]}>
            <tr>
              <td className={tdMono}>model</td>
              <td className={td}>A model configured for the project, e.g. gemini-2.5-flash.</td>
            </tr>
            <tr>
              <td className={tdMono}>messages</td>
              <td className={td}>Standard role/content array.</td>
            </tr>
            <tr>
              <td className={tdMono}>max_tokens</td>
              <td className={td}>Optional ceiling on the response.</td>
            </tr>
            <tr>
              <td className={tdMono}>temperature</td>
              <td className={td}>Passed through to the provider.</td>
            </tr>
          </Table>
          <p className={pm}>
            Also available: <code className={ic}>/v1/completions</code>,{" "}
            <code className={ic}>/v1/embeddings</code>, and <code className={ic}>/v1/usage/:id</code>{" "}
            for per-request accounting.
          </p>
        </DocSection>

        <DocSection id="models" title="Models">
          <p className={pm}>
            <code className={ic}>GET /v1/models</code> lists what the gateway will route to, and{" "}
            <code className={ic}>GET /v1/models/:model</code> returns one model&apos;s capabilities.
          </p>
          <DocCode>{`curl http://localhost:8000/v1/models \\
  -H "x-api-key: $GAUSSMERIDIAN_KEY"`}</DocCode>
          <p className={pm}>
            The catalog is seeded at startup and covers the providers you have credentials for. A
            model missing from this list will not route, even if the provider supports it.
          </p>
        </DocSection>

        <DocSection id="headers" title="Response headers">
          <p className={pm}>
            Every completion carries headers describing what happened. They are the fastest way to
            understand a routing decision without opening the console.
          </p>
          <Table head={["Header", "Meaning"]}>
            {headerRows.map(([name, desc]) => (
              <tr key={name}>
                <td className={tdMono}>{name}</td>
                <td className={td}>{desc}</td>
              </tr>
            ))}
          </Table>
        </DocSection>

        <DocSection id="errors" title="Errors">
          <p className={pm}>
            Errors are JSON with a <code className={ic}>type</code> and a{" "}
            <code className={ic}>code</code>. The codes below are the ones you are most likely to
            meet while setting up.
          </p>
          <Table head={["Status", "Code", "What it means"]}>
            {errorRows.map(([status, code, desc]) => (
              <tr key={status + code}>
                <td className={tdMono}>{status}</td>
                <td className={tdMono}>{code}</td>
                <td className={td}>{desc}</td>
              </tr>
            ))}
          </Table>
        </DocSection>

        <DocSection id="configuration" title="Configuration">
          <p className={pm}>
            Configuration is environment variables, read at startup. Compose refuses to start if a
            required one is missing rather than falling back to a default.
          </p>
          <Table head={["Variable", "", "Purpose"]}>
            {envRows.map(([name, req, desc]) => (
              <tr key={name}>
                <td className={tdMono}>{name}</td>
                <td className={cn(td, "text-muted-foreground font-mono text-[11px]")}>{req}</td>
                <td className={td}>{desc}</td>
              </tr>
            ))}
          </Table>
          <Callout tone="warn">
            Redis is configured through <code className={ic}>REDIS_URL</code>, deliberately without
            a prefix — the server reads that name and no other. Setting{" "}
            <code className={ic}>GAUSSMERIDIAN_REDIS_URL</code> silently leaves the gateway on its
            built-in localhost default.
          </Callout>
        </DocSection>

        <DocSection id="routing" title="Routing features">
          <p className={pm}>
            These are off unless you turn them on. Each is a single environment variable.
          </p>
          <div className={h3}>Guardrails</div>
          <p className={pm}>
            <code className={ic}>GAUSSMERIDIAN_GUARDRAIL_PII</code> and{" "}
            <code className={ic}>GAUSSMERIDIAN_GUARDRAIL_INJECTION</code> scan responses and block
            ones that trip them. A blocked response is reported in{" "}
            <code className={ic}>x-gaussmeridian-guardrail</code>.
          </p>
          <div className={h3}>Cascade</div>
          <p className={pm}>
            <code className={ic}>GAUSSMERIDIAN_CASCADE</code> tries a cheaper model first and
            escalates when confidence falls below{" "}
            <code className={ic}>GAUSSMERIDIAN_CASCADE_THRESHOLD</code>.
          </p>
          <div className={h3}>Mixture of agents</div>
          <p className={pm}>
            <code className={ic}>GAUSSMERIDIAN_MOA</code> runs several models on complex prompts and
            reconciles their answers. <code className={ic}>GAUSSMERIDIAN_MOA_AGENTS</code> is the
            comma-separated roster and <code className={ic}>GAUSSMERIDIAN_TAU_MOA</code> is the
            complexity threshold — 0.7 means high-complexity prompts only, 0.0 every request, 1.0
            never.
          </p>
        </DocSection>

        <DocSection id="services" title="Services and ports">
          <p className={pm}>
            <code className={ic}>docker compose up</code> starts these. Anything bound to{" "}
            <code className={ic}>127.0.0.1</code> is deliberately not reachable from the network.
          </p>
          <Table head={["Service", "Port", "Purpose"]}>
            <tr>
              <td className={td}>gaussmeridian</td>
              <td className={tdMono}>8000</td>
              <td className={td}>The API.</td>
            </tr>
            <tr>
              <td className={td}>webui</td>
              <td className={tdMono}>3001</td>
              <td className={td}>The console. Needs the webui profile.</td>
            </tr>
            <tr>
              <td className={td}>surrealdb</td>
              <td className={tdMono}>127.0.0.1:8001</td>
              <td className={td}>Database. Loopback only.</td>
            </tr>
            <tr>
              <td className={td}>redis</td>
              <td className={tdMono}>internal</td>
              <td className={td}>Cache and rate-limit state. Password protected.</td>
            </tr>
            <tr>
              <td className={td}>mock-provider</td>
              <td className={tdMono}>internal</td>
              <td className={td}>Deterministic stand-in so the stack runs with no keys.</td>
            </tr>
            <tr>
              <td className={td}>prometheus / grafana</td>
              <td className={tdMono}>127.0.0.1:9091 / 3000</td>
              <td className={td}>Metrics. Needs the observability profile.</td>
            </tr>
          </Table>
        </DocSection>

        <DocSection id="troubleshooting" title="Troubleshooting">
          <div className={h3}>Compose will not start</div>
          <p className={pm}>
            A required variable is missing from <code className={ic}>.env</code>. The error names it.
            Copying <code className={ic}>.env.example</code> again is usually the fastest fix — an
            older <code className={ic}>.env</code> may predate a newly required variable.
          </p>

          <div className={h3}>A key that should work returns 401</div>
          <p className={pm}>
            Check the header. API keys go in <code className={ic}>x-api-key</code>;{" "}
            <code className={ic}>Authorization: Bearer</code> is parsed as a session token and will
            reject a valid key.
          </p>

          <div className={h3}>Completions return 402</div>
          <p className={pm}>
            The project&apos;s monthly budget is zero. Projects are created that way; set a budget in
            project settings before generating.
          </p>

          <div className={h3}>A model will not route</div>
          <p className={pm}>
            Confirm it appears in <code className={ic}>GET /v1/models</code>. If it does not, the
            provider credential is missing or the model is outside the seeded catalog.
          </p>

          <div className={h3}>The console logs an error on first load</div>
          <p className={pm}>
            A signed-out session check is expected and is logged at info level. If you see it as an
            error, the console build predates that fix.
          </p>
        </DocSection>
      </main>
    </div>
  );
}
