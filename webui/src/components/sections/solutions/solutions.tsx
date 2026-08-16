import Link from "next/link";

import { cn } from "@core/lib/utils";

import { Reveal, ScrollReveal } from "@/components/motion";
import { buttonVariants } from "@/components/ui/button";

import type { ReactNode } from "react";

/** Page intro — the first screen, so it lands on load (mount reveal, not scroll-gated). */
export function SolutionsIntro() {
  const chips: [string, string][] = [
    ["Developers", "#developers"],
    ["Product teams", "#teams"],
    ["Enterprise", "#enterprise"],
  ];
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-5 py-24 sm:px-6">
      <Reveal>
        <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
          Solutions
        </div>
      </Reveal>
      <Reveal delay={0.05}>
        <h1 className="font-display mt-4 max-w-[15ch] text-5xl font-semibold tracking-tight sm:text-6xl">
          One line. <span className="text-gradient">Three ways</span> onto it.
        </h1>
      </Reveal>
      <Reveal delay={0.1}>
        <p className="text-muted-foreground mt-5 max-w-[50ch] text-lg leading-relaxed">
          One customer contract, met where you stand — whether you&rsquo;re testing a first project,
          funding a team, or standardizing model access across an organization.
        </p>
      </Reveal>
      <Reveal delay={0.15}>
        <div className="mt-7 flex flex-wrap gap-2.5 font-mono text-xs">
          {chips.map(([label, href]) => (
            <a
              key={href}
              href={href}
              className="border-border hover:border-ring/50 rounded-full border bg-[color-mix(in_srgb,var(--card)_60%,transparent)] px-4 py-2"
            >
              ↳ {label}
            </a>
          ))}
        </div>
      </Reveal>
    </section>
  );
}

interface SceneProps {
  id: string;
  kicker: string;
  title: string;
  desc: string;
  bullets?: ReactNode[];
  ctas: { label: string; href: string; brand?: boolean }[];
  proof: ReactNode;
  reversed?: boolean;
}

/** One full-screen audience scene: text + a proof panel, alternating sides. */
export function SolutionScene({
  id,
  kicker,
  title,
  desc,
  bullets,
  ctas,
  proof,
  reversed,
}: SceneProps) {
  return (
    <section id={id} className="mx-auto flex min-h-dvh max-w-6xl items-center px-5 py-20 sm:px-6">
      <div className="grid w-full items-center gap-14 md:grid-cols-2">
        <div className={cn(reversed && "md:order-2")}>
          <ScrollReveal>
            <div className="font-mono text-[11px] tracking-[0.2em] text-[var(--gauss-500)] uppercase">
              {kicker}
            </div>
            <h2 className="font-display mt-3 text-4xl font-semibold tracking-tight sm:text-5xl">
              {title}
            </h2>
            <p className="text-muted-foreground mt-4 max-w-[46ch] text-lg leading-relaxed">
              {desc}
            </p>
            {bullets && bullets.length > 0 && (
              <ul className="mt-5 flex flex-col gap-2.5">
                {bullets.map((b, i) => (
                  <li key={i} className="flex items-center gap-2.5 text-[14.5px]">
                    <span className="size-[6px] flex-none rounded-full bg-[var(--gauss-500)] shadow-[0_0_8px_color-mix(in_srgb,var(--accent)_70%,transparent)]" />
                    <span>{b}</span>
                  </li>
                ))}
              </ul>
            )}
            <div className="mt-7 flex flex-wrap gap-3">
              {ctas.map((c) => (
                <Link
                  key={c.href + c.label}
                  href={c.href}
                  className={cn(
                    buttonVariants({ variant: c.brand ? "brand" : "outline", size: "lg" }),
                  )}
                >
                  {c.label}
                </Link>
              ))}
            </div>
          </ScrollReveal>
        </div>
        <div className={cn(reversed && "md:order-1")}>
          <ScrollReveal>{proof}</ScrollReveal>
        </div>
      </div>
    </section>
  );
}

/** A brand-gradient code/terminal panel (the dev + self-host proofs). */
export function ProofCode({ lines }: { lines: ReactNode[] }) {
  return (
    <div className="bg-brand-gradient shadow-glow overflow-x-auto rounded-[18px] p-5 font-mono text-[13px] leading-[1.85] text-white">
      <div className="mb-3.5 flex gap-1.5">
        {[0, 1, 2].map((i) => (
          <span key={i} className="size-2.5 rounded-full bg-white/25" />
        ))}
      </div>
      {lines.map((l, i) => (
        <div key={i}>{l}</div>
      ))}
    </div>
  );
}

/** The enterprise trust grid proof. */
export function TrustGrid({ cells }: { cells: { t: string; s: string }[] }) {
  return (
    <div className="grid grid-cols-2 gap-3">
      {cells.map((c) => (
        <div
          key={c.t}
          className="border-border rounded-xl border bg-[color-mix(in_srgb,var(--card)_80%,transparent)] px-4 py-3.5"
        >
          <div className="text-sm font-semibold">{c.t}</div>
          <div className="text-muted-foreground mt-0.5 text-[12.5px]">{c.s}</div>
        </div>
      ))}
    </div>
  );
}

export function SolutionsCta() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl items-center px-5 py-20 sm:px-6">
      <ScrollReveal className="w-full">
        <div className="bg-brand-gradient shadow-glow rounded-[22px] px-6 py-14 text-center text-white">
          <h2 className="font-display text-3xl font-semibold tracking-tight md:text-4xl">
            Whichever way you come onto it —
          </h2>
          <p className="mx-auto mt-3 max-w-[44ch] text-white/75">
            the Meridian contract is the same. Project keys, supported models, and prepaid usage.
          </p>
          <div className="mt-7 flex flex-wrap justify-center gap-3">
            <Link
              href="/signup"
              className={cn(
                buttonVariants({ size: "lg" }),
                "bg-white text-[var(--gauss-900)] hover:bg-white/90",
              )}
            >
              Get API key →
            </Link>
            <Link
              href="/pricing"
              className={cn(
                buttonVariants({ variant: "outline", size: "lg" }),
                "border-white/30 bg-white/10 text-white hover:bg-white/20 hover:text-white",
              )}
            >
              See prepaid credit
            </Link>
          </div>
        </div>
      </ScrollReveal>
    </section>
  );
}
