import { ROLES_RESOURCE } from "@core/config/resources";

import {
  buildPermissionMatrix,
  mapMemberResponse,
  mapOrgProjectResponse,
  mapOrgResponse,
  roleNameToTier,
  slugify,
} from "./console.mapper";
import { gaussMeridianRawRequest } from "./gaussmeridian-data.adapter";
import {
  MemberResponseSchema,
  OrgProjectResponseSchema,
  OrgResponseSchema,
  RoleResponseSchema,
} from "./schemas/console-real.schema";

import type { RoleResponse } from "./schemas/console-real.schema";
import type { Role } from "./schemas/console.schema";
import type { DataQueryAdapter, DataQueryInput } from "./types";

const ORG_LIST_RE = /^v1\/orgs$/;
const ORG_ITEM_RE = /^v1\/orgs\/([^/]+)$/;
const ORG_PROJECTS_RE = /^v1\/orgs\/([^/]+)\/projects$/;
const ORG_PROJECT_ITEM_RE = /^v1\/orgs\/([^/]+)\/projects\/([^/]+)$/;
const ORG_MEMBERS_RE = /^v1\/orgs\/([^/]+)\/members$/;
const ORG_MEMBER_ITEM_RE = /^v1\/orgs\/([^/]+)\/members\/([^/]+)$/;

/**
 * Tiny in-memory, module-level cache of `GET /v1/roles`. Roles are platform-seeded and change
 * rarely, and multiple org-layer resources (member list/invite/role-change, permission matrix)
 * all need the same role_id<->name mapping within a session — without this, every member-list
 * render would trigger its own `/v1/roles` round trip on top of the members fetch. A short TTL
 * (rather than "forever") keeps a stale cache from surviving an admin editing roles out-of-band
 * for more than a minute.
 */
let rolesCache: { roles: RoleResponse[]; fetchedAt: number } | null = null;
const ROLES_CACHE_TTL_MS = 60_000;

async function fetchRoles(): Promise<RoleResponse[]> {
  if (rolesCache && Date.now() - rolesCache.fetchedAt < ROLES_CACHE_TTL_MS) {
    return rolesCache.roles;
  }
  const json = await gaussMeridianRawRequest({ resource: ROLES_RESOURCE });
  const roles = RoleResponseSchema.array().parse(json);
  rolesCache = { roles, fetchedAt: Date.now() };
  return roles;
}

function resolveRoleIdForTier(roles: RoleResponse[], tier: Role): string {
  const match = roles.find((role) => roleNameToTier(role.name) === tier);
  if (!match) {
    throw new Error(`console-org.adapter: no backend role seeded for tier "${tier}"`);
  }
  return match.id;
}

function resolveTierNameForRoleId(roles: RoleResponse[], roleId: string): string | undefined {
  return roles.find((role) => role.id === roleId)?.name;
}

/**
 * Resource-aware decorator around a base `DataQueryAdapter` (normally
 * `createGaussMeridianDataAdapter()`). Every resource under `v1/orgs/**` (plus the global
 * `v1/roles` catalog) is intercepted and run through the Wave-2 anti-corruption mapping
 * (`console.mapper.ts`): fetch the REAL backend DTO, validate it against `console-real.schema`,
 * reshape it into the stable `console.schema` UI contract, then validate the reshaped result
 * against the schema the caller actually asked for (the same "never trust type alone" Zod
 * discipline the base adapter uses). Every other resource (`v1/logs`, `v1/byok/*`,
 * `v1/analytics/*`, …) passes straight through to `base` untouched — this decorator has zero
 * opinion about resources it doesn't recognize.
 *
 * `useConsoleQueries.ts` and every component under `src/components/orgs/**` are unaware this
 * exists — they keep calling `useDataQuery().query({...})` exactly as before. The mapping lives
 * entirely at this one seam, which is the whole point of an anti-corruption adapter: the deep,
 * stable interface (`Org`/`Project`/`Member`/`Role`) doesn't leak the messy real DTOs upward.
 *
 * `v1/onboarding/*` and `v1/projects/:id/password[/verify]` (PRD-21 Wave B / DR-010) are
 * deliberately given NO regex here — their real backend response shapes already match the FE
 * contract 1:1 (`console.schema.ts`'s `OnboardingStateResponseSchema` etc.), so they fall
 * through to `base.query(input)` at the bottom of this function untouched, exactly like every
 * other resource this adapter doesn't recognize. Nothing to remap = no bespoke branch.
 */
