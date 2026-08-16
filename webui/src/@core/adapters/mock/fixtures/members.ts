import type { Member } from "@core/adapters/schemas/console.schema";

/**
 * `org_meridian` covers all three roles and both member statuses (active + invited).
 * `org_born_empty` has only its owner — the same identity as `mockUser` (see `user.ts`),
 * consistent with "new tenants are born empty."
 */
export const members: Member[] = [
  {
    id: "mem_owner",
    org_id: "org_meridian",
    user_id: "user_owner",
    email: "owner@meridianlabs.dev",
    display_name: "Ada Meridian",
    role: "owner",
    status: "active",
    invited_at: null,
    joined_at: "2026-03-01T00:00:00Z",
  },
  {
    id: "mem_admin",
    org_id: "org_meridian",
    user_id: "user_admin",
    email: "admin@meridianlabs.dev",
    display_name: "Bo Carter",
    role: "admin",
    status: "active",
    invited_at: "2026-03-02T00:00:00Z",
    joined_at: "2026-03-03T00:00:00Z",
  },
  {
    id: "mem_dev_invited",
    org_id: "org_meridian",
    user_id: "user_dev",
    email: "dev@meridianlabs.dev",
    display_name: "Cy Nolan",
    role: "developer",
    status: "invited",
    invited_at: "2026-07-01T00:00:00Z",
    joined_at: null,
  },
  {
    id: "mem_empty_owner",
    org_id: "org_born_empty",
    user_id: "user_owner",
    email: "owner@meridianlabs.dev",
    display_name: "Ada Meridian",
    role: "owner",
    status: "active",
    invited_at: null,
    joined_at: "2026-07-10T00:00:00Z",
  },
];
