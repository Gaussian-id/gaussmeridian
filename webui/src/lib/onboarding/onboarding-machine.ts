/**
 * Pure onboarding step-machine (PRD-21 Wave B / DR-010 §7-RESOLVED item 3, 13-Auth-Onboarding-PRD
 * "Implementation Decisions"). No React, no fetch — just `{ steps, currentStep, completed,
 * canSkip(step), nextIncomplete() }` as data + pure functions over it, so the flow logic is
 * unit-testable in isolation and the step views stay thin.
 *
 * Canonical order (13-PRD, defaulted in 20-Requirements-Model §7-RESOLVED item 3):
 * Bridge order: Welcome → About-you survey → Profile → Create org → Create project →
 * First API key → Finish. The legacy project-password step guarded BYOK and is intentionally
 * absent while P27-KEY-008 keeps BYOK unavailable.
 *
 * Survey and Profile are walked through rather than skippable, though only their designated fields
 * are required by the views. Create-org can defer the whole workspace-setup branch. If workspace
 * setup is chosen, organization, project, and API key are all required before Finish.
 */

export const ONBOARDING_STEPS = [
  "welcome",
  "survey",
  "profile",
  "create_org",
  "create_project",
  "api_key",
  "finish",
] as const;

export type OnboardingStep = (typeof ONBOARDING_STEPS)[number];

export const WORKSPACE_SETUP_STEPS = ["create_org", "create_project", "api_key"] as const;

const EMPTY_SKIPPED_STEPS: ReadonlySet<OnboardingStep> = new Set();

// Create-org is the branch point: skipping it defers all workspace setup.
const SKIPPABLE_STEPS: ReadonlySet<OnboardingStep> = new Set(["create_org"]);

/** Required when the user chooses to set up a workspace during onboarding. */
export const REQUIRED_STEPS: readonly OnboardingStep[] = WORKSPACE_SETUP_STEPS;

export function isValidStep(value: string): value is OnboardingStep {
  return (ONBOARDING_STEPS as readonly string[]).includes(value);
}

export function canSkip(step: OnboardingStep): boolean {
  return SKIPPABLE_STEPS.has(step);
}

export function isRequired(step: OnboardingStep): boolean {
  return REQUIRED_STEPS.includes(step);
}

export function stepIndex(step: OnboardingStep): number {
  return ONBOARDING_STEPS.indexOf(step);
}

/**
 * The first step (in canonical order) neither completed nor explicitly skipped. `finish` is
 * deliberately excluded from the scan because it is the terminal step, never an incomplete task.
 */
export function nextIncomplete(
  completed: ReadonlySet<OnboardingStep>,
  skipped: ReadonlySet<OnboardingStep> = EMPTY_SKIPPED_STEPS,
): OnboardingStep {
  const next = ONBOARDING_STEPS.find(
    (step) => step !== "finish" && !completed.has(step) && !skipped.has(step),
  );
  return next ?? "finish";
}

/** Finish is reachable after either the complete setup branch or the wholly skipped branch. */
export function canFinish(
  completed: ReadonlySet<OnboardingStep>,
  skipped: ReadonlySet<OnboardingStep> = EMPTY_SKIPPED_STEPS,
): boolean {
  const setupComplete = REQUIRED_STEPS.every((step) => completed.has(step) && !skipped.has(step));
  const setupSkipped = REQUIRED_STEPS.every((step) => skipped.has(step) && !completed.has(step));
  return setupComplete || setupSkipped;
}

export function markComplete(
  completed: ReadonlySet<OnboardingStep>,
  step: OnboardingStep,
): ReadonlySet<OnboardingStep> {
  if (completed.has(step)) return completed;
  const next = new Set(completed);
  next.add(step);
  return next;
}

/** Ordered `completed_steps` array for the `POST /v1/onboarding/advance` payload — the server
 *  stores whatever array it's given, so the client (which owns the canonical order) sends it
 *  pre-sorted rather than relying on insertion order. */
export function toCompletedStepsArray(completed: ReadonlySet<OnboardingStep>): OnboardingStep[] {
  return ONBOARDING_STEPS.filter((step) => completed.has(step));
}

export interface OnboardingMachineState {
  currentStep: OnboardingStep;
  completed: ReadonlySet<OnboardingStep>;
  skipped: ReadonlySet<OnboardingStep>;
}

export function hasSkippedWorkspaceSetup(skipped: ReadonlySet<OnboardingStep>): boolean {
  return WORKSPACE_SETUP_STEPS.every((step) => skipped.has(step));
}

export function markWorkspaceSetupSkipped(
  completed: ReadonlySet<OnboardingStep>,
): OnboardingMachineState {
  let branchCompleted = completed;
  if (WORKSPACE_SETUP_STEPS.some((step) => completed.has(step))) {
    const sanitized = new Set(completed);
    for (const step of WORKSPACE_SETUP_STEPS) sanitized.delete(step);
    branchCompleted = sanitized;
  }

  return {
    currentStep: "finish",
    completed: branchCompleted,
    skipped: new Set<OnboardingStep>(WORKSPACE_SETUP_STEPS),
  };
}

/**
 * Rebuild machine state from the server's persisted `GET /v1/onboarding/state` response
 * (US O8 — resume at the next incomplete step, across devices/sessions). Unrecognized step
 * strings (a legacy value, a future step this build doesn't know about yet) are dropped rather
 * than crashing. The server's explicit disposition is the only authority for reconstructing the
 * skipped workspace branch; `current_step` never implies user intent.
 */
export function fromServerState(input: {
  current_step: string | null;
  completed_steps: string[];
  workspace_disposition: "pending" | "configured" | "skipped";
}): OnboardingMachineState {
  const completed = new Set(input.completed_steps.filter(isValidStep));
  const skipped =
    input.workspace_disposition === "skipped"
      ? new Set<OnboardingStep>(WORKSPACE_SETUP_STEPS)
      : EMPTY_SKIPPED_STEPS;

  return {
    currentStep: nextIncomplete(completed, skipped),
    completed,
    skipped,
  };
}
