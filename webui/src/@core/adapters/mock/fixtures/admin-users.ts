/**
 * `GET /v1/admin/users` fixture — the global user directory. Reuses the same identities already
 * seeded in `members.ts` (`user_owner`/`user_admin`/`user_dev`) so the admin directory and the
 * org member list agree with each other, plus three standalone users with no org membership to
 * exercise onboarding-incomplete, deactivated, and (paired with `admin-deletion-requests.ts`)
 * deletion-pending states. `deletion_status` is deliberately NOT baked in here — the mock route
 * handler computes it from `store.deletionRequests` at read time, mirroring the real backend's
 * `deletion_repo.pending_for_user` per-row lookup (`list_admin_users`).
 */
export interface AdminUserOrgMembershipFixture {
  org_id: string;
  org_name: string;
  role: string;
}

export interface AdminUserFixture {
  id: string;
  email: string;
  username: string;
  created_at: string;
  active: boolean;
  onboarding_completed: boolean;
  orgs: AdminUserOrgMembershipFixture[];
  last_active_api: string | null;
  last_active_console: string | null;
}

export const adminUsers: AdminUserFixture[] = [
  {
    id: "user_owner",
    email: "owner@meridianlabs.dev",
    username: "ada.meridian",
    created_at: "2026-03-01T00:00:00Z",
    active: true,
    onboarding_completed: true,
    orgs: [
      { org_id: "org_meridian", org_name: "Meridian Labs", role: "owner" },
      { org_id: "org_born_empty", org_name: "Fresh Start Inc", role: "owner" },
    ],
    last_active_api: "2026-07-16T14:32:00Z",
    last_active_console: "2026-07-17T08:05:00Z",
  },
  {
    id: "user_admin",
    email: "admin@meridianlabs.dev",
    username: "bo.carter",
    created_at: "2026-03-03T00:00:00Z",
    active: true,
    onboarding_completed: true,
    orgs: [{ org_id: "org_meridian", org_name: "Meridian Labs", role: "admin" }],
    last_active_api: null,
    last_active_console: "2026-07-15T11:20:00Z",
  },
  {
    id: "user_dev",
    email: "dev@meridianlabs.dev",
    username: "cy.nolan",
    created_at: "2026-07-01T00:00:00Z",
    active: true,
    onboarding_completed: false,
    orgs: [{ org_id: "org_meridian", org_name: "Meridian Labs", role: "developer" }],
    last_active_api: null,
    last_active_console: null,
  },
  {
    id: "user_jordan",
    email: "jordan.churn@meridianlabs.dev",
    username: "jordan.churn",
    created_at: "2026-05-14T00:00:00Z",
    active: true,
    onboarding_completed: true,
    orgs: [],
    last_active_api: "2026-06-20T09:00:00Z",
    last_active_console: "2026-06-21T09:00:00Z",
  },
  {
    id: "user_sam",
    email: "sam.ito@meridianlabs.dev",
    username: "sam.ito",
    created_at: "2026-07-14T00:00:00Z",
    active: true,
    onboarding_completed: false,
    orgs: [],
    last_active_api: null,
    last_active_console: "2026-07-14T10:15:00Z",
  },
  {
    id: "user_taylor",
    email: "taylor.reed@meridianlabs.dev",
    username: "taylor.reed",
    created_at: "2026-04-02T00:00:00Z",
    active: false,
    onboarding_completed: true,
    orgs: [],
    last_active_api: "2026-05-01T00:00:00Z",
    last_active_console: null,
  },
];
