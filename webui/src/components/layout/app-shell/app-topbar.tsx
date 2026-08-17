"use client";

import { Search } from "lucide-react";
import Link from "next/link";

import { BrandLogo } from "@/components/brand";
import { useCommandPaletteTrigger } from "@/components/command";
import { AccountMenu } from "@/components/layout/navbar/account-menu";

import { AppMobileNavigation } from "./app-mobile-navigation";

/** The "search" affordance is the ⌘K command palette's trigger, not a search field of its own —
 *  clicking or focusing it opens the palette rather than accepting typed input inline. The
 *  top-right slot is the same `AccountMenu` the marketing navbar uses once signed in (avatar,
 *  Console, Account preferences, inline theme switcher, Read changelogs, Sign out) — the single
 *  account surface for the whole app, so it replaces the standalone `ThemeToggle` this topbar
 *  used to render on its own. */
export function AppTopbar() {
  const palette = useCommandPaletteTrigger();

  return (
    <header className="border-border bg-background/70 sticky top-0 z-40 flex h-16 items-center gap-2 border-b px-3 backdrop-blur sm:gap-4 sm:px-6">
      <AppMobileNavigation />
      {/* The sidebar that carries the brand is hidden below `md`, which left the console with no
          identity at all on a phone. The mark fills that gap without crowding the topbar. */}
      <Link
        href="/"
        className="focus-visible:ring-ring flex shrink-0 items-center rounded-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none md:hidden"
      >
        <BrandLogo variant="mark" height={26} />
      </Link>
      <button
        type="button"
        onClick={palette.open}
        onFocus={palette.open}
        aria-label="Open command palette"
        className="border-input bg-background text-muted-foreground hover:text-foreground focus-visible:ring-ring focus-visible:ring-offset-background relative flex h-10 w-full max-w-md items-center gap-2 rounded-md border px-3 text-sm transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
      >
        <Search className="h-4 w-4 shrink-0" aria-hidden="true" />
        <span className="flex-1 text-left">Search…</span>
        <kbd className="border-border bg-secondary text-muted-foreground hidden shrink-0 items-center gap-0.5 rounded border px-1.5 py-0.5 font-mono text-[10px] sm:inline-flex">
          ⌘K
        </kbd>
      </button>
      <div className="ml-auto flex items-center gap-2">
        <AccountMenu />
      </div>
    </header>
  );
}
