import { COMPLEXITY_BAND_LABEL } from "./route-decision-utils";

import type { ComplexityBand } from "./route-decision-utils";

interface ComplexityDistributionProps {
  /** Client-side bucketing of a sample of recent `RouteDecision`s — see
   *  `route-decision-utils.ts::complexityDistributionFrom`. NOT a period-wide backend
   *  aggregate (`GET /v1/analytics/savings` has no such histogram); `sampleLabel` says exactly
   *  what this sample is so the panel never implies more coverage than it has. */
  distribution: { band: ComplexityBand; count: number }[];
  isLoading?: boolean;
  /** e.g. "last 50 routed" — rendered as "CARROT complexity · {sampleLabel}". */
  sampleLabel?: string;
}

/** CARROT complexity histogram — token-driven bars (no chart library needed for three
 *  categories), styled for the dark `bg-brand-gradient` hero panel it lives inside. */
export function ComplexityDistribution({
  distribution,
  isLoading,
  sampleLabel = "recent activity",
}: ComplexityDistributionProps) {
  const total = distribution.reduce((sum, entry) => sum + entry.count, 0);

  return (
    <div>
      <p className="font-mono text-[11px] tracking-[0.28em] text-white/55 uppercase">
        CARROT complexity · {sampleLabel}
      </p>
      <div className="mt-3 flex flex-col gap-2.5">
        {isLoading ? (
          <p className="text-sm text-white/65">Loading…</p>
        ) : total === 0 ? (
          <p className="text-sm text-white/65">No routed requests yet.</p>
        ) : (
          distribution.map((entry) => {
            const pct = (entry.count / total) * 100;
            return (
              <div key={entry.band} className="flex items-center gap-3">
                <span className="w-16 shrink-0 font-mono text-xs tracking-wide text-white/70 uppercase">
                  {COMPLEXITY_BAND_LABEL[entry.band]}
                </span>
                <div className="h-2 flex-1 overflow-hidden rounded-full bg-white/15">
                  <div
                    className="h-full rounded-full bg-[var(--gauss-400)]"
                    style={{ width: `${pct}%` }}
                  />
                </div>
                <span className="w-8 shrink-0 text-right font-mono text-xs text-white/70">
                  {entry.count}
                </span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
