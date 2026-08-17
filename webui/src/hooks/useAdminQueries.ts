"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";

import { useDataQuery } from "@core/adapters";
import {
  AdminAuditResponseSchema,
  AdminControlResponseSchema,
  AdminCostResponseSchema,
  AdminDeletionRequestResolutionSchema,
  AdminDeletionRequestsResponseSchema,
  AdminImpactResponseSchema,
  AdminMeSchema,
  AdminMetricsResponseSchema,
  AdminOrgDetailResponseSchema,
  AdminOrgsResponseSchema,
  AdminOverviewResponseSchema,
  AdminProjectDetailResponseSchema,
  AdminProjectsResponseSchema,
  AdminUsersResponseSchema,
  AdminWatchlistResponseSchema,
} from "@core/adapters/schemas/admin.schema";
import {
  ADMIN_AUDIT_RESOURCE,
  ADMIN_COST_RESOURCE,
  ADMIN_DELETION_REQUESTS_RESOURCE,
  ADMIN_ME_RESOURCE,
  ADMIN_METRICS_RESOURCE,
  ADMIN_ORGS_RESOURCE,
  ADMIN_OVERVIEW_RESOURCE,
  ADMIN_PROJECTS_RESOURCE,
  ADMIN_USERS_RESOURCE,
  ADMIN_WATCHLIST_RESOURCE,
  adminControlResource,
  adminDeletionRequestFulfillResource,
  adminDeletionRequestRejectResource,
  adminImpactResource,
  adminOrgResource,
  adminProjectResource,
  type AdminControlAction,
  type AdminControlTarget,
} from "@core/config/resources";

import { useResourceQuery } from "./useResourceQuery";

/**
 * `GET /v1/admin/me` — the allowlist self-check. A non-allowlisted caller gets a 404, which
 * surfaces here as a TanStack Query error state (`isError: true`, `data: undefined`) — never a
 * `{ superadmin: false }` body (the real endpoint never returns that shape; see
 * `AdminMeSchema`'s doc comment). `SuperadminGate` and `useIsSuperadmin` both read this same
 * cached query (identical `queryKey`), so the probe only ever fires once per `staleTime` window
 * no matter how many surfaces (gate, sidebar, account menu) ask.
 */
export function useAdminMe() {
  return useResourceQuery({ resource: ADMIN_ME_RESOURCE, schema: AdminMeSchema });
}

/** Whether the signed-in caller is an allowlisted superadmin. `false` while loading, on error
 *  (the 404 "not allowlisted" case), or once resolved `false` — callers never need to
 *  distinguish those three; all of them mean "don't show the admin surface". */
export function useIsSuperadmin(): boolean {
  const me = useAdminMe();
  return me.data?.superadmin === true;
}

/** `GET /v1/admin/metrics?months=N`. `months` is left undefined to take the backend's own
 *  default (6, clamped [1, 24]) rather than duplicating that default client-side. */
export function useAdminMetrics(months?: number) {
  return useResourceQuery({
    resource: ADMIN_METRICS_RESOURCE,
    params: months !== undefined ? { months } : undefined,
    schema: AdminMetricsResponseSchema,
  });
}

/** `GET /v1/admin/users?limit&start&q` — one server-side window of the global user directory.
 *  The caller (`/admin/users`) owns paging by varying `start`; this hook does no client-side
 *  accumulation across windows. */
export function useAdminUsers(params: { limit?: number; start?: number; q?: string }) {
  return useResourceQuery({
    resource: ADMIN_USERS_RESOURCE,
    params: { limit: params.limit, start: params.start, q: params.q || undefined },
    schema: AdminUsersResponseSchema,
  });
}

/** `GET /v1/admin/deletion-requests?status=`. `status` omitted returns every request
 *  regardless of status (the real backend's `repo.list(None, ...)` behavior). */
export function useAdminDeletionRequests(status?: string) {
  return useResourceQuery({
    resource: ADMIN_DELETION_REQUESTS_RESOURCE,
    params: status ? { status } : undefined,
    schema: AdminDeletionRequestsResponseSchema,
  });
}

/**
 * `POST /v1/admin/deletion-requests/:id/fulfill` — 204 No Content on success (hence
 * `z.unknown()`; there is no body to validate), 409 if the request is no longer pending
 * (already resolved by a concurrent admin action). Invalidates both the deletion-request queue
 * and the user directory — a fulfilled request changes a user's `deletion_status` there too.
 */
