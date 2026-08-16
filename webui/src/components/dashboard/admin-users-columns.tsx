"use client";

import type { AdminUser } from "@core/adapters/schemas/admin.schema";

import { Badge } from "@/components/ui/badge";
import { DataTableColumnHeader } from "@/components/ui/data-table";

import type { ColumnDef } from "@tanstack/react-table";

/** Coarse relative-time label for "last active" cells. No shared formatter exists elsewhere in
 *  the codebase (grepped) — kept local since it's presentation-only and only this column needs
 *  it. Falls back to a plain date past 30 days rather than an ever-growing "312d ago". */
function relativeTime(iso: string | null): string {
  if (!iso) return "Never";
  const diffMs = Date.now() - new Date(iso).getTime();
  const diffMin = Math.round(diffMs / 60_000);
  if (diffMin < 1) return "Just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHour = Math.round(diffMin / 60);
  if (diffHour < 24) return `${diffHour}h ago`;
  const diffDay = Math.round(diffHour / 24);
  if (diffDay < 30) return `${diffDay}d ago`;
  return new Date(iso).toLocaleDateString();
}

/** Column defs for the `/admin/users` `DataTable` over `AdminUser` rows (PRD-23 Wave C).
 *  Pattern: `route-decision-columns.tsx`. */
export const adminUsersColumns: ColumnDef<AdminUser>[] = [
  {
    accessorKey: "email",
    header: ({ column }) => <DataTableColumnHeader column={column} title="User" />,
    cell: ({ row }) => (
      <div className="flex flex-col">
        <span className="text-foreground font-medium">{row.original.email}</span>
        <span className="text-muted-foreground text-xs">{row.original.username}</span>
      </div>
    ),
  },
  {
    id: "orgs",
    header: "Organizations",
    cell: ({ row }) => {
      const orgs = row.original.orgs;
      if (orgs.length === 0) return <span className="text-muted-foreground">—</span>;
      return (
        <div className="flex flex-wrap gap-1.5">
          {orgs.map((org) => (
            <Badge key={org.org_id} variant="solid">
              {org.org_name} · {org.role}
            </Badge>
          ))}
        </div>
      );
    },
  },
  {
    accessorKey: "created_at",
    header: ({ column }) => <DataTableColumnHeader column={column} title="Created" />,
    cell: ({ row }) => new Date(row.original.created_at).toLocaleDateString(),
  },
  {
    id: "last_active",
    header: "Last active",
    cell: ({ row }) => (
      <div className="flex flex-col text-xs">
        <span>API: {relativeTime(row.original.last_active_api)}</span>
        <span className="text-muted-foreground">
          Console: {relativeTime(row.original.last_active_console)}
        </span>
      </div>
    ),
  },
  {
    id: "status",
    header: "Status",
    cell: ({ row }) => {
      const user = row.original;
      return (
        <div className="flex flex-wrap gap-1.5">
          <Badge variant={user.active ? "outline" : "solid"}>
            {user.active ? "Active" : "Deactivated"}
          </Badge>
          {!user.onboarding_completed && <Badge variant="mono">Onboarding</Badge>}
          {user.deletion_status === "pending" && (
            <Badge className="border-destructive/40 text-destructive">Deletion pending</Badge>
          )}
        </div>
      );
    },
  },
];
