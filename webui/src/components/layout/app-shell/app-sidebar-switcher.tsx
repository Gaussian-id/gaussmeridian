"use client";

import { Building2, FolderPlus, Plus } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";

import type { Tenancy } from "@core/providers";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useOrgProjects, useOrgs } from "@/hooks/useConsoleQueries";

/**
 * Org/project switcher rendered above the nav in org- and project-mode. Lets the operator
 * jump between orgs (and, in project-mode, between projects within the current org) or start
 * a new one, without leaving the sidebar. Hidden in global mode — there's no org/project to
 * switch between on the Org Chooser or a global surface like Playground.
 */
export function AppSidebarSwitcher({
  mode,
  org,
  project,
}: Pick<Tenancy, "mode" | "org" | "project">) {
  const router = useRouter();
  const orgs = useOrgs();
  const projects = useOrgProjects(org?.id ?? "");

  if (mode === "global") return null;

  return (
    <div className="border-border flex flex-col gap-2 border-b px-3 py-3">
      <div className="flex items-center gap-2">
        <Select value={org?.id ?? ""} onValueChange={(orgId) => router.push(`/orgs/${orgId}`)}>
          <SelectTrigger size="sm" className="w-full" aria-label="Switch organization">
            <Building2 className="h-4 w-4 shrink-0" aria-hidden="true" />
            <SelectValue placeholder="Select organization" />
          </SelectTrigger>
          <SelectContent>
            {orgs.data?.orgs.map((candidate) => (
              <SelectItem key={candidate.id} value={candidate.id}>
                {candidate.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Link
          href="/orgs/new"
          aria-label="Create organization"
          className="text-muted-foreground hover:text-foreground shrink-0"
        >
          <Plus className="h-4 w-4" aria-hidden="true" />
        </Link>
      </div>

      {mode === "project" && org && (
        <div className="flex items-center gap-2">
          <Select
            value={project?.id ?? ""}
            onValueChange={(projectId) => router.push(`/orgs/${org.id}/projects/${projectId}`)}
          >
            <SelectTrigger size="sm" className="w-full" aria-label="Switch project">
              <SelectValue placeholder="Select project" />
            </SelectTrigger>
            <SelectContent>
              {projects.data?.projects.map((candidate) => (
                <SelectItem key={candidate.id} value={candidate.id}>
                  {candidate.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Link
            href={`/orgs/${org.id}/projects/new`}
            aria-label="Create project"
            className="text-muted-foreground hover:text-foreground shrink-0"
          >
            <FolderPlus className="h-4 w-4" aria-hidden="true" />
          </Link>
        </div>
      )}
    </div>
  );
}
