import { describe, expect, it } from "vitest";
import { z } from "zod";

import {
  AdminAuditResponseSchema,
  AdminControlResponseSchema,
  AdminImpactResponseSchema,
  AdminOrgDetailResponseSchema,
  AdminOrgsResponseSchema,
  AdminUsersResponseSchema,
} from "../../schemas/admin.schema";
import {
  MemberListSchema,
  MemberSchema,
  OrgListSchema,
  OrgSchema,
  ProjectListSchema,
  ProjectSchema,
} from "../../schemas/console.schema";
import {
  ApiKeySchema,
  ByokProvidersSchema,
  CreateApiKeyResponseSchema,
  ModelInfoSchema,
} from "../../schemas/gaussmeridian.schema";
import { createMockRegistry } from "../create-mock-registry";
import { mockUser } from "../fixtures/user";

describe("createMockRegistry — data.query parity with the real adapter", () => {
  it("throws on an unhandled resource, exactly like a real 404 would surface", async () => {
    const registry = createMockRegistry();
    await expect(
      registry.data.query({ resource: "v1/not-a-real-resource", schema: z.unknown() }),
    ).rejects.toThrow("mock: unhandled resource v1/not-a-real-resource");
  });

  it("parses a known resource against the caller's schema and returns validated data", async () => {
    const registry = createMockRegistry();
    const result = await registry.data.query({ resource: "v1/orgs", schema: OrgListSchema });
    expect(result.orgs.length).toBeGreaterThan(0);
    expect(result.orgs.some((org) => org.project_count === 0)).toBe(true); // the born-empty org
  });

  it("throws a ZodError when the fixture does not satisfy the caller's schema — a malformed fixture cannot pass silently", async () => {
    const registry = createMockRegistry();
    const wrongSchema = z.object({ this_field_does_not_exist_on_an_org_list: z.string() });
    await expect(
      registry.data.query({ resource: "v1/orgs", schema: wrongSchema }),
    ).rejects.toBeInstanceOf(z.ZodError);
  });

  it("creates an org via POST and reflects it immediately on the next GET (in-memory store)", async () => {
    const registry = createMockRegistry();
    const created = await registry.data.query({
      resource: "v1/orgs",
      method: "POST",
      body: { name: "Acme Rockets" },
      schema: OrgSchema,
    });
    expect(created.name).toBe("Acme Rockets");
    expect(created.member_count).toBe(1);
    expect(created.project_count).toBe(0); // born empty

    const list = await registry.data.query({ resource: "v1/orgs", schema: OrgListSchema });
    expect(list.orgs.some((org) => org.id === created.id)).toBe(true);
  });

  it("isolates state across separate createMockRegistry() calls", async () => {
    const first = createMockRegistry();
    await first.data.query({
      resource: "v1/orgs",
      method: "POST",
      body: { name: "Only In First" },
      schema: OrgSchema,
    });

    const second = createMockRegistry();
    const list = await second.data.query({ resource: "v1/orgs", schema: OrgListSchema });
    expect(list.orgs.some((org) => org.name === "Only In First")).toBe(false);
  });
});

