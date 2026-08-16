"use client";

import Link from "next/link";

import { siteConfig } from "@core/config";
import { cn } from "@core/lib/utils";

import { ThemeToggle } from "@/components/theme/theme-toggle";
import { buttonVariants } from "@/components/ui/button";
import { useSession } from "@/hooks/useSession";

import { AccountMenu } from "./account-menu";

/**
 * Auth-aware navbar actions. Defaults to the signed-out CTAs (Sign in / Get API key) — a
 * stable default while `useSession()` resolves — then swaps to the Supabase-style avatar
 * `AccountMenu` once a session is confirmed. Prevents a logged-in user from being sent back
 * through `/login` (Bug #2). `AccountMenu` folds in what a plain "Console" link + the
 * standalone `ThemeToggle` used to cover here (Console is still its first item) plus Account
 * preferences / theme (light/dark/system) / Read changelogs / Sign out — see its own doc
 * comment. The standalone `ThemeToggle` stays for the signed-out state, which has no account
 * menu to fold theme control into.
 */
export function NavbarActions() {
  const { data: session } = useSession();

  if (session) {
    return (
      <div className="flex items-center gap-2">
        <AccountMenu />
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      {siteConfig.navbar.showThemeToggle && <ThemeToggle />}
      <Link
        href="/login"
        className="text-muted-foreground hover:text-foreground hidden text-sm font-medium sm:inline-flex"
      >
        Sign in
      </Link>
      <Link
        href="/signup"
        className={cn(buttonVariants({ variant: "brand", size: "sm" }), "hidden sm:inline-flex")}
      >
        Get API key
      </Link>
    </div>
  );
}
