"use client";

import { MemberSchema } from "@core/adapters/schemas/console.schema";
import type { Role } from "@core/adapters/schemas/console.schema";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTableColumnHeader } from "@/components/ui/data-table";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import type { ColumnDef } from "@tanstack/react-table";
import type { z } from "zod";

type MemberRow = z.infer<typeof MemberSchema>;

const ROLE_OPTIONS: { value: Role; label: string }[] = [
  { value: "owner", label: "Owner" },
  { value: "admin", label: "Admin" },
  { value: "developer", label: "Developer" },
];

interface MembersColumnsOptions {
  /** Called with the row's `id` and the newly selected role. */
  onRoleChange: (userId: string, role: Role) => void;
  onRemove: (member: MemberRow) => void;
  /** The canonical user id currently being updated, if any. */
  pendingRoleUserId: string | null;
  pendingRemoveUserId: string | null;
  /** Whether the caller's own org role permits changing OTHER members' roles (Owner/Admin
   *  tiers only — see `canManageMembers` in `@core/lib/rbac.ts`). When `false`, the role cell
   *  renders as a read-only badge instead of an editable `Select`, matching what the real
   *  backend would 403 on anyway. */
  canManageMembers: boolean;
  callerRole: Role | undefined;
}

/**
 * Column defs for the Team & Members `DataTable` — mirrors `createApiKeysColumns`'s shape
 * (options object with an action callback + a pending-row id).
 */
export function createMembersColumns({
  onRoleChange,
  onRemove,
  pendingRoleUserId,
  pendingRemoveUserId,
  canManageMembers,
  callerRole,
}: MembersColumnsOptions): ColumnDef<MemberRow>[] {
  return [
    {
      accessorKey: "display_name",
      header: ({ column }) => <DataTableColumnHeader column={column} title="Member" />,
      cell: ({ row }) => (
        <div>
          <p className="font-medium">{row.original.display_name}</p>
          <p className="text-muted-foreground text-xs">{row.original.email}</p>
        </div>
      ),
    },
    {
      accessorKey: "role",
      header: "Role",
      cell: ({ row }) => {
        const userId = row.original.user_id;
        const isPending = pendingRoleUserId === userId;
        const isOrganizationOwner = row.original.role === "owner";
        if (!canManageMembers || isOrganizationOwner) {
          return (
            <Badge variant="outline">
              {ROLE_OPTIONS.find((option) => option.value === row.original.role)?.label ??
                row.original.role}
            </Badge>
          );
        }
        return (
          <Select
            value={row.original.role}
            onValueChange={(value) => onRoleChange(userId, value as Role)}
            disabled={isPending}
          >
            <SelectTrigger
              size="sm"
              className="w-[130px]"
              aria-label={`Role for ${row.original.email}`}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ROLE_OPTIONS.filter(
                (option) => callerRole === "owner" || option.value !== "owner",
              ).map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        );
      },
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => (
        <Badge variant={row.original.status === "active" ? "outline" : "solid"}>
          {row.original.status === "active" ? "Active" : "Invited"}
        </Badge>
      ),
    },
    {
      id: "since",
      header: ({ column }) => <DataTableColumnHeader column={column} title="Since" />,
      accessorFn: (row) => row.joined_at ?? row.invited_at ?? "",
      cell: ({ row }) => {
        const date = row.original.joined_at ?? row.original.invited_at;
        return date ? new Date(date).toLocaleDateString() : "—";
      },
    },
    {
      id: "actions",
      header: () => <span className="sr-only">Actions</span>,
      cell: ({ row }) => {
        const member = row.original;
        if (!canManageMembers || member.role === "owner") return null;
        const isPending = pendingRemoveUserId === member.user_id;
        return (
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isPending}
            onClick={() => onRemove(member)}
          >
            {isPending ? "Removing…" : "Remove"}
          </Button>
        );
      },
    },
  ];
}
