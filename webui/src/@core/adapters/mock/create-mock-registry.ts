import { GaussMeridianAdapterError } from "../gaussmeridian-data.adapter";

import {
  adminAuditSeed,
  adminDeletionRequests as adminDeletionRequestFixtures,
  adminMetricsSeries,
  adminUsers as adminUserFixtures,
  apiKeys as apiKeyFixtures,
  balance,
  budget,
  businessMonths,
  byokProviders as byokProviderFixtures,
  costPivots,
  logs,
  members as memberFixtures,
  mockUser,
  moaCandidates,
  modelDetails,
  modelPricing,
  models,
  orgRows,
  orgs as orgFixtures,
  permissionMatrix,
  projectRows,
  projects as projectFixtures,
  projectSettings,
  routeDecisions,
  savings,
  usage,
  watchlistIdle,
} from "./fixtures";

import type { Member, Org, Project } from "../schemas/console.schema";
import type { ApiKeySchema } from "../schemas/gaussmeridian.schema";
import type { AdapterRegistry, AuthAdapter, AuthSession, LlmByokAdapter } from "../types";
import type {
  AdminAuditEntryFixture,
  AdminDeletionRequestFixture,
  AdminUserFixture,
  MockUser,
  OrgRowFixture,
  ProjectRowFixture,
} from "./fixtures";
import type { z } from "zod";

type ApiKey = z.infer<typeof ApiKeySchema>;

/**
 * Everything the mock keeps in memory for one app session: a deep clone of the tenancy
 * fixtures (orgs/projects/members), mutated in place by POST/PATCH/DELETE so a create-org
 * or invite reflects immediately on the next read. Cloned fresh per `createMockRegistry()`
 * call — `AdapterProvider` builds exactly one registry per app lifetime (`useMemo`), so this
 * behaves like a real in-memory backend for the running app while keeping every
 * `createMockRegistry()` call (each test, for instance) isolated from every other.
 */
interface MockStore {
  orgs: Org[];
  projects: Project[];
  members: Member[];
  /** Project-scoped credentials — cloned once per store so M4's create/revoke/register/delete
   *  flows mutate an in-memory copy instead of the shared fixture module. */
  apiKeys: ApiKey[];
  byokProviders: string[];
  /** The single signed-in identity's mutable record — cloned from `mockUser` so `PATCH
   *  v1/onboarding/profile` (also reused post-onboarding by the /account/me page) and `GET
   *  v1/auth/me` read/write the same in-memory state instead of the frozen fixture module.
   *  `user.deletion_requested` is the ONE source of truth for the account danger zone's pending
   *  state — set/cleared by the auth adapter's `requestAccountDeletion`/`cancelAccountDeletion`
   *  below, mirroring the real backend recomputing `PublicUser.deletion_requested` from whether
   *  a pending `deletion_requests` row exists. */
  user: MockUser;
  /** PRD-23 Wave C — the global user directory (`GET /v1/admin/users`), cloned from
   *  `admin-users.ts`. `deletion_status` is NOT stored here; it's computed at read time from
   *  `deletionRequests`, mirroring the real backend's per-row `pending_for_user` lookup. */
  adminUsers: AdminUserFixture[];
  /** PRD-23 Wave C — the account-deletion request queue/history (`GET /v1/admin/deletion-
   *  requests`), cloned from `admin-deletion-requests.ts`. The `/account/me` danger zone's
   *  `requestAccountDeletion()` pushes a new pending row here for `store.user`; `/admin/
   *  deletions`' fulfill/reject mutate a row's status and, when it belongs to `store.user`,
   *  clear `store.user.deletion_requested` too — this is the "both sides" link the demo needs. */
  deletionRequests: AdminDeletionRequestFixture[];
  /** PRD-24 Wave C2 — the org/project directory rows the observability reads AND the control
   *  writes share, cloned from `admin-observability.ts` so a Lock/Suspend/Reactivate flips a
   *  status that the directory, detail, and watchlist reads then reflect on their next fetch
   *  (unlike the frozen `businessMonths`/`costPivots`, which control actions never change). */
  adminOrgRows: OrgRowFixture[];
  adminProjectRows: ProjectRowFixture[];
  /** PRD-24 Wave C2 — the control-action trail (`GET /v1/admin/audit`), seeded from
   *  `adminAuditSeed` and grown by one row on every control action so the Audit surface demos a
   *  live, appending log under `NEXT_PUBLIC_USE_MOCKS`. */
  adminAudit: AdminAuditEntryFixture[];
}

