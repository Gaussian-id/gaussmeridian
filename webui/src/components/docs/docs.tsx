"use client";

import { useEffect, useState } from "react";

import { cn } from "@core/lib/utils";

import type { ReactNode } from "react";

const nav: { group: string; items: [string, string][] }[] = [
  {
    group: "Getting started",
    items: [
      ["quickstart", "Quickstart"],
      ["auth", "Authentication"],
      ["routing", "Routing"],
    ],
  },
  {
    group: "Reference",
    items: [
      ["headers", "Response headers"],
      ["chat", "Chat completions"],
      ["models", "Models"],
    ],
  },
  {
    group: "Deploy",
    items: [
      ["selfhost", "Self-host"],
      ["sdks", "SDKs"],
    ],
  },
];
const flat = nav.flatMap((g) => g.items);

const headerRows: [string, ReactNode][] = [
  ["x-gaussmeridian-model", <>The model that served, e.g. openai/gpt-4o-mini</>],
  ["x-gaussmeridian-provider", <>The provider used</>],
  ["x-gaussmeridian-complexity", <>CARROT complexity score, 0–1</>],
  ["x-gaussmeridian-candidates", <>Ranked candidates with scores</>],
  [
    "x-gaussmeridian-r-binary",
    <>
      <code>1</code> = charged (passed) · <code>0</code> = $0 (failed, retried)
    </>,
  ],
  ["x-gaussmeridian-cost", <>Amount charged for this request</>],
  ["x-gaussmeridian-moa", <>true if mixture-of-agents fired</>],
  ["x-gaussmeridian-guardrail", <>blocked if a guardrail tripped</>],
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
    <div className="bg-brand-gradient shadow-glow my-3.5 overflow-x-auto rounded-[14px] p-4 font-mono text-[13px] leading-[1.8] text-white">
      {children}
    </div>
  );
}

const p = "text-foreground mt-2.5 text-[15.5px] leading-relaxed";
const pm = "text-muted-foreground mt-2.5 text-[15px] leading-relaxed";
const h3 = "text-muted-foreground mt-6 mb-2 font-mono text-[15px]";
const ic =
  "text-primary rounded-md bg-[color-mix(in_srgb,var(--accent)_10%,transparent)] px-1.5 py-0.5 font-mono text-[0.9em]";

