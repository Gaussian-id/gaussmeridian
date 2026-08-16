"use client";

import { UserPlus } from "lucide-react";
import { useParams } from "next/navigation";
import { useMemo, useState } from "react";

import type { Role } from "@core/adapters/schemas/console.schema";
import type { Member } from "@core/adapters/schemas/console.schema";
import { canManageMembers } from "@core/lib/rbac";
import { useTenancy } from "@core/providers";

import { Button } from "@/components/ui/button";
import { ConfirmDestructiveDialog } from "@/components/ui/confirm-destructive-dialog";
import { DataTable } from "@/components/ui/data-table";
import {
  useInviteMember,
  useOrgMembers,
  useRemoveMember,
  useUpdateMemberRole,
} from "@/hooks/useConsoleQueries";

import { InviteMemberDialog } from "./invite-member-dialog";
import { createMembersColumns } from "./members-columns";

/**
 * Team & Members: everyone with access to this org, their role, and an invite flow.
 *
 * Invite and role-change are gated to Owner/Admin (`canManageMembers`) — the real backend's
 * RBAC rules 403 a Developer attempting either, so both are hidden/disabled here up front
 * rather than surfacing that 403 after the fact.
 */
export function MembersTable() {
  const { orgId } = useParams<{ orgId: string }>();
  const { role } = useTenancy();
  const members = useOrgMembers(orgId);
  const inviteMember = useInviteMember(orgId);
  const updateRole = useUpdateMemberRole(orgId);
  const removeMember = useRemoveMember(orgId);
  const permitted = canManageMembers(role);

  const [isInviteOpen, setIsInviteOpen] = useState(false);
  const [removeTarget, setRemoveTarget] = useState<Member | null>(null);

  function handleRoleChange(userId: string, nextRole: Role) {
    updateRole.mutate({ userId, role: nextRole });
  }

  function handleInvite(input: { email: string; role: Role }) {
    inviteMember.mutate(input, { onSuccess: () => setIsInviteOpen(false) });
  }

  const columns = useMemo(
    () =>
      createMembersColumns({
        onRoleChange: handleRoleChange,
        onRemove: (member) => {
          removeMember.reset();
          setRemoveTarget(member);
        },
        pendingRoleUserId: updateRole.isPending ? (updateRole.variables?.userId ?? null) : null,
        pendingRemoveUserId: removeMember.isPending ? (removeMember.variables ?? null) : null,
        canManageMembers: permitted,
        callerRole: role,
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- handleRoleChange is stable per render intent, deps below cover the values it closes over
    [
      updateRole.isPending,
      updateRole.variables,
      removeMember.isPending,
      removeMember.variables,
      orgId,
      permitted,
      role,
    ],
  );

  const assignableRoles: readonly Role[] =
    role === "owner" ? ["developer", "admin", "owner"] : ["developer", "admin"];

  return (
    <div className="flex flex-col gap-4">
      {permitted && (
        <div className="flex justify-end">
          <InviteMemberDialog
            open={isInviteOpen}
            onOpenChange={setIsInviteOpen}
            isPending={inviteMember.isPending}
            isError={inviteMember.isError}
            onInvite={handleInvite}
            assignableRoles={assignableRoles}
            trigger={
              <Button type="button">
                <UserPlus className="h-4 w-4" aria-hidden="true" />
                Invite member
              </Button>
            }
          />
        </div>
      )}

      <DataTable
        columns={columns}
        data={members.data?.members ?? []}
        isLoading={members.isLoading}
        isError={members.isError}
        errorMessage="Could not load this organization's members. Try again shortly."
        emptyMessage="No members yet."
      />

      {updateRole.isError && (
        <p role="alert" className="text-destructive text-sm">
          Could not change this member&apos;s role. Refresh the list and try again.
        </p>
      )}

      <ConfirmDestructiveDialog
        open={removeTarget !== null}
        onOpenChange={(open) => {
          if (!open) setRemoveTarget(null);
        }}
        title="Remove organization member"
        description={
          <>
            <strong className="text-foreground">
              {removeTarget?.display_name ?? removeTarget?.email}
            </strong>{" "}
            will immediately lose access to this organization and its projects.
          </>
        }
        confirmLabel={removeMember.isPending ? "Removing…" : "Remove member"}
        isBusy={removeMember.isPending}
        error={removeMember.isError ? "Could not remove this member. Try again." : null}
        onConfirm={() => {
          if (!removeTarget) return;
          removeMember.mutate(removeTarget.user_id, {
            onSuccess: () => setRemoveTarget(null),
          });
        }}
      />
    </div>
  );
}
