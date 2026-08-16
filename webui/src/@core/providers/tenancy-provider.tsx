"use client";

import { useParams } from "next/navigation";
import { createContext, useContext, useMemo, type ReactNode } from "react";

import { GaussMeridianAdapterError } from "@core/adapters/gaussmeridian-data.adapter";
import type { Org, Project, Role } from "@core/adapters/schemas/console.schema";

import { useOrg, useOrgMembers, useProject } from "@/hooks/useConsoleQueries";
import { useSession } from "@/hooks/useSession";

export type TenancyMode = "global" | "org" | "project";
export type TenancyRoleStatus = "not-applicable" | "loading" | "resolved" | "denied" | "error";

export interface Tenancy {
  org?: Org;
  project?: Project;
  /** The current user's org-level RBAC role (owner/admin/developer). Undefined in global
   *  mode, or while membership hasn't resolved yet. */
  role?: Role;
  /** Explicit resolution state for fail-closed privileged screens. */
  roleStatus: TenancyRoleStatus;
  retryRole(): void;
  /** Derived from which id params are present in the URL: no `orgId` -> "global", `orgId`
   *  only -> "org", both `orgId` and `projectId` -> "project". */
  mode: TenancyMode;
}

const TenancyContext = createContext<Tenancy | null>(null);

/**
 * Resolves org/project/role context from the URL for every screen under `(app)`. Reads
 * `orgId`/`projectId` off `useParams()`, fetches the matching org/project (mock-backed in
 * Phase 1), and derives the caller's role by cross-referencing the org's member list against
 * the current session.
 *
 * `AuthSession` deliberately carries no `tenant_id`/`role` (see `@core/adapters/types.ts`) —
 * RBAC is a property of org membership, not of the session, so it must be looked up per org.
 * The match is exclusively `member.user_id === session.userId`. An unmatched session is denied;
 * it is never presented as the organization's owner.
 *
 * Mounted once, inside `AppShell`, above the sidebar/topbar — every authenticated screen sits
 * beneath it.
 */
export function TenancyProvider({ children }: { children: ReactNode }) {
  const params = useParams<{ orgId?: string; projectId?: string }>();
  const orgId = typeof params.orgId === "string" ? params.orgId : undefined;
  const projectId = typeof params.projectId === "string" ? params.projectId : undefined;

  const session = useSession();
  const org = useOrg(orgId ?? "");
  const project = useProject(orgId ?? "", projectId ?? "");
  const members = useOrgMembers(orgId ?? "");

  const role = useMemo<Role | undefined>(() => {
    const roster = members.data?.members;
    if (!roster || roster.length === 0) return undefined;
    const userId = session.data?.userId;
    return userId ? roster.find((member) => member.user_id === userId)?.role : undefined;
  }, [members.data?.members, session.data?.userId]);

  const mode: TenancyMode = projectId ? "project" : orgId ? "org" : "global";
  const membershipDenied =
    members.error instanceof GaussMeridianAdapterError && members.error.status === 403;
  const roleStatus: TenancyRoleStatus = !orgId
    ? "not-applicable"
    : session.isLoading || members.isLoading
      ? "loading"
      : membershipDenied
        ? "denied"
        : session.isError || members.isError
          ? "error"
          : role
            ? "resolved"
            : "denied";

  const value: Tenancy = {
    org: org.data,
    project: project.data,
    role,
    roleStatus,
    retryRole() {
      void session.refetch();
      void members.refetch();
    },
    mode,
  };

  return <TenancyContext.Provider value={value}>{children}</TenancyContext.Provider>;
}

export function useTenancy(): Tenancy {
  const ctx = useContext(TenancyContext);
  if (!ctx) {
    throw new Error("useTenancy must be used within a <TenancyProvider>");
  }
  return ctx;
}
