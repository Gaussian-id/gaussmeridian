"use client";

import Link from "next/link";

import { BrandLogo } from "@/components/brand";

import { AppNavigation } from "./app-navigation";

/** No longer owns a bottom profile/sign-out block — `AppTopbar`'s `AccountMenu` avatar is now
 *  the single account surface for the console, so this stays nav-only (the `flex-1` nav list
 *  fills the height the old block used to occupy). */
export function AppSidebar() {
  return (
    <aside className="bg-card border-border hidden w-60 shrink-0 flex-col border-r md:flex">
      <div className="flex h-16 items-center px-6">
        <Link
          href="/"
          className="focus-visible:ring-ring flex items-center rounded-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
        >
          <BrandLogo height={32} />
        </Link>
      </div>

      <AppNavigation />
    </aside>
  );
}
