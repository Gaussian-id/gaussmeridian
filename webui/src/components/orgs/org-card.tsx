"use client";

import { Building2 } from "lucide-react";
import Link from "next/link";

import type { Org } from "@core/adapters/schemas/console.schema";

import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useOrgMembers } from "@/hooks/useConsoleQueries";
import { useSession } from "@/hooks/useSession";

const PLAN_LABEL: Record<Org["plan"], string> = {
  free: "Free",
  pro: "Pro",
  enterprise: "Enterprise",
};

const ROLE_LABEL: Record<string, string> = {
  owner: "Owner",
  admin: "Admin",
  developer: "Developer",
};

/**
 * One org tile in the chooser grid. Resolves "your role" the same way `TenancyProvider`
 * does for the single active org (match by `user_id`, fall back to the org's owner) — but
 * here for N orgs at once, so it can't reuse `useTenancy()`, which is scoped to the URL's
 * current `orgId`.
 */
export function OrgCard({ org, href = `/orgs/${org.id}` }: { org: Org; href?: string }) {
  const session = useSession();
  const members = useOrgMembers(org.id);

  const roster = members.data?.members;
  const userId = session.data?.userId;
  const matched = userId ? roster?.find((member) => member.user_id === userId) : undefined;
  const role = (matched ?? roster?.find((member) => member.role === "owner"))?.role;

  return (
    <Link href={href} className="group block">
      <Card className="hover:border-accent/60 hover:shadow-glow flex h-full flex-col gap-4 p-5 transition-colors">
        <div className="flex items-start justify-between gap-3">
          <div className="bg-secondary flex h-10 w-10 shrink-0 items-center justify-center rounded-lg">
            <Building2 className="text-accent h-5 w-5" aria-hidden="true" />
          </div>
          <Badge variant="mono">{PLAN_LABEL[org.plan]}</Badge>
        </div>

        <div>
          <h3 className="font-display group-hover:text-accent text-lg font-semibold tracking-tight transition-colors">
            {org.name}
          </h3>
          <p className="text-muted-foreground mt-0.5 font-mono text-xs">{org.slug}</p>
        </div>

        <div className="border-border mt-auto flex items-center justify-between border-t pt-4 text-sm">
          <span className="text-muted-foreground">
            {org.project_count} {org.project_count === 1 ? "project" : "projects"} ·{" "}
            {org.member_count} {org.member_count === 1 ? "member" : "members"}
          </span>
          {members.isLoading ? (
            <Skeleton className="h-5 w-14" />
          ) : role ? (
            <Badge variant="outline">{ROLE_LABEL[role] ?? role}</Badge>
          ) : null}
        </div>
      </Card>
    </Link>
  );
}