function cloneFixtures(): MockStore {
  return {
    orgs: JSON.parse(JSON.stringify(orgFixtures)) as Org[],
    projects: JSON.parse(JSON.stringify(projectFixtures)) as Project[],
    members: JSON.parse(JSON.stringify(memberFixtures)) as Member[],
    apiKeys: JSON.parse(JSON.stringify(apiKeyFixtures)) as ApiKey[],
    byokProviders: JSON.parse(JSON.stringify(byokProviderFixtures.providers)) as string[],
    user: JSON.parse(JSON.stringify(mockUser)) as MockUser,
    adminUsers: JSON.parse(JSON.stringify(adminUserFixtures)) as AdminUserFixture[],
    deletionRequests: JSON.parse(
      JSON.stringify(adminDeletionRequestFixtures),
    ) as AdminDeletionRequestFixture[],
    adminOrgRows: JSON.parse(JSON.stringify(orgRows)) as OrgRowFixture[],
    adminProjectRows: JSON.parse(JSON.stringify(projectRows)) as ProjectRowFixture[],
    adminAudit: JSON.parse(JSON.stringify(adminAuditSeed)) as AdminAuditEntryFixture[],
  };
}

function nextId(prefix: string): string {
  return `${prefix}_${Math.random().toString(36).slice(2, 10)}`;
}

type Method = "GET" | "POST" | "PATCH" | "DELETE";

interface RouteContext {
  method: Method;
  body: unknown;
  params?: Record<string, string | number | boolean | undefined>;
  match: RegExpMatchArray;
  store: MockStore;
}

interface MockRoute {
  pattern: RegExp;
  handle: (ctx: RouteContext) => unknown;
}

/**
 * The mock's route table: a regex on `resource` picks the handler, mirroring how the real
 * adapter maps a resource string onto an HTTP path. GET reads; POST/PATCH/DELETE mutate
 * `store` in place. Every handler's return value still goes through the caller's Zod
 * `schema.parse` in `createMockRegistry` below — the mock never bypasses boundary
 * validation, so a fixture that drifted from its schema fails exactly like a bad response
 * from a real backend would.
 */
