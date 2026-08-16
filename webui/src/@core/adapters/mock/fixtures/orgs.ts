import type { Org } from "@core/adapters/schemas/console.schema";

/**
 * `org_meridian` is fully populated (members, projects, route history). `org_born_empty`
 * exercises the "new tenants are born empty" locked decision — one member (its owner), zero
 * projects — so the empty-state surfaces (M2) have a real fixture to render against.
 */
export const orgs: Org[] = [
  {
    id: "org_meridian",
    name: "Meridian Labs",
    slug: "meridian-labs",
    plan: "pro",
    created_at: "2026-03-01T00:00:00Z",
    member_count: 3,
    project_count: 2,
  },
  {
    id: "org_born_empty",
    name: "Fresh Start Inc",
    slug: "fresh-start-inc",
    plan: "free",
    created_at: "2026-07-10T00:00:00Z",
    member_count: 1,
    project_count: 0,
  },
];
