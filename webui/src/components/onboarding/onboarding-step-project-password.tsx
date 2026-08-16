"use client";

import { useState } from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useSetProjectPassword } from "@/hooks/useProjectPassword";

import { Prompt } from "./prompt";

interface OnboardingStepProjectPasswordProps {
  projectId: string;
  onNext: () => void;
}

const MIN_LENGTH = 8;

/**
 * Step 6 (O6, optional) — a second factor guarding this project's BYOK vault (P5/US #33/#47).
 * Optional and last-before-the-key per DR-010 §7-RESOLVED item 4 ("keep, ship optional & last,
 * behind the same gate as BYOK"). Skip advances immediately without setting a password — same as
 * before. Continue no-ops (rather than being disabled) while the password is too short or the
 * confirmation doesn't match: `<Prompt>`'s single `isBusy` flag is reserved for "a request is in
 * flight" (it also disables Skip), so validity is gated inside `submit()` instead, same as the
 * pre-Phase-C disabled-button behavior's effect — nothing invalid is ever submitted — with the
 * inline hint text below each field carrying the reason.
 */
export function OnboardingStepProjectPassword({
  projectId,
  onNext,
}: OnboardingStepProjectPasswordProps) {
  const setPassword = useSetProjectPassword(projectId);
  const [password, setPasswordValue] = useState("");
  const [confirm, setConfirm] = useState("");

  const mismatch = confirm.length > 0 && password !== confirm;
  const tooShort = password.length > 0 && password.length < MIN_LENGTH;
  const valid = password.length >= MIN_LENGTH && password === confirm;

  function submit() {
    if (!valid) return;
    setPassword.mutate({ password }, { onSuccess: onNext });
  }

  return (
    <Prompt
      kicker="Security · optional"
      title="Add a second lock?"
      description="A project password gates this project's BYOK vault — a second factor beyond your account login. You can set this later from project settings instead."
      onSkip={onNext}
      onContinue={submit}
      isBusy={setPassword.isPending}
      continueLabel={setPassword.isPending ? "Saving…" : "Set & continue"}
      error={
        setPassword.isError
          ? "Could not set the project password. You can try again or skip for now."
          : undefined
      }
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="onboarding-project-password">Project password</Label>
          <Input
            id="onboarding-project-password"
            type="password"
            minLength={MIN_LENGTH}
            value={password}
            onChange={(event) => setPasswordValue(event.target.value)}
            placeholder="••••••••"
            disabled={setPassword.isPending}
            aria-invalid={tooShort ? true : undefined}
            aria-describedby={tooShort ? "onboarding-project-password-hint" : undefined}
          />
          {tooShort && (
            <p id="onboarding-project-password-hint" className="text-muted-foreground text-xs">
              At least {MIN_LENGTH} characters.
            </p>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <Label htmlFor="onboarding-project-password-confirm">Confirm password</Label>
          <Input
            id="onboarding-project-password-confirm"
            type="password"
            value={confirm}
            onChange={(event) => setConfirm(event.target.value)}
            placeholder="••••••••"
            disabled={setPassword.isPending}
            aria-invalid={mismatch ? true : undefined}
            aria-describedby={mismatch ? "onboarding-project-password-mismatch" : undefined}
          />
          {mismatch && (
            <p
              id="onboarding-project-password-mismatch"
              role="alert"
              aria-live="polite"
              className="text-destructive text-xs"
            >
              Passwords don&apos;t match.
            </p>
          )}
        </div>
      </div>
    </Prompt>
  );
}
