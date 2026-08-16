"use client";

import { useState } from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useSaveProfile } from "@/hooks/useOnboarding";

import { Prompt } from "./prompt";

interface OnboardingStepProfileProps {
  onNext: () => void;
}

/** Step 3 (US O3) — profile fields. Full name is required; display name and company are optional. */
export function OnboardingStepProfile({ onNext }: OnboardingStepProfileProps) {
  const saveProfile = useSaveProfile();
  const [fullName, setFullName] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [company, setCompany] = useState("");
  const [timezone] = useState(() => Intl.DateTimeFormat().resolvedOptions().timeZone || "");
  const [fullNameTouched, setFullNameTouched] = useState(false);
  const trimmedFullName = fullName.trim();
  const fullNameInvalid = fullNameTouched && !trimmedFullName;

  function submit() {
    if (!trimmedFullName) return;
    saveProfile.mutate(
      {
        full_name: trimmedFullName,
        display_name: displayName.trim() || undefined,
        company: company.trim() || undefined,
        timezone: timezone || undefined,
      },
      { onSuccess: onNext },
    );
  }

  return (
    <Prompt
      kicker="Your profile"
      title="A little about you."
      description="Add your name now. Display name and company can be added later."
      onContinue={submit}
      isBusy={saveProfile.isPending}
      continueDisabled={!trimmedFullName}
      continueDisabledReason={!trimmedFullName ? "Enter your full name to continue." : undefined}
      continueLabel={saveProfile.isPending ? "Saving…" : "Continue"}
      error={saveProfile.isError ? "Could not save your profile. Please try again." : undefined}
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="profile-full-name">
            Full name <span className="text-accent text-xs">Required</span>
          </Label>
          <Input
            id="profile-full-name"
            value={fullName}
            onChange={(event) => setFullName(event.target.value)}
            onBlur={() => setFullNameTouched(true)}
            placeholder="Ada Lovelace"
            required
            aria-invalid={fullNameInvalid || undefined}
            aria-describedby={fullNameInvalid ? "profile-full-name-error" : undefined}
            autoComplete="name"
            disabled={saveProfile.isPending}
            className="h-11 text-base"
          />
          {fullNameInvalid && (
            <p id="profile-full-name-error" role="alert" className="text-destructive text-sm">
              Enter your full name to continue.
            </p>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="profile-display-name">
            Display name <span className="text-muted-foreground text-xs">(optional)</span>
          </Label>
          <Input
            id="profile-display-name"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
            placeholder="Ada"
            autoComplete="nickname"
            disabled={saveProfile.isPending}
            className="h-11 text-base"
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="profile-company">
            Company <span className="text-muted-foreground text-xs">(optional)</span>
          </Label>
          <Input
            id="profile-company"
            value={company}
            onChange={(event) => setCompany(event.target.value)}
            placeholder="Acme Inc."
            autoComplete="organization"
            disabled={saveProfile.isPending}
            className="h-11 text-base"
          />
        </div>
      </div>
    </Prompt>
  );
}