describe("createMockRegistry — store mutations", () => {
  it("POST v1/orgs/:orgId/projects creates a project and increments the org's project_count", async () => {
    const registry = createMockRegistry();

    const orgBefore = await registry.data.query({
      resource: "v1/orgs/org_born_empty",
      schema: OrgSchema,
    });
    expect(orgBefore.project_count).toBe(0); // born empty

    const created = await registry.data.query({
      resource: "v1/orgs/org_born_empty/projects",
      method: "POST",
      body: { name: "First Project" },
      schema: ProjectSchema,
    });
    expect(created.org_id).toBe("org_born_empty");
    expect(created.environment).toBe("development"); // default when unspecified

    const list = await registry.data.query({
      resource: "v1/orgs/org_born_empty/projects",
      schema: ProjectListSchema,
    });
    expect(list.projects.some((project) => project.id === created.id)).toBe(true);

    const orgAfter = await registry.data.query({
      resource: "v1/orgs/org_born_empty",
      schema: OrgSchema,
    });
    expect(orgAfter.project_count).toBe(1);
  });

  it("POST v1/orgs/:orgId/members invites a member and increments the org's member_count", async () => {
    const registry = createMockRegistry();

    const orgBefore = await registry.data.query({
      resource: "v1/orgs/org_born_empty",
      schema: OrgSchema,
    });
    expect(orgBefore.member_count).toBe(1); // just the owner

    const created = await registry.data.query({
      resource: "v1/orgs/org_born_empty/members",
      method: "POST",
      body: { email: "new.dev@meridianlabs.dev", role: "developer" },
      schema: MemberSchema,
    });
    expect(created.org_id).toBe("org_born_empty");
    expect(created.status).toBe("invited");
    expect(created.joined_at).toBeNull();

    const list = await registry.data.query({
      resource: "v1/orgs/org_born_empty/members",
      schema: MemberListSchema,
    });
    expect(list.members.some((member) => member.id === created.id)).toBe(true);

    const orgAfter = await registry.data.query({
      resource: "v1/orgs/org_born_empty",
      schema: OrgSchema,
    });
    expect(orgAfter.member_count).toBe(2);
  });

  it("PATCH v1/orgs/:orgId/members/:memberId updates just that member's role", async () => {
    const registry = createMockRegistry();

    const updated = await registry.data.query({
      resource: "v1/orgs/org_meridian/members/mem_dev_invited",
      method: "PATCH",
      body: { role: "admin" },
      schema: MemberSchema,
    });
    expect(updated.id).toBe("mem_dev_invited");
    expect(updated.role).toBe("admin");

    const list = await registry.data.query({
      resource: "v1/orgs/org_meridian/members",
      schema: MemberListSchema,
    });
    const changed = list.members.find((member) => member.id === "mem_dev_invited");
    const untouched = list.members.find((member) => member.id === "mem_admin");
    expect(changed?.role).toBe("admin");
    expect(untouched?.role).toBe("admin"); // was already admin — unaffected by the patch above
  });

  it("DELETE v1/orgs/:orgId removes the org and cascades to its projects, members, and API keys", async () => {
    const registry = createMockRegistry();

    await registry.data.query({
      resource: "v1/orgs/org_meridian",
      method: "DELETE",
      schema: z.unknown(),
    });

    const orgs = await registry.data.query({ resource: "v1/orgs", schema: OrgListSchema });
    expect(orgs.orgs.some((org) => org.id === "org_meridian")).toBe(false);

    const projects = await registry.data.query({
      resource: "v1/orgs/org_meridian/projects",
      schema: ProjectListSchema,
    });
    expect(projects.projects).toHaveLength(0);

    const members = await registry.data.query({
      resource: "v1/orgs/org_meridian/members",
      schema: MemberListSchema,
    });
    expect(members.members).toHaveLength(0);

    // `ApiKeySchema.tenant_id` is the owning org's id (mock/fixtures/api-keys.ts) — the org
    // fixture's one seeded key (`tenant_id: "org_meridian"`) is cascaded away too.
    const keys = await registry.data.query({
      resource: "v1/api/keys",
      schema: z.array(ApiKeySchema),
    });
    expect(keys.some((key) => key.tenant_id === "org_meridian")).toBe(false);

    // the other org is untouched by the cascade
    const survivingOrg = await registry.data.query({
      resource: "v1/orgs/org_born_empty",
      schema: OrgSchema,
    });
    expect(survivingOrg.id).toBe("org_born_empty");
  });
});

