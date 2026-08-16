"use client";

import { useState } from "react";

interface UseServerWindowOptions {
  /** Rows per server window — the `limit` the caller passes to its resource query. */
  pageSize: number;
}

export interface ServerWindow {
  /** The current window's start offset — feed this to the query as `start`. */
  start: number;
  /** The configured window size — feed this to the query as `limit`. */
  pageSize: number;
  /** Advance to the next window. */
  next: () => void;
  /** Go back one window (never below 0). */
  prev: () => void;
  /** Jump back to the first window — call this whenever the query's filters change. */
  reset: () => void;
  /** Whether a previous window exists (`start > 0`). */
  canPrev: boolean;
  /** Whether a next window exists — pass the current `total` and how many rows loaded. */
  canNext: (total: number, loaded: number) => boolean;
  /** 1-based index of the first row shown, given `total` (0 when empty). */
  rangeStart: (total: number) => number;
  /** 1-based index of the last row shown, given how many rows loaded. */
  rangeEnd: (loaded: number) => number;
}

/**
 * Page-owned "Prev/Next window" pagination over a server that returns one `start`/`limit` slice
 * at a time — extracted from the bespoke logic in `admin/users/page.tsx`. This deliberately does
 * NOT add `manualPagination` to the shared `DataTable` primitive (which has three other
 * client-paginated consumers): the page varies `start`, feeds each loaded window to a plain
 * `DataTable`, and `DataTable`'s own client-side pager chunks whatever window is loaded — correct
 * because it's never asked to represent more than the currently loaded slice.
 *
 * `canNext`/`rangeStart`/`rangeEnd` take `total`/`loaded` as arguments rather than closing over
 * the query result, so this hook stays decoupled from any specific response shape.
 */
export function useServerWindow({ pageSize }: UseServerWindowOptions): ServerWindow {
  const [start, setStart] = useState(0);

  return {
    start,
    pageSize,
    next: () => setStart((current) => current + pageSize),
    prev: () => setStart((current) => Math.max(0, current - pageSize)),
    reset: () => setStart(0),
    canPrev: start > 0,
    canNext: (total, loaded) => start + loaded < total,
    rangeStart: (total) => (total === 0 ? 0 : start + 1),
    rangeEnd: (loaded) => start + loaded,
  };
}