export function createConsoleOrgDataAdapter(base: DataQueryAdapter): DataQueryAdapter {
  return {
    async query<T>(input: DataQueryInput<T>): Promise<T> {
      const { resource, method = "GET", body, schema } = input;

      if (resource === ROLES_RESOURCE && method === "GET") {
        const roles = await fetchRoles();
        return schema.parse(buildPermissionMatrix(roles)) as T;
      }

      if (ORG_LIST_RE.test(resource)) {
        if (method === "GET") {
          const json = await gaussMeridianRawRequest({ resource });
          const dtos = OrgResponseSchema.array().parse(json);
          const orgs = dtos.map(mapOrgResponse);
          return schema.parse({ orgs }) as T;
        }
        if (method === "POST") {
          // Real CreateOrgRequest requires `slug` — `CreateOrgForm` only collects `name`
          // (Phase-1 UI never asks for a slug). Derive one rather than let every org creation
          // 422 against the real backend. DEFERRED: an editable slug field was drafted for this
          // session but reopened as a design question (user-chosen slug vs. a system-generated
          // opaque id) — see `create-org-form.tsx`'s doc comment. Until that's resolved this
          // derive-and-send path stays the only one. FLAGGED: if the derived slug collides with
          // an existing org, the backend rejects it and the caller sees a normal create-org
          // error — there is no client-side uniqueness check here.
          const createInput = body as { name: string; slug?: string; plan?: string };
          const realBody = { ...createInput, slug: createInput.slug || slugify(createInput.name) };
          const json = await gaussMeridianRawRequest({ resource, method, body: realBody });
          const dto = OrgResponseSchema.parse(json);
          const org = mapOrgResponse(dto);
          return schema.parse(org) as T;
        }
      }

      const orgItemMatch = ORG_ITEM_RE.exec(resource);
      if (orgItemMatch) {
        if (method === "GET" || method === "PATCH") {
          const json = await gaussMeridianRawRequest({ resource, method, body });
          const dto = OrgResponseSchema.parse(json);
          const org = mapOrgResponse(dto);
          return schema.parse(org) as T;
        }
        // DELETE has no shape to remap — pass straight through.
        return base.query(input);
      }

      if (ORG_PROJECTS_RE.test(resource)) {
        if (method === "GET") {
          const json = await gaussMeridianRawRequest({ resource });
          const dtos = OrgProjectResponseSchema.array().parse(json);
          const projects = dtos.map(mapOrgProjectResponse);
          return schema.parse({ projects }) as T;
        }
        if (method === "POST") {
          // Real CreateOrgProjectRequest only accepts `{ name }` — `useCreateProject` also
          // sends `slug`/`environment` (Phase-1 UI concepts, see `mapOrgProjectResponse`'s doc
          // comment). Strip them here rather than change the hook's call signature.
          const realBody = { name: (body as { name: string }).name };
          const json = await gaussMeridianRawRequest({ resource, method, body: realBody });
          const dto = OrgProjectResponseSchema.parse(json);
          return schema.parse(mapOrgProjectResponse(dto)) as T;
        }
      }

      if (ORG_PROJECT_ITEM_RE.test(resource)) {
        if (method === "GET") {
          const json = await gaussMeridianRawRequest({ resource });
          const dto = OrgProjectResponseSchema.parse(json);
          return schema.parse(mapOrgProjectResponse(dto)) as T;
        }
        if (method === "PATCH") {
          const json = await gaussMeridianRawRequest({ resource, method, body });
          const dto = OrgProjectResponseSchema.parse(json);
          return schema.parse(mapOrgProjectResponse(dto)) as T;
        }
        // DELETE has no shape to remap — pass straight through.
        return base.query(input);
      }

      if (ORG_MEMBERS_RE.test(resource)) {
        if (method === "GET") {
          const [json, roles] = await Promise.all([
            gaussMeridianRawRequest({ resource }),
            fetchRoles(),
          ]);
          const dtos = MemberResponseSchema.array().parse(json);
          const members = dtos.map((dto) =>
            mapMemberResponse(dto, resolveTierNameForRoleId(roles, dto.role_id)),
          );
          return schema.parse({ members }) as T;
        }
        if (method === "POST") {
          const invite = body as { email: string; role: Role };
          const roles = await fetchRoles();
          const role_id = resolveRoleIdForTier(roles, invite.role);
          const json = await gaussMeridianRawRequest({
            resource,
            method,
            body: { email: invite.email, role_id },
          });
          const dto = MemberResponseSchema.parse(json);
          const member = mapMemberResponse(dto, resolveTierNameForRoleId(roles, dto.role_id));
          return schema.parse(member) as T;
        }
      }

      if (ORG_MEMBER_ITEM_RE.test(resource)) {
        if (method === "PATCH") {
          const update = body as { role: Role };
          const roles = await fetchRoles();
          const role_id = resolveRoleIdForTier(roles, update.role);
          const json = await gaussMeridianRawRequest({ resource, method, body: { role_id } });
          const dto = MemberResponseSchema.parse(json);
          const member = mapMemberResponse(dto, resolveTierNameForRoleId(roles, dto.role_id));
          return schema.parse(member) as T;
        }
        // DELETE has no shape to remap — pass straight through.
        return base.query(input);
      }

      return base.query(input);
    },
  };
}
