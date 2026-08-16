import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@core/lib/utils";

import type { ComponentProps } from "react";

/**
 * shadcn-style baseline button, themed to Gaussian tokens. Use `buttonVariants` to style
 * links (e.g. Next <Link>) without nesting a button. Deeper restyling is progressive.
 */
const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        accent: "bg-accent text-accent-foreground hover:brightness-110",
        brand: "bg-brand-gradient text-white hover:brightness-110",
        outline:
          "border border-border bg-transparent hover:bg-secondary hover:text-secondary-foreground",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        ghost: "hover:bg-secondary hover:text-secondary-foreground",
        /** Solid destructive accent — reserved for the armed confirm action inside
         *  `ConfirmDestructiveDialog` (irreversible deletes). Not for ordinary error states;
         *  those stay `outline` + `text-destructive` per the existing danger-zone convention. */
        destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
      },
      size: {
        sm: "h-8 px-3",
        md: "h-10 px-5",
        lg: "h-12 px-6 text-base",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: { variant: "default", size: "md" },
  },
);

export type ButtonProps = ComponentProps<"button"> & VariantProps<typeof buttonVariants>;

export function Button({ className, variant, size, ...props }: ButtonProps) {
  return <button className={cn(buttonVariants({ variant, size }), className)} {...props} />;
}

export { buttonVariants };
