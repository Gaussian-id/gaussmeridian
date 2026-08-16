import { describe, expect, it } from "vitest";

import {
  ONBOARDING_STEPS,
  WORKSPACE_SETUP_STEPS,
  canFinish,
  canSkip,
  fromServerState,
  hasSkippedWorkspaceSetup,
  isRequired,
  markComplete,
  markWorkspaceSetupSkipped,
  nextIncomplete,
  toCompletedStepsArray,
} from "../onboarding-machine";

import type { OnboardingStep } from "../onboarding-machine";

describe("nextIncomplete", () => {
  it("starts at welcome when nothing is complete", () => {
    expect(nextIncomplete(new Set())).toBe("welcome");
  });

  it("returns the first step not yet in the completed set, in canonical order", () => {
    expect(nextIncomplete(new Set(["welcome", "survey"]))).toBe("profile");
  });

  it("skips over out-of-order completions and still finds the first gap", () => {
    // api_key completed before create_org (shouldn't happen via the UI, but the machine
    // must still resume at the true first gap, not just the highest completed index).
    expect(nextIncomplete(new Set(["welcome", "survey", "profile", "api_key"]))).toBe("create_org");
  });

  it("resolves to finish once every step but finish itself is complete", () => {
    const allButFinish = new Set(ONBOARDING_STEPS.filter((step) => step !== "finish"));
    expect(nextIncomplete(allButFinish)).toBe("finish");
  });
});

describe("canSkip", () => {
  it("only create_org is skippable in the bridge flow", () => {
    expect(canSkip("create_org")).toBe(true);
  });

  it("welcome, survey, profile, create_project, api_key, and finish are not skippable", () => {
    expect(canSkip("welcome")).toBe(false);
    // Survey/Profile are required steps now (Shelby, onboarding-refinement) — no skip.
    expect(canSkip("survey")).toBe(false);
    expect(canSkip("profile")).toBe(false);
    expect(canSkip("create_project")).toBe(false);
    expect(canSkip("api_key")).toBe(false);
    expect(canSkip("finish")).toBe(false);
  });

  it("lets create_org defer the setup branch while remaining required on the created branch", () => {
    expect(canSkip("create_org")).toBe(true);
    expect(isRequired("create_org")).toBe(true);
  });
});

describe("isRequired", () => {
  it("marks organization, project, and API key as required within the created branch", () => {
    expect(isRequired("create_org")).toBe(true);
    expect(isRequired("create_project")).toBe(true);
    expect(isRequired("api_key")).toBe(true);
  });

  it("survey and profile are not part of the backend completion gate", () => {
    expect(isRequired("survey")).toBe(false);
    expect(isRequired("profile")).toBe(false);
  });
});

describe("canFinish (the conditional setup gate)", () => {
  it("is false until org + project + key are all complete on the created branch", () => {
    expect(canFinish(new Set())).toBe(false);
    expect(canFinish(new Set(["create_org"]))).toBe(false);
    expect(canFinish(new Set(["create_org", "create_project"]))).toBe(false);
  });

  it("is true once org + project + key are complete", () => {
    expect(canFinish(new Set(["create_org", "create_project", "api_key"]))).toBe(true);
  });

  it("does not require optional identity fields beyond the completed profile frontier", () => {
    const completed = new Set<OnboardingStep>(["create_org", "create_project", "api_key"]);
    expect(canFinish(completed)).toBe(true);
    expect(completed.has("survey")).toBe(false);
    expect(completed.has("profile")).toBe(false);
  });
});

