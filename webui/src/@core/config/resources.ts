/**
 * Single source of resource path strings for the net-new console data. `useConsoleQueries`
 * and the mock registry's route table (`@core/adapters/mock`) both import from here so the
 * two can never drift on a path. Keys follow the `[resource, params ?? null]` TanStack
 * Query key convention used by `useResourceQuery`.
 *
 * Scope: the Org -> Project -> Member/Role console tree (see `console.schema.ts` for the
 * payload contracts) plus the one resource `useMoaCandidates` needs. The pre-existing
 * project-scoped resources (`v1/logs`, `v1/api/keys`, `v1/byok/keys`, `v1/analytics/usage`,
 * `v1/models`, `v1/balance`, `v1/project/settings`, `v1/billing/*`) are unchanged and keep
 * their string literals where they're already defined (`useGaussmeridianQueries.ts`) — Phase 1
 * keeps them server-resolved against the caller's active project; id-parameterizing them is
 * a Phase-2 backend decision, not made here.
 */

export const ORGS_RESOURCE = "v1/orgs";

/**
 * Global roles catalog — `GET /v1/roles` on the real Wave-2 backend is NOT org-scoped (roles
 * are platform-defined, seeded once, referenced by every org via `role_id`). `orgRolesResource`
 * below keeps its `(orgId)` signature for caller compatibility but resolves to this same global
 * resource string regardless of `orgId`, so all org pages share one TanStack Query cache entry.
 */
export const ROLES_RESOURCE = "v1/roles";

/** Global fixture resource for the Playground GaussMoA candidate panel (wired in M5). */
export const MOA_CANDIDATES_RESOURCE = "v1/moa-candidates";

export function orgResource(orgId: string): string {
  return `v1/orgs/${orgId}`;
}

export function orgProjectsResource(orgId: string): string {
  return `v1/orgs/${orgId}/projects`;
}

export function orgProjectResource(orgId: string, projectId: string): string {
  return `v1/orgs/${orgId}/projects/${projectId}`;
}

export function orgMembersResource(orgId: string): string {
  return `v1/orgs/${orgId}/members`;
}

export function orgBillingCatalogResource(orgId: string): string {
  return `v1/orgs/${orgId}/billing/catalog`;
}

export function orgBillingWalletResource(orgId: string): string {
  return `v1/orgs/${orgId}/billing/wallet`;
}

export function orgBillingInvoicesResource(orgId: string): string {
  return `v1/orgs/${orgId}/billing/invoices`;
}

export function orgBillingInvoiceDocumentResource(orgId: string, invoiceId: string): string {
  return `${orgBillingInvoicesResource(orgId)}/${invoiceId}/document`;
}

export function orgBillingTopUpsResource(orgId: string): string {
  return `v1/orgs/${orgId}/billing/topups`;
}

export function orgBillingPaymentMethodsResource(orgId: string): string {
  return `v1/orgs/${orgId}/billing/payment-methods`;
}

export function orgBillingTopUpResource(orgId: string, orderId: string): string {
  return `${orgBillingTopUpsResource(orgId)}/${orderId}`;
}

export function orgBillingTopUpReconcileResource(orgId: string, orderId: string): string {
  return `${orgBillingTopUpResource(orgId, orderId)}/reconcile`;
}

export function orgBillingTopUpPaymentActionResource(orgId: string, orderId: string): string {
  return `${orgBillingTopUpResource(orgId, orderId)}/payment-action`;
}

export function orgBillingTopUpReceiptResource(orgId: string, orderId: string): string {
  return `${orgBillingTopUpResource(orgId, orderId)}/receipt`;
}

export function orgBillingSubscriptionsResource(orgId: string): string {
  return `v1/orgs/${orgId}/billing/subscriptions`;
}

export function orgBillingSubscriptionResource(orgId: string, subscriptionId: string): string {
  return `${orgBillingSubscriptionsResource(orgId)}/${subscriptionId}`;
}

export function orgBillingSubscriptionCancelResource(
  orgId: string,
  subscriptionId: string,
): string {
  return `${orgBillingSubscriptionResource(orgId, subscriptionId)}/cancel`;
}

export function orgBillingSubscriptionChangePlanResource(
  orgId: string,
  subscriptionId: string,
): string {
  return `${orgBillingSubscriptionResource(orgId, subscriptionId)}/change-plan`;
}

