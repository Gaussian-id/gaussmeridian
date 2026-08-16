import { z } from "zod";

/**
 * NET-NEW front-end contract — pending maintainer ratification as the Phase-2 SurrealDB
 * backend spec.
 *
 * Every shape in this file (Org/Project/Member/Role/RouteDecision/OutcomeSavings) is
 * invented for the Phase-1 mock console — none of it corresponds to a live GaussMeridian
 * endpoint yet. `RouteDecisionSchema` is a superset of the real, backend-verified
 * `RequestLogSchema` (see gaussmeridian.schema.ts) so the Charged/Free pill keeps using the
 * identical `r_binary === 1` rule once this contract goes live. This file becomes the
 * Phase-2 backend spec once ratified — do not treat it as authoritative until then.
 */

// ---- Org ----

export const OrgPlanSchema = z.enum(["free", "pro", "enterprise"]);
export type OrgPlan = z.infer<typeof OrgPlanSchema>;

export const OrgSchema = z.object({
  id: z.string(),
  name: z.string(),
  slug: z.string(),
  plan: OrgPlanSchema,
  created_at: z.string(),
  member_count: z.number(),
  project_count: z.number(),
});
export type Org = z.infer<typeof OrgSchema>;

export const OrgListSchema = z.object({ orgs: z.array(OrgSchema) });
export type OrgList = z.infer<typeof OrgListSchema>;

// ---- Project ----

export const ProjectEnvironmentSchema = z.enum(["production", "development"]);
export type ProjectEnvironment = z.infer<typeof ProjectEnvironmentSchema>;

export const ProjectSchema = z.object({
  id: z.string(),
  org_id: z.string(),
  name: z.string(),
  slug: z.string(),
  environment: ProjectEnvironmentSchema,
  created_at: z.string(),
});
export type Project = z.infer<typeof ProjectSchema>;

export const ProjectListSchema = z.object({ projects: z.array(ProjectSchema) });
export type ProjectList = z.infer<typeof ProjectListSchema>;

// ---- Role / Member (RBAC lives at the org level; a project has settings only) ----

export const RoleSchema = z.enum(["owner", "admin", "developer"]);
export type Role = z.infer<typeof RoleSchema>;

export const MemberStatusSchema = z.enum(["active", "invited"]);
export type MemberStatus = z.infer<typeof MemberStatusSchema>;

export const MemberSchema = z.object({
  id: z.string(),
  org_id: z.string(),
  user_id: z.string(),
  email: z.string(),
  display_name: z.string(),
  role: RoleSchema,
  status: MemberStatusSchema,
  // A member is either invited (invited_at set, joined_at null) or has joined
  // (both set) — an owner created alongside their org has invited_at null.
  invited_at: z.string().nullable(),
  joined_at: z.string().nullable(),
});
export type Member = z.infer<typeof MemberSchema>;

export const MemberListSchema = z.object({ members: z.array(MemberSchema) });
export type MemberList = z.infer<typeof MemberListSchema>;

export const PermissionMatrixSchema = z.object({
  permissions: z.array(
    z.object({
      key: z.string(),
      label: z.string(),
      group: z.string(),
    }),
  ),
  roles: z.array(
    z.object({
      role: RoleSchema,
      grants: z.array(z.string()),
    }),
  ),
});
export type PermissionMatrix = z.infer<typeof PermissionMatrixSchema>;

