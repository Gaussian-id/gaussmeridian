"use client";

import { Trash2 } from "lucide-react";
import { useParams, useRouter } from "next/navigation";
import { useState } from "react";

import { GaussMeridianAdapterError } from "@core/adapters/gaussmeridian-data.adapter";
import { canDeleteOrg } from "@core/lib/rbac";
import { useTenancy } from "@core/providers";

import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { ConfirmDestructiveDialog } from "@/components/ui/confirm-destructive-dialog";
import { useDeleteOrg } from "@/hooks/useConsoleQueries";

/** Human-readable message for the org-deletion failure modes the backend actually returns.
 *  Never surfaces the raw adapter error — mirrors `registerErrorMessage` in `byok/page.tsx`. */
function deleteOrgErrorMessage(error: unknown): string {
  if (error instanceof GaussMeridianAdapterError) {
    if (error.status === 403) return "Only the organization owner can delete this organization.";
    if (error.status === 404) return "This organization was already deleted.";
  }
  return "Could not delete the organization. Try again.";
}

/**
 * Irreversible org deletion, gated behind `ConfirmDestructiveDialog`'s typed confirmation (the
 * caller must type the exact org name to arm the delete button — GitHub-style). Deleting
 * cascades to every project, API key, and membership on the mock backend (see
 * `createMockRegistry`'s `DELETE v1/orgs/:orgId` handler) — the consequences copy says so
 * explicitly rather than hiding the blast radius.
 *
 * Gated to the Owner tier only (`canDeleteOrg`) — the real backend's RBAC rules reserve
 * delete/transfer org for Owner and would 403 an Admin or Developer attempt. Rather than let
 * that round-trip happen, the button is disabled up front with an explanatory line.
 */
export function DangerZone() {
  const { orgId } = useParams<{ orgId: string }>();
  const { org, role } = useTenancy();
  const router = useRouter();
  const deleteOrg = useDeleteOrg(orgId);
  const permitted = canDeleteOrg(role);

  const [open, setOpen] = useState(false);

  function handleConfirm() {
    deleteOrg.mutate(undefined, {
      onSuccess: () => {
        setOpen(false);
        router.push("/orgs");
      },
    });
  }

  return (
    <Card className="border-destructive/40 flex flex-col gap-4 p-6">
      <div>
        <CardTitle className="text-destructive text-base">Danger zone</CardTitle>
        <CardDescription className="mt-1">
          Deleting this organization removes every project, API key, member, and route history it
          holds. There is no undo.
        </CardDescription>
      </div>

      {!permitted && role && (
        <p className="text-muted-foreground text-sm">
          Only the organization owner can delete this organization.
        </p>
      )}

      {org && (
        <ConfirmDestructiveDialog
          open={open}
          onOpenChange={setOpen}
          title="Delete organization"
          resourceName={org.name}
          resourceLabel="organization"
          confirmLabel={deleteOrg.isPending ? "Deleting…" : "Delete organization"}
          isBusy={deleteOrg.isPending}
          error={deleteOrg.isError ? deleteOrgErrorMessage(deleteOrg.error) : null}
          onConfirm={handleConfirm}
          description={
            <>
              This permanently deletes <strong className="text-foreground">{org.name}</strong>,
              including:
              <ul className="mt-2 list-disc pl-5">
                <li>Every project under this organization</li>
                <li>Every API key issued under those projects</li>
                <li>Every member&apos;s access (memberships)</li>
                <li>All route/request history</li>
              </ul>
              This cannot be undone.
            </>
          }
          trigger={
            <Button
              type="button"
              variant="outline"
              className="border-destructive text-destructive hover:bg-destructive/10 hover:text-destructive self-start"
              disabled={!permitted || deleteOrg.isPending}
            >
              <Trash2 className="h-4 w-4" aria-hidden="true" />
              Delete organization
            </Button>
          }
        />
      )}
    </Card>
  );
}