/**
 * `_orgId` is intentionally unused — see `ROLES_RESOURCE` above. Kept as a same-shaped function
 * (not a bare constant) so `usePermissionMatrix(orgId)`'s call site didn't need to change.
 */
export function orgRolesResource(_orgId: string): string {
  return ROLES_RESOURCE;
}

/**
 * PRD-21 Wave C / DR-009 D4 — this string stays project-scoped as the FE-facing TanStack Query
 * cache key even though the REAL backend endpoint (`GET /v1/route-decisions`) takes no project
 * id at all: it resolves the caller's project server-side from the auth context. The rewrite to
 * the real path happens at the adapter seam (`console-transparency.adapter.ts`), not here — this
 * function's job is only to give each project its own cache entry.
 */
export function projectRoutesResource(projectId: string): string {
  return `v1/projects/${projectId}/routes`;
}

/** Same non-parameterized-on-the-real-backend story as `projectRoutesResource` above, for
 *  `GET /v1/analytics/savings`. */
export function projectSavingsResource(projectId: string): string {
  return `v1/projects/${projectId}/savings`;
}

/** Project-scoped cache identity for the canonical ledger-backed usage endpoint. */
export function projectUsageAnalyticsResource(projectId: string): string {
  return `v1/projects/${projectId}/usage-analytics`;
}

/** Project-scoped cache identity for settled request activity from the delivery ledger. */
export function projectRequestLogsResource(projectId: string): string {
  return `v1/projects/${projectId}/request-logs`;
}

// ---- Onboarding (PRD-21 Wave B / DR-010) — the gated 7-step wizard ----
// User-scoped (resolved from the auth context server-side), not org/project-parameterized —
// mirrors the real router's `/v1/onboarding/*` nesting (routes.rs).

export const ONBOARDING_STATE_RESOURCE = "v1/onboarding/state";
export const ONBOARDING_ADVANCE_RESOURCE = "v1/onboarding/advance";
export const ONBOARDING_SURVEY_RESOURCE = "v1/onboarding/survey";
export const ONBOARDING_PROFILE_RESOURCE = "v1/onboarding/profile";
export const ONBOARDING_COMPLETE_RESOURCE = "v1/onboarding/complete";

/**
 * `GET /v1/auth/me` — read-only fetch of the caller's full `PublicUser` for the /account/me
 * page (`useAccountProfile`). Edits go back out through `ONBOARDING_PROFILE_RESOURCE` above
 * (`update_profile` in handlers.rs is the one write path for these fields, regardless of
 * whether the caller arrived via onboarding or account settings) — this constant exists only
 * for the read side, which `getSession()`'s slimmer `AuthSession` doesn't carry.
 */
export const ACCOUNT_ME_RESOURCE = "v1/auth/me";

/**
 * Project password ("access_secret", DR-010 D4) — `/v1/projects/:id/...`, deliberately NOT
 * nested under `v1/orgs/:id`, matching the real router (routes.rs: `project_password_routes`).
 */
export function projectPasswordResource(projectId: string): string {
  return `v1/projects/${projectId}/password`;
}

export function projectPasswordVerifyResource(projectId: string): string {
  return `v1/projects/${projectId}/password/verify`;
}

// ---- Superadmin (PRD-23 Wave C) — the platform-wide `/admin` surface. Every resource here
// maps 1:1 onto the real backend path (`v1/admin/*`, `require_superadmin`-gated) — no id
// remapping like `projectRoutesResource` needs, so these stay plain constants/builders that
// fall straight through `createGaussMeridianDataAdapter` with no decorator.

/** `GET /v1/admin/me` — the allowlist self-check `useIsSuperadmin`/`SuperadminGate` probe.
 *  404 (not 403) for a non-allowlisted caller — the surface is never advertised. */
export const ADMIN_ME_RESOURCE = "v1/admin/me";

/** `GET /v1/admin/metrics?months=N` — platform-wide MAU/revenue/margin series. */
export const ADMIN_METRICS_RESOURCE = "v1/admin/metrics";

/** `GET /v1/admin/users?limit&start&q` — paginated global user directory. */
export const ADMIN_USERS_RESOURCE = "v1/admin/users";

/** `GET /v1/admin/deletion-requests?status=` — paginated deletion-request queue/history. */
export const ADMIN_DELETION_REQUESTS_RESOURCE = "v1/admin/deletion-requests";