// ---- Route decisions (the OutcomeGate / CARROT / xRouter / GaussMoA transparency trace) ----
//
// PRD-21 Wave C / DR-009 D4 — traced against the REAL backend `RouteDecision`
// (gaussmeridian-db/src/schema.rs) and `RouteDecisionInsert`/`RouteDecisionCandidate`/
// `RouteDecisionMoa`/`MoaAgentSnapshot` (route_decision_repository.rs). This replaces the
// Phase-1 invented shape (`RequestLogSchema.extend({...})`) — the real `route_decision` table
// is NOT a superset of the ledger-sourced `RequestLogSchema`: it carries no
// model/provider/tokens/cost_charged/r_binary of its own. Those live on the separate
// `ledger_entry` table (`RequestLogSchema`, `/v1/logs`) and are correlated with a route
// decision only by `request_id` — never joined server-side by these endpoints. Concretely:
//  - no `prompt_excerpt` (never stored)
//  - no `complexity_band` enum — only the raw CARROT `complexity` float; banding for display
//    is a client-side presentation concern (see `components/overview/route-decision-utils.ts`)
//  - no per-candidate `reason` string (only `model`/`provider`/`score`/`selected`)
//  - the delivered model/provider must be derived from `candidates[].selected`, or (for a
//    GaussMoA dispatch, where no candidate is ever marked selected — see
//    `middleware.rs::build_route_decision_entry`) from `moa.winner`
//  - `guardrail_status` is an open string server-side (`passed`/`disabled`/`skipped` observed;
//    not a closed enum), so this stays `z.string()` rather than re-inventing a stricter
//    contract the backend doesn't actually enforce

export const RouteCandidateSchema = z.object({
  model: z.string(),
  provider: z.string(),
  score: z.number(),
  selected: z.boolean(),
});
export type RouteCandidate = z.infer<typeof RouteCandidateSchema>;

export const MoaAgentSnapshotSchema = z.object({
  model: z.string(),
  confidence: z.number(),
  /** The winner carries the real charged cost; every loser is stamped `0` — MoA agents that
   *  didn't win are never billed (DR-009 D4). */
  cost: z.number(),
});
export type MoaAgentSnapshot = z.infer<typeof MoaAgentSnapshotSchema>;

export const RouteDecisionMoaSchema = z.object({
  enabled: z.boolean().default(false),
  winner: MoaAgentSnapshotSchema.nullable().default(null),
  losers: z.array(MoaAgentSnapshotSchema).default([]),
});
export type RouteDecisionMoa = z.infer<typeof RouteDecisionMoaSchema>;

/**
 * One routed request's transparency trace. `id`/`project_id`/`org_id` are `Option<String>`
 * server-side with `skip_serializing_if = "Option::is_none"` — omitted from the wire (not
 * `null`) when absent, hence `.optional()` rather than `.nullable()`.
 */
export const RouteDecisionSchema = z.object({
  id: z.string().optional(),
  request_id: z.string(),
  project_id: z.string().optional(),
  org_id: z.string().optional(),
  candidates: z.array(RouteCandidateSchema),
  moa: RouteDecisionMoaSchema,
  guardrail_status: z.string(),
  cascade_used: z.boolean(),
  complexity: z.number(),
  baseline_cost: z.number(),
  created_at: z.string(),
});
export type RouteDecision = z.infer<typeof RouteDecisionSchema>;

// GET /v1/route-decisions returns a bare JSON array (`Json(decisions)`, `decisions:
// Vec<RouteDecision>`), NOT an `{ decisions: [...] }` envelope.
export const RouteDecisionListSchema = z.array(RouteDecisionSchema);
export type RouteDecisionList = z.infer<typeof RouteDecisionListSchema>;

/**
 * The SSE payload on `GET /v1/route-decisions/stream` is a `RouteDecisionInsert`
 * (middleware.rs's `route_decision_tx` broadcast channel) — identical fields to `RouteDecision`
 * minus the DB-assigned `id` (the row doesn't have one yet when it's published).
 */
export const RouteDecisionStreamEventSchema = RouteDecisionSchema.omit({ id: true });
export type RouteDecisionStreamEvent = z.infer<typeof RouteDecisionStreamEventSchema>;

