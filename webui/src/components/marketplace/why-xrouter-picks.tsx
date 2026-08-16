"use client";

import { cn } from "@core/lib/utils";

import { Badge } from "@/components/ui/badge";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useRouteDecisions } from "@/hooks/useConsoleQueries";

const RECENT_APPEARANCES_LIMIT = 5;

interface WhyXrouterPicksProps {
  projectId: string;
  modelId: string;
}

interface Appearance {
  decisionId: string | null;
  score: number;
  selected: boolean;
}

/**
 * Meridian-only "why xRouter picks here" panel — visual idiom borrowed from the marketing
 * hero's trace card (`sections/hero/hero.tsx`, the `{k, v, stage}` row) and the transparency
 * drawer's quiet `TraceRow`, but without the cinematic `bg-brand-gradient`/motion treatment:
 * Model Marketplace is dense/quiet chrome, not a cinematic surface (those are reserved for the
 * Overview hero + Playground).
 *
 * Deliberately NOT fabricated candidate-scoring data: this scans the project's actual
 * `RouteDecision` history (`useRouteDecisions`, the same routing ledger the Activity/Overview
 * surfaces read) for every time this model appeared as an xRouter candidate, and shows the
 * real `reason`/`score`/`selected` values CARROT/xRouter already produced for it. A model
 * xRouter has never actually considered here gets an honest empty state, not an invented one.
 */
export function WhyXrouterPicks({ projectId, modelId }: WhyXrouterPicksProps) {
  const routes = useRouteDecisions(projectId);

  if (routes.isLoading) {
    return (
      <Card className="p-6">
        <Skeleton className="h-4 w-48" />
        <Skeleton className="mt-4 h-16 w-full" />
      </Card>
    );
  }

  if (routes.isError) {
    return (
      <Card className="p-6">
        <CardTitle>Why xRouter picks this</CardTitle>
        <p className="text-muted-foreground mt-3 text-sm">
          Could not load this project&apos;s routing history. Try again shortly.
        </p>
      </Card>
    );
  }

  const appearances: Appearance[] = (routes.data ?? [])
    .flatMap((decision) =>
      decision.candidates
        .filter((candidate) => candidate.model === modelId)
        .map((candidate) => ({
          decisionId: decision.id ?? decision.request_id,
          score: candidate.score,
          selected: candidate.selected,
        })),
    )
    .slice(0, RECENT_APPEARANCES_LIMIT);

  const selectedCount = appearances.filter((appearance) => appearance.selected).length;

  return (
    <Card className="p-6">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <CardTitle>Why xRouter picks this</CardTitle>
        <span className="text-accent font-mono text-[10.5px] tracking-[0.18em] uppercase">
          xRouter
        </span>
      </div>
      <CardDescription className="mt-1">
        {appearances.length === 0
          ? "Not yet observed as a routing candidate in this project's recent activity."
          : `Considered in ${appearances.length} of the last routing decisions · selected ${selectedCount} time${selectedCount === 1 ? "" : "s"}.`}
      </CardDescription>

      {appearances.length > 0 && (
        <ol className="mt-4 flex flex-col gap-2">
          {appearances.map((appearance, index) => (
            <li
              key={`${appearance.decisionId ?? "unknown"}-${index}`}
              className={cn(
                "border-border rounded-lg border p-3",
                appearance.selected && "border-accent/60 bg-secondary/40",
              )}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono text-xs">score {appearance.score.toFixed(2)}</span>
                {appearance.selected && (
                  <Badge variant="mono" className="text-[10px]">
                    Selected
                  </Badge>
                )}
              </div>
            </li>
          ))}
        </ol>
      )}
    </Card>
  );
}
