"use client";

import { Prompt } from "./prompt";

interface OnboardingStepWelcomeProps {
  displayName?: string;
  onNext: () => void;
}

/** Step 1 (US O1) — "You're in." A greeting, not a form. */
export function OnboardingStepWelcome({ displayName, onNext }: OnboardingStepWelcomeProps) {
  return (
    <Prompt
      kicker="Welcome"
      title={`You're in${displayName ? `, ${displayName}` : ""}.`}
      description="Let's stand up your workspace — about a minute, one thing at a time. Anything marked optional can be finished later from settings."
      onContinue={onNext}
      continueLabel="Let's go"
    />
  );
}
