"use client";

import { useParams, useRouter } from "next/navigation";
import { useState } from "react";

import type { Project } from "@core/adapters/schemas/console.schema";
import { canManageProjects } from "@core/lib/rbac";
import { useTenancy } from "@core/providers";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useCreateProject } from "@/hooks/useConsoleQueries";

import type { FormEvent } from "react";

const ENVIRONMENT_OPTIONS: { value: Project["environment"]; label: string }[] = [
  { value: "development", label: "Development" },
  { value: "production", label: "Production" },
];

/**
 * Names a project and picks its environment, then lands the caller in the new project.
 *
 * Gated to Owner/Admin (`canManageProjects`) — the real backend's RBAC rules treat project
 * creation as an org-admin action a Developer would 403 on.
 */
export function CreateProjectForm() {
  const { orgId } = useParams<{ orgId: string }>();
  const { role } = useTenancy();
  const router = useRouter();
  const createProject = useCreateProject(orgId);
  const permitted = canManageProjects(role);

  const [name, setName] = useState("");
  const [environment, setEnvironment] = useState<Project["environment"]>("development");

  if (!permitted) {
    return (
      <Card className="mx-auto flex w-full max-w-md flex-col gap-2 p-6 text-center">
        <p className="text-muted-foreground text-sm">
          Only organization owners and admins can create projects.
        </p>
      </Card>
    );
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) return;
    createProject.mutate(
      { name: trimmed, environment },
      { onSuccess: (project) => router.push(`/orgs/${orgId}/projects/${project.id}`) },
    );
  }

  return (
    <Card className="mx-auto w-full max-w-md p-6">
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="project-name">Project name</Label>
          <Input
            id="project-name"
            type="text"
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Production API"
            disabled={createProject.isPending}
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="project-environment">Environment</Label>
          <Select
            value={environment}
            onValueChange={(value) => setEnvironment(value as Project["environment"])}
            disabled={createProject.isPending}
          >
            <SelectTrigger id="project-environment" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ENVIRONMENT_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {createProject.isError && (
          <p role="alert" className="text-destructive text-sm">
            Could not create the project. Try again.
          </p>
        )}

        <Button
          type="submit"
          variant="accent"
          size="lg"
          disabled={createProject.isPending || !name.trim()}
        >
          {createProject.isPending ? "Creating…" : "Create project"}
        </Button>
      </form>
    </Card>
  );
}