function buildRoutes(): MockRoute[] {
  return [
    // ---- Console: Orgs ----
    {
      pattern: /^v1\/orgs$/,
      handle: ({ method, body, store }) => {
        if (method === "POST") {
          const input = body as { name: string; slug?: string; plan?: Org["plan"] };
          const org: Org = {
            id: nextId("org"),
            name: input.name,
            slug: input.slug ?? input.name.toLowerCase().replace(/\s+/g, "-"),
            plan: input.plan ?? "free",
            created_at: new Date().toISOString(),
            member_count: 1, // the creating owner
            project_count: 0, // born empty — no default project
          };
          store.orgs.push(org);
          return org;
        }
        return { orgs: store.orgs };
      },
    },
    {
      pattern: /^v1\/orgs\/([^/]+)$/,
      handle: ({ method, match, store }) => {
        const orgId = match[1];
        if (method === "DELETE") {
          store.orgs = store.orgs.filter((org) => org.id !== orgId);
          store.projects = store.projects.filter((project) => project.org_id !== orgId);
          store.members = store.members.filter((member) => member.org_id !== orgId);
          // `ApiKeySchema.tenant_id` is the owning org's id (see mock/fixtures/api-keys.ts) —
          // cascading here matches the danger-zone copy's "ALL API keys under them" claim.
          store.apiKeys = store.apiKeys.filter((key) => key.tenant_id !== orgId);
          return { deleted: true, org_id: orgId };
        }
        const org = store.orgs.find((candidate) => candidate.id === orgId);
        if (!org) throw new Error(`mock: org not found: ${orgId}`);
        return org;
      },
    },
    // ---- Console: Projects ----
    {
      pattern: /^v1\/orgs\/([^/]+)\/projects$/,
      handle: ({ method, match, body, store }) => {
        const orgId = match[1];
        if (method === "POST") {
          const input = body as {
            name: string;
            slug?: string;
            environment?: Project["environment"];
          };
          const project: Project = {
            id: nextId("proj"),
            org_id: orgId,
            name: input.name,
            slug: input.slug ?? input.name.toLowerCase().replace(/\s+/g, "-"),
            environment: input.environment ?? "development",
            created_at: new Date().toISOString(),
          };
          store.projects.push(project);
          const org = store.orgs.find((candidate) => candidate.id === orgId);
          if (org) org.project_count += 1;
          return project;
        }
        return { projects: store.projects.filter((project) => project.org_id === orgId) };
      },
    },
    {
      pattern: /^v1\/orgs\/([^/]+)\/projects\/([^/]+)$/,
      handle: ({ method, match, store }) => {
        const [, orgId, projectId] = match;
        if (method === "DELETE") {
          // Unlike the org-level DELETE handler above, this does NOT cascade to `apiKeys`:
          // `ApiKeySchema` carries `tenant_id` (the owning ORG) but no `project_id` at all —
          // API keys aren't project-scoped in this data model, so there is nothing correct to
          // filter on here. This mirrors the real backend too (`handlers.rs::delete_project`
          // is a single-row `project_repo.delete`, no key cascade) — see `project-danger-zone.tsx`'s
          // doc comment for the resulting consequences-copy decision.
          store.projects = store.projects.filter(
            (project) => !(project.org_id === orgId && project.id === projectId),
          );
          const org = store.orgs.find((candidate) => candidate.id === orgId);
          if (org) org.project_count = Math.max(0, org.project_count - 1);
          return { deleted: true, project_id: projectId };
        }
        const project = store.projects.find(
          (candidate) => candidate.org_id === orgId && candidate.id === projectId,
        );
        if (!project) throw new Error(`mock: project not found: ${projectId}`);
        return project;
      },
    },
    // ---- Console: Members ----
    {
      pattern: /^v1\/orgs\/([^/]+)\/members$/,
      handle: ({ method, match, body, store }) => {
        const orgId = match[1];
        if (method === "POST") {
          const input = body as { email: string; role: Member["role"] };
          const member: Member = {
            id: nextId("mem"),
            org_id: orgId,
            user_id: nextId("user"),
            email: input.email,
            display_name: input.email.split("@")[0],
            role: input.role,
            status: "invited",
            invited_at: new Date().toISOString(),
            joined_at: null,
          };
          store.members.push(member);
          const org = store.orgs.find((candidate) => candidate.id === orgId);
          if (org) org.member_count += 1;
          return member;
        }
        return { members: store.members.filter((member) => member.org_id === orgId) };
      },
    },
    {
      pattern: /^v1\/orgs\/([^/]+)\/members\/([^/]+)$/,
      handle: ({ method, match, body, store }) => {
        const [, orgId, memberId] = match;
        const member = store.members.find(
          (candidate) => candidate.org_id === orgId && candidate.id === memberId,
        );
        if (!member) throw new Error(`mock: member not found: ${memberId}`);
        if (method === "PATCH") {
          const input = body as { role: Member["role"] };
          member.role = input.role;
        }
        return member;
      },
    },
    // ---- Console: Roles / permission matrix ----
    {
      pattern: /^v1\/orgs\/([^/]+)\/roles$/,
      handle: () => permissionMatrix,
    },
    // ---- Console: Route decisions ----
    // Bare array, matching the real `GET /v1/route-decisions` (`Json(decisions)`) — not an
    // `{ decisions: [...] }` envelope.
    {
      pattern: /^v1\/projects\/([^/]+)\/routes$/,
      handle: ({ match, params }) => {
        const projectId = match[1];
        const decisions = routeDecisions[projectId] ?? [];
        const limit = typeof params?.limit === "number" ? params.limit : undefined;
        return limit !== undefined ? decisions.slice(0, limit) : decisions;
      },
    },
    // ---- Console: Outcome savings ----
    // The real `GET /v1/analytics/savings` never 404s for a valid project with zero activity —
    // `savings_summary`'s two `GROUP ALL` queries just return zeroed aggregates (see
    // `route_decision_repository.rs::merge_savings`'s `Default`). A brand-new project with no
    // fixture entry gets that same honest all-zero summary here, not a thrown error.
    {
      pattern: /^v1\/projects\/([^/]+)\/savings$/,
      handle: ({ match }) => {
        const projectId = match[1];
        return (
          savings[projectId] ?? {
            total_requests: 0,
            total_cost_charged: 0,
            total_baseline_cost: 0,
            total_saved: 0,
            zero_charge_count: 0,
            zero_charge_saved: 0,
            avg_r_binary: 0,
            cascade_adoption_pct: 0,
            moa_adoption_pct: 0,
          }
        );
      },
    },
    // ---- Playground: GaussMoA candidates (wired in M5) ----
    {
      pattern: /^v1\/moa-candidates$/,
      handle: () => ({ candidates: moaCandidates }),
    },
    // ---- Existing resources (unchanged strings; server-resolved to the active project) ----
    // Each handler returns the raw fixture; `query` below runs it through the caller's own
    // schema (the same one the real adapter would use) — one validation point, not two.
    { pattern: /^v1\/models$/, handle: () => models },
    // Model detail (M4 Model Detail page, `useModelDetail`) — not every catalog id has a
    // detail fixture; an unknown id throws, matching a real 404 rather than fabricating data.
    {
      pattern: /^v1\/models\/([^/]+)$/,
      handle: ({ match }) => {
        const modelId = match[1];
        const detail = modelDetails[modelId];
        if (!detail) throw new Error(`mock: model not found: ${modelId}`);
        return detail;
      },
    },
    { pattern: /^v1\/billing\/models$/, handle: () => modelPricing },
    { pattern: /^v1\/logs$/, handle: () => logs },
    // API keys: GET lists; POST mints a new key (raw secret returned once, matching
    // `CreateApiKeyResponseSchema`) and appends its hashed record to the store.
    {
      pattern: /^v1\/api\/keys$/,
      handle: ({ method, body, store }) => {
        if (method === "POST") {
          const input = body as { name?: string };
          const keyId = nextId("key");
          const rawKey = `grk_live_${Math.random().toString(36).slice(2, 10)}${Math.random().toString(36).slice(2, 10)}`;
          const entry: ApiKey = {
            id: keyId,
            key_hash: `hashed_${keyId}`,
            key_prefix: rawKey.slice(0, 13),
            user_id: mockUser.id,
            tenant_id: mockUser.tenant_id,
            name: input.name ?? null,
            rate_limit_per_minute: 600,
            rate_limit_per_day: 100000,
            created_at: new Date().toISOString(),
            expires_at: null,
            last_used_at: null,
            active: true,
          };
          store.apiKeys = [entry, ...store.apiKeys];
          return {
            key_id: keyId,
            api_key: rawKey,
            key_prefix: entry.key_prefix,
            message: "API key created — copy it now, it won't be shown again.",
          };
        }
        return store.apiKeys;
      },
    },
    // Revoke: flips `active` on the matching store entry — the list route reflects it on the
    // next fetch (the keys page invalidates `v1/api/keys` on success).
    {
      pattern: /^v1\/api\/keys\/revoke$/,
      handle: ({ body, store }) => {
        const input = body as { key_id: string };
        store.apiKeys = store.apiKeys.map((key) =>
          key.id === input.key_id ? { ...key, active: false } : key,
        );
        return { message: "revoked", key_id: input.key_id };
      },
    },
    // BYOK: GET lists registered provider names only (never key material); POST registers a
    // provider (idempotent — registering twice doesn't duplicate); DELETE removes one.
    {
      pattern: /^v1\/byok\/keys$/,
      handle: ({ method, body, store }) => {
        if (method === "POST") {
          const input = body as { provider: string; api_key: string };
          if (!store.byokProviders.includes(input.provider)) {
            store.byokProviders = [...store.byokProviders, input.provider];
          }
          return { provider: input.provider, registered: true };
        }
        return { providers: store.byokProviders };
      },
    },
    {
      pattern: /^v1\/byok\/keys\/([^/]+)$/,
      handle: ({ match, store }) => {
        const provider = match[1];
        store.byokProviders = store.byokProviders.filter((candidate) => candidate !== provider);
        return { deleted: true, provider };
      },
    },
    { pattern: /^v1\/analytics\/usage$/, handle: () => usage },
    {
      pattern: /^v1\/project\/settings$/,
      handle: ({ method, body }) => {
        if (method === "PATCH") return { ...projectSettings, ...(body as object) };
        return projectSettings;
      },
    },
    { pattern: /^v1\/balance$/, handle: () => balance },
    { pattern: /^v1\/billing\/budget$/, handle: () => budget },
    // ---- Account (/account/me) ----
    // `GET /v1/auth/me` — the real backend's `PublicUser`, including the PRD-21 Wave B profile
    // fields. Read-only here; edits go through the PATCH route below (both real and mock reuse
    // the same underlying record, matching how `update_profile` in handlers.rs is the one
    // write path for these fields regardless of whether the caller arrived via onboarding or
    // /account/me).
    { pattern: /^v1\/auth\/me$/, handle: ({ store }) => store.user },
    // `PATCH /v1/onboarding/profile` — the onboarding wizard's profile-save endpoint (US O3),
    // reused unchanged for /account/me edits per `update_profile`'s own doc comment ("a later
    // 'complete later' edit from settings... works with the same endpoint"). Partial update:
    // an omitted field leaves the stored value untouched.
    {
      pattern: /^v1\/onboarding\/profile$/,
      handle: ({ method, body, store }) => {
        if (method === "PATCH") {
          const input = body as {
            full_name?: string;
            display_name?: string;
            company?: string;
            timezone?: string;
          };
          if (input.full_name !== undefined) store.user.full_name = input.full_name;
          if (input.display_name !== undefined) store.user.display_name = input.display_name;
          if (input.company !== undefined) store.user.company = input.company;
          if (input.timezone !== undefined) store.user.timezone = input.timezone;
        }
        return store.user;
      },
    },
    // ---- Admin (PRD-23 Wave C) ----
    // The mock's one signed-in identity is always treated as an allowlisted superadmin — that's
    // the whole point of the mock: the entire admin flow must be demoable under
    // `NEXT_PUBLIC_USE_MOCKS=1` without a separate "log in as an admin" story.
    { pattern: /^v1\/admin\/me$/, handle: () => ({ superadmin: true }) },
    {
      pattern: /^v1\/admin\/metrics$/,
      handle: ({ params }) => {
        const requested = Number(params?.months ?? 6);
        const months = Math.min(24, Math.max(1, Number.isFinite(requested) ? requested : 6));
        const series = adminMetricsSeries.slice(-months);
        const current = series.at(-1) ?? adminMetricsSeries[adminMetricsSeries.length - 1];
        return { current, series };
      },
    },
    {
      pattern: /^v1\/admin\/users$/,
      handle: ({ params, store }) => {
        const q = typeof params?.q === "string" ? params.q.toLowerCase() : undefined;
        const limit = typeof params?.limit === "number" ? params.limit : 50;
        const start = typeof params?.start === "number" ? params.start : 0;

        const withDeletionStatus = store.adminUsers.map((user) => ({
          ...user,
          deletion_status:
            store.deletionRequests.find((r) => r.user_id === user.id && r.status === "pending")
              ?.status ?? null,
        }));
        const filtered = q
          ? withDeletionStatus.filter(
              (user) =>
                user.email.toLowerCase().includes(q) || user.username.toLowerCase().includes(q),
            )
          : withDeletionStatus;

        return { users: filtered.slice(start, start + limit), total: filtered.length };
      },
    },
    {
      pattern: /^v1\/admin\/deletion-requests$/,
      handle: ({ params, store }) => {
        const status = typeof params?.status === "string" ? params.status : undefined;
        const rows = status
          ? store.deletionRequests.filter((r) => r.status === status)
          : store.deletionRequests;
        // Newest first — the real backend's `repo.list` orders by `requested_at DESC`.
        return {
          requests: [...rows].sort((a, b) => b.requested_at.localeCompare(a.requested_at)),
        };
      },
    },
    {
      pattern: /^v1\/admin\/deletion-requests\/([^/]+)\/fulfill$/,
      handle: ({ match, store }) => {
        const request = store.deletionRequests.find((r) => r.id === match[1]);
        if (!request) throw new GaussMeridianAdapterError("Request failed with status 404", 404);
        if (request.status !== "pending") {
          throw new GaussMeridianAdapterError("Request failed with status 409", 409);
        }
        request.status = "fulfilled";
        request.resolved_at = new Date().toISOString();
        request.resolved_by = store.user.email;
        // Reflect on the account side when this was the signed-in identity's own request.
        if (request.user_id === store.user.id) store.user.deletion_requested = false;
        // Best-effort demo of the real cascade: drop the target from the directory.
        store.adminUsers = store.adminUsers.filter((user) => user.id !== request.user_id);
        return undefined; // 204 No Content, matching the real handler.
      },
    },
    {
      pattern: /^v1\/admin\/deletion-requests\/([^/]+)\/reject$/,
      handle: ({ match, body, store }) => {
        const request = store.deletionRequests.find((r) => r.id === match[1]);
        if (!request) throw new GaussMeridianAdapterError("Request failed with status 404", 404);
        if (request.status !== "pending") {
          throw new GaussMeridianAdapterError("Request failed with status 409", 409);
        }
        const input = body as { note?: string } | undefined;
        request.status = "rejected";
        request.note = input?.note ?? null;
        request.resolved_at = new Date().toISOString();
        request.resolved_by = store.user.email;
        if (request.user_id === store.user.id) store.user.deletion_requested = false;
        return {
          id: request.id,
          status: request.status,
          note: request.note,
          resolved_at: request.resolved_at,
          resolved_by: request.resolved_by,
        };
      },
    },
    // ---- Admin observability (PRD-24 Wave C) ----
    // `window` is an integer month count; clamp to [1, 12] and slice the series tail, mirroring
    // the backend's `AdminObservabilityRepository` windowing. `overview`/`finance`/`cost` are pure
    // over the FROZEN financial fixtures (control actions never change the money); the org/project
    // directory + detail + watchlist reads go through the mutable `store.adminOrgRows`/
    // `adminProjectRows` so a Lock/Suspend/Reactivate flips a status these reads then reflect.
    {
      pattern: /^v1\/admin\/overview$/,
      handle: ({ params }) => {
        const series = businessMonths.slice(-monthWindow(params?.window));
        return { current: series.at(-1) ?? null, series };
      },
    },
    {
      pattern: /^v1\/admin\/finance$/,
      handle: ({ params }) => ({
        series: businessMonths.slice(-monthWindow(params?.window)),
        by_model: costPivots.model,
        by_provider: costPivots.provider,
      }),
    },
    {
      pattern: /^v1\/admin\/cost$/,
      handle: ({ params }) => {
        const groupBy = typeof params?.group_by === "string" ? params.group_by : "org";
        const sort = params?.sort === "asc" ? "asc" : "desc";
        const rows = [...(costPivots[groupBy] ?? [])].sort((a, b) =>
          sort === "asc" ? a.cost - b.cost : b.cost - a.cost,
        );
        return { group_by: groupBy, rows };
      },
    },
    // ---- Admin control (PRD-24 Wave C2) — write routes that MUTATE the store, so the flip is
    // visible on the next directory/detail/watchlist read (the whole point of mock parity). Impact
    // is a read-only dry-run; audit grows by one row per action. These 3-segment routes are listed
    // BEFORE the 2-segment detail routes below only for readability — the regexes are disjoint
    // (`[^/]+` never spans a slash), so match order doesn't actually matter here.
    {
      pattern: /^v1\/admin\/(orgs|projects)\/([^/]+)\/impact$/,
      handle: ({ match, params, store }) => {
        const [, target, id] = match;
        const row =
          target === "orgs"
            ? store.adminOrgRows.find((o) => o.id === id)
            : store.adminProjectRows.find((p) => p.id === id);
        if (!row) throw new GaussMeridianAdapterError("Request failed with status 404", 404);
        const window = impactWindow(params?.window);
        // The row totals read as ~monthly; a dry-run over the last N days is that per-day rate
        // times the window — enough to make the preview's numbers move with the entity.
        return {
          target_type: SINGULAR_TARGET[target],
          target_id: id,
          window,
          requests: Math.round((row.requests / 30) * window),
          provider_cost: Number(((row.provider_cost / 30) * window).toFixed(2)),
          sample: impactSample(),
        };
      },
    },
    {
      pattern: /^v1\/admin\/(orgs|projects|users|keys)\/([^/]+)\/(lock|suspend|reactivate)$/,
      handle: ({ match, params, store }) => {
        const [, target, id, action] = match;
        // `lock` only exists for orgs/projects server-side — mirror the 404 a users/keys lock hits.
        if (action === "lock" && target !== "orgs" && target !== "projects") {
          throw new GaussMeridianAdapterError("Request failed with status 404", 404);
        }
        const reason = typeof params?.reason === "string" ? params.reason : null;
        const minutes =
          action === "lock" ? (typeof params?.minutes === "number" ? params.minutes : 60) : null;
        const lockedUntil =
          minutes !== null ? new Date(Date.now() + minutes * 60_000).toISOString() : null;

        let status: string;
        if (target === "orgs" || target === "projects") {
          const rows = target === "orgs" ? store.adminOrgRows : store.adminProjectRows;
          const row = rows.find((r) => r.id === id);
          if (!row) throw new GaussMeridianAdapterError("Request failed with status 404", 404);
          status = action === "suspend" ? "suspended" : action === "lock" ? "locked" : "active";
          row.status = status;
        } else if (target === "users") {
          const user = store.adminUsers.find((u) => u.id === id);
          if (!user) throw new GaussMeridianAdapterError("Request failed with status 404", 404);
          user.active = action === "reactivate";
          status = user.active ? "active" : "suspended";
        } else {
          // keys — no key entity is modelled in the admin store, so nothing to flip; the audit row
          // + response still record the action (the real backend flips `api_keys.active`).
          status = action === "reactivate" ? "active" : "suspended";
        }

        appendAudit(store, {
          action,
          target_type: SINGULAR_TARGET[target],
          target_id: id,
          reason,
        });

        return {
          target_type: SINGULAR_TARGET[target],
          target_id: id,
          action,
          status,
          locked_until: lockedUntil,
        };
      },
    },
    {
      pattern: /^v1\/admin\/audit$/,
      handle: ({ params, store }) => {
        const limit = typeof params?.limit === "number" ? params.limit : undefined;
        const sorted = [...store.adminAudit].sort((a, b) =>
          b.created_at.localeCompare(a.created_at),
        );
        return { actions: limit !== undefined ? sorted.slice(0, limit) : sorted };
      },
    },
    {
      pattern: /^v1\/admin\/orgs$/,
      handle: ({ store }) => ({
        orgs: [...store.adminOrgRows].sort((a, b) => b.bleed - a.bleed),
      }),
    },
    {
      pattern: /^v1\/admin\/orgs\/([^/]+)$/,
      handle: ({ match, store }) => {
        const org = store.adminOrgRows.find((o) => o.id === match[1]);
        if (!org) throw new GaussMeridianAdapterError("Request failed with status 404", 404);
        return { org, projects: store.adminProjectRows.filter((p) => p.org_id === org.id) };
      },
    },
    {
      pattern: /^v1\/admin\/projects$/,
      handle: ({ store }) => ({
        projects: [...store.adminProjectRows].sort((a, b) => b.bleed - a.bleed),
      }),
    },
    {
      pattern: /^v1\/admin\/projects\/([^/]+)$/,
      handle: ({ match, store }) => {
        const project = store.adminProjectRows.find((p) => p.id === match[1]);
        if (!project) throw new GaussMeridianAdapterError("Request failed with status 404", 404);
        return { project };
      },
    },
    {
      pattern: /^v1\/admin\/watchlist$/,
      handle: ({ store }) => ({
        bleeders: [...store.adminOrgRows].sort((a, b) => b.bleed - a.bleed).slice(0, 8),
        // Reflect any status flip on the idle set too (e.g. suspending an idle org), matching it
        // back to the mutable directory row by id.
        idle: watchlistIdle.map((entry) => {
          const org = store.adminOrgRows.find((o) => o.id === entry.id);
          return org ? { ...entry, status: org.status } : entry;
        }),
      }),
    },
  ];
}

