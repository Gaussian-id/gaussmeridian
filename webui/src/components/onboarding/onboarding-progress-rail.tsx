import { CheckIcon, MinusIcon } from "lucide-react";
import { useEffect, useRef } from "react";

import { cn } from "@core/lib/utils";

import { ONBOARDING_STEPS, canSkip, stepIndex } from "@/lib/onboarding/onboarding-machine";
import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";

const STEP_LABELS: Record<OnboardingStep, string> = {
  welcome: "Welcome",
  survey: "About you",
  profile: "Your profile",
  create_org: "Workspace",
  create_project: "First project",
  api_key: "API key",
  finish: "Finish",
};

const EMPTY_SKIPPED_STEPS: ReadonlySet<OnboardingStep> = new Set();

interface OnboardingProgressRailProps {
  currentStep: OnboardingStep;
  completed: ReadonlySet<OnboardingStep>;
  skipped?: ReadonlySet<OnboardingStep>;
  /** Vertical (desktop aside) or horizontal (mobile, compact). */
  orientation: "vertical" | "horizontal";
}

/** Progress state is always conveyed in text as well as shape/color, including deferred setup. */
export function OnboardingProgressRail({
  currentStep,
  completed,
  skipped = EMPTY_SKIPPED_STEPS,
  orientation,
}: OnboardingProgressRailProps) {
  const currentIndex = stepIndex(currentStep);
  const currentItemRef = useRef<HTMLLIElement>(null);

  useEffect(() => {
    const currentItem = currentItemRef.current;
    if (orientation !== "horizontal" || typeof currentItem?.scrollIntoView !== "function") return;

    const centerCurrentItem = () => {
      currentItem.scrollIntoView({ behavior: "auto", block: "nearest", inline: "center" });
    };
    const mobileBreakpoint = window.matchMedia("(max-width: 759px)");
    const handleBreakpointChange = (event: MediaQueryListEvent) => {
      if (event.matches) centerCurrentItem();
    };

    centerCurrentItem();
    mobileBreakpoint.addEventListener("change", handleBreakpointChange);
    return () => mobileBreakpoint.removeEventListener("change", handleBreakpointChange);
  }, [currentStep, orientation]);

  return (
    <ol
      aria-label="Onboarding progress"
      className={cn(
        orientation === "vertical" ? "flex flex-col gap-1" : "flex flex-row gap-2 pb-1",
      )}
    >
      {ONBOARDING_STEPS.map((step) => {
        const isSkipped = skipped.has(step);
        const isDone = completed.has(step) && !isSkipped;
        const isCurrent = step === currentStep;
        const isPast = stepIndex(step) < currentIndex;

        return (
          <li
            key={step}
            ref={isCurrent ? currentItemRef : undefined}
            aria-current={isCurrent ? "step" : undefined}
            className={cn(
              "flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors",
              orientation === "horizontal" && "shrink-0",
              isCurrent && orientation === "vertical" && "bg-muted",
            )}
          >
            <span
              aria-hidden="true"
              className={cn(
                "grid h-5 w-5 shrink-0 place-items-center rounded-full text-[10px] font-semibold",
                isDone
                  ? "bg-foreground text-background"
                  : isSkipped
                    ? "border-muted-foreground/50 text-muted-foreground border"
                    : isCurrent
                      ? "border-foreground text-foreground border-2"
                      : isPast
                        ? "border-muted-foreground text-muted-foreground border"
                        : "border-muted-foreground/60 text-muted-foreground border",
              )}
            >
              {isDone ? (
                <CheckIcon className="h-3 w-3" strokeWidth={3} />
              ) : isSkipped ? (
                <MinusIcon className="h-3 w-3" strokeWidth={2.5} />
              ) : (
                stepIndex(step) + 1
              )}
            </span>
            <span
              className={cn(
                "truncate font-medium",
                isCurrent ? "text-foreground" : "text-muted-foreground",
              )}
            >
              {STEP_LABELS[step]}
              {isSkipped ? (
                <span className="text-muted-foreground ml-1.5 font-mono text-[10px] font-normal tracking-wide uppercase">
                  skipped
                </span>
              ) : (
                canSkip(step) && (
                  <span className="text-muted-foreground ml-1.5 font-mono text-[10px] font-normal tracking-wide uppercase">
                    optional
                  </span>
                )
              )}
            </span>
            {isDone && <span className="sr-only">completed</span>}
            {!isDone && !isSkipped && !isCurrent && <span className="sr-only">upcoming</span>}
          </li>
        );
      })}
    </ol>
  );
}
