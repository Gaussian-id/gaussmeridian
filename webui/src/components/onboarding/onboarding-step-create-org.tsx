"use client";

import { useState } from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCreateOrg } from "@/hooks/useConsoleQueries";

import { Prompt } from "./prompt";

interface OnboardingStepCreateOrgProps {
  onNext: (orgId: string) => void;
  onSkip: () => void;
  onCreateIntent?: () => void;
  isSkipping?: boolean;
  skipError?: string;
}

/**
 * Workspace setup's branch point. Creating requires a real name and continues through project/key
 * setup; skipping makes no resource request and lets the wizard move directly to its deferred
 * Finish state. The backend generates the slug, so the client sends only the trimmed name.
 */
export function OnboardingStepCreateOrg({
  onNext,
  onSkip,
  onCreateIntent,
  isSkipping = false,
  skipError,
}: OnboardingStepCreateOrgProps) {
  const createOrg = useCreateOrg();
  const [name, setName] = useState("");
  const [nameTouched, setNameTouched] = useState(false);
  const trimmed = name.trim();
  const nameInvalid = nameTouched && !trimmed;

  function submit() {
    if (!trimmed) return;
    onCreateIntent?.();
    createOrg.mutate({ name: trimmed }, { onSuccess: (org) => onNext(org.id) });
  }

  return (
    <Prompt
      kicker="Workspace"
      title="Create your first workspace?"
      description="A workspace holds your projects, keys, and settings. You can skip this setup for now."
      onSkip={onSkip}
      skipLabel="Skip workspace setup"
      onContinue={submit}
      isBusy={createOrg.isPending || isSkipping}
      continueDisabled={!trimmed}
      continueDisabledReason={
        nameInvalid ? undefined : "Enter a workspace name or skip setup for now."
      }
      continueLabel={createOrg.isPending ? "Creating…" : "Create workspace"}
      error={
        skipError ?? (createOrg.isError ? "Could not create the workspace. Try again." : undefined)
      }
    >
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="onboarding-org-name">
          Workspace name <span className="text-accent text-xs">Required</span>
        </Label>
        <Input
          id="onboarding-org-name"
          type="text"
          required
          value={name}
          onChange={(event) => setName(event.target.value)}
          onBlur={() => setNameTouched(true)}
          aria-invalid={nameInvalid || undefined}
          aria-describedby={nameInvalid ? "onboarding-org-name-error" : undefined}
          autoComplete="organization"
          placeholder="Acme Inc."
          disabled={createOrg.isPending}
          className="h-11 text-base"
        />
        {nameInvalid && (
          <p id="onboarding-org-name-error" role="alert" className="text-destructive text-sm">
            Enter a workspace name or skip setup for now.
          </p>
        )}
      </div>
    </Prompt>
  );
}
