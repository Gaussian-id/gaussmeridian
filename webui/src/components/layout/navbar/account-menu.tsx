"use client";

import { LogOut, Megaphone, Monitor, Moon, Shield, Sun, User } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useTheme } from "next-themes";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useIsSuperadmin } from "@/hooks/useAdminQueries";
import { useSession } from "@/hooks/useSession";
import { useSignOut } from "@/hooks/useSignOut";

/**
 * Supabase-style account menu: a single circular avatar (the user's initial, neutral tokens —
 * never the brand gradient reserved for primary CTAs) replacing the navbar's separate
 * Console-link + theme-toggle controls once a session exists (`NavbarActions`/`NavbarMobile`).
 * Folds in what those two controls covered plus the new asks: Account preferences (-> /account/me,
 * where password/profile/danger-zone live), an inline theme switcher (light/dark/system — the
 * `useTheme()` logic `theme-toggle.tsx` used, absorbed here rather than duplicated as a
 * standalone control), and Read changelogs. This same component is also `AppTopbar`'s account
 * surface (the in-console chrome) — `theme-toggle.tsx` itself stays only because the signed-out
 * marketing navbar (`NavbarActions`) still renders its own copy, with no account menu to fold
 * theme control into.
 *
 * The theme `DropdownMenuRadioGroup` only exists inside `DropdownMenuContent`, which Radix
 * mounts on open (not at first paint), so reading `theme` here never risks the SSR/client
 * hydration mismatch `ThemeToggle`'s dual-icon trick works around for its always-rendered button.
 */
export function AccountMenu() {
  const { data: session } = useSession();
  const { theme, setTheme } = useTheme();
  const signOut = useSignOut();
  // PRD-23 Wave C — reads the same cached `GET /v1/admin/me` probe `SuperadminGate`/
  // `AppSidebar` read; gated here (not in a static config) since this menu is hand-assembled,
  // not data-driven from `nav.config.ts`.
  const isSuperadmin = useIsSuperadmin();
  // The admin console is a self-contained operator surface: inside it we show no path back to the
  // tenant app — no "Console → /orgs", no "Account preferences" (that page still lives in the tenant
  // shell), no redundant "Admin" jump. Only theme, changelog, and sign-out remain.
  const inAdmin = usePathname()?.startsWith("/admin") ?? false;

  if (!session) return null;

  const initial = session.displayName.charAt(0).toUpperCase();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={`Account menu for ${session.displayName}`}
          className="bg-secondary text-secondary-foreground border-border focus-visible:ring-ring focus-visible:ring-offset-background grid h-9 w-9 shrink-0 place-items-center rounded-full border text-sm font-semibold focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
        >
          {initial}
        </button>
      </DropdownMenuTrigger>

      <DropdownMenuContent>
        <DropdownMenuLabel className="flex flex-col gap-0.5 font-normal">
          <span className="text-foreground truncate text-sm font-medium">
            {session.displayName}
          </span>
          {session.email && (
            <span className="text-muted-foreground truncate text-xs">{session.email}</span>
          )}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />

        {!inAdmin && (
          <DropdownMenuItem asChild>
            <Link href="/orgs">
              <User aria-hidden="true" />
              Console
            </Link>
          </DropdownMenuItem>
        )}
        {!inAdmin && (
          <DropdownMenuItem asChild>
            <Link href="/account/me">
              <User aria-hidden="true" />
              Account preferences
            </Link>
          </DropdownMenuItem>
        )}
        {isSuperadmin && !inAdmin && (
          <DropdownMenuItem asChild>
            <Link href="/admin">
              <Shield aria-hidden="true" />
              Admin
            </Link>
          </DropdownMenuItem>
        )}

        <DropdownMenuSeparator />

        <DropdownMenuLabel>Theme</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={theme ?? "system"} onValueChange={setTheme}>
          <DropdownMenuRadioItem value="light">
            <Sun aria-hidden="true" className="size-4" />
            Light
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="dark">
            <Moon aria-hidden="true" className="size-4" />
            Dark
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="system">
            <Monitor aria-hidden="true" className="size-4" />
            System
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>

        <DropdownMenuSeparator />

        <DropdownMenuItem asChild>
          <Link href="/changelog">
            <Megaphone aria-hidden="true" />
            Read changelogs
          </Link>
        </DropdownMenuItem>

        <DropdownMenuSeparator />

        <DropdownMenuItem
          variant="destructive"
          disabled={signOut.isPending}
          onSelect={() => signOut.mutate()}
        >
          {/* `text-destructive` also opts the icon out of the menu's default gray-icon rule
              (`[&_svg:not([class*='text-'])]`), so the icon turns red with the text, not just the label. */}
          <LogOut aria-hidden="true" className="text-destructive" />
          {signOut.isPending ? "Signing out…" : "Sign out"}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
