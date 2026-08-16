"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";

import { useAuth, useDataQuery } from "@core/adapters";
import { AccountProfileSchema } from "@core/adapters/schemas/account.schema";
import { ACCOUNT_ME_RESOURCE, ONBOARDING_PROFILE_RESOURCE } from "@core/config/resources";

import { useResourceQuery } from "./useResourceQuery";

/** `GET /v1/auth/me` — the full account profile for /account/me. Distinct from `useSession()`:
 *  the session shape stays intentionally slim (see `AuthSession.email`'s doc comment); this is
 *  the one place the PRD-21 Wave B profile fields (full name/display name/company/timezone) are
 *  read. */
export function useAccountProfile() {
  return useResourceQuery({ resource: ACCOUNT_ME_RESOURCE, schema: AccountProfileSchema });
}

/**
 * `PATCH /v1/onboarding/profile` — the onboarding wizard's profile-save endpoint, reused
 * unchanged for /account/me edits (`update_profile`'s own doc comment anticipates exactly this:
 * "a later 'complete later' edit from settings... works with the same endpoint"). A field
 * omitted from `input` leaves the stored value untouched (partial update).
 *
 * Deliberately a separate hook from onboarding's `useSaveProfile` rather than a shared one:
 * that hook has no cache invalidation (the wizard just advances to the next step on success),
 * while this one must invalidate `ACCOUNT_ME_RESOURCE` so the account page reflects the edit
 * immediately without a manual refetch.
 */
export function useUpdateAccountProfile() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      full_name?: string;
      display_name?: string;
      company?: string;
      timezone?: string;
    }) =>
      data.query({
        resource: ONBOARDING_PROFILE_RESOURCE,
        method: "PATCH",
        body: input,
        schema: AccountProfileSchema,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ACCOUNT_ME_RESOURCE, null] });
    },
  });
}

/**
 * Account-deletion request (`/account/me` danger zone). Goes through the `auth` adapter seam
 * (`requestAccountDeletion`), not `useDataQuery` — it's an account-identity action alongside
 * sign-in/sign-out/password-reset, not a resource CRUD call. The real backend has no lifecycle
 * for this yet (a superadmin PRD will own approval/fulfillment); the real adapter maps a
 * 404/405 to an honest "not enabled yet" `AuthError` rather than the UI pretending it worked.
 */
export function useRequestAccountDeletion() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => auth.requestAccountDeletion(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ACCOUNT_ME_RESOURCE, null] });
    },
  });
}

/**
 * Cancel the caller's own pending deletion request (PRD-23 Wave C — `DELETE
 * v1/auth/me/deletion-request`, mirrors `useRequestAccountDeletion` above). Invalidates
 * `ACCOUNT_ME_RESOURCE` so the danger zone's "pending review" state clears immediately — the
 * real backend recomputes `deletion_requested` from whether a pending row still exists, so a
 * refetch is enough; there's no separate flag to flip client-side.
 */
export function useCancelAccountDeletion() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => auth.cancelAccountDeletion(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ACCOUNT_ME_RESOURCE, null] });
    },
  });
}
