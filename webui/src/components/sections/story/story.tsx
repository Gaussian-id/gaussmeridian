import { Reveal, ScrollReveal } from "@/components/motion";

export function StoryHero() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-24">
      <Reveal>
        <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
          Story
        </div>
      </Reveal>
      <Reveal delay={0.05}>
        <h1 className="font-display mt-4 max-w-[15ch] text-5xl font-semibold tracking-tight sm:text-6xl md:text-7xl">
          Every model is a point. <span className="text-gradient">Meridian is the line.</span>
        </h1>
      </Reveal>
      <Reveal delay={0.1}>
        <p className="text-muted-foreground mt-6 max-w-[48ch] text-xl leading-relaxed">
          We built Meridian to give applications one durable contract for supported models — with
          project identity, metered usage, and payment evidence that remain understandable.
        </p>
      </Reveal>
    </section>
  );
}

export function StoryProblem() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <div className="max-w-[40rem]">
        <ScrollReveal>
          <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
            The map kept changing
          </div>
          <h2 className="font-display mt-3 max-w-[16ch] text-3xl font-semibold tracking-tight md:text-5xl">
            Choosing a model became a full-time job.
          </h2>
        </ScrollReveal>
        <ScrollReveal delay={0.05}>
          <p className="text-foreground mt-5 max-w-[44ch] text-lg leading-relaxed">
            A new model lands every week. Prices change, integrations drift, and teams lose the
            connection between a project call, its usage, and the money that funded it.
          </p>
        </ScrollReveal>
        <ScrollReveal delay={0.1}>
          <p className="text-muted-foreground mt-4 max-w-[44ch] text-lg leading-relaxed">
            An API gateway should reduce that work: one dependable request shape, an explicit model
            catalog, and a ledger that keeps the result tied to the right organization.
          </p>
        </ScrollReveal>
      </div>
    </section>
  );
}

export function StoryName() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <div className="max-w-[40rem]">
        <ScrollReveal>
          <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
            Why &ldquo;Meridian&rdquo;
          </div>
        </ScrollReveal>
        <ScrollReveal delay={0.05}>
          <p className="font-display mt-4 max-w-[20ch] text-2xl leading-snug font-medium tracking-tight md:text-[34px]">
            A meridian is the reference line that runs across the whole globe — it{" "}
            <span className="text-gradient font-semibold">
              connects distant points into one orientation
            </span>
            .
          </p>
        </ScrollReveal>
        <ScrollReveal delay={0.1}>
          <p className="text-foreground mt-6 max-w-[44ch] text-lg leading-relaxed">
            That&rsquo;s exactly what routing should be. Not a black box that hides its choices, but
            a line you can see: it connects a project key, a supported model, a normalized response,
            and the organization whose verified purchased credit authorized the work.
          </p>
        </ScrollReveal>
      </div>
    </section>
  );
}

const principles = [
  {
    n: "01",
    t: "Keep the contract stable.",
    d: "Applications use one OpenAI-compatible request and response shape while Meridian operates the supplier connection behind it.",
  },
  {
    n: "02",
    t: "Record real usage.",
    d: "Completed model usage and provider cost are attributed to the exact calling project without fabricating a retail wallet debit.",
  },
  {
    n: "03",
    t: "Make money traceable.",
    d: "A credit purchase remains connected to its order, verified payment, invoice, and exactly-once wallet grant.",
  },
];

export function StoryPrinciples() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <ScrollReveal>
        <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
          What we believe
        </div>
        <h2 className="font-display mt-3 text-3xl font-semibold tracking-tight md:text-4xl">
          Three lines we won&rsquo;t cross.
        </h2>
      </ScrollReveal>
      <div className="mt-11 grid gap-4 md:grid-cols-3">
        {principles.map((pr, i) => (
          <ScrollReveal key={pr.n} delay={i * 0.06} className="h-full">
            <div className="border-border h-full rounded-[18px] border bg-[color-mix(in_srgb,var(--card)_82%,transparent)] p-6">
              <div className="font-mono text-[11px] tracking-[0.16em] text-[var(--gauss-500)]">
                {pr.n}
              </div>
              <div className="font-display mt-3 text-xl font-semibold tracking-tight">{pr.t}</div>
              <p className="text-muted-foreground mt-2.5 text-[14.5px] leading-relaxed">{pr.d}</p>
            </div>
          </ScrollReveal>
        ))}
      </div>
    </section>
  );
}

export function StoryCompany() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <div className="max-w-[40rem]">
        <ScrollReveal>
          <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
            Built by Gaussian
          </div>
        </ScrollReveal>
        <ScrollReveal delay={0.05}>
          <h2 className="font-display mt-3 max-w-[18ch] text-3xl font-semibold tracking-tight md:text-5xl">
            Precision instruments for AI teams.
          </h2>
        </ScrollReveal>
        <ScrollReveal delay={0.1}>
          <p className="text-foreground mt-5 max-w-[44ch] text-lg leading-relaxed">
            Meridian is made by <b>Gaussian</b> — we build the calm, exact tooling that sits between
            your product and the models it depends on. Meridian is the line. We keep it steady.
          </p>
        </ScrollReveal>
      </div>
    </section>
  );
}
