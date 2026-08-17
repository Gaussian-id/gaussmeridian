import { z } from "zod";

import { CommerceInvoiceSchema, CommerceMoneySchema } from "./gaussmeridian.schema";

/**
 * PRD-23 Wave B/C — the platform-wide superadmin surface (`/v1/admin/*`). Traced against the
 * real, landed backend handlers (`handlers.rs`, commits c3ea5d4 + d98638c): `get_admin_me`,
 * `get_admin_metrics`, `list_admin_users`, `list_admin_deletion_requests`,
 * `fulfill_deletion_request`, `reject_deletion_request`. Every object schema is `.passthrough()`
 * — additive-friendly, matching `AccountProfileSchema`'s convention — so a future backend field
 * this UI doesn't render yet never breaks parsing.
 */

// ---- GET /v1/admin/me ----
// 200 `{ superadmin: true }` for an allowlisted caller; the real endpoint 404s for everyone
// else (never advertised — `require_superadmin`'s doc comment), which surfaces as a thrown
// adapter error at this seam, not a `{ superadmin: false }` body. `SuperadminGate` /
// `useIsSuperadmin` treat "not superadmin" as "no data OR query error", not a field to read.
export const AdminMeSchema = z.object({ superadmin: z.boolean() }).passthrough();
export type AdminMe = z.infer<typeof AdminMeSchema>;

// ---- GET /v1/admin/metrics?months=N ----
// `MonthMetrics` (gaussmeridian-db) — `margin` can be <= 0 by design (outcome-gate write-offs;
// no billing markup exists yet). Never hide or clamp a negative margin client-side.
export const MonthMetricsSchema = z
  .object({
    month: z.string(),
    mau_api: z.number(),
    mau_console: z.number(),
    revenue: z.number(),
    provider_cost: z.number(),
    margin: z.number(),
  })
  .passthrough();
export type MonthMetrics = z.infer<typeof MonthMetricsSchema>;

export const AdminMetricsResponseSchema = z.object({
  current: MonthMetricsSchema,
  series: z.array(MonthMetricsSchema),
});
export type AdminMetricsResponse = z.infer<typeof AdminMetricsResponseSchema>;

// ---- GET /v1/admin/users?limit&start&q ----

export const AdminUserOrgMembershipSchema = z
  .object({
    org_id: z.string(),
    org_name: z.string(),
    role: z.string(),
  })
  .passthrough();
export type AdminUserOrgMembership = z.infer<typeof AdminUserOrgMembershipSchema>;

export const AdminUserSchema = z
  .object({
    id: z.string(),
    email: z.string(),
    username: z.string(),
    created_at: z.string(),
    active: z.boolean(),
    onboarding_completed: z.boolean(),
    orgs: z.array(AdminUserOrgMembershipSchema),
    /** The user's current PENDING deletion-request status, if any — `null` when there is no
     *  outstanding pending request. A resolved (fulfilled/rejected) request doesn't show up
     *  here; see `AdminDeletionRequestSchema` for history. */
    deletion_status: z.string().nullable(),
    last_active_api: z.string().nullable(),
    last_active_console: z.string().nullable(),
  })
  .passthrough();
export type AdminUser = z.infer<typeof AdminUserSchema>;

export const AdminUsersResponseSchema = z.object({
  users: z.array(AdminUserSchema),
  total: z.number(),
});
export type AdminUsersResponse = z.infer<typeof AdminUsersResponseSchema>;

// ---- GET /v1/admin/deletion-requests?status= ----

export const AdminDeletionRequestSchema = z
  .object({
    id: z.string(),
    user_id: z.string(),
    email: z.string().nullable(),
    username: z.string().nullable(),
    status: z.string(),
    note: z.string().nullable(),
    requested_at: z.string(),
    resolved_at: z.string().nullable(),
    resolved_by: z.string().nullable(),
  })
  .passthrough();
export type AdminDeletionRequest = z.infer<typeof AdminDeletionRequestSchema>;

export const AdminDeletionRequestsResponseSchema = z.object({
  requests: z.array(AdminDeletionRequestSchema),
});
export type AdminDeletionRequestsResponse = z.infer<typeof AdminDeletionRequestsResponseSchema>;

/** Response of `POST /v1/admin/deletion-requests/:id/reject` — a narrower shape than
 *  `AdminDeletionRequestSchema` (no `user_id`/`email`/`username` on the wire for this one). */
