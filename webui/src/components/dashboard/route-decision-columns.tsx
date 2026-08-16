"use client";

import type { RouteDecision } from "@core/adapters/schemas/console.schema";

import {
  complexityBand,
  deliveredBy,
  guardrailLabel,
} from "@/components/overview/route-decision-utils";
import { Badge } from "@/components/ui/badge";
import { DataTableColumnHeader } from "@/components/ui/data-table";

import type { ColumnDef } from "@tanstack/react-table";

/** Column defs for the `/activity` `DataTable` over real `RouteDecision` rows (PRD-21 Wave C).
 *  Replaces the earlier cast onto `logsColumns` (`RequestLogSchema`-typed): the real
 *  `route_decision` row has no model/provider/cost_charged/r_binary of its own — see
 *  `console.schema.ts`'s `RouteDecisionSchema` doc comment — so those columns don't apply here.
 *  The delivered model/provider and guardrail outcome are the closest real analogs. */
export const routeDecisionColumns: ColumnDef<RouteDecision>[] = [
  {
    accessorKey: "created_at",
    header: ({ column }) => <DataTableColumnHeader column={column} title="Time" />,
    cell: ({ row }) => new Date(row.original.created_at).toLocaleString(),
  },
  {
    id: "model",
    header: "Model",
    cell: ({ row }) => {
      const delivered = deliveredBy(row.original);
      return delivered ? `${delivered.model} → ${delivered.provider}` : "—";
    },
  },
  {
    accessorKey: "guardrail_status",
    header: "Guardrail",
    cell: ({ row }) => (
      <Badge variant={row.original.guardrail_status === "passed" ? "outline" : "solid"}>
        {guardrailLabel(row.original.guardrail_status)}
      </Badge>
    ),
  },
  {
    id: "complexity",
    header: "Complexity",
    cell: ({ row }) =>
      `${row.original.complexity.toFixed(2)} · ${complexityBand(row.original.complexity)}`,
  },
  {
    accessorKey: "cascade_used",
    header: "Cascade",
    cell: ({ row }) => (row.original.cascade_used ? "Used" : "Not used"),
  },
  {
    accessorKey: "baseline_cost",
    header: "Baseline cost",
    cell: ({ row }) => `$${row.original.baseline_cost.toFixed(4)}`,
  },
];
