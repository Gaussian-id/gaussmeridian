import { ArrowRight, Boxes, KeyRound, Route } from "lucide-react";
import Link from "next/link";

import { cn } from "@core/lib/utils";

import { buttonVariants } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

const STEPS = [
  {
    icon: Boxes,
    title: "Create a project",
    description: "Projects separate production traffic from development — pick an environment.",
  },
  {
    icon: KeyRound,
    title: "Generate an API key",
    description: "Every project gets its own scoped keys, so a leak never spans environments.",
  },
  {
    icon: Route,
    title: "Route your first request",
    description: "Point your app at the key and watch CARROT and OutcomeGate do the rest.",
  },
] as const;

/**
 * The org has zero projects — either just-created (born empty, no default project) or
 * genuinely cleared out. This is a first-class state, not a loading fallback: it explains
 * what a project is, why there isn't one yet, and the exact next step. Load-bearing for the
 * "new tenants are born empty" decision, since it's the very first thing a new owner sees.
 */
export function EmptyProjects({ orgId }: { orgId: string }) {
  return (
    <Card className="flex flex-col items-center gap-8 px-8 py-16 text-center">
      <div>
        <h2 className="font-display text-2xl font-semibold tracking-tight">No projects yet</h2>
        <p className="text-muted-foreground mx-auto mt-2 max-w-md text-sm">
          This organization was created empty — that&apos;s intentional. Create a project to get an
          environment, API keys, and a routing dashboard of your own.
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-3">
        {STEPS.map((step) => (
          <div key={step.title} className="flex flex-col items-center gap-2 sm:max-w-[180px]">
            <div className="bg-secondary flex h-10 w-10 items-center justify-center rounded-full">
              <step.icon className="text-accent h-5 w-5" aria-hidden="true" />
            </div>
            <p className="text-sm font-medium">{step.title}</p>
            <p className="text-muted-foreground text-xs leading-relaxed">{step.description}</p>
          </div>
        ))}
      </div>

      <Link
        href={`/orgs/${orgId}/projects/new`}
        className={cn(buttonVariants({ variant: "accent", size: "lg" }))}
      >
        Create your first project
        <ArrowRight className="h-4 w-4" aria-hidden="true" />
      </Link>
    </Card>
  );
}
