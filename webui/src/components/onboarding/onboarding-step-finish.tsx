"use client";

import { CheckIcon, CircleDashedIcon } from "lucide-react";

import { Prompt } from "./prompt";

interface OnboardingStepFinishProps {
  onFinish: () => void;
  isPending: boolean;
  error?: string;
  workspaceSkipped?: boolean;
}

/** Bridge hand-off: copy and status reflect whether workspace setup was completed or deferred. */
export function OnboardingStepFinish({
  onFinish,
  isPending,
  error,
  workspaceSkipped = false,
}: OnboardingStepFinishProps) {
  const completedItems = workspaceSkipped
    ? ["Profile complete"]
    : ["Workspace", "Project", "API key"];

  return (
    <Prompt
      kicker="Done"
      title={workspaceSkipped ? "Your profile is ready." : "You're all set."}
      description={
        workspaceSkipped
          ? "Create a workspace whenever you're ready to route your first request."
          : "Your workspace is live across the network. Time to route something."
      }
      onContinue={onFinish}
      isBusy={isPending}
      continueLabel={isPending ? "Finishing…" : "Open dashboard →"}
      error={error}
    >
      <div className="flex flex-wrap gap-2">
        {completedItems.map((item) => (
          <span
            key={item}
            className="border-accent/40 bg-secondary/60 text-foreground inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium"
          >
            <CheckIcon className="text-accent h-3.5 w-3.5" aria-hidden="true" />
            {item}
          </span>
        ))}
        {workspaceSkipped && (
          <span className="border-border bg-muted/60 text-muted-foreground inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-xs font-medium">
            <CircleDashedIcon className="h-3.5 w-3.5" aria-hidden="true" />
            Workspace setup deferred
          </span>
        )}
      </div>
    </Prompt>
  );
}
