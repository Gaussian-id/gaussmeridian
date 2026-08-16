import { isValidSlug, slugify } from "@/utils/slug";

import type {
  MemberResponse,
  OrgProjectResponse,
  OrgResponse,
  RoleResponse,
} from "./schemas/console-real.schema";
import type {
  Member,
  MemberStatus,
  Org,
  PermissionMatrix,
  Project,
  Role,
} from "./schemas/console.schema";

/**
 * Anti-corruption layer: real Wave-2 backend DTOs (`console-real.schema.ts`) -> the Phase-1
 * console's stable UI contract (`console.schema.ts`). Every remaining mapping decision here is a
 * DELIBERATE degrade-and-flag choice, documented inline, because a couple of real backend DTOs
 * are still narrower than what `console.schema.ts` (originally invented for the Phase-1 mock
 * console) asks for. Nothing outside this file should know the real DTO shapes exist —
 * `useConsoleQueries` and every component under `src/components/orgs/**` keep consuming
 * `Org`/`Project`/`Member`/`Role` exactly as before.
 */

const ROLE_NAMES: readonly Role[] = ["owner", "admin", "developer"];

export { slugify, isValidSlug };

/** Case-insensitive match of a real `RoleResponse.name` against the fixed 3-tier UI enum. */
export function roleNameToTier(name: string): Role | null {
  const normalized = name.trim().toLowerCase();
  return ROLE_NAMES.find((tier) => tier === normalized) ?? null;
}

/**
 * Truncates a user id into a display-safe placeholder for `Member.display_name`/`email` when
 * the backend's (now real) identity fields come back null. Intentionally NOT a fabricated email
 * address — never invent a `@`-shaped string that looks like real contact info.
 */
export function truncateUserId(userId: string): string {
  return userId.length <= 10 ? userId : `${userId.slice(0, 8)}…`;
}

/**
 * Maps one `OrgResponse` to the UI `Org` shape. `member_count`/`project_count` are now real
 * backend fields (Wave-2 gap-closure, backend commit `8fb7646`) — the org list no longer
 * degrades to 0/0 and single-org reads no longer need the two-extra-fetch N+1 workaround.
 */
export function mapOrgResponse(dto: OrgResponse): Org {
  return {
    id: dto.id,
    name: dto.name,
    slug: dto.slug,
    plan: dto.plan,
    created_at: dto.created_at,
    member_count: dto.member_count,
    project_count: dto.project_count,
  };
}

/**
 * Maps one `OrgProjectResponse` to the UI `Project` shape.
 *
 * `created_at` is now a real backend field (Wave-2 gap-closure, backend commit `8fb7646`) — no
 * more synthesized "now" timestamp.
 *
 * Still-open FLAGGED BACKEND GAPS (both fields remain UI-only concepts the real project model
 * doesn't have):
 *  - `slug`: `OrgProjectResponse` has no slug field at all. Degraded to a display-only slug
 *    derived from `name` (lowercase, non-alnum -> `-`) — this is NOT a stable identifier and
 *    must never be used to address the project (the adapter always uses `id` for that).
 *    Recommend the backend add a persisted `slug` if the UI's slug display needs to stay real.
 *  - `environment`: `OrgProjectResponse` has no production/development concept server-side.
 *    Degraded to a fixed `"production"` — recommend the backend either add an `environment`
 *    enum or the console retire the distinction once Wave-2 is the source of truth.
 */
export function mapOrgProjectResponse(dto: OrgProjectResponse): Project {
  return {
    id: dto.id,
    org_id: dto.org_id,
    name: dto.name,
    slug: slugify(dto.name),
    environment: "production",
    created_at: dto.created_at,
  };
}

/**
 * Maps one `MemberResponse` to the UI `Member` shape, given the member's resolved role name
 * (looked up by `role_id` against `GET /v1/roles` — see `console-org.adapter.ts`).
 *
 * `email`/`display_name` are now real backend fields (Wave-2 gap-closure, backend commit
 * `8fb7646`). They are modeled nullable on the wire, so this still falls back to a truncated
 * user id (`truncateUserId`) for the rare member whose identity fields come back null, rather
 * than fabricating contact info.
 *
 * Still-open FLAGGED BEHAVIOR:
 *  - `invited_at`/`joined_at`: `MemberResponse` carries a single `created_at`, not the two
 *    lifecycle timestamps `MemberSchema` distinguishes. Best-effort inference: an `"invited"`
 *    member has `invited_at = created_at`, `joined_at = null`; an `"active"` member is treated
 *    as already through both stages, so both timestamps are set to `created_at`. This mirrors
 *    `MemberSchema`'s documented invariant ("joined -> both set") without inventing a second,
 *    unknowable timestamp.
 *  - role resolution: if the member's `role_id` doesn't resolve to a known role (deleted role,
 *    seed drift), `roleName` is undefined and this throws rather than silently mislabeling an
 *    RBAC-relevant field — a wrong committed role is a security-relevant misrepresentation, not
 *    a display nicety.
 */