describe("createMockRegistry — DELETE v1/orgs/:orgId/projects/:pid (project deletion)", () => {
  it("removes only the targeted project and decrements the org's project_count", async () => {
    const registry = createMockRegistry();

    const before = await registry.data.query({
      resource: "v1/orgs/org_meridian",
      schema: OrgSchema,
    });
    expect(before.project_count).toBe(2);

    await registry.data.query({
      resource: "v1/orgs/org_meridian/projects/proj_prod",
      method: "DELETE",
      schema: z.unknown(),
    });

    const projects = await registry.data.query({
      resource: "v1/orgs/org_meridian/projects",
      schema: ProjectListSchema,
    });
    expect(projects.projects.map((project) => project.id)).toEqual(["proj_dev"]);

    const after = await registry.data.query({
      resource: "v1/orgs/org_meridian",
      schema: OrgSchema,
    });
    expect(after.project_count).toBe(1);
  });

  it("does not touch API keys — ApiKeySchema has no project_id to cascade on", async () => {
    const registry = createMockRegistry();
    const before = await registry.data.query({
      resource: "v1/api/keys",
      schema: z.array(ApiKeySchema),
    });

    await registry.data.query({
      resource: "v1/orgs/org_meridian/projects/proj_prod",
      method: "DELETE",
      schema: z.unknown(),
    });

    const after = await registry.data.query({
      resource: "v1/api/keys",
      schema: z.array(ApiKeySchema),
    });
    expect(after).toEqual(before);
  });

  it("GET on a deleted project throws, matching a real 404 rather than returning stale data", async () => {
    const registry = createMockRegistry();
    await registry.data.query({
      resource: "v1/orgs/org_meridian/projects/proj_prod",
      method: "DELETE",
      schema: z.unknown(),
    });

    await expect(
      registry.data.query({
        resource: "v1/orgs/org_meridian/projects/proj_prod",
        schema: ProjectSchema,
      }),
    ).rejects.toThrow("mock: project not found: proj_prod");
  });
});

describe("createMockRegistry — model detail (M4 Model Detail page)", () => {
  it("GET v1/models/:modelId returns detail for a known model", async () => {
    const registry = createMockRegistry();
    const detail = await registry.data.query({
      resource: "v1/models/gpt-4o-mini",
      schema: ModelInfoSchema,
    });
    expect(detail.id).toBe("gpt-4o-mini");
    expect(detail.pricing.input_cost_per_1k_tokens).toBeGreaterThan(0);
  });

  it("throws for an id with no detail fixture, matching a real 404 rather than fabricating data", async () => {
    const registry = createMockRegistry();
    await expect(
      registry.data.query({ resource: "v1/models/not-a-real-model", schema: ModelInfoSchema }),
    ).rejects.toThrow("mock: model not found: not-a-real-model");
  });
});

describe("createMockRegistry — API keys (M4 create/revoke flow)", () => {
  it("POST v1/api/keys mints a raw secret once and lists a hashed record afterward", async () => {
    const registry = createMockRegistry();

    const created = await registry.data.query({
      resource: "v1/api/keys",
      method: "POST",
      body: { name: "CI key" },
      schema: CreateApiKeyResponseSchema,
    });
    expect(created.api_key).toMatch(/^grk_live_/);
    expect(created.key_prefix.length).toBeGreaterThan(0);

    const list = await registry.data.query({
      resource: "v1/api/keys",
      schema: z.array(ApiKeySchema),
    });
    const stored = list.find((key) => key.id === created.key_id);
    expect(stored).toBeDefined();
    expect(stored?.active).toBe(true);
    expect(stored?.name).toBe("CI key");
  });

  it("POST v1/api/keys/revoke flips the matching key to inactive without touching others", async () => {
    const registry = createMockRegistry();

    const before = await registry.data.query({
      resource: "v1/api/keys",
      schema: z.array(ApiKeySchema),
    });
    const target = before[0];
    expect(target.active).toBe(true);

    await registry.data.query({
      resource: "v1/api/keys/revoke",
      method: "POST",
      body: { key_id: target.id },
      schema: z.unknown(),
    });

    const after = await registry.data.query({
      resource: "v1/api/keys",
      schema: z.array(ApiKeySchema),
    });
    expect(after.find((key) => key.id === target.id)?.active).toBe(false);
  });
});

