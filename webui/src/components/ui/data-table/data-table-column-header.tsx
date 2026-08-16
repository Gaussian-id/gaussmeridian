import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";

import { cn } from "@core/lib/utils";

import { Button } from "@/components/ui/button";

import type { Column } from "@tanstack/react-table";

/** Sortable column header for a `DataTable`. Falls back to a plain label when sorting is disabled. */
export function DataTableColumnHeader<TData, TValue>({
  column,
  title,
}: {
  column: Column<TData, TValue>;
  title: string;
}) {
  if (!column.getCanSort()) {
    return <span className="text-foreground">{title}</span>;
  }

  const sorted = column.getIsSorted();
  const Icon = sorted === "asc" ? ArrowUp : sorted === "desc" ? ArrowDown : ArrowUpDown;

  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn("-ml-3 h-8 gap-1.5 px-3", sorted ? "text-accent" : "text-muted-foreground")}
      onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
    >
      {title}
      <Icon className="h-4 w-4" aria-hidden="true" />
    </Button>
  );
}