export const AdminDeletionRequestResolutionSchema = z
  .object({
    id: z.string(),
    status: z.string(),
    note: z.string().nullable(),
    resolved_at: z.string().nullable(),
    resolved_by: z.string().nullable(),
  })
  .passthrough();
export type AdminDeletionRequestResolution = z.infer<typeof AdminDeletionRequestResolutionSchema>;

/* ------------------------------------------------------------------------------------------------
 * PRD-24 Wave C — the revenue/resource observability surface (`/v1/admin/{overview,finance,cost,
 * orgs,projects,watchlist}`). Wave-A backend shapes, LOCKED. Every object is `.passthrough()` —
 * a future backend field this UI doesn't render yet never breaks parsing. All monetary/rate
 * numbers arrive as plain numbers (recovery_rate/write_off_rate are 0..1 fractions; the UI
 * formats them as percentages). `last_activity`/`last_seen` are ISO strings or null.
 * ---------------------------------------------------------------------------------------------- */

/** One month of the platform business series (`overview`/`finance`). `bleed = written_off +
 *  uncollected`; the backend still sends `bleed` explicitly rather than making the UI recompute
 *  it. `recovery_rate` is a 0..1 fraction of provider cost we actually recovered. */
export const BusinessMonthSchema = z
  .object({
    month: z.string(),
    revenue: z.number(),
    provider_cost: z.number(),
    written_off: z.number(),
    uncollected: z.number(),
    bleed: z.number(),
    recovery_rate: z.number(),
    mau_api: z.number(),
    mau_console: z.number(),
    new_users: z.number(),
    new_orgs: z.number(),
    new_projects: z.number(),
    active_orgs: z.number(),
    active_projects: z.number(),
    requests: z.number(),
    tokens: z.number(),
  })
  .passthrough();
export type BusinessMonth = z.infer<typeof BusinessMonthSchema>;

/** `GET /v1/admin/overview?window=` — the current month (`null` before any activity) plus the
 *  windowed series (tail of N months). */
export const AdminOverviewResponseSchema = z.object({
  current: BusinessMonthSchema.nullable(),
  series: z.array(BusinessMonthSchema),
});
export type AdminOverviewResponse = z.infer<typeof AdminOverviewResponseSchema>;

/** One ranked row of a cost pivot (`cost`/`finance`). `last_seen` null means "never observed". */
export const CostPivotRowSchema = z
  .object({
    key: z.string(),
    label: z.string(),
    cost: z.number(),
    requests: z.number(),
    recovery_rate: z.number(),
    last_seen: z.string().nullable(),
  })
  .passthrough();
export type CostPivotRow = z.infer<typeof CostPivotRowSchema>;

/** `GET /v1/admin/cost?group_by=&window=&sort=` — one pivot dimension, ranked rows. `group_by`
 *  echoes the requested dimension so the client can confirm what it's rendering. */
export const AdminCostResponseSchema = z.object({
  group_by: z.string(),
  rows: z.array(CostPivotRowSchema),
});
export type AdminCostResponse = z.infer<typeof AdminCostResponseSchema>;

/** A row of the org directory (`orgs`) / a bleeder in the watchlist. Numbers everywhere except
 *  the identity/label/status strings; `last_activity` null means "never active". */
export const OrgRowSchema = z
  .object({
    id: z.string(),
    name: z.string(),
    plan: z.string(),
    status: z.string(),
    revenue: z.number(),
    provider_cost: z.number(),
    written_off: z.number(),
    uncollected: z.number(),
    bleed: z.number(),
    write_off_rate: z.number(),
    recovery_rate: z.number(),
    requests: z.number(),
    tokens: z.number(),
    last_activity: z.string().nullable(),
  })
  .passthrough();
export type OrgRow = z.infer<typeof OrgRowSchema>;

/** A row of the project directory (`projects`) — every `OrgRow` field plus the owning org and
 *  the project's live API-key count. */
export const ProjectRowSchema = OrgRowSchema.extend({
  org_id: z.string(),
  org_name: z.string(),
  key_count: z.number(),
}).passthrough();
export type ProjectRow = z.infer<typeof ProjectRowSchema>;

/** A row of the watchlist idle set — an org quiet past the 7-day threshold (or never active). */
export const IdleRowSchema = z
  .object({
    id: z.string(),
    name: z.string(),
    plan: z.string(),
    status: z.string(),
    last_activity: z.string().nullable(),
  })
  .passthrough();
