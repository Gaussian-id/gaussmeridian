"use client";

import { useId } from "react";

import { cn } from "@core/lib/utils";

import { Button } from "@/components/ui/button";

import type { FormEvent, ReactNode } from "react";

interface PromptProps {
  title: string;
  kicker?: string;
  description?: string;
  error?: string;
  onSkip?: () => void;
  skipLabel?: string;
  onContinue: () => void;
  continueLabel?: string;
  isBusy?: boolean;
  /** Disables Continue only (e.g. a required field is still empty) while leaving Skip usable —
   *  distinct from `isBusy`, which disables both buttons during a pending mutation. */
  continueDisabled?: boolean;
  guidance?: string;
  continueDisabledReason?: string;
  children?: ReactNode;
  className?: string;
}

/**
 * `<Prompt>` is the one-question-at-a-time chrome every onboarding step renders through
 * (PRD-22 §4): kicker + heading + description, the step's own body, an `aria-live` error slot,
 * and a skip/continue action row. Enter submits the wrapped `<form>`, so callers never wire
 * keyboard handling themselves. The `<h1>` carries `tabIndex={-1}` so `ConversationalStage`
 * (Phase C) can move focus to it on each step transition without adding a focusable control.
 * `skipLabel` defaults to "Skip"; steps whose skip carries more context (create-org — "Skip
 * workspace setup", which defers the resource branch) pass it explicitly rather than `<Prompt>`
 * guessing intent.
 */
export function Prompt({
  title,
  kicker,
  description,
  error,
  onSkip,
  skipLabel = "Skip",
  onContinue,
  continueLabel = "Continue",
  isBusy = false,
  continueDisabled = false,
  guidance,
  continueDisabledReason,
  children,
  className,
}: PromptProps) {
  const guidanceId = useId();
  const disabledReasonId = useId();
  const describedBy = [
    guidance ? guidanceId : undefined,
    continueDisabled && continueDisabledReason ? disabledReasonId : undefined,
  ]
    .filter((id): id is string => Boolean(id))
    .join(" ");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!isBusy && !continueDisabled) onContinue();
  }

  return (
    <form onSubmit={handleSubmit} className={cn("flex flex-col gap-6", className)} noValidate>
      <div className="flex flex-col gap-2">
        {kicker && (
          <span className="text-accent text-xs font-semibold tracking-[0.14em] uppercase">
            {kicker}
          </span>
        )}
        <h1 tabIndex={-1} className="font-display text-2xl font-semibold outline-none sm:text-3xl">
          {title}
        </h1>
        {description && (
          <p className="text-muted-foreground text-sm leading-relaxed">{description}</p>
        )}
      </div>

      {children}

      {error && (
        <p role="alert" aria-live="polite" className="text-destructive text-sm">
          {error}
        </p>
      )}

      <div className="flex flex-col gap-2">
        {guidance && (
          <p id={guidanceId} className="text-muted-foreground text-sm leading-relaxed">
            {guidance}
          </p>
        )}
        {continueDisabled && continueDisabledReason && (
          <p id={disabledReasonId} className="text-muted-foreground text-sm leading-relaxed">
            {continueDisabledReason}
          </p>
        )}
        <div className="flex flex-col gap-2 sm:flex-row sm:justify-between">
          {onSkip && (
            <Button
              type="button"
              variant="ghost"
              size="lg"
              onClick={onSkip}
              disabled={isBusy}
              className="order-2 enabled:cursor-pointer sm:order-1"
            >
              {skipLabel}
            </Button>
          )}
          <Button
            type="submit"
            variant="accent"
            size="lg"
            disabled={isBusy || continueDisabled}
            aria-describedby={describedBy || undefined}
            className={cn("order-1 enabled:cursor-pointer sm:order-2", !onSkip && "sm:ml-auto")}
          >
            {continueLabel}
          </Button>
        </div>
      </div>
    </form>
  );
}
