import type { OutcomeSavings } from "@core/adapters/schemas/console.schema";

/**
 * Keyed by project id, powering the Overview hero's savings counter + adoption stats. Shape
 * traced against the real backend `SavingsSummary` (route_decision_repository.rs) — see
 * `console.schema.ts`'s `OutcomeSavingsSchema` doc comment for why this replaced the Phase-1
 * invented shape (`not_charged_total`, `complexity_distribution`, etc.).
 */
export const savings: Record<string, OutcomeSavings> = {
  proj_prod: {
    total_requests: 549,
    total_cost_charged: 128.44,
    total_baseline_cost: 133.26,
    total_saved: 4.82,
    zero_charge_count: 37,
    zero_charge_saved: 4.82,
    avg_r_binary: 0.93,
    cascade_adoption_pct: 0.24,
    moa_adoption_pct: 0.18,
  },
  proj_dev: {
    total_requests: 26,
    total_cost_charged: 2.06,
    total_baseline_cost: 2.37,
    total_saved: 0.31,
    zero_charge_count: 4,
    zero_charge_saved: 0.31,
    avg_r_binary: 0.86,
    cascade_adoption_pct: 0.09,
    moa_adoption_pct: 0.05,
  },
};
