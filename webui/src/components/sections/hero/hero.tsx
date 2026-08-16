"use client";

import Link from "next/link";

import { cn } from "@core/lib/utils";

import { Reveal, useParallax, useTilt } from "@/components/motion";
import { buttonVariants } from "@/components/ui/button";

/** Illustrative — the stable customer contract with representative values. */
const trace: { k: string; v: string; stage: string; ok?: boolean }[] = [
  { k: "project", v: "acme/support-copilot", stage: "authenticated" },
  { k: "model", v: "google/gemini-2.5-flash", stage: "supported" },
  { k: "response", v: "chat.completion", stage: "normalized" },
  { k: "tokens", v: "812 in · 146 out", stage: "metered" },
  { k: "access", v: "purchased credit", stage: "verified", ok: true },
  { k: "budget", v: "$1.00 hard limit", stage: "project" },
];

/** The hero lands fully on load; motion only enhances. Nothing here is gated behind scroll. */
export function Hero() {
  const textRef = useParallax<HTMLDivElement>(-8);
  const traceRef = useParallax<HTMLDivElement>(14);
  const cardRef = useTilt<HTMLDivElement>(7);

  return (
    <section className="relative isolate flex min-h-dvh items-center overflow-hidden px-6 py-24">
      <div className="bg-radial-glow pointer-events-none absolute inset-0 -z-10" />
      <div className="mx-auto grid w-full max-w-6xl items-center gap-14 md:grid-cols-[1.02fr_0.98fr]">
        <div ref={textRef}>
          <Reveal>
            <div className="text-muted-foreground font-mono text-[11px] tracking-[0.28em] uppercase">
              Meridian · one API for supported models
            </div>
          </Reveal>
          <Reveal delay={0.05}>
            <h1 className="font-display mt-4 text-5xl leading-[1.02] font-semibold tracking-tight sm:text-6xl">
              Ship model calls with
              <br /> one <span className="text-gradient">stable contract.</span>
            </h1>
          </Reveal>
          <Reveal delay={0.1}>
            <p className="text-muted-foreground mt-5 max-w-[44ch] text-lg leading-relaxed">
              Create a project key, call a supported model, and get one consistent response with
              usage evidence tied to your organization and project.
            </p>
          </Reveal>
          <Reveal delay={0.15}>
            <div className="mt-7 flex flex-wrap gap-3">
              <Link href="/signup" className={cn(buttonVariants({ variant: "brand", size: "lg" }))}>
                Get API key →
              </Link>
              <Link
                href="#how-it-works"
                className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
              >
                See how a request runs ▸
              </Link>
            </div>
          </Reveal>
          <Reveal delay={0.2}>
            <p className="text-muted-foreground mt-6 flex items-center gap-2.5 text-sm">
              <span className="size-[7px] rounded-full bg-[var(--gauss-500)] shadow-[0_0_10px_1px_color-mix(in_srgb,var(--accent)_70%,transparent)]" />
              <span>
                <span className="text-foreground font-semibold">
                  Prepaid credit. Metered usage. No subscription.
                </span>{" "}
                Purchased credit and payment evidence stay visible in Billing.
              </span>
            </p>
          </Reveal>
        </div>

        <div ref={traceRef}>
          <Reveal delay={0.15}>
            <div
              ref={cardRef}
              className="bg-brand-gradient shadow-glow overflow-hidden rounded-[18px] p-5 font-mono text-white"
            >
              <div className="mb-3 flex items-center justify-between text-[10.5px] tracking-[0.22em] text-white/55 uppercase">
                <span>request lifecycle · POST /v1/chat/completions</span>
                <span className="flex items-center gap-1.5">
                  <span className="size-[7px] animate-pulse rounded-full bg-[var(--gauss-400)]" />
                  complete
                </span>
              </div>
              {trace.map((r) => (
                <div
                  key={r.k}
                  className="grid grid-cols-[92px_1fr_auto] items-baseline gap-3 border-t border-white/10 py-1.5 text-[13px] first:border-t-0"
                >
                  <span className="text-white/50">{r.k}</span>
                  <span className={r.ok ? "text-[#7ee0b8]" : "text-white"}>{r.v}</span>
                  <span className="text-[10.5px] tracking-[0.12em] text-[var(--gauss-400)] uppercase">
                    {r.stage}
                  </span>
                </div>
              ))}
              <p className="mt-3 font-sans text-[12px] leading-relaxed text-white/60">
                The public contract stays stable while Meridian operates the model connection,
                metering, and payment evidence behind it.
              </p>
            </div>
          </Reveal>
        </div>
      </div>
    </section>
  );
}
