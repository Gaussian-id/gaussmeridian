import { cn } from "@core/lib/utils";

import type { ComponentProps } from "react";

/** Generic label primitive. Associate it at the call site via `htmlFor` + a control `id`. */
export function Label({ className, children, ...props }: ComponentProps<"label">) {
  return (
    <label className={cn("text-sm font-medium", className)} {...props}>
      {children}
    </label>
  );
}
