"use client";

import { Trash2, X } from "lucide-react";
import { useState } from "react";

import { isAuthError } from "@core/adapters/auth-error";

import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { ConfirmDestructiveDialog } from "@/components/ui/confirm-destructive-dialog";
import {
  useAccountProfile,
  useCancelAccountDeletion,
  useRequestAccountDeletion,
} from "@/hooks/useAccount";

/** Human-readable message for the deletion-request failure modes the adapter seam actually
 *  returns. The real adapter maps a 404/405 (no backend endpoint yet — see
 *  `requestAccountDeletion`'s doc comment) to `deletion_request_unavailable`; that gets its own
 *  copy so the operator understands this is a known gap, not a bug, and never sees a raw
 *  "Request failed with 404". Mirrors `deleteOrgErrorMessage` in `orgs/danger-zone.tsx`. */
function deletionRequestErrorMessage(error: unknown): string {
  if (isAuthError(error)) {
    if (error.code === "deletion_request_unavailable") {
      return "Deletion requests aren't enabled on this server yet. Contact an administrator directly.";
    }
    if (error.status === 401) return "Your session has expired. Sign in again and retry.";
  }
  return "Could not submit your deletion request. Try again.";
}

/** Human-readable message for a cancel-request failure. A 404 means there was nothing pending
 *  to cancel (already resolved by an admin, or cancelled from another tab) — a benign race, not
 *  an error worth alarming over. */
function cancelDeletionErrorMessage(error: unknown): string {
  if (isAuthError(error) && error.status === 404) {
    return "There is no pending request to cancel — it may already have been resolved.";
  }
  return "Could not cancel your deletion request. Try again.";
}

/**
 * Request (or cancel) account deletion — /account/me's danger zone. Unlike org/project deletion
 * (`orgs/danger-zone.tsx`, `project-danger-zone.tsx`), this never deletes anything itself: it
 * files a *request* that an administrator reviews and fulfills via the PRD-23 Wave C `/admin`
 * surface. `profile.data.deletion_requested` (`GET /v1/auth/me`) is the source of truth for
 * whether a request is currently pending — while it's `true`, this renders the pending banner +
 * a Cancel request action (`DELETE v1/auth/me/deletion-request`) instead of the request trigger.
 */
export function AccountDangerZone() {
  const profile = useAccountProfile();
  const requestDeletion = useRequestAccountDeletion();
  const cancelDeletion = useCancelAccountDeletion();
  const [open, setOpen] = useState(false);

  const username = profile.data?.username;
  const isPending = profile.data?.deletion_requested === true;

  function handleConfirm() {
    requestDeletion.mutate(undefined, {
      onSuccess: () => setOpen(false),
    });
  }

  return (
    <Card className="border-destructive/40 flex flex-col gap-4 p-6">
      <div>
        <CardTitle className="text-destructive text-base">Danger zone</CardTitle>
        <CardDescription className="mt-1">
          Request permanent deletion of your account. This does not happen immediately.
        </CardDescription>
      </div>

      {requestDeletion.isSuccess && (
        <p role="status" className="text-accent text-sm">
          Deletion request submitted. An administrator will review and follow up.
        </p>
      )}

      {isPending ? (
        <div className="flex flex-col gap-3">
          <p role="status" className="text-destructive text-sm font-medium">
            Deletion requested — pending review.
          </p>
          {cancelDeletion.isError && (
            <p role="alert" className="text-destructive text-sm">
              {cancelDeletionErrorMessage(cancelDeletion.error)}
            </p>
          )}
          <Button
            type="button"
            variant="outline"
            className="self-start"
            disabled={cancelDeletion.isPending}
            onClick={() => cancelDeletion.mutate()}
          >
            <X className="h-4 w-4" aria-hidden="true" />
            {cancelDeletion.isPending ? "Cancelling…" : "Cancel request"}
          </Button>
        </div>
      ) : (
        username && (
          <ConfirmDestructiveDialog
            open={open}
            onOpenChange={setOpen}
            title="Request account deletion"
            resourceName={username}
            resourceLabel="account"
            confirmLabel={requestDeletion.isPending ? "Submitting…" : "Request deletion"}
            isBusy={requestDeletion.isPending}
            error={
              requestDeletion.isError ? deletionRequestErrorMessage(requestDeletion.error) : null
            }
            onConfirm={handleConfirm}
            description={
              <>
                This submits a request to permanently delete your account,{" "}
                <strong className="text-foreground">{username}</strong>. It is reviewed and
                fulfilled by an administrator — it does not happen instantly.
                <p className="text-muted-foreground mt-2 text-xs">
                  Once fulfilled, data removal is permanent and cannot be undone. You can keep using
                  your account until an administrator completes the request.
                </p>
              </>
            }
            trigger={
              <Button
                type="button"
                variant="outline"
                className="border-destructive text-destructive hover:bg-destructive/10 hover:text-destructive self-start"
                disabled={requestDeletion.isPending}
              >
                <Trash2 className="h-4 w-4" aria-hidden="true" />
                Request account deletion
              </Button>
            }
          />
        )
      )}
    </Card>
  );
}
