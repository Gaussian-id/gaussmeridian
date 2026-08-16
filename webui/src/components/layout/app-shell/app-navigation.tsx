"use client";

import Link from "next/link";
import { useParams, usePathname } from "next/navigation";

import {
  ADMIN_NAV_HREF,
  globalNav,
  orgNav,
  projectNav,
  resolveActiveHref,
  type NavGroup,
} from "@core/config";
import { cn } from "@core/lib/utils";
import { useTenancy } from "@core/providers";

import { useIsSuperadmin } from "@/hooks/useAdminQueries";

import { AppSidebarSwitcher } from "./app-sidebar-switcher";

interface AppNavigationProps {
  ariaLabel?: string;
  onNavigate?: () => void;
}

/**
 * The authenticated product-navigation module shared by the permanent desktop sidebar and
 * the mobile sheet. Keeping route selection, active-state resolution, tenant switching, and
 * superadmin filtering here prevents the two responsive surfaces from drifting apart.
 */
export function AppNavigation({ ariaLabel = "Application", onNavigate }: AppNavigationProps) {
  const pathname = usePathname();
  const params = useParams<{ orgId?: string; projectId?: string }>();
  const { mode, org, project } = useTenancy();
  const isSuperadmin = useIsSuperadmin();

  const orgId = typeof params.orgId === "string" ? params.orgId : undefined;
  const projectId = typeof params.projectId === "string" ? params.projectId : undefined;

  const rawNavGroups: NavGroup[] =
    mode === "project" && orgId && projectId
      ? projectNav(orgId, projectId)
      : mode === "org" && orgId
        ? orgNav(orgId)
        : globalNav();

  const navGroups: NavGroup[] = isSuperadmin
    ? rawNavGroups
    : rawNavGroups.map((navGroup) => ({
        ...navGroup,
        items: navGroup.items.filter((item) => item.href !== ADMIN_NAV_HREF),
      }));

  const activeHref = resolveActiveHref(
    pathname,
    navGroups.flatMap((navGroup) => navGroup.items.map((item) => item.href)),
  );

  return (
    <>
      <AppSidebarSwitcher mode={mode} org={org} project={project} />

      <nav className="flex flex-1 flex-col gap-4 overflow-y-auto px-3 py-3" aria-label={ariaLabel}>
        {navGroups.map((navGroup) => (
          <div key={navGroup.label} className="flex flex-col gap-1">
            <span className="text-muted-foreground px-3 text-xs font-medium tracking-wide uppercase">
              {navGroup.label}
            </span>
            {navGroup.items.map((item) => {
              const Icon = item.icon;
              const active = item.href === activeHref;
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  onClick={onNavigate}
                  aria-current={active ? "page" : undefined}
                  className={cn(
                    "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                    active
                      ? "bg-secondary text-secondary-foreground"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  <Icon className="h-4 w-4" aria-hidden="true" />
                  {item.label}
                </Link>
              );
            })}
          </div>
        ))}
      </nav>
    </>
  );
}
