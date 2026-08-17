import { z } from "zod";

/**
 * `GET /v1/auth/me` and `PATCH /v1/onboarding/profile` (reused post-onboarding, see
 * `ONBOARDING_PROFILE_RESOURCE`'s doc comment) both return the backend's full `PublicUser`
 * (handlers.rs). `.passthrough()` because that record carries fields this page doesn't render
 * (`roles`, `tenant_id`, `active`, …) — this schema only asserts what /account/me actually
 * reads or edits: identity (read-only here) and the PRD-21 Wave B profile fields (editable).
 */
export const AccountProfileSchema = z
  .object({
    id: z.string(),
    email: z.string(),
    username: z.string(),
    full_name: z.string().nullable().optional(),
    display_name: z.string().nullable().optional(),
    company: z.string().nullable().optional(),
    timezone: z.string().nullable().optional(),
    // PRD-23 Wave C — additive on `PublicUser` (handlers.rs). `.default(false)` so a fixture
    // or cached response that predates this field never wrongly hides an active pending
    // request; mirrors `onboarding_completed`'s identical default in
    // `gaussmeridian-auth.adapter.ts`'s `UserSchema`.
    deletion_requested: z.boolean().default(false),
  })
  .passthrough();

export type AccountProfile = z.infer<typeof AccountProfileSchema>;