export function adminDeletionRequestFulfillResource(id: string): string {
  return `v1/admin/deletion-requests/${id}/fulfill`;
}

export function adminDeletionRequestRejectResource(id: string): string {
  return `v1/admin/deletion-requests/${id}/reject`;
}

// ---- Superadmin observability (PRD-24 Wave C) — the revenue/resource console read surfaces.
// Every string maps 1:1 onto the real `v1/admin/*` backend path (`require_superadmin`-gated).
// `window` is an integer month count (3/6/12, default 6 server-side).

/** `GET /v1/admin/overview?window=` — the CEO Business dashboard: current month + windowed series. */
export const ADMIN_OVERVIEW_RESOURCE = "v1/admin/overview";

/** `GET /v1/admin/finance?window=` — the Finance Home: windowed series + cost by model/provider. */
export const ADMIN_FINANCE_RESOURCE = "v1/admin/finance";

/** Redacted operator correlation view for one tenant-owned top-up order. */
export function adminTopUpTimelineResource(orgId: string, orderId: string): string {
  return `v1/admin/finance/topups/${orgId}/${orderId}/timeline`;
}

/** Auditable replay of already-accepted evidence; never accepts a provider fact or balance. */
export function adminTopUpRepairResource(orgId: string, orderId: string): string {
  return `v1/admin/finance/topups/${orgId}/${orderId}/repair`;
}

/** `GET /v1/admin/cost?group_by=&window=&sort=` — the cost pivot (group_by ∈
 *  org|project|user|model|provider|key; sort ∈ desc|asc). */
export const ADMIN_COST_RESOURCE = "v1/admin/cost";

/** `GET /v1/admin/orgs?window=` — the org directory. */
export const ADMIN_ORGS_RESOURCE = "v1/admin/orgs";

/** `GET /v1/admin/projects?window=` — the project directory. */
export const ADMIN_PROJECTS_RESOURCE = "v1/admin/projects";

/** `GET /v1/admin/watchlist?window=` — bleed-ranked orgs + the idle set. */
export const ADMIN_WATCHLIST_RESOURCE = "v1/admin/watchlist";

/** `GET /v1/admin/orgs/:id?window=` — one org summary + its projects. */
export function adminOrgResource(id: string): string {
  return `v1/admin/orgs/${id}`;
}

/** `GET /v1/admin/projects/:id?window=` — one project summary. */
export function adminProjectResource(id: string): string {
  return `v1/admin/projects/${id}`;
}

// ---- Superadmin control (PRD-24 Wave C2) — the Lock/Suspend/Reactivate write surface + the
// dry-run impact preview + the audit trail. Every string maps 1:1 onto the real (Wave B1, LOCKED)
// `v1/admin/*` backend path (`require_superadmin`-gated). The target segment is the plural noun the
// backend nests under (`orgs|projects|users|keys`); `lock` is orgs/projects-only server-side.

/** The plural resource segment a control action nests under — matches the backend's routing. */
export type AdminControlTarget = "orgs" | "projects" | "users" | "keys";
/** The three control verbs. `lock` is org/project-only; users/keys accept only suspend/reactivate. */
export type AdminControlAction = "lock" | "suspend" | "reactivate";

/** `POST /v1/admin/{orgs|projects|users|keys}/:id/{lock|suspend|reactivate}` — query `?minutes=`
 *  (lock only, default 60 server-side) & `?reason=` (optional). One builder for all 12 combos so
 *  the resource string can never drift between the hook, the dialog, and the mock route table. */
export function adminControlResource(
  target: AdminControlTarget,
  id: string,
  action: AdminControlAction,
): string {
  return `v1/admin/${target}/${id}/${action}`;
}

/** `GET /v1/admin/{orgs|projects}/:id/impact?window=` — the dry-run: what a suspend/lock would
 *  block over the last N days (blocked-request count + provider cost + a small sample). Only orgs
 *  and projects expose an impact preview; users/keys don't (there's nothing windowed to preview). */
export function adminImpactResource(target: "orgs" | "projects", id: string): string {
  return `v1/admin/${target}/${id}/impact`;
}

/** `GET /v1/admin/audit?limit=` — the platform-wide control-action trail, newest first. */
export const ADMIN_AUDIT_RESOURCE = "v1/admin/audit";