/** Plural URL segment → the singular `target_type` the control response + audit rows carry. */
const SINGULAR_TARGET: Record<string, string> = {
  orgs: "org",
  projects: "project",
  users: "user",
  keys: "key",
};

/** Clamp a raw impact `window` (a day-count) to [1, 90], default 7 (the v1 dry-run lookback). */
function impactWindow(raw: unknown): number {
  const requested = Number(raw ?? 7);
  return Math.min(90, Math.max(1, Number.isFinite(requested) ? requested : 7));
}

/** A representative slice of "what would be blocked" for a dry-run preview — the top few models by
 *  cost, priced per-request. Pure over the frozen model pivot; the exact rows don't need to belong
 *  to the target (the preview is illustrative, and the real backend samples the target's ledger). */
function impactSample(): {
  model: string;
  provider: string;
  cost: number;
  created_at: string;
}[] {
  const providerByModelKey: Record<string, string> = {
    "gpt-4o": "openai",
    "gpt-4o-mini": "openai",
    "claude-3-7-sonnet": "anthropic",
    "claude-3-5-haiku": "anthropic",
    "gemini-2-5-flash": "google",
    "llama-3-3-70b": "meta",
  };
  return costPivots.model.slice(0, 4).map((m) => ({
    model: m.label,
    provider: providerByModelKey[m.key] ?? "openai",
    cost: Number((m.cost / m.requests).toFixed(4)),
    created_at: m.last_seen ?? new Date().toISOString(),
  }));
}

