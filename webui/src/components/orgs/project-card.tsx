"use client";

import { Boxes } from "lucide-react";
import Link from "next/link";

import type { Project } from "@core/adapters/schemas/console.schema";

import { Badge } from "@/components/ui/badge";
import { Card } from "@/components/ui/card";

const ENVIRONMENT_VARIANT: Record<Project["environment"], "solid" | "outline"> = {
  production: "solid",
  development: "outline",
};

/** One project tile in `ProjectList`. Links into the project's Overview (built in M3). */
export function ProjectCard({ orgId, project }: { orgId: string; project: Project }) {
  return (
    <Link href={`/orgs/${orgId}/projects/${project.id}`} className="group block">
      <Card className="hover:border-accent/60 hover:shadow-glow flex h-full flex-col gap-4 p-5 transition-colors">
        <div className="flex items-start justify-between gap-3">
          <div className="bg-secondary flex h-10 w-10 shrink-0 items-center justify-center rounded-lg">
            <Boxes className="text-accent h-5 w-5" aria-hidden="true" />
          </div>
          <Badge variant={ENVIRONMENT_VARIANT[project.environment]}>{project.environment}</Badge>
        </div>

        <div>
          <h3 className="font-display group-hover:text-accent text-lg font-semibold tracking-tight transition-colors">
            {project.name}
          </h3>
          <p className="text-muted-foreground mt-0.5 font-mono text-xs">{project.slug}</p>
        </div>

        <div className="border-border mt-auto border-t pt-4 text-sm">
          <span className="text-muted-foreground">
            Created {new Date(project.created_at).toLocaleDateString()}
          </span>
        </div>
      </Card>
    </Link>
  );
}
