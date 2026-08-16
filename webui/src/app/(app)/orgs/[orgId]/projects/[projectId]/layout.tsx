import type { ReactNode } from "react";

/**
 * Project-scoped segment. Presence of `projectId` alongside `orgId` at this point in the
 * route tree is what makes `TenancyProvider` (mounted once in `AppShell`) switch the sidebar
 * into project-mode — see the sibling `orgs/[orgId]/layout.tsx` comment for why no
 * server-side resolution happens here in Phase 1.
 */
export default function ProjectLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
