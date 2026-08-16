import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@core/lib/utils";

import type { ComponentProps } from "react";

const badgeVariants = cva(
  "inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-medium",
  {
    variants: {
      variant: {
        outline: "border-border text-muted-foreground",
        solid: "border-transparent bg-secondary text-secondary-foreground",
        mono: "border-border text-muted-foreground font-mono tracking-wider uppercase",
      },
    },
    defaultVariants: { variant: "outline" },
  },
);

export type BadgeProps = ComponentProps<"span"> & VariantProps<typeof badgeVariants>;

export function Badge({ className, variant, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />;
}