/** Calm, scannable docs layout: sticky sidebar + content + on-this-page, scrollspy-highlighted. */
export function Docs() {
  const [active, setActive] = useState("quickstart");

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
    <div className="mx-auto grid max-w-[84rem] grid-cols-1 gap-11 px-6 pt-24 pb-24 lg:grid-cols-[220px_minmax(0,1fr)_200px]">
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
            Docs
          </div>
          <h1 className="font-display mt-3 text-4xl font-semibold tracking-tight sm:text-[44px]">
            Get on the line in <span className="text-gradient">one call</span>.
          </h1>
          <p className={cn(pm, "max-w-[46ch]")}>
            Meridian speaks the OpenAI API. Point your existing SDK at it, and every request is
            routed, verified, and billed only when the answer holds.
          </p>
        </div>

        <DocSection id="quickstart" title="Quickstart">
          <p className={pm}>Three steps — from zero to a routed completion.</p>
          <div className={h3}>1 · Point the base URL at Meridian</div>
          <DocCode>
            <div>
              <span className="text-[#bcd4ff]">export</span> OPENAI_BASE_URL=
              <span className="text-[#7ee0b8]">&quot;https://api.meridian.dev/v1&quot;</span>
            </div>
            <div>
              <span className="text-[#bcd4ff]">export</span> MERIDIAN_KEY=
              <span className="text-[#7ee0b8]">&quot;mrd-…&quot;</span>
            </div>
          </DocCode>
          <div className={h3}>2 · Call it like OpenAI — let Meridian route</div>
          <DocCode>
            <div>
              <span className="text-white/50">
                curl https://api.meridian.dev/v1/chat/completions \
              </span>
            </div>
            <div>
              &nbsp;&nbsp;-H{" "}
              <span className="text-[#7ee0b8]">&quot;x-api-key: $MERIDIAN_KEY&quot;</span> \
            </div>
            <div>
              &nbsp;&nbsp;-d{" "}
              <span className="text-[#7ee0b8]">
                &apos;&#123;&quot;model&quot;:&quot;auto&quot;,&quot;messages&quot;:[…]&#125;&apos;
              </span>
            </div>
          </DocCode>
          <div className={h3}>3 · Read the decision in the response headers</div>
          <p className={pm}>
            Every route comes back on the response — see{" "}
            <a href="#headers" className="text-primary underline-offset-2 hover:underline">
              Response headers
            </a>
            .
          </p>
        </DocSection>

        <DocSection id="auth" title="Authentication">
          <p className={p}>Two credential types, for two jobs:</p>
          <table className="mt-3 w-full border-collapse text-[13.5px]">
            <thead>
              <tr>
                {["Credential", "Use for", "Header"].map((h) => (
                  <th
                    key={h}
                    className="text-muted-foreground border-border border-b px-2.5 py-2.5 text-left font-mono text-[11px] tracking-[0.06em] uppercase"
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="border-border border-b px-2.5 py-2.5 align-top">
                  <b>Bearer JWT</b>
                </td>
                <td className="border-border border-b px-2.5 py-2.5 align-top">
                  Account, keys, settings, BYOK
                </td>
                <td className="border-border border-b px-2.5 py-2.5 align-top">
                  <code className="text-primary">Authorization: Bearer …</code>
                </td>
              </tr>
              <tr>
                <td className="border-border border-b px-2.5 py-2.5 align-top">
                  <b>x-api-key</b>
                </td>
                <td className="border-border border-b px-2.5 py-2.5 align-top">
                  Inference (<code>/chat/completions</code>)
                </td>
                <td className="border-border border-b px-2.5 py-2.5 align-top">
                  <code className="text-primary">x-api-key: mrd-…</code>
                </td>
              </tr>
            </tbody>
          </table>
        </DocSection>

        <DocSection id="routing" title="Routing">
          <p className={p}>
            Send <code className={ic}>model:&quot;auto&quot;</code> and Meridian scores every
            servable model on cost × quality and routes for you — keeping a ranked fallback list. Or
            pin a model by name to route yourself.
          </p>
          <p className={pm}>
            Hard prompts can escalate (cascade) or fan out across models (MoA) automatically; a
            provider outage reroutes mid-request. All of it is reported back in your headers.
          </p>
        </DocSection>

        <DocSection id="headers" title="Response headers">
          <p className={p}>
            The whole routing decision rides back on every response — no dashboard required.
          </p>
          <table className="mt-3 w-full border-collapse text-[13.5px]">
            <thead>
              <tr>
                {["Header", "Meaning"].map((h) => (
                  <th
                    key={h}
                    className="text-muted-foreground border-border border-b px-2.5 py-2.5 text-left font-mono text-[11px] tracking-[0.06em] uppercase"
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {headerRows.map(([k, v]) => (
                <tr key={k}>
                  <td className="border-border border-b px-2.5 py-2.5 align-top">
                    <code className="text-primary">{k}</code>
                  </td>
                  <td className="border-border border-b px-2.5 py-2.5 align-top">{v}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </DocSection>

        <DocSection id="chat" title="Chat completions">
          <div className="border-border inline-flex items-center gap-2 rounded-[10px] border bg-[color-mix(in_srgb,var(--card)_80%,transparent)] px-3 py-2 font-mono text-[13px]">
            <span className="rounded-md bg-[var(--gauss-700)] px-2 py-0.5 text-[11px] font-semibold text-white">
              POST
            </span>
            /v1/chat/completions
          </div>
          <p className={pm}>
            OpenAI-shaped request and response. <code className={ic}>stream:true</code> and{" "}
            <code className={ic}>/batch</code> are supported.
          </p>
          <DocCode>
            <div>&#123;</div>
            <div>
              &nbsp;&nbsp;<span className="text-[#bcd4ff]">&quot;model&quot;</span>:{" "}
              <span className="text-[#7ee0b8]">&quot;auto&quot;</span>,
            </div>
            <div>
              &nbsp;&nbsp;<span className="text-[#bcd4ff]">&quot;messages&quot;</span>: [&#123;{" "}
              <span className="text-[#bcd4ff]">&quot;role&quot;</span>:
              <span className="text-[#7ee0b8]">&quot;user&quot;</span>,{" "}
              <span className="text-[#bcd4ff]">&quot;content&quot;</span>:
              <span className="text-[#7ee0b8]">&quot;…&quot;</span> &#125;],
            </div>
            <div>
              &nbsp;&nbsp;<span className="text-[#bcd4ff]">&quot;max_tokens&quot;</span>: 256
            </div>
            <div>&#125;</div>
          </DocCode>
        </DocSection>

        <DocSection id="models" title="Models">
          <p className={p}>
            <code className={ic}>GET /v1/models</code> returns the catalog — provider, price per
            million tokens, and tier. Request an unservable model and Meridian skips it and routes
            to the best servable one.
          </p>
        </DocSection>

        <DocSection id="selfhost" title="Self-host">
          <p className={p}>
            Run the whole router in your own infrastructure — same API, same headers.
          </p>
          <DocCode>
            <div>
              <span className="text-white/50">$</span> git clone github.com/gaussian/meridian
            </div>
            <div>
              <span className="text-white/50">$</span> docker compose up -d
            </div>
            <div>
              <span className="text-[#7ee0b8]">✓</span> router :8000{" "}
              <span className="text-white/50">· surrealdb · redis</span>
            </div>
          </DocCode>
        </DocSection>

        <DocSection id="sdks" title="SDKs">
          <p className={p}>
            Anything OpenAI-compatible works unchanged — Python, Node, Go, or raw HTTP. Just swap
            the <code className={ic}>base_url</code>.
          </p>
        </DocSection>
      </main>

      <nav className="top-24 hidden self-start text-[13px] lg:sticky lg:block">
        <div className="text-muted-foreground mb-2 font-mono text-[10.5px] tracking-[0.16em] uppercase">
          On this page
        </div>
        {flat.map(([id, label]) => (
          <a
            key={id}
            href={`#${id}`}
            className={cn(
              "block py-1",
              active === id ? "text-primary" : "text-muted-foreground hover:text-foreground",
            )}
          >
            {label}
          </a>
        ))}
      </nav>
    </div>
  );
}