/** Prepend a fresh audit row for a control action, attributed to the signed-in mock identity. */
function appendAudit(
  store: MockStore,
  entry: { action: string; target_type: string; target_id: string; reason: string | null },
): void {
  store.adminAudit = [
    {
      id: nextId("aud"),
      actor_email: store.user.email,
      created_at: new Date().toISOString(),
      ...entry,
    },
    ...store.adminAudit,
  ];
}

/** Clamp a raw `window` query param to the backend's supported month window [1, 12], default 6. */
function monthWindow(raw: unknown): number {
  const requested = Number(raw ?? 6);
  return Math.min(12, Math.max(1, Number.isFinite(requested) ? requested : 6));
}

/** Reads `store.user` (not the frozen `mockUser` fixture) so a profile edit made via `PATCH
 *  v1/onboarding/profile` (the /account/me page's save) is reflected the next time the caller
 *  refetches the session — mirrors the real adapter, where `getSession()` always re-reads the
 *  backend's current row. */
function createMockSession(store: MockStore): AuthSession {
  return {
    userId: store.user.id,
    displayName: store.user.username,
    token: "mock_session_token",
    expiresAt: "2099-01-01T00:00:00Z",
    // The mock registry exists to exercise the already-built console (org/project/BYOK
    // fixtures) — the mock user is always past onboarding, never routed into the wizard.
    onboardingCompleted: true,
    email: store.user.email,
  };
}

