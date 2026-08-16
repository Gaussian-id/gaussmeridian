import type { Role } from "@core/adapters/schemas/console.schema";

/**
 * Pure UI-side RBAC gates, derived from the org-level RBAC tiers the Wave-2 backend enforces
 * (this session's ground truth): Owner = all actions including billing and delete/transfer
 * org; Admin = all org-admin actions EXCEPT billing and delete-org; Developer = project-
 * operational only, no org-admin actions at all.
 *
 * These gates exist so the UI doesn't dangle actions the backend will 403 on — `useTenancy()`
 * resolves `role` for the current org, components pass it here before rendering an action. This
 * is a UX nicety, NOT the enforcement boundary: the backend's 403 is still the real gate, and a
 * stale/incorrectly-resolved client-side role must never be treated as authorization proof.
 * `role` is `undefined` while membership hasn't resolved yet (or in global/no-org context) —
 * every gate below treats "unknown role" as "not permitted" (fail closed).
 */

export function canDeleteOrg(role: Role | undefined): boolean {
  return role === "owner";
}

export function canManageBilling(role: Role | undefined): boolean {
  return role === "owner";
}

export function canViewBilling(role: Role | undefined): boolean {
  return role !== undefined;
}

export function canManageMembers(role: Role | undefined): boolean {
  return role === "owner" || role === "admin";
}

export function canManageProjects(role: Role | undefined): boolean {
  return role === "owner" || role === "admin";
}
