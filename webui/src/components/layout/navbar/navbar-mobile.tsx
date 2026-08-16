"use client";

import { Menu, X } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

import { navConfig } from "@core/config";
import { cn } from "@core/lib/utils";

import { Button, buttonVariants } from "@/components/ui/button";
import { useSession } from "@/hooks/useSession";

/**
 * Mobile navigation: a hamburger that drops the full menu below the bar. Auth-aware in the
 * same way as `NavbarActions` — signed-out CTAs (Sign in / Get API key) live in this panel
 * because `NavbarActions` hides its own copies below `sm:`. Once `useSession()` confirms a
 * session there is nothing to add here: `NavbarActions`' `AccountMenu` avatar renders in the
 * header row at every breakpoint (no `hidden sm:` class, unlike the old "Console" link it
 * replaced), so this panel just shows the plain nav items for a signed-in visitor.
 */
export function NavbarMobile() {
  const [open, setOpen] = useState(false);
  const close = () => setOpen(false);
  const { data: session } = useSession();

  return (
    <div className="md:hidden">
      <Button
        variant="ghost"
        size="icon"
        aria-label="Toggle menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {open ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
      </Button>
      {open && (
        <div className="glass border-border absolute inset-x-0 top-16 z-40 flex flex-col gap-1 border-b p-4">
          {navConfig.main.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              onClick={close}
              className="text-foreground hover:bg-secondary rounded-md px-3 py-2.5 text-base font-medium"
            >
              {item.label}
            </Link>
          ))}
          {!session && (
            <>
              <Link
                href="/login"
                onClick={close}
                className="text-muted-foreground rounded-md px-3 py-2.5 text-base font-medium"
              >
                Sign in
              </Link>
              <Link
                href="/signup"
                onClick={close}
                className={cn(buttonVariants({ variant: "brand", size: "md" }), "mt-2 w-full")}
              >
                Get API key
              </Link>
            </>
          )}
        </div>
      )}
    </div>
  );
}
