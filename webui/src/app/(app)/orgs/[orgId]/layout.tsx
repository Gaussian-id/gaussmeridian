import type { ReactNode } from "react";

/**
 * Org-scoped segment. Presence of `orgId` at this point in the route tree is what makes it
 * available app-wide via `useParams()` — `TenancyProvider` (mounted once in `AppShell`) reads
 * it reactively and switches the sidebar into org-mode. No server-side org resolution happens
 * here in Phase 1: the adapters are a client-only seam (mock-backed for now), so there's
 * nothing this layout could fetch that `TenancyProvider` doesn't already fetch on the client.
 */
export default function OrgLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