export function useFulfillDeletionRequest() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) =>
      data.query({
        resource: adminDeletionRequestFulfillResource(id),
        method: "POST",
        schema: z.unknown(),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ADMIN_DELETION_REQUESTS_RESOURCE] });
      queryClient.invalidateQueries({ queryKey: [ADMIN_USERS_RESOURCE] });
    },
  });
}

/** `POST /v1/admin/deletion-requests/:id/reject { note }` — 409 if the request is no longer
 *  pending. Same invalidation as fulfill above. */
export function useRejectDeletionRequest() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, note }: { id: string; note?: string }) =>
      data.query({
        resource: adminDeletionRequestRejectResource(id),
        method: "POST",
        body: { note },
        schema: AdminDeletionRequestResolutionSchema,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ADMIN_DELETION_REQUESTS_RESOURCE] });
      queryClient.invalidateQueries({ queryKey: [ADMIN_USERS_RESOURCE] });
    },
  });
}

/* ------------------------------------------------------------------------------------------------
 * PRD-24 Wave C — the revenue/resource observability surfaces. `window` is an integer month count
 * (3/6/12); left undefined here, each hook takes the backend's own default (6) rather than
 * duplicating it client-side. Every hook is a plain read through `useResourceQuery` (the mock and
 * real adapters both resolve the resource string to `/api/gaussmeridian/v1/admin/*`).
 * ---------------------------------------------------------------------------------------------- */

/** A cost-pivot dimension and sort direction — the `/cost` query parameters, LOCKED by Wave A. */
export type CostGroupBy = "org" | "project" | "user" | "model" | "provider" | "key";
export type CostSort = "desc" | "asc";

/** `GET /v1/admin/overview?window=` — the CEO Business dashboard series + current month. */
export function useAdminOverview(window?: number) {
  return useResourceQuery({
    resource: ADMIN_OVERVIEW_RESOURCE,
    params: window !== undefined ? { window } : undefined,
    schema: AdminOverviewResponseSchema,
  });
}


/** `GET /v1/admin/cost?group_by=&window=&sort=` — the Finance cost pivot. Changing `groupBy` or
 *  `sort` varies the TanStack Query key (via `params`), so a dimension change refetches and
 *  re-ranks rather than mutating a shared cache entry. */
export function useAdminCost(params: { groupBy: CostGroupBy; sort?: CostSort; window?: number }) {
  return useResourceQuery({
    resource: ADMIN_COST_RESOURCE,
    params: {
      group_by: params.groupBy,
      sort: params.sort,
      window: params.window,
    },
    schema: AdminCostResponseSchema,
  });
}

/** `GET /v1/admin/orgs?window=` — the org directory. */
export function useAdminOrgs(window?: number) {
  return useResourceQuery({
    resource: ADMIN_ORGS_RESOURCE,
    params: window !== undefined ? { window } : undefined,
    schema: AdminOrgsResponseSchema,
  });
}

/** `GET /v1/admin/orgs/:id?window=` — one org summary + its projects. */
export function useAdminOrgDetail(id: string, window?: number) {
  return useResourceQuery({
    resource: adminOrgResource(id),
    params: window !== undefined ? { window } : undefined,
    schema: AdminOrgDetailResponseSchema,
  });
}

/** `GET /v1/admin/projects?window=` — the project directory. */
export function useAdminProjects(window?: number) {
  return useResourceQuery({
    resource: ADMIN_PROJECTS_RESOURCE,
    params: window !== undefined ? { window } : undefined,
    schema: AdminProjectsResponseSchema,
  });
}

/** `GET /v1/admin/projects/:id?window=` — one project summary. */
export function useAdminProjectDetail(id: string, window?: number) {
  return useResourceQuery({
    resource: adminProjectResource(id),
    params: window !== undefined ? { window } : undefined,
    schema: AdminProjectDetailResponseSchema,
  });
}

/** `GET /v1/admin/watchlist?window=` — bleed-ranked orgs + the idle set. */
export function useAdminWatchlist(window?: number) {
  return useResourceQuery({
    resource: ADMIN_WATCHLIST_RESOURCE,
    params: window !== undefined ? { window } : undefined,
    schema: AdminWatchlistResponseSchema,
  });
}

