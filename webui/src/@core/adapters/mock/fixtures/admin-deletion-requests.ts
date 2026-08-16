/**
 * `GET /v1/admin/deletion-requests` seed data — one pending request (`user_jordan`, exercised
 * by `/admin/deletions`'s default Pending tab) and one already-resolved pair (fulfilled /
 * rejected) so the history tabs aren't empty on first render. `store.deletionRequests` in
 * `create-mock-registry.ts` clones this array and mutates it in place from there — this module
 * itself is never mutated (same "clone the fixture, mutate the store" convention every other
 * mock fixture follows).
 */
export interface AdminDeletionRequestFixture {
  id: string;
  user_id: string;
  email: string | null;
  username: string | null;
  status: "pending" | "fulfilled" | "rejected";
  note: string | null;
  requested_at: string;
  resolved_at: string | null;
  resolved_by: string | null;
}

export const adminDeletionRequests: AdminDeletionRequestFixture[] = [
  {
    id: "delreq_jordan",
    user_id: "user_jordan",
    email: "jordan.churn@meridianlabs.dev",
    username: "jordan.churn",
    status: "pending",
    note: null,
    requested_at: "2026-07-15T16:40:00Z",
    resolved_at: null,
    resolved_by: null,
  },
  {
    id: "delreq_taylor",
    user_id: "user_taylor",
    email: "taylor.reed@meridianlabs.dev",
    username: "taylor.reed",
    status: "fulfilled",
    note: null,
    requested_at: "2026-05-01T09:00:00Z",
    resolved_at: "2026-05-03T12:00:00Z",
    resolved_by: "owner@meridianlabs.dev",
  },
  {
    id: "delreq_declined",
    user_id: "user_admin",
    email: "admin@meridianlabs.dev",
    username: "bo.carter",
    status: "rejected",
    note: "Still the sole billing contact for Meridian Labs — asked them to transfer ownership first.",
    requested_at: "2026-04-10T08:30:00Z",
    resolved_at: "2026-04-11T09:15:00Z",
    resolved_by: "owner@meridianlabs.dev",
  },
];
