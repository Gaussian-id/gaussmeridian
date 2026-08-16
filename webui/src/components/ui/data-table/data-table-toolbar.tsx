import type { ReactNode } from "react";

/** Optional filter/action bar rendered above a `DataTable`. Renders nothing when empty. */
export function DataTableToolbar({ children }: { children?: ReactNode }) {
  if (!children) return null;
  return <div className="flex flex-wrap items-center gap-2 pb-4">{children}</div>;
}
