/**
 * The single signed-in identity behind every mock session. Shaped like the real backend's
 * `GaussMeridianUser` (see `gaussmeridian-auth.adapter.ts`) so a future swap to the live auth
 * adapter needs no consumer changes. `AuthSession` (the adapter's actual return type) has no
 * `tenant_id`/`roles` fields — those live here, on the backing fixture, for the mock's own
 * use (e.g. matching this user up with the owner `Member` row in `members.ts`).
 */
export interface MockUser {
  id: string;
  email: string;
  username: string;
  tenant_id: string;
  roles: string[];
  created_at: string;
  active: boolean;
  // PRD-21 Wave B / DR-010 D3 profile fields — mirrors `PublicUser` (handlers.rs) so the
  // /account/me surface (and the onboarding profile step) can be exercised through the mock
  // registry exactly like the real `GET /v1/auth/me`.
  full_name: string | null;
  display_name: string | null;
  company: string | null;
  timezone: string | null;
  // PRD-23 Wave C — mirrors `PublicUser.deletion_requested` (handlers.rs). Toggled by the mock
  // auth adapter's `requestAccountDeletion`/`cancelAccountDeletion`, read back by `GET
  // v1/auth/me` (the `/account/me` danger zone's pending-state banner).
  deletion_requested: boolean;
}

export const mockUser: MockUser = {
  id: "user_owner",
  email: "owner@meridianlabs.dev",
  username: "ada.meridian",
  tenant_id: "org_meridian",
  roles: ["owner"],
  created_at: "2026-03-01T00:00:00Z",
  active: true,
  full_name: "Ada Meridian",
  display_name: "Ada",
  company: "Meridian Labs",
  timezone: "America/New_York",
  deletion_requested: false,
};