/** getSession/signIn/signUp all resolve the same identity (there is one mock user), now backed
 *  by the shared, mutable `store` so profile edits and the account-deletion request are visible
 *  across both the `auth` and `data` seams. */
function createMockAuthAdapter(store: MockStore): AuthAdapter {
  return {
    signIn: async () => createMockSession(store),
    signUp: async () => createMockSession(store),
    getSession: async () => createMockSession(store),
    signOut: async () => undefined,
    forgotPassword: async () => undefined,
    resetPassword: async () => undefined,
    // PRD-23 Wave C — pushes a real row into `store.deletionRequests` (idempotent: a second
    // call while one is already pending doesn't duplicate it) so it shows up in `/admin/
    // deletions`' Pending tab, and flips `store.user.deletion_requested` so `/account/me`'s
    // danger zone reflects the pending state on its next read — the "requesting deletion from
    // /account/me pushes into the mock queue" wiring the demo needs. Never rejects, unlike the
    // real adapter's legacy "not enabled on this server" mapping (see
    // `gaussmeridian-auth.adapter.ts`'s doc comment) — the mock always has somewhere to record it.
    requestAccountDeletion: async () => {
      const alreadyPending = store.deletionRequests.some(
        (r) => r.user_id === store.user.id && r.status === "pending",
      );
      if (!alreadyPending) {
        store.deletionRequests = [
          {
            id: nextId("delreq"),
            user_id: store.user.id,
            email: store.user.email,
            username: store.user.username,
            status: "pending",
            note: null,
            requested_at: new Date().toISOString(),
            resolved_at: null,
            resolved_by: null,
          },
          ...store.deletionRequests,
        ];
      }
      store.user.deletion_requested = true;
    },
    // PRD-23 Wave C — the real backend's `cancel_for_user` REMOVES the pending row entirely
    // (not a "cancelled" status — there is no such status; see
    // `deletion_request_repository.rs::cancel_for_user_removes_the_pending_row`), so the mock
    // mirrors that rather than marking a status.
    cancelAccountDeletion: async () => {
      store.deletionRequests = store.deletionRequests.filter(
        (r) => !(r.user_id === store.user.id && r.status === "pending"),
      );
      store.user.deletion_requested = false;
    },
  };
}

