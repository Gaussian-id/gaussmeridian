"use client";

import { RequestLogSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import { Badge } from "@/components/ui/badge";
import { DataTableColumnHeader } from "@/components/ui/data-table";

import type { ColumnDef } from "@tanstack/react-table";
import type { z } from "zod";

type LogRow = z.infer<typeof RequestLogSchema>;

/** Column defs for the `/dashboard/logs` `DataTable` — one column per field surfaced from
 *  `/v1/logs` (the outcome-billing ledger). The Outcome column is driven by `r_binary`:
 *  requests that failed outcome validation are charged $0, and the UI says so plainly. */
export const logsColumns: ColumnDef<LogRow>[] = [
  {
    accessorKey: "created_at",
    header: ({ column }) => <DataTableColumnHeader column={column} title="Time" />,
    cell: ({ row }) =>
      row.original.created_at ? new Date(row.original.created_at).toLocaleString() : "—",
  },
  {
    accessorKey: "model",
    header: "Model",
    // A GaussMoA run is billed as one aggregate row with model/provider "gaussmoa" — badge it
    // so multi-agent requests are visible at a glance.
    cell: ({ row }) =>
      row.original.provider === "gaussmoa" ? (
        <span className="flex items-center gap-2">
          <Badge variant="outline">MoA</Badge>
          <span className="text-muted-foreground text-xs">mixture-of-agents</span>
        </span>
      ) : (
        row.original.model
      ),
  },
  {
    accessorKey: "provider",
    header: "Provider",
  },
  {
    accessorKey: "r_binary",
    header: "Outcome",
    cell: ({ row }) => (
      <Badge variant={row.original.r_binary === 1 ? "outline" : "solid"}>
        {row.original.r_binary === 1 ? "Charged" : "Not charged"}
      </Badge>
    ),
  },
  {
    accessorKey: "validator_result",
    header: "Validation",
    cell: ({ row }) => <span className="font-mono text-xs">{row.original.validator_result}</span>,
  },
  {
    id: "tokens",
    header: "Tokens",
    cell: ({ row }) => `${row.original.tokens_in} in / ${row.original.tokens_out} out`,
  },
  {
    accessorKey: "latency_ms",
    header: ({ column }) => <DataTableColumnHeader column={column} title="Latency" />,
    cell: ({ row }) => `${row.original.latency_ms}ms`,
  },
  {
    accessorKey: "cost_charged",
    header: "Cost",
    cell: ({ row }) => `$${row.original.cost_charged.toFixed(4)}`,
  },
];
