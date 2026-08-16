"use client";

import { useState } from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCreateProject } from "@/hooks/useConsoleQueries";

import { Prompt } from "./prompt";

interface OnboardingStepCreateProjectProps {
  orgId: string;
  onNext: (projectId: string, projectName: string) => void;
}

/** Step 5 (O5, required) — creates the first project inside the org just created. */
export function OnboardingStepCreateProject({ orgId, onNext }: OnboardingStepCreateProjectProps) {
  const createProject = useCreateProject(orgId);
  const [name, setName] = useState("");
  const [nameTouched, setNameTouched] = useState(false);
  const trimmed = name.trim();
  const nameInvalid = nameTouched && !trimmed;

  function submit() {
    if (!trimmed) return;
    createProject.mutate(
      { name: trimmed },
      { onSuccess: (project) => onNext(project.id, project.name) },
    );
  }

  return (
    <Prompt
      kicker="First project"
      title="And your first project?"
      description="This is where routed traffic, keys, and settings live."
      onContinue={submit}
      isBusy={createProject.isPending}
      continueDisabled={!trimmed}
      continueDisabledReason={nameInvalid ? undefined : "Enter a project name to continue."}
      continueLabel={createProject.isPending ? "Creating…" : "Create project"}
      error={createProject.isError ? "Could not create the project. Try again." : undefined}
    >
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="onboarding-project-name">
          Project name <span className="text-accent text-xs">Required</span>
        </Label>
        <Input
          id="onboarding-project-name"
          type="text"
          required
          value={name}
          onChange={(event) => setName(event.target.value)}
          onBlur={() => setNameTouched(true)}
          aria-invalid={nameInvalid || undefined}
          aria-describedby={nameInvalid ? "onboarding-project-name-error" : undefined}
          placeholder="Production API"
          disabled={createProject.isPending}
          className="h-11 text-base"
        />
        {nameInvalid && (
          <p id="onboarding-project-name-error" role="alert" className="text-destructive text-sm">
            Enter a project name to continue.
          </p>
        )}
      </div>
    </Prompt>
  );
}
