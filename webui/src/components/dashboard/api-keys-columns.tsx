"use client";

import { ProjectApiKeySchema } from "@core/adapters/schemas/gaussmeridian.schema";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DataTableColumnHeader } from "@/components/ui/data-table";

import type { ColumnDef } from "@tanstack/react-table";
import type { z } from "zod";

type ApiKeyRow = z.infer<typeof ProjectApiKeySchema>;

interface ApiKeysColumnsOptions {
  /** Called with the full row when the "Revoke" action is clicked — opens the caller's
   *  confirmation dialog rather than mutating directly (see `keys/page.tsx`). */
  onRevoke: (key: ApiKeyRow) => void;
  /** The `id` of the key currently being revoked, if any — disables just that row's button. */
  pendingKeyId: string | null;
}

/**
 * Column defs for the `/dashboard/api-keys` `DataTable` — one column per field surfaced from
 * the project-scoped key endpoint. The response cannot contain a stored key hash or raw secret.
 */
export function createApiKeysColumns({
  onRevoke,
  pendingKeyId,
}: ApiKeysColumnsOptions): ColumnDef<ApiKeyRow>[] {
  return [
    {
      accessorKey: "key_prefix",
      header: "Key",
      cell: ({ row }) => (
        <code className="bg-secondary/40 rounded px-1.5 py-0.5 font-mono text-xs">
          {row.original.key_prefix}
        </code>
      ),
    },
    {
      accessorKey: "name",
      header: "Name",
      cell: ({ row }) => row.original.name ?? "—",
    },
    {
      accessorKey: "created_at",
      header: ({ column }) => <DataTableColumnHeader column={column} title="Created" />,
      cell: ({ row }) => new Date(row.original.created_at).toLocaleString(),
    },
    {
      accessorKey: "last_used_at",
      header: ({ column }) => <DataTableColumnHeader column={column} title="Last used" />,
      cell: ({ row }) =>
        row.original.last_used_at ? new Date(row.original.last_used_at).toLocaleString() : "Never",
    },
    {
      accessorKey: "active",
      header: "Status",
      cell: ({ row }) => (
        <Badge variant={row.original.active ? "outline" : "solid"}>
          {row.original.active ? "Active" : "Revoked"}
        </Badge>
      ),
    },
    {
      // The real scope of a key today: rate limits + expiry, both live `ApiKeySchema` fields.
      // Fine-grained allow/deny IAM (restrict a key to specific models/providers/CIDRs) has no
      // backing contract yet — that's a Phase-2 item, not represented here as a dummy panel.
      id: "scope",
      header: "Scope",
      cell: ({ row }) => {
        const { rate_limit_per_minute, rate_limit_per_day, expires_at } = row.original;
        const hasLimits = rate_limit_per_minute != null || rate_limit_per_day != null;
        return (
          <div className="flex flex-col gap-0.5">
            <span className="font-mono text-xs">
              {hasLimits
                ? `${rate_limit_per_minute ?? "∞"}/min · ${rate_limit_per_day ?? "∞"}/day`
                : "No rate limit set"}
            </span>
            <span className="text-muted-foreground text-xs">
              {expires_at ? `Expires ${new Date(expires_at).toLocaleDateString()}` : "No expiry"}
            </span>
          </div>
        );
      },
    },
    {
      id: "actions",
      header: () => <span className="sr-only">Actions</span>,
      cell: ({ row }) => {
        const keyId = row.original.id;
        if (!keyId) return null;
        const isPending = pendingKeyId === keyId;
        return (
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={!row.original.active || isPending}
            onClick={() => onRevoke(row.original)}
          >
            {isPending ? "Revoking…" : "Revoke"}
          </Button>
        );
      },
    },
  ];
}
