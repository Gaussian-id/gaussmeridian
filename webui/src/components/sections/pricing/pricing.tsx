import Link from "next/link";

import { cn } from "@core/lib/utils";

import { Reveal, ScrollReveal } from "@/components/motion";
import { buttonVariants } from "@/components/ui/button";

import type { ReactNode } from "react";

export function PricingHero() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col items-center justify-center px-6 py-24 text-center">
      <Reveal>
        <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
          Pricing
        </div>
      </Reveal>
      <Reveal delay={0.05}>
        <h1 className="font-display mx-auto mt-4 max-w-[18ch] text-5xl font-semibold tracking-tight sm:text-6xl">
          Pricing that only charges for <span className="text-gradient">answers that hold</span>.
        </h1>
      </Reveal>
      <Reveal delay={0.1}>
        <p className="text-muted-foreground mx-auto mt-5 max-w-[52ch] text-lg leading-relaxed">
          Bring your own provider keys at zero markup. Let Meridian route. And when an answer fails
          the outcome check, you pay nothing for it — automatically.
        </p>
      </Reveal>
    </section>
  );
}

interface Tier {
  name: string;
  price: string;
  who: string;
  feats: ReactNode[];
  cta: { label: string; href: string };
  featured?: boolean;
}

const tiers: Tier[] = [
  {
    name: "Free",
    price: "$0",
    who: "Try Meridian on platform-billed models.",
    feats: [
      <>
        Smart routing with <code>model:&quot;auto&quot;</code>
      </>,
      "Automatic fallback",
      "Decisions in your headers",
      "Community support",
    ],
    cta: { label: "Start free", href: "/signup" },
  },
  {
    name: "Enterprise",
    price: "Custom",
    who: "Governance, scale, and private deployment.",
    feats: [
      "SSO / RBAC · audit log",
      "Self-host or private VPC",
      "Budget caps & alerts",
      "SLAs & priority support",
    ],
    cta: { label: "Talk to us", href: "/signup" },
    featured: true,
  },
];

export function PricingTiers() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <ScrollReveal>
        <h2 className="font-display text-center text-3xl font-semibold tracking-tight md:text-4xl">
          Two ways to pay for exactly what works.
        </h2>
      </ScrollReveal>
      <div className="mx-auto mt-11 grid w-full max-w-md gap-4 md:max-w-4xl md:grid-cols-2">
        {tiers.map((tier, index) => (
          <ScrollReveal key={tier.name} delay={index * 0.06} className="h-full">
            <div
              className={cn(
                "relative flex h-full flex-col rounded-[20px] border p-7",
                tier.featured
                  ? "bg-brand-gradient border-transparent text-white md:-translate-y-1.5"
                  : "border-border bg-[color-mix(in_srgb,var(--card)_88%,transparent)]",
              )}
            >
              <div
                className={cn(
                  "font-mono text-[11px] tracking-[0.16em] uppercase",
                  tier.featured ? "text-white/70" : "text-muted-foreground",
                )}
              >
                {tier.name}
              </div>
              <div className="font-display mt-3 text-4xl font-semibold tracking-tight">
                {tier.price}
              </div>
              <div
                className={cn(
                  "mt-2 text-sm",
                  tier.featured ? "text-white/80" : "text-muted-foreground",
                )}
              >
                {tier.who}
              </div>
              <ul className="mt-5 mb-6 flex flex-1 flex-col gap-2.5">
                {tier.feats.map((feature) => (
                  <li key={String(feature)} className="flex gap-2.5 text-sm leading-snug">
                    <span
                      className={cn(
                        "font-bold",
                        tier.featured ? "text-[#9fe6c4]" : "text-[var(--gauss-500)]",
                      )}
                    >
                      ✓
                    </span>
                    <span>{feature}</span>
                  </li>
                ))}
              </ul>
              <Link
                href={tier.cta.href}
                className={
                  tier.featured
                    ? cn(
                        buttonVariants({ size: "lg" }),
                        "bg-white text-[var(--gauss-900)] hover:bg-white/90",
                      )
                    : cn(buttonVariants({ variant: "outline", size: "lg" }))
                }
              >
                {tier.cta.label}
              </Link>
            </div>
          </ScrollReveal>
        ))}
      </div>
    </section>
  );
}