export function mapMemberResponse(dto: MemberResponse, roleName: string | undefined): Member {
  const tier = roleName ? roleNameToTier(roleName) : null;
  if (!tier) {
    throw new Error(
      `console.mapper: member ${dto.id} has role_id ${dto.role_id} which did not resolve to a known role name — refusing to guess an RBAC tier`,
    );
  }
  const placeholder = truncateUserId(dto.user_id);
  const isInvited: MemberStatus = dto.status === "invited" ? "invited" : "active";
  return {
    id: dto.id,
    org_id: dto.org_id,
    user_id: dto.user_id,
    email: dto.email ?? placeholder,
    display_name: dto.display_name ?? placeholder,
    role: tier,
    status: isInvited,
    invited_at: dto.created_at,
    joined_at: isInvited === "invited" ? null : dto.created_at,
  };
}

// ---- Permission matrix ----

const RESOURCE_LABELS: Record<string, string> = {
  org: "Organization",
  members: "Members",
  billing: "Billing",
  projects: "Projects",
  keys: "API Keys",
  byok: "BYOK",
  playground: "Playground",
  analytics: "Analytics",
};

const ACTION_LABELS: Record<string, string> = {
  read: "View",
  manage: "Manage",
  write: "Write",
  use: "Use",
};

function titleCase(word: string): string {
  return word.length === 0 ? word : word.charAt(0).toUpperCase() + word.slice(1);
}

function humanizeResource(resource: string): string {
  return RESOURCE_LABELS[resource] ?? titleCase(resource);
}

/** "org.read" -> "View Organization", "keys.write" -> "Write API Keys", etc. */
function humanizePermissionLabel(permission: string): string {
  const [resource, action] = permission.split(".");
  if (!action) return humanizeResource(resource);
  const actionLabel = ACTION_LABELS[action] ?? titleCase(action);
  return `${actionLabel} ${humanizeResource(resource)}`;
}

/**
 * The real `resource.action` permission vocabulary confirmed for this session (ground truth
 * handed down alongside the Wave-2 backend commit `8fb7646`). Seeding the catalog with this
 * known list — rather than deriving it purely from the union of concrete strings actually
 * returned — matters because an owner-only permission (`billing.manage`) may never appear as a
 * concrete string on ANY role: Owner grants it via the literal `"*"`, and no other tier lists
 * it explicitly. A pure union would silently drop that row. Any additional permission string
 * the backend returns beyond this list is still picked up (see `buildPermissionMatrix`) so the
 * matrix stays forward-compatible with new grants without a code change.
 */
const KNOWN_PERMISSIONS: readonly string[] = [
  "org.read",
  "members.manage",
  "billing.manage",
  "projects.manage",
  "keys.write",
  "byok.write",
  "playground.use",
  "analytics.read",
];

/**
 * Builds the `PermissionMatrix` the `PermissionMatrix` component renders, from the real
 * `GET /v1/roles` response's `permissions: string[]` (Wave-2 gap-closure, backend commit
 * `8fb7646`) — the real `resource.action` vocabulary, no longer a hand-maintained static
 * approximation of WHO holds WHAT.
 *
 * The permission catalog (rows) is `KNOWN_PERMISSIONS` plus any additional concrete permission
 * string actually seen across recognized-tier roles, grouped by the `resource` segment before
 * the first `.`. A role whose `permissions` contains the literal `"*"` (Owner, per this
 * session's ground truth) is treated as holding every permission in the catalog rather than a
 * literal, meaningless `"*"` grant.
 *
 * Only roles whose `name` resolves to a known tier are represented — an unrecognized role name
 * is dropped (with the loss visible as a missing column) rather than silently defaulting it to
 * the least-privileged tier or throwing and breaking the whole page.
 */
export function buildPermissionMatrix(roles: RoleResponse[]): PermissionMatrix {
  const tierRoles = roles
    .map((role) => ({ tier: roleNameToTier(role.name), role }))
    .filter((entry): entry is { tier: Role; role: RoleResponse } => entry.tier !== null);

  const catalogKeys = new Set<string>(KNOWN_PERMISSIONS);
  for (const { role } of tierRoles) {
    for (const permission of role.permissions) {
      if (permission !== "*") catalogKeys.add(permission);
    }
  }

  const permissions = [...catalogKeys].sort().map((key) => ({
    key,
    label: humanizePermissionLabel(key),
    group: humanizeResource(key.split(".")[0]),
  }));

  // De-dupe in case the backend seeds more than one role row per tier name — first-seen wins.
  const seenTiers = new Set<Role>();
  const rolesOut: PermissionMatrix["roles"] = [];
  for (const { tier, role } of tierRoles) {
    if (seenTiers.has(tier)) continue;
    seenTiers.add(tier);
    const hasWildcard = role.permissions.includes("*");
    const grants = hasWildcard
      ? permissions.map((permission) => permission.key)
      : role.permissions.filter((permission) => catalogKeys.has(permission));
    rolesOut.push({ role: tier, grants });
  }
  rolesOut.sort((a, b) => ROLE_NAMES.indexOf(a.role) - ROLE_NAMES.indexOf(b.role));

  return { permissions, roles: rolesOut };
}
