import Link from "next/link";

import { ScrollReveal } from "@/components/motion";

const doors = [
  {
    k: "Developers",
    t: "Build on it",
    d: "One OpenAI-compatible API, project-scoped keys, and a supported model catalog.",
    href: "/solutions",
  },
  {
    k: "Teams",
    t: "Share one wallet",
    d: "Fund an organization once and attribute model activity to each project.",
    href: "/solutions",
  },
  {
    k: "Enterprise",
    t: "Deploy at scale",
    d: "Governance, reliability, and cost control across the org.",
    href: "/solutions",
  },
];

/** Home teaser for the three audiences — each door leads to /solutions. */
export function Doors() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <ScrollReveal>
        <h2 className="font-display text-3xl font-semibold tracking-tight md:text-4xl">
          One line. Three ways onto it.
        </h2>
        <p className="text-muted-foreground mt-3 max-w-[52ch] text-base">
          The same Meridian contract, from first key to production traffic.
        </p>
      </ScrollReveal>
      <div className="mt-10 grid gap-4 md:grid-cols-3">
        {doors.map((d, i) => (
          <ScrollReveal key={d.k} delay={i * 0.06} className="h-full">
            <Link
              href={d.href}
              className="border-border hover:border-ring/50 block h-full rounded-2xl border bg-[color-mix(in_srgb,var(--card)_82%,transparent)] p-6 transition-colors"
            >
              <div className="text-muted-foreground font-mono text-[10.5px] tracking-[0.16em] uppercase">
                {d.k}
              </div>
              <div className="font-display mt-3 text-xl font-semibold">{d.t}</div>
              <p className="text-muted-foreground mt-2 text-sm leading-relaxed">{d.d}</p>
            </Link>
          </ScrollReveal>
        ))}
      </div>
    </section>
  );
}