export type IdleRow = z.infer<typeof IdleRowSchema>;

/** `GET /v1/admin/orgs?window=` — the full org directory. */
export const AdminOrgsResponseSchema = z.object({ orgs: z.array(OrgRowSchema) });
export type AdminOrgsResponse = z.infer<typeof AdminOrgsResponseSchema>;

/** `GET /v1/admin/orgs/:id?window=` — one org summary plus its projects. */
export const AdminOrgDetailResponseSchema = z.object({
  org: OrgRowSchema,
  projects: z.array(ProjectRowSchema),
});
export type AdminOrgDetailResponse = z.infer<typeof AdminOrgDetailResponseSchema>;

/** `GET /v1/admin/projects?window=` — the full project directory. */
export const AdminProjectsResponseSchema = z.object({ projects: z.array(ProjectRowSchema) });
export type AdminProjectsResponse = z.infer<typeof AdminProjectsResponseSchema>;

/** `GET /v1/admin/projects/:id?window=` — one project summary. */
export const AdminProjectDetailResponseSchema = z.object({ project: ProjectRowSchema });
export type AdminProjectDetailResponse = z.infer<typeof AdminProjectDetailResponseSchema>;

/** `GET /v1/admin/watchlist?window=` — the bleed-ranked orgs plus the idle set. */
export const AdminWatchlistResponseSchema = z.object({
  bleeders: z.array(OrgRowSchema),
  idle: z.array(IdleRowSchema),
});
export type AdminWatchlistResponse = z.infer<typeof AdminWatchlistResponseSchema>;

/* ------------------------------------------------------------------------------------------------
 * PRD-24 Wave C2 — the control surface (`POST /v1/admin/{orgs|projects|users|keys}/:id/{lock|
 * suspend|reactivate}`, `GET …/:id/impact`, `GET /v1/admin/audit`). Wave-B1 backend shapes, LOCKED.
 * `.passthrough()` throughout, matching the read schemas above. Monetary values are plain numbers.
 * ---------------------------------------------------------------------------------------------- */

/** Response of every control action (`lock`/`suspend`/`reactivate` on any target). `status` is the
 *  target's new lifecycle state (`active|locked|suspended`); `locked_until` is the auto-expiry ISO
 *  timestamp for a `lock` (null for suspend/reactivate, and for a lock the backend never expires). */
export const AdminControlResponseSchema = z
  .object({
    target_type: z.string(),
    target_id: z.string(),
    action: z.string(),
    status: z.string(),
    locked_until: z.string().nullable(),
  })
  .passthrough();
export type AdminControlResponse = z.infer<typeof AdminControlResponseSchema>;

/** One row of a dry-run's sample — a request that WOULD have been blocked, for the operator to see
 *  what the action stops. Cost is the provider cost of that single sampled request. */
export const AdminImpactSampleSchema = z
  .object({
    model: z.string(),
    provider: z.string(),
    cost: z.number(),
    created_at: z.string(),
  })
  .passthrough();
export type AdminImpactSample = z.infer<typeof AdminImpactSampleSchema>;

/** `GET /v1/admin/{orgs|projects}/:id/impact?window=` — the dry-run preview. `window` echoes the
 *  requested day-count so the client can confirm what it's previewing; `requests`/`provider_cost`
 *  are the totals that a suspend/lock would block over that window; `sample` is a small slice. */
export const AdminImpactResponseSchema = z
  .object({
    target_type: z.string(),
    target_id: z.string(),
    window: z.number(),
    requests: z.number(),
    provider_cost: z.number(),
    sample: z.array(AdminImpactSampleSchema),
  })
  .passthrough();
export type AdminImpactResponse = z.infer<typeof AdminImpactResponseSchema>;

/** One audited control action (`admin_action` table). `reason` is the optional operator note. */
export const AdminAuditEntrySchema = z
  .object({
    id: z.string(),
    actor_email: z.string(),
    action: z.string(),
    target_type: z.string(),
    target_id: z.string(),
    reason: z.string().nullable(),
    created_at: z.string(),
  })
  .passthrough();
export type AdminAuditEntry = z.infer<typeof AdminAuditEntrySchema>;

/** `GET /v1/admin/audit?limit=` — the control-action trail, newest first. */
export const AdminAuditResponseSchema = z.object({ actions: z.array(AdminAuditEntrySchema) });
export type AdminAuditResponse = z.infer<typeof AdminAuditResponseSchema>;
