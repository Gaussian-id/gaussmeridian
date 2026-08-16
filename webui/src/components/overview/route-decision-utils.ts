import { GaussMeridianAdapterError } from "@core/adapters/gaussmeridian-data.adapter";
import type { RouteDecision } from "@core/adapters/schemas/console.schema";

export type ComplexityBand = "low" | "medium" | "high";

/**
 * Prefers the backend's own error message when the failure is a real, adapter-surfaced
 * `GaussMeridianAdapterError` (carries the server's normalized `error.message`), falling back to a
 * generic line otherwise (Reviewer F2). Mirrors the honest-degrade posture `useChat`'s
 * `describeChatError` takes for the playground — surface the real, checkable cause where we have
 * one rather than a blanket "something went wrong".
 */
export function queryErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof GaussMeridianAdapterError && error.message) return error.message;
  return fallback;
}

/**
 * Client-side bucketing of the raw CARROT `complexity` float (0..1) into the three display
 * bands the UI has always used. The real backend only stores the raw score — banding is a
 * presentation concern, not a backend contract field (see `console.schema.ts`'s
 * `RouteDecisionSchema` doc comment) — so it lives here instead of being invented as a fake
 * schema field. Thresholds are an even 3-way split; there is no backend-defined cutover.
 */
export function complexityBand(score: number): ComplexityBand {
  if (score < 1 / 3) return "low";
  if (score < 2 / 3) return "medium";
  return "high";
}

export const COMPLEXITY_BAND_LABEL: Record<ComplexityBand, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
};

/**
 * The model that actually delivered this decision's response: the candidate marked `selected`,
 * or — for a GaussMoA dispatch, where no single candidate is ever marked selected (see
 * `middleware.rs::build_route_decision_entry`, which always passes `selected_index: None` on
 * the MoA path) — the MoA winner. `null` only when a decision has neither, which callers must
 * render as an honest "unresolved" state rather than guessing.
 */
export function deliveredBy(decision: RouteDecision): { model: string; provider: string } | null {
  const selected = decision.candidates.find((candidate) => candidate.selected);
  if (selected) return { model: selected.model, provider: selected.provider };
  if (decision.moa.winner) return { model: decision.moa.winner.model, provider: "gaussmoa" };
  return null;
}

/**
 * Real `guardrail_status` values observed server-side (`middleware.rs`): `"passed"` (ran and
 * passed), `"disabled"` (guardrail config off for this project), `"skipped"` (GaussMoA
 * responses aren't guardrail-inspected today). The field is an open string on the backend, not
 * a closed enum, so an unrecognized value still renders — just without a friendlier label.
 */
const GUARDRAIL_LABEL: Record<string, string> = {
  passed: "Passed",
  disabled: "Disabled",
  skipped: "Skipped",
  failed: "Failed",
};

export function guardrailLabel(status: string): string {
  return GUARDRAIL_LABEL[status] ?? status;
}

// No semantic "success" token exists in `@theme/tokens.css` — mirrors the one-off green the
// marketing `sections/hero/hero.tsx` trace card already uses for its own "ok" state.
const GUARDRAIL_TONE: Record<string, string> = {
  passed: "text-[#38b678]",
  disabled: "text-muted-foreground",
  skipped: "text-muted-foreground",
  failed: "text-destructive",
};

export function guardrailTone(status: string): string {
  return GUARDRAIL_TONE[status] ?? "text-muted-foreground";
}

/**
 * Buckets a sample of recent decisions into complexity-band counts, for the Overview hero's
 * "complexity mix" panel. NOT a period-wide aggregate — `GET /v1/analytics/savings` has no such
 * histogram (see `console.schema.ts`'s `OutcomeSavingsSchema` doc comment) — this is an honest
 * read of whatever sample of `useRouteDecisions` the caller already fetched, labeled as such by
 * the caller rather than presented as a full-period stat.
 */
export function complexityDistributionFrom(
  decisions: RouteDecision[],
): { band: ComplexityBand; count: number }[] {
  const counts: Record<ComplexityBand, number> = { low: 0, medium: 0, high: 0 };
  for (const decision of decisions) {
    counts[complexityBand(decision.complexity)] += 1;
  }
  return (["low", "medium", "high"] as const).map((band) => ({ band, count: counts[band] }));
}
