"use client";

import { Plus } from "lucide-react";
import Link from "next/link";
import { useParams } from "next/navigation";

import { cn } from "@core/lib/utils";

import { buttonVariants } from "@/components/ui/button";
import { ErrorState } from "@/components/ui/error-state";
import { Skeleton } from "@/components/ui/skeleton";
import { useOrgProjects } from "@/hooks/useConsoleQueries";

import { EmptyProjects } from "./empty-projects";
import { ProjectCard } from "./project-card";

/** Org Home: every project in this org, or the guided empty state when there are none. */
export function ProjectList() {
  const { orgId } = useParams<{ orgId: string }>();
  const projects = useOrgProjects(orgId);

  if (projects.isLoading) {
    return (
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <Skeleton key={i} className="h-40 w-full rounded-xl" />
        ))}
      </div>
    );
  }

  if (projects.isError || !projects.data) {
    return <ErrorState message="Could not load this organization's projects. Try again shortly." />;
  }

  if (projects.data.projects.length === 0) {
    return <EmptyProjects orgId={orgId} />;
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex justify-end">
        <Link
          href={`/orgs/${orgId}/projects/new`}
          className={cn(buttonVariants({ variant: "accent" }))}
        >
          <Plus className="h-4 w-4" aria-hidden="true" />
          New project
        </Link>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {projects.data.projects.map((project) => (
          <ProjectCard key={project.id} orgId={orgId} project={project} />
        ))}
      </div>
    </div>
  );
}
