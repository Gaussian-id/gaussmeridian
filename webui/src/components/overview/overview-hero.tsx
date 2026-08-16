"use client";

import { StatCard } from "@/components/dashboard/stat-card";
import { Reveal, useParallax, useTilt } from "@/components/motion";
import { ErrorState } from "@/components/ui/error-state";
import { useOutcomeSavings, useRouteDecisions } from "@/hooks/useConsoleQueries";
import type { RouteDecisionStreamStatus } from "@/hooks/useRouteDecisionStream";

import { ComplexityDistribution } from "./complexity-distribution";
import { complexityDistributionFrom, queryErrorMessage } from "./route-decision-utils";
import { SavingsCounter } from "./savings-counter";
import { StreamStatusIndicator } from "./stream-status-indicator";

interface OverviewHeroProps {
  projectId: string;
  /** SSE connection state, lifted to the page and shared with `RecentRoutesFeed` — the hero's
   *  status badge and the feed's badge are the same source of truth (Reviewer F1), so the word
   *  next to "OutcomeGate · honest billing" only ever says "Live" when the feed genuinely is. */
  streamStatus: RouteDecisionStreamStatus;
}

// How many of the most recent decisions the "complexity mix" panel samples — see
// `complexityDistributionFrom`'s doc comment for why this is a sample, not a period aggregate.
const COMPLEXITY_SAMPLE_LIMIT = 50;

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`;
}

/**
 * The Project Overview's cinematic band — the surface where a developer "tastes the
 * difference." A `bg-brand-gradient` panel carries the OutcomeGate savings counter and CARROT
 * complexity histogram; the GaussMoA/Cascade adoption + reliability tiles (plain `StatCard`)
 * sit beneath it. Motion (`Reveal`/`useParallax`/`useTilt`) is decorative only — every number
 * here is fully legible with motion off, and the hooks themselves no-op under
 * `prefers-reduced-motion` (see `components/motion/use-cursor.ts`).
 */
export function OverviewHero({ projectId, streamStatus }: OverviewHeroProps) {
  const savings = useOutcomeSavings(projectId);
  const recentForComplexity = useRouteDecisions(projectId, { limit: COMPLEXITY_SAMPLE_LIMIT });
  const glowRef = useParallax<HTMLDivElement>(10);
  const panelRef = useTilt<HTMLDivElement>(4);

  return (
    <section className="relative isolate overflow-hidden rounded-[28px]">
      <div className="bg-radial-glow pointer-events-none absolute inset-0 -z-10" />

      <div ref={glowRef}>
        <Reveal>
          <div
            ref={panelRef}
            className="bg-brand-gradient shadow-glow relative overflow-hidden rounded-[24px] p-8 sm:p-10"
          >
            <div className="flex flex-wrap items-baseline justify-between gap-3">
              <span className="font-mono text-[11px] tracking-[0.28em] text-white/55 uppercase">
                OutcomeGate · honest billing
              </span>
              <StreamStatusIndicator
                status={streamStatus}
                onDark
                className="text-[10.5px] tracking-[0.22em]"
              />
            </div>

            {savings.isError ? (
              <div className="mt-6">
                <ErrorState
                  message={queryErrorMessage(
                    savings.error,
                    "Could not load this project's outcome savings. Try again shortly.",
                  )}
                />
              </div>
            ) : (
              <div className="mt-6 grid gap-8 md:grid-cols-[1.1fr_0.9fr] md:items-end">
                <SavingsCounter
                  total={savings.data?.zero_charge_saved ?? 0}
                  count={savings.data?.zero_charge_count ?? 0}
                  isLoading={savings.isLoading}
                />
                <ComplexityDistribution
                  distribution={
                    recentForComplexity.data
                      ? complexityDistributionFrom(recentForComplexity.data)
                      : []
                  }
                  isLoading={recentForComplexity.isLoading}
                  sampleLabel={`last ${recentForComplexity.data?.length ?? 0} routed`}
                />
              </div>
            )}
          </div>
        </Reveal>
      </div>

      <Reveal delay={0.1}>
        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          <StatCard
            label="GaussMoA adoption"
            value={savings.data ? formatPercent(savings.data.moa_adoption_pct) : undefined}
            isLoading={savings.isLoading}
          />
          <StatCard
            label="Cascade adoption"
            value={savings.data ? formatPercent(savings.data.cascade_adoption_pct) : undefined}
            isLoading={savings.isLoading}
          />
          <StatCard
            label="Reliability"
            value={savings.data ? formatPercent(savings.data.avg_r_binary) : undefined}
            isLoading={savings.isLoading}
          />
        </div>
      </Reveal>
    </section>
  );
}
