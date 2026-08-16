import type { RouteDecision } from "@core/adapters/schemas/console.schema";

/**
 * Keyed by project id (the URL already scopes the request in the mock's route table; the real
 * `route_decision` row's own `project_id` is a record link the mock keeps in sync with the key
 * for realism). Shape traced against the real backend `RouteDecision`/`RouteDecisionInsert`
 * (see `console.schema.ts`'s `RouteDecisionSchema` doc comment) — NOT the earlier Phase-1
 * invented shape (`prompt_excerpt`, `complexity_band`, `r_binary`, per-candidate `reason`, none
 * of which the real table carries). Covers, deliberately:
 * - a mix of `guardrail_status` values (`passed`/`disabled`/`skipped`)
 * - a spread of raw `complexity` scores across all three display bands
 * - at least one `moa.enabled: true` row with a winner and losers stamped `cost: 0`
 * - a spread of `cascade_used` values
 * - a decision with no candidate marked `selected` (the real MoA-dispatch shape — see
 *   `middleware.rs::build_route_decision_entry`), so `deliveredBy` falls back to `moa.winner`
 */
export const routeDecisions: Record<string, RouteDecision[]> = {
  proj_prod: [
    {
      id: "route_1",
      request_id: "req_1",
      project_id: "proj_prod",
      org_id: "org_1",
      candidates: [
        { model: "gpt-4o-mini", provider: "openai", score: 0.92, selected: true },
        { model: "gpt-4o", provider: "openai", score: 0.81, selected: false },
      ],
      moa: { enabled: false, winner: null, losers: [] },
      guardrail_status: "passed",
      cascade_used: false,
      complexity: 0.25,
      baseline_cost: 0.0123,
      created_at: "2026-07-14T09:12:00Z",
    },
    {
      id: "route_2",
      request_id: "req_2",
      project_id: "proj_prod",
      org_id: "org_1",
      candidates: [
        { model: "claude-3-5-sonnet", provider: "anthropic", score: 0.74, selected: true },
        { model: "gpt-4o", provider: "openai", score: 0.7, selected: false },
      ],
      moa: { enabled: false, winner: null, losers: [] },
      guardrail_status: "disabled",
      cascade_used: true,
      complexity: 0.58,
      baseline_cost: 0.0512,
      created_at: "2026-07-14T10:03:00Z",
    },
    {
      id: "route_3",
      request_id: "req_3",
      project_id: "proj_prod",
      org_id: "org_1",
      // GaussMoA dispatch — no candidate marked `selected` (matches the real backend's
      // `build_route_decision_entry` MoA path); the delivering model resolves from `moa.winner`.
      candidates: [
        { model: "gpt-4o", provider: "openai", score: 0.89, selected: false },
        { model: "claude-3-5-sonnet", provider: "anthropic", score: 0.85, selected: false },
        { model: "gemini-1.5-pro", provider: "google", score: 0.79, selected: false },
      ],
      moa: {
        enabled: true,
        winner: { model: "gpt-4o", confidence: 0.89, cost: 0.0672 },
        losers: [
          { model: "claude-3-5-sonnet", confidence: 0.85, cost: 0 },
          { model: "gemini-1.5-pro", confidence: 0.79, cost: 0 },
        ],
      },
      guardrail_status: "skipped",
      cascade_used: false,
      complexity: 0.87,
      baseline_cost: 0.0672,
      created_at: "2026-07-14T11:47:00Z",
    },
    {
      id: "route_4",
      request_id: "req_4",
      project_id: "proj_prod",
      org_id: "org_1",
      candidates: [{ model: "gpt-4o-mini", provider: "openai", score: 0.95, selected: true }],
      moa: { enabled: false, winner: null, losers: [] },
      guardrail_status: "passed",
      cascade_used: false,
      complexity: 0.12,
      baseline_cost: 0.0041,
      created_at: "2026-07-14T12:15:00Z",
    },
    {
      id: "route_5",
      request_id: "req_5",
      project_id: "proj_prod",
      org_id: "org_1",
      candidates: [
        { model: "claude-3-5-sonnet", provider: "anthropic", score: 0.88, selected: true },
        { model: "gpt-4o-mini", provider: "openai", score: 0.52, selected: false },
      ],
      moa: { enabled: false, winner: null, losers: [] },
      guardrail_status: "passed",
      cascade_used: false,
      complexity: 0.71,
      baseline_cost: 0.1187,
      created_at: "2026-07-14T13:02:00Z",
    },
  ],
  proj_dev: [
    {
      id: "route_dev_1",
      request_id: "req_dev_1",
      project_id: "proj_dev",
      org_id: "org_1",
      candidates: [{ model: "gpt-4o-mini", provider: "openai", score: 0.94, selected: true }],
      moa: { enabled: false, winner: null, losers: [] },
      guardrail_status: "passed",
      cascade_used: false,
      complexity: 0.18,
      baseline_cost: 0.0022,
      created_at: "2026-07-13T08:00:00Z",
    },
    {
      id: "route_dev_2",
      request_id: "req_dev_2",
      project_id: "proj_dev",
      org_id: "org_1",
      candidates: [{ model: "gpt-4o", provider: "openai", score: 0.8, selected: true }],
      moa: { enabled: false, winner: null, losers: [] },
      guardrail_status: "disabled",
      cascade_used: true,
      complexity: 0.66,
      baseline_cost: 0.0284,
      created_at: "2026-07-13T09:40:00Z",
    },
  ],
};