describe("createMockRegistry — BYOK (M4 register/list/delete flow)", () => {
  it("POST v1/byok/keys registers a provider and GET reflects it immediately", async () => {
    const registry = createMockRegistry();

    const before = await registry.data.query({
      resource: "v1/byok/keys",
      schema: ByokProvidersSchema,
    });
    expect(before.providers).not.toContain("mistral");

    await registry.data.query({
      resource: "v1/byok/keys",
      method: "POST",
      body: { provider: "mistral", api_key: "sk-fake" },
      schema: z.unknown(),
    });

    const after = await registry.data.query({
      resource: "v1/byok/keys",
      schema: ByokProvidersSchema,
    });
    expect(after.providers).toContain("mistral");
  });

  it("registering the same provider twice does not duplicate it", async () => {
    const registry = createMockRegistry();
    await registry.data.query({
      resource: "v1/byok/keys",
      method: "POST",
      body: { provider: "openai", api_key: "sk-fake" },
      schema: z.unknown(),
    });
    const after = await registry.data.query({
      resource: "v1/byok/keys",
      schema: ByokProvidersSchema,
    });
    expect(after.providers.filter((p) => p === "openai")).toHaveLength(1);
  });

  it("DELETE v1/byok/keys/:provider removes it from the list", async () => {
    const registry = createMockRegistry();
    await registry.data.query({
      resource: "v1/byok/keys/anthropic",
      method: "DELETE",
      schema: z.unknown(),
    });
    const after = await registry.data.query({
      resource: "v1/byok/keys",
      schema: ByokProvidersSchema,
    });
    expect(after.providers).not.toContain("anthropic");
  });
});

describe("createMockRegistry — auth", () => {
  it("getSession resolves a session for the backing mock user", async () => {
    const registry = createMockRegistry();
    const session = await registry.auth.getSession();
    expect(session).not.toBeNull();
    expect(session?.userId).toBe(mockUser.id);
  });

  it("the backing user fixture carries tenant_id + an owner role, even though AuthSession itself doesn't expose them", () => {
    expect(mockUser.tenant_id).toBe("org_meridian");
    expect(mockUser.roles).toContain("owner");
  });

  it("signIn and signUp both succeed", async () => {
    const registry = createMockRegistry();
    await expect(registry.auth.signIn({ email: "a@b.com", password: "x" })).resolves.toMatchObject({
      userId: mockUser.id,
    });
    await expect(
      registry.auth.signUp({ email: "a@b.com", username: "a", password: "x" }),
    ).resolves.toMatchObject({ userId: mockUser.id });
  });

  it("signOut resolves without throwing", async () => {
    const registry = createMockRegistry();
    await expect(registry.auth.signOut()).resolves.toBeUndefined();
  });
});

