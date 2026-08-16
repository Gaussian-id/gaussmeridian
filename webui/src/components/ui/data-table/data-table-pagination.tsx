import { Button } from "@/components/ui/button";

import type { Table } from "@tanstack/react-table";

/** Prev/next pagination controls for a `DataTable`, styled to the Gaussian dashboard idiom. */
export function DataTablePagination<TData>({ table }: { table: Table<TData> }) {
  return (
    <div className="border-border flex items-center justify-end gap-3 border-t pt-4">
      <span className="text-muted-foreground font-mono text-xs">
        Page {table.getState().pagination.pageIndex + 1} of {Math.max(table.getPageCount(), 1)}
      </span>
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={() => table.previousPage()}
          disabled={!table.getCanPreviousPage()}
        >
          Previous
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => table.nextPage()}
          disabled={!table.getCanNextPage()}
        >
          Next
        </Button>
      </div>
    </div>
  );
}