function createMockLlmAdapter(): LlmByokAdapter {
  return {
    // Word-by-word canned stream so the mock Playground visibly exercises pending → settled.
    streamChat: async function* ({ model, messages }) {
      const last = messages.at(-1)?.content ?? "your request";
      const reply = `GaussMeridian mock response from ${model} about "${last}."`;
      for (const word of reply.split(" ")) {
        yield `${word} `;
      }
    },
  };
}

/**
 * Builds an in-memory `AdapterRegistry` — the Phase-1 stand-in for a live GaussMeridian
 * backend. `data.query` always validates its result through the caller's schema before
 * returning it, exactly like `createGaussMeridianDataAdapter`, so a fixture that drifts from
 * its schema fails the same way a bad live response would.
 */
export function createMockRegistry(): AdapterRegistry {
  const store = cloneFixtures();
  const routes = buildRoutes();

  return {
    data: {
      async query({ resource, params, schema, method, body }) {
        const route = routes.find((candidate) => candidate.pattern.test(resource));
        if (!route) throw new Error("mock: unhandled resource " + resource);
        const match = resource.match(route.pattern);
        if (!match) throw new Error("mock: unhandled resource " + resource);
        const result = route.handle({ method: method ?? "GET", body, params, match, store });
        return schema.parse(result);
      },
    },
    auth: createMockAuthAdapter(store),
    llm: createMockLlmAdapter(),
  };
}