describe("createMockRegistry — admin control (PRD-24 Wave C2)", () => {
  it("POST /orgs/:id/suspend flips the org status in detail + directory and appends an audit row", async () => {
    const registry = createMockRegistry();

    const before = await registry.data.query({
      resource: "v1/admin/orgs/org_northwind",
      schema: AdminOrgDetailResponseSchema,
    });
    expect(before.org.status).toBe("active");

    const res = await registry.data.query({
      resource: "v1/admin/orgs/org_northwind/suspend",
      method: "POST",
      params: { reason: "abuse" },
      schema: AdminControlResponseSchema,
    });
    expect(res).toMatchObject({
      target_type: "org",
      target_id: "org_northwind",
      action: "suspend",
      status: "suspended",
      locked_until: null,
    });

    const detail = await registry.data.query({
      resource: "v1/admin/orgs/org_northwind",
      schema: AdminOrgDetailResponseSchema,
    });
    expect(detail.org.status).toBe("suspended");

    const directory = await registry.data.query({
      resource: "v1/admin/orgs",
      schema: AdminOrgsResponseSchema,
    });
    expect(directory.orgs.find((o) => o.id === "org_northwind")?.status).toBe("suspended");

    const audit = await registry.data.query({
      resource: "v1/admin/audit",
      schema: AdminAuditResponseSchema,
    });
    expect(audit.actions[0]).toMatchObject({
      action: "suspend",
      target_type: "org",
      target_id: "org_northwind",
      reason: "abuse",
    });
  });

  it("POST /orgs/:id/lock sets status locked with a future locked_until", async () => {
    const registry = createMockRegistry();
    const res = await registry.data.query({
      resource: "v1/admin/orgs/org_northwind/lock",
      method: "POST",
      params: { minutes: 30 },
      schema: AdminControlResponseSchema,
    });
    expect(res.status).toBe("locked");
    expect(res.locked_until).not.toBeNull();
    expect(new Date(res.locked_until as string).getTime()).toBeGreaterThan(Date.now());
  });

  it("POST /orgs/:id/reactivate restores a suspended org to active", async () => {
    const registry = createMockRegistry();
    const before = await registry.data.query({
      resource: "v1/admin/orgs/org_ghost", // suspended in the fixtures
      schema: AdminOrgDetailResponseSchema,
    });
    expect(before.org.status).toBe("suspended");

    await registry.data.query({
      resource: "v1/admin/orgs/org_ghost/reactivate",
      method: "POST",
      schema: AdminControlResponseSchema,
    });

    const after = await registry.data.query({
      resource: "v1/admin/orgs/org_ghost",
      schema: AdminOrgDetailResponseSchema,
    });
    expect(after.org.status).toBe("active");
  });

  it("POST /users/:id/suspend deactivates the user in the directory", async () => {
    const registry = createMockRegistry();
    const users = await registry.data.query({
      resource: "v1/admin/users",
      schema: AdminUsersResponseSchema,
    });
    const target = users.users[0];
    expect(target.active).toBe(true);

    const res = await registry.data.query({
      resource: `v1/admin/users/${target.id}/suspend`,
      method: "POST",
      schema: AdminControlResponseSchema,
    });
    expect(res).toMatchObject({ target_type: "user", status: "suspended", locked_until: null });

    const after = await registry.data.query({
      resource: "v1/admin/users",
      schema: AdminUsersResponseSchema,
    });
    expect(after.users.find((u) => u.id === target.id)?.active).toBe(false);
  });

  it("GET /orgs/:id/impact returns a dry-run WITHOUT mutating the org's status", async () => {
    const registry = createMockRegistry();
    const impact = await registry.data.query({
      resource: "v1/admin/orgs/org_northwind/impact",
      params: { window: 7 },
      schema: AdminImpactResponseSchema,
    });
    expect(impact.window).toBe(7);
    expect(impact.requests).toBeGreaterThan(0);
    expect(impact.sample.length).toBeGreaterThan(0);

    const detail = await registry.data.query({
      resource: "v1/admin/orgs/org_northwind",
      schema: AdminOrgDetailResponseSchema,
    });
    expect(detail.org.status).toBe("active"); // impact never writes
  });

  it("POST /users/:id/lock 404s — lock exists only for orgs/projects", async () => {
    const registry = createMockRegistry();
    const users = await registry.data.query({
      resource: "v1/admin/users",
      schema: AdminUsersResponseSchema,
    });
    await expect(
      registry.data.query({
        resource: `v1/admin/users/${users.users[0].id}/lock`,
        method: "POST",
        schema: AdminControlResponseSchema,
      }),
    ).rejects.toThrow(/404/);
  });
});

describe("createMockRegistry — llm.streamChat", () => {
  it("yields a canned reply word by word", async () => {
    const registry = createMockRegistry();
    const chunks: string[] = [];
    for await (const chunk of registry.llm.streamChat({
      projectId: "project-27",
      model: "openai/gpt-4o-mini",
      messages: [{ role: "user", content: "hello" }],
    })) {
      chunks.push(chunk);
    }
    expect(chunks.length).toBeGreaterThan(1);
    expect(chunks.join("")).toContain("hello");
  });
});