describe("workspace-setup skip branch", () => {
  const profileCompleted = new Set<OnboardingStep>(["welcome", "survey", "profile"]);

  it("lands directly on finish while preserving completed answers", () => {
    const state = markWorkspaceSetupSkipped(profileCompleted);

    expect(state.currentStep).toBe("finish");
    expect(state.completed).toBe(profileCompleted);
    expect(Array.from(state.skipped)).toEqual(WORKSPACE_SETUP_STEPS);
    expect(nextIncomplete(state.completed, state.skipped)).toBe("finish");
    expect(canFinish(state.completed, state.skipped)).toBe(true);
    expect(hasSkippedWorkspaceSetup(state.skipped)).toBe(true);
  });

  it("removes setup completions so completed and skipped remain disjoint", () => {
    const inconsistent = new Set<OnboardingStep>([
      "welcome",
      "survey",
      "profile",
      ...WORKSPACE_SETUP_STEPS,
    ]);

    const state = markWorkspaceSetupSkipped(inconsistent);

    expect(Array.from(state.completed)).toEqual(["welcome", "survey", "profile"]);
    expect(WORKSPACE_SETUP_STEPS.every((step) => !state.completed.has(step))).toBe(true);
    expect(WORKSPACE_SETUP_STEPS.every((step) => state.skipped.has(step))).toBe(true);
    expect(inconsistent.has("create_org")).toBe(true);
  });

  it("rejects a mixed completed/skipped setup state", () => {
    expect(
      canFinish(
        new Set<OnboardingStep>(["create_project", "api_key"]),
        new Set<OnboardingStep>(["create_org"]),
      ),
    ).toBe(false);
    expect(
      canFinish(
        new Set<OnboardingStep>(["create_org", "create_project", "api_key"]),
        new Set<OnboardingStep>(["create_org"]),
      ),
    ).toBe(false);
  });

  it("reconstructs the workspace branch only from the explicit server disposition", () => {
    const pending = fromServerState({
      current_step: "finish",
      completed_steps: ["welcome", "survey", "profile"],
      workspace_disposition: "pending",
    });
    const skipped = fromServerState({
      current_step: "finish",
      completed_steps: ["welcome", "survey", "profile"],
      workspace_disposition: "skipped",
    });

    expect(pending.currentStep).toBe("create_org");
    expect(pending.skipped.size).toBe(0);
    expect(skipped.currentStep).toBe("finish");
    expect(Array.from(skipped.skipped)).toEqual(WORKSPACE_SETUP_STEPS);
  });

  it("does not misread a partially-created workspace as an intentional skip", () => {
    const state = fromServerState({
      current_step: "finish",
      completed_steps: ["welcome", "survey", "profile", "create_org"],
      workspace_disposition: "pending",
    });

    expect(state.currentStep).toBe("create_project");
    expect(state.skipped.size).toBe(0);
  });
});

describe("markComplete", () => {
  it("adds the step to the completed set", () => {
    const result = markComplete(new Set(), "welcome");
    expect(result.has("welcome")).toBe(true);
  });

  it("is idempotent — marking an already-complete step returns an equivalent set", () => {
    const once = markComplete(new Set(), "welcome");
    const twice = markComplete(once, "welcome");
    expect(Array.from(twice)).toEqual(Array.from(once));
  });

  it("does not mutate the input set", () => {
    const original = new Set<OnboardingStep>(["welcome"]);
    markComplete(original, "survey");
    expect(original.has("survey")).toBe(false);
  });
});

describe("toCompletedStepsArray", () => {
  it("orders the set in canonical step order regardless of insertion order", () => {
    const completed = new Set<OnboardingStep>(["api_key", "welcome", "create_org"]);
    expect(toCompletedStepsArray(completed)).toEqual(["welcome", "create_org", "api_key"]);
  });
});

describe("fromServerState (US O8 — resume at next incomplete step)", () => {
  it("resumes at welcome for a brand-new user (current_step: null, completed_steps: [])", () => {
    const state = fromServerState({
      current_step: null,
      completed_steps: [],
      workspace_disposition: "pending",
    });
    expect(state.currentStep).toBe("welcome");
    expect(state.completed.size).toBe(0);
  });

  it("resumes at the next incomplete step for a partially-completed user", () => {
    const state = fromServerState({
      current_step: "profile",
      completed_steps: ["welcome", "survey"],
      workspace_disposition: "pending",
    });
    expect(state.currentStep).toBe("profile");
  });

  it("ignores the server's current_step in favor of the computed nextIncomplete", () => {
    // The server says "profile" but completed_steps proves create_org is still missing —
    // nextIncomplete(completed) is the single source of truth, not the stored pointer.
    const state = fromServerState({
      current_step: "profile",
      completed_steps: ["welcome", "survey", "profile"],
      workspace_disposition: "pending",
    });
    expect(state.currentStep).toBe("create_org");
  });

  it("drops unrecognized step strings instead of crashing, and still resumes safely", () => {
    const state = fromServerState({
      current_step: "some_future_step",
      completed_steps: ["welcome", "some_future_step", "survey"],
      workspace_disposition: "pending",
    });
    expect(state.currentStep).toBe("profile");
  });

  it("resumes at an arbitrary saved state from an arbitrary point in the sequence", () => {
    const state = fromServerState({
      current_step: "api_key",
      completed_steps: ["welcome", "survey", "profile", "create_org", "create_project"],
      workspace_disposition: "pending",
    });
    expect(state.currentStep).toBe("api_key");
  });
});