/* ------------------------------------------------------------------------------------------------
 * PRD-24 Wave C2 — the control surface. One parametrized mutation hook (`useResourceControl`)
 * backs every Lock/Suspend/Reactivate across all four target types instead of 12 near-identical
 * hooks; the named one-liner aliases below exist for call-site clarity. Each success invalidates
 * every read query a control action can move (the two directories, the watchlist, the user
 * directory, the touched detail page, and the audit trail) so the UI reflects the flip on refetch.
 * ---------------------------------------------------------------------------------------------- */

/** Variables for a control mutation. `minutes` applies only to `lock` (auto-expiry window, default
 *  60 server-side); `reason` is the optional audited operator note carried on every action. */
export interface ResourceControlVars {
  id: string;
  minutes?: number;
  reason?: string;
}

/**
 * `POST /v1/admin/{target}/:id/{action}` — the workhorse control mutation, parametrized by target
 * type and action so one hook serves all combinations. `lock` sends `?minutes=`; `?reason=` rides
 * along on every action when provided. On success it broadly invalidates the admin read surfaces —
 * a status flip can change directory rankings, the watchlist, a user's row, the open detail page,
 * and always appends an audit row — so every one of them refetches.
 */
export function useResourceControl(target: AdminControlTarget, action: AdminControlAction) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, minutes, reason }: ResourceControlVars) =>
      data.query({
        resource: adminControlResource(target, id, action),
        method: "POST",
        params: {
          minutes: action === "lock" ? (minutes ?? undefined) : undefined,
          reason: reason || undefined,
        },
        schema: AdminControlResponseSchema,
      }),
    onSuccess: (_result, variables) => {
      for (const resource of [
        ADMIN_ORGS_RESOURCE,
        ADMIN_PROJECTS_RESOURCE,
        ADMIN_WATCHLIST_RESOURCE,
        ADMIN_USERS_RESOURCE,
        ADMIN_AUDIT_RESOURCE,
      ]) {
        queryClient.invalidateQueries({ queryKey: [resource] });
      }
      // The open detail page keys off the id-specific resource string (a different first key
      // element than the directory), so invalidate it explicitly. Both are harmless no-ops when
      // the touched entity is the other type.
      queryClient.invalidateQueries({ queryKey: [adminOrgResource(variables.id)] });
      queryClient.invalidateQueries({ queryKey: [adminProjectResource(variables.id)] });
    },
  });
}

// Named aliases — thin wrappers over `useResourceControl` for readable call sites. `lock` is
// org/project-only; users/keys expose only suspend/reactivate (no lock endpoint server-side).
export const useLockOrg = () => useResourceControl("orgs", "lock");
export const useSuspendOrg = () => useResourceControl("orgs", "suspend");
export const useReactivateOrg = () => useResourceControl("orgs", "reactivate");
export const useLockProject = () => useResourceControl("projects", "lock");
export const useSuspendProject = () => useResourceControl("projects", "suspend");
export const useReactivateProject = () => useResourceControl("projects", "reactivate");
export const useSuspendUser = () => useResourceControl("users", "suspend");
export const useReactivateUser = () => useResourceControl("users", "reactivate");
export const useSuspendKey = () => useResourceControl("keys", "suspend");
export const useReactivateKey = () => useResourceControl("keys", "reactivate");

/**
 * `GET /v1/admin/{orgs|projects}/:id/impact?window=` — the dry-run preview a control dialog runs
 * before a suspend/lock, so the operator sees exactly what would be blocked. `enabled` gates the
 * fetch (the caller passes `open` so the dry-run only fires while the dialog is mounted/open).
 * `window` is an optional day-count; left undefined, the backend applies its own default.
 */
export function useResourceImpact(
  target: "orgs" | "projects",
  id: string,
  options?: { window?: number; enabled?: boolean },
) {
  return useResourceQuery({
    resource: adminImpactResource(target, id),
    params: options?.window !== undefined ? { window: options.window } : undefined,
    schema: AdminImpactResponseSchema,
    enabled: options?.enabled,
  });
}

/** `GET /v1/admin/audit?limit=` — the control-action trail, newest first. `limit` left undefined
 *  takes the backend's own default page size. */
export function useAdminAudit(limit?: number) {
  return useResourceQuery({
    resource: ADMIN_AUDIT_RESOURCE,
    params: limit !== undefined ? { limit } : undefined,
    schema: AdminAuditResponseSchema,
  });
}
