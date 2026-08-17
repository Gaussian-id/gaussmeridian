import {
  BarChart3,
  BookOpen,
  Building2,
  CreditCard,
  KeyRound,
  LayoutDashboard,
  ListOrdered,
  MessageSquare,
  Package,
  Settings,
  Shield,
  ShieldCheck,
  Users,
} from "lucide-react";

import type { LucideIcon } from "lucide-react";

export interface NavItem {
  label: string;
  href: string;
  external?: boolean;
}

/** Data-driven public navigation. The marketing Navbar renders whatever is configured here. */
export const navConfig: { main: NavItem[] } = {
  main: [
    { label: "Home", href: "/" },
    { label: "Story", href: "/story" },
    { label: "Solutions", href: "/solutions" },
    { label: "Changelog", href: "/changelog" },
    { label: "Docs", href: "/docs" },
  ],
};

/** One entry in a grouped console nav (`orgNav`/`projectNav`/`globalNav`) — unlike the flat
 *  `NavItem` above, every entry owns its icon and the section (`group`) it renders under. */
export interface ConsoleNavItem {
  label: string;
  href: string;
  icon: LucideIcon;
  group: string;
}

/** A rendered section of the console sidebar: a group heading + its items. */
export interface NavGroup {
  label: string;
  items: ConsoleNavItem[];
}

function group(label: string, items: ConsoleNavItem[]): NavGroup {
  return { label, items };
}

/** Org-mode sidebar: membership/RBAC and org-level billing/settings. Projects live under
 *  the org, but project navigation is `projectNav` below — a project has settings only, no
 *  membership of its own. */
export function orgNav(orgId: string): NavGroup[] {
  const base = `/orgs/${orgId}`;
  return [
    group("Workspace", [
      { label: "Overview", href: base, icon: LayoutDashboard, group: "Workspace" },
      { label: "Members", href: `${base}/members`, icon: Users, group: "Workspace" },
      { label: "Roles", href: `${base}/roles`, icon: ShieldCheck, group: "Workspace" },
    ]),
    group("Account", [
      { label: "Settings", href: `${base}/settings`, icon: Settings, group: "Account" },
    ]),
  ];
}

/** Project-mode sidebar: routing overview, catalog, credentials, and project settings. */
export function projectNav(orgId: string, projectId: string): NavGroup[] {
  const base = `/orgs/${orgId}/projects/${projectId}`;
  return [
    group("Monitor", [
      { label: "Overview", href: base, icon: LayoutDashboard, group: "Monitor" },
      { label: "Activity", href: `${base}/activity`, icon: ListOrdered, group: "Monitor" },
      { label: "Usage", href: `${base}/usage`, icon: BarChart3, group: "Monitor" },
    ]),
    group("Build", [
      { label: "Playground", href: `${base}/playground`, icon: MessageSquare, group: "Build" },
      { label: "Models", href: `${base}/models`, icon: Package, group: "Build" },
      { label: "API keys", href: `${base}/keys`, icon: KeyRound, group: "Build" },
    ]),
    group("Configure", [
      { label: "BYOK", href: `${base}/byok`, icon: KeyRound, group: "Configure" },
      { label: "Settings", href: `${base}/settings`, icon: Settings, group: "Configure" },
    ]),
  ];
}

/** The `/admin` nav item's href — a stable reference so the rendering component (`AppSidebar`)
 *  can identify and filter it without a magic string. See `globalNav`'s doc comment for why
 *  filtering happens there, not here. */
export const ADMIN_NAV_HREF = "/admin";

/**
 * Sidebar fallback for `global` mode (Org Chooser, Create Org, embedded Docs) — routes that
 * aren't scoped to an org or project.
 *
 * There is deliberately no Playground here. The Playground is project-scoped
 * (`projectNav`'s Build group): it needs a project's API key, budget and model catalog to route
 * against, none of which exist at the global level.
 *
 * PRD-23 Wave C: this unconditionally includes the Admin item — `globalNav()` is a plain,
 * static function (no hooks, called from render), so it cannot itself know whether the current
 * caller is an allowlisted superadmin. `AppSidebar` (the one renderer of this list) filters the
 * item out via `useIsSuperadmin()` before rendering; `account-menu.tsx`'s Admin item is gated
 * the same way, independently (it doesn't read this list at all).
 */
export function globalNav(): NavGroup[] {
  return [
    group("Global", [
      { label: "Organizations", href: "/orgs", icon: Building2, group: "Global" },
      { label: "Docs", href: "/docs", icon: BookOpen, group: "Global" },
      { label: "Admin", href: ADMIN_NAV_HREF, icon: Shield, group: "Global" },
    ]),
  ];
}

/**
 * Longest-prefix-match active resolution for a set of nav hrefs against the current
 * pathname: exact match wins; otherwise the href with the longest strict-prefix match
 * (`pathname === href || pathname.startsWith(href + "/")`) wins. This matters because a
 * group's "Overview" href (e.g. `/orgs/x/projects/y`) is itself a prefix of every other item
 * in that group (`/orgs/x/projects/y/models`, `/orgs/x/projects/y/models/z`, ...) — without
 * longest-match, a model-detail URL would incorrectly highlight Overview instead of Models.
 * Returns null when no href matches.
 */
export function resolveActiveHref(pathname: string, hrefs: string[]): string | null {
  let best: string | null = null;
  for (const href of hrefs) {
    const isMatch = pathname === href || pathname.startsWith(`${href}/`);
    if (!isMatch) continue;
    if (best === null || href.length > best.length) best = href;
  }
  return best;
}
