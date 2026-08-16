import { FileCheck, Globe, Lock, ShieldCheck } from "lucide-react";

import { Card, CardDescription, CardTitle } from "@/components/ui/card";

const capabilities = [
  {
    icon: ShieldCheck,
    title: "Prepaid access",
    body: "Add one-time organization credit when you need it. Verified purchased credit unlocks model calls without a recurring plan.",
  },
  {
    icon: Lock,
    title: "Intelligent Routing",
    body: "Route requests across providers using mixture-of-agents logic. Automatically select the best model for each request.",
  },
  {
    icon: FileCheck,
    title: "Full Observability",
    body: "Complete request tracing, response caching, usage analytics, and cost attribution per model and provider.",
  },
  {
    icon: Globe,
    title: "Payment evidence",
    body: "Every purchase stays connected to its order, verified payment, invoice, and exactly-once credit grant.",
  },
];

export function Capabilities() {
  return (
    <section
      aria-labelledby="capabilities-heading"
      className="dark border-border bg-background text-foreground relative overflow-hidden border-y"
    >
      <div className="bg-grid pointer-events-none absolute inset-0 opacity-[0.12]" />
      <div className="bg-radial-glow pointer-events-none absolute inset-0" />
      <div className="relative mx-auto w-full max-w-6xl px-6 py-24">
        <h2
          id="capabilities-heading"
          className="font-display max-w-xl text-3xl font-semibold tracking-tight md:text-4xl"
        >
          Everything you need to route and manage LLMs.
        </h2>

        <div className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {capabilities.map(({ icon: Icon, title, body }) => (
            <Card
              key={title}
              className="group p-6 transition-colors hover:border-[var(--gauss-500)]"
            >
              <span className="bg-secondary text-primary group-hover:shadow-glow inline-flex h-10 w-10 items-center justify-center rounded-lg transition-shadow">
                <Icon className="h-5 w-5" aria-hidden="true" />
              </span>
              <CardTitle className="mt-4">{title}</CardTitle>
              <CardDescription className="mt-2">{body}</CardDescription>
            </Card>
          ))}
        </div>
      </div>
    </section>
  );
}
