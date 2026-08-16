interface ErrorStateProps {
  message: string;
}

/** Inline error message shown in place of a failed query's data — used by DataTable and any
 *  other view that needs the same "query failed" treatment outside a table. */
export function ErrorState({ message }: ErrorStateProps) {
  return (
    <div className="border-destructive/40 bg-destructive/5 text-destructive rounded-xl border p-4 text-sm">
      {message}
    </div>
  );
}
