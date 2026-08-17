import { z } from "zod";

/**
 * Phase-2 Wave-2 REAL backend DTOs for the org/project/member/role layer — traced against the
 * ground-truth endpoint/DTO spec handed down for this session (backend committed at `8fb7646`
 * in the sibling `gaussmeridian` repo: org/membership/role schema + repos + seed). These schemas
 * validate the raw wire shape BEFORE any anti-corruption mapping runs (`console.mapper.ts`),
 * exactly the same "validate untrusted JSON, then reshape" two-step Wave-1 established for the
 * generic `gaussmeridian-data.adapter.ts` (`schema.parse` at the fetch boundary).
 *
 * Do NOT import these into UI components — they are adapter-internal. Components only ever see
 * `console.schema.ts` (Org/Project/Member/Role), which stays the deep, stable interface.
 */

// ---- Org ----

export const OrgPlanResponseSchema = z.enum(["free", "pro", "enterprise"]);
export const OrgStatusResponseSchema = z.enum(["active", "suspended", "deleted"]);

export const OrgResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  slug: z.string(),
  plan: OrgPlanResponseSchema,
  owner_user: z.string(),
  balance: z.number(),
  currency: z.string(),
  status: OrgStatusResponseSchema,
  created_at: z.string(),
  updated_at: z.string(),
  // Wave-2 gap-closure (backend commit 8fb7646): real aggregate counts, no more N+1 workaround.
  member_count: z.number(),
  project_count: z.number(),
});
export type OrgResponse = z.infer<typeof OrgResponseSchema>;

export const CreateOrgRequestSchema = z.object({
  name: z.string(),
  slug: z.string(),
  plan: OrgPlanResponseSchema.optional(),
});
export type CreateOrgRequest = z.infer<typeof CreateOrgRequestSchema>;

export const UpdateOrgRequestSchema = z.object({
  name: z.string().optional(),
  plan: OrgPlanResponseSchema.optional(),
  status: OrgStatusResponseSchema.optional(),
});
export type UpdateOrgRequest = z.infer<typeof UpdateOrgRequestSchema>;

// ---- Project ----

export const OrgProjectResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  org_id: z.string(),
  lambda: z.number(),
  quality_floor: z.number(),
  tau_moa: z.number(),
  budget_monthly: z.number().nullable(),
  hard_limit: z.boolean(),
  alert_webhook_url: z.string().nullable(),
  validator_type: z.string(),
  // Wave-2 gap-closure (backend commit 8fb7646): real creation timestamp, no more synthesis.
  created_at: z.string(),
});
export type OrgProjectResponse = z.infer<typeof OrgProjectResponseSchema>;

export const CreateOrgProjectRequestSchema = z.object({ name: z.string() });
export type CreateOrgProjectRequest = z.infer<typeof CreateOrgProjectRequestSchema>;

export const UpdateOrgProjectRequestSchema = z.object({
  name: z.string().optional(),
  lambda: z.number().optional(),
  quality_floor: z.number().optional(),
  budget_monthly: z.number().optional(),
  hard_limit: z.boolean().optional(),
  alert_webhook_url: z.string().optional(),
});
export type UpdateOrgProjectRequest = z.infer<typeof UpdateOrgProjectRequestSchema>;

// ---- Member ----

export const MemberStatusResponseSchema = z.enum(["active", "invited"]);

/**
 * Wave-2 gap-closure (backend commit 8fb7646): `email`/`display_name` are now real fields.
 * Modeled as nullable (not just optional-absent) rather than required strings — the console
 * mapper still falls back to a truncated `user_id` for the rare member whose identity fields
 * come back null (e.g. an account mid-deprovision), so a null here is a normal, handled case,
 * not a contract violation.
 */
export const MemberResponseSchema = z.object({
  id: z.string(),
  org_id: z.string(),
  user_id: z.string(),
  role_id: z.string(),
  status: MemberStatusResponseSchema,
  created_at: z.string(),
  email: z.string().nullable(),
  display_name: z.string().nullable(),
});
export type MemberResponse = z.infer<typeof MemberResponseSchema>;

export const InviteMemberRequestSchema = z.object({ email: z.string(), role_id: z.string() });
export type InviteMemberRequest = z.infer<typeof InviteMemberRequestSchema>;

export const UpdateMemberRoleRequestSchema = z.object({ role_id: z.string() });
export type UpdateMemberRoleRequest = z.infer<typeof UpdateMemberRoleRequestSchema>;

// ---- Role ----

/**
 * `permissions` is the raw backend permission-string vocabulary — NOT cross-verified against
 * this session's ground truth (only the coarse Owner/Admin/Developer capability tiers were
 * specified, not the exact string constants). `console.mapper.ts` uses `name` to resolve the
 * fixed 3-tier `Role` enum and derives `PermissionMatrix` grants from the documented tiers
 * rather than trusting unverified permission strings — see the mapper's file-level comment.
 */
export const RoleResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  permissions: z.array(z.string()),
});
export type RoleResponse = z.infer<typeof RoleResponseSchema>;