// One row of the GaussMoA candidate panel used by the Playground's fixture-backed global panel
// (`useMoaCandidates` / `MOA_CANDIDATES_RESOURCE` — wired in M5, NOT part of the Wave-C real
// backend contract; `route_decision.moa` above is the real per-decision MoA shape). Losers are
// always stamped `stamped_cost: 0` — the same "not charged" idea as OutcomeGate's `$0.00`.
export const MoaCandidateSchema = z.object({
  model: z.string(),
  provider: z.string(),
  contribution: z.number(),
  is_winner: z.boolean(),
  stamped_cost: z.number(),
});
export type MoaCandidate = z.infer<typeof MoaCandidateSchema>;

export const MoaCandidateListSchema = z.object({ candidates: z.array(MoaCandidateSchema) });
export type MoaCandidateList = z.infer<typeof MoaCandidateListSchema>;

// ---- Outcome savings (Overview hero) ----
//
// Traced against the real backend `SavingsSummary` (route_decision_repository.rs). Replaces
// the Phase-1 invented shape entirely — field names and meaning differ from the mock fixture
// this superseded (`not_charged_total` -> `zero_charge_saved`, etc.). `cascade_adoption_pct`/
// `moa_adoption_pct`/`avg_r_binary` are fractions in `[0, 1]`, not already-scaled percentages.
// There is no `complexity_distribution` in this aggregate — the backend doesn't compute a
// period-wide band histogram; the Overview hero derives an honest "recent mix" from a sample of
// `useRouteDecisions` instead (see `route-decision-utils.ts::complexityDistributionFrom`).

export const OutcomeSavingsSchema = z.object({
  total_requests: z.number(),
  total_cost_charged: z.number(),
  total_baseline_cost: z.number(),
  /** `max(0, total_baseline_cost - total_cost_charged)` — never negative. */
  total_saved: z.number(),
  zero_charge_count: z.number(),
  zero_charge_saved: z.number(),
  avg_r_binary: z.number(),
  cascade_adoption_pct: z.number(),
  moa_adoption_pct: z.number(),
});
export type OutcomeSavings = z.infer<typeof OutcomeSavingsSchema>;

// ---- Onboarding (PRD-21 Wave B / DR-010) ----
// Traced against the real backend `OnboardingStateResponse` / `PublicUser` (handlers.rs).
// `current_step` is a free-form string server-side (no backend-enforced enum) — the FE's
// `OnboardingStep` union (`lib/onboarding/onboarding-machine.ts`) is the client-side contract;
// an unrecognized value here is handled by `fromServerState`, not by this schema.

export const WorkspaceDispositionSchema = z.enum(["pending", "configured", "skipped"]);
export type WorkspaceDisposition = z.infer<typeof WorkspaceDispositionSchema>;

export const OnboardingStateResponseSchema = z.object({
  current_step: z.string().nullable(),
  completed_steps: z.array(z.string()),
  onboarding_completed: z.boolean(),
  workspace_disposition: WorkspaceDispositionSchema,
});
export type OnboardingStateResponse = z.infer<typeof OnboardingStateResponseSchema>;

/**
 * Response of both `POST /v1/onboarding/survey` (204 No Content → `undefined`) and
 * `PATCH /v1/onboarding/profile` (200 → the full `PublicUser`). `passthrough()` because the
 * profile response carries the whole user record (id/email/roles/…) and this schema only
 * needs to assert the fields the wizard actually reads.
 */
export const OnboardingProfileResponseSchema = z
  .object({
    full_name: z.string().nullable().optional(),
    display_name: z.string().nullable().optional(),
    company: z.string().nullable().optional(),
    timezone: z.string().nullable().optional(),
  })
  .passthrough();
export type OnboardingProfileResponse = z.infer<typeof OnboardingProfileResponseSchema>;

/** `POST /v1/onboarding/complete` response — the full `PublicUser`, accepted for an explicitly
 *  skipped workspace or a server-verified org/project/key setup. An unmet configured-path gate is
 *  surfaced as a thrown adapter error rather than parsed by this success schema. */
export const OnboardingCompleteResponseSchema = z
  .object({ onboarding_completed: z.boolean() })
  .passthrough();
export type OnboardingCompleteResponse = z.infer<typeof OnboardingCompleteResponseSchema>;
