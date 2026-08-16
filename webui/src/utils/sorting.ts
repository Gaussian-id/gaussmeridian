/** The minimal shape of a TanStack Table row these comparators depend on. */
interface SortableRow {
  getValue<TValue = unknown>(columnId: string): TValue;
}

export const caseInsensitiveSort = (rowA: SortableRow, rowB: SortableRow, columnId: string) => {
  const a = String(rowA.getValue(columnId) || "").toLowerCase();
  const b = String(rowB.getValue(columnId) || "").toLowerCase();

  return a < b ? -1 : a > b ? 1 : 0;
};

export const numericSort = (rowA: SortableRow, rowB: SortableRow, columnId: string) => {
  const valueA = rowA.getValue<number>(columnId);
  const valueB = rowB.getValue<number>(columnId);

  return valueA - valueB;
};