export function OutcomeBilling() {
  const passRows: [string, string][] = [
    ["model", "openai/gpt-4o-mini"],
    ["r_binary", "1"],
    ["charged", "$0.0003"],
  ];
  const failRows: [string, string][] = [
    ["model", "some-model"],
    ["r_binary", "0"],
    ["charged", "$0.00 · retried"],
  ];
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20 text-center">
      <ScrollReveal>
        <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
          Outcome billing
        </div>
        <h2 className="font-display mx-auto mt-3 max-w-[20ch] text-3xl font-semibold tracking-tight md:text-4xl">
          You only pay for answers that <span className="text-gradient">hold</span>.
        </h2>
        <p className="text-muted-foreground mx-auto mt-3 max-w-[54ch] text-base leading-relaxed">
          Every response runs the OutcomeGate. Passes → you&rsquo;re charged. Fails →{" "}
          <code>r_binary 0</code>, you pay <b>$0.00</b>, and Meridian retries. No one else bills
          this honestly.
        </p>
      </ScrollReveal>
      <div className="mx-auto mt-9 grid w-full max-w-3xl gap-4 text-left md:grid-cols-2">
        <ScrollReveal>
          <div
            className="rounded-[18px] p-5 font-mono text-white"
            style={{ backgroundImage: "linear-gradient(160deg,#0b3c8c,#1456c7)" }}
          >
            <div className="mb-3.5 text-[11px] tracking-[0.18em] text-white/55 uppercase">
              answer passed the gate
            </div>
            {passRows.map(([key, value]) => (
              <div
                key={key}
                className="flex justify-between border-t border-white/10 py-1.5 text-[13px] first:border-t-0"
              >
                <span className="text-white/55">{key}</span>
                <span>{value}</span>
              </div>
            ))}
            <div className="font-display mt-3.5 text-3xl font-semibold text-[#7ee0b8]">$0.0003</div>
            <div className="mt-2 font-sans text-[12.5px] text-white/60">
              Good answer → you pay for it.
            </div>
          </div>
        </ScrollReveal>
        <ScrollReveal delay={0.06}>
          <div
            className="rounded-[18px] border border-white/10 p-5 font-mono text-white"
            style={{ backgroundImage: "linear-gradient(160deg,#12203f,#1b2742)" }}
          >
            <div className="mb-3.5 text-[11px] tracking-[0.18em] text-white/55 uppercase">
              answer failed the gate
            </div>
            {failRows.map(([key, value]) => (
              <div
                key={key}
                className="flex justify-between border-t border-white/10 py-1.5 text-[13px] first:border-t-0"
              >
                <span className="text-white/55">{key}</span>
                <span>{value}</span>
              </div>
            ))}
            <div className="font-display mt-3.5 text-3xl font-semibold text-white">$0.00</div>
            <div className="mt-2 font-sans text-[12.5px] text-white/60">
              Bad answer → you pay nothing, it reroutes.
            </div>
          </div>
        </ScrollReveal>
      </div>
    </section>
  );
}

const faqs: { q: string; a: ReactNode }[] = [
  {
    q: "How does Enterprise pricing work?",
    a: "Enterprise terms are tailored to your organization, deployment, governance, support, and usage requirements.",
  },
  {
    q: "How does outcome billing decide an answer “passed”?",
    a: (
      <>
        The OutcomeGate runs your project&rsquo;s validator (schema, calibrated confidence, a
        webhook, or a test). Pass → <code>r_binary 1</code>. Fail → <code>r_binary 0</code>, $0
        charged, and it retries the next candidate.
      </>
    ),
  },
  {
    q: "Can I self-host instead?",
    a: (
      <>
        Yes — the whole router runs in your own infrastructure with one{" "}
        <code>docker compose up</code>. Same API, same headers. See Solutions → Self-host.
      </>
    ),
  },
  {
    q: "Which models can I route to?",
    a: (
      <>
        Any provider you bring a key for. Meridian scores every servable model on cost × quality and
        routes — or you pin one with the <code>model</code> field.
      </>
    ),
  },
];

export function PricingFaq() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <ScrollReveal>
        <h2 className="font-display text-3xl font-semibold tracking-tight md:text-4xl">
          Questions.
        </h2>
      </ScrollReveal>
      <div className="mx-auto mt-8 w-full max-w-3xl">
        {faqs.map((faq, index) => (
          <ScrollReveal key={faq.q} delay={index * 0.04}>
            <div className="border-border border-t py-5">
              <div className="text-base font-semibold">{faq.q}</div>
              <div className="text-muted-foreground mt-2 text-[14.5px] leading-relaxed">
                {faq.a}
              </div>
            </div>
          </ScrollReveal>
        ))}
      </div>
    </section>
  );
}
