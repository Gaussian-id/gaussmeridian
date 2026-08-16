import { describe, expect, it } from "vitest";

import { ONBOARDING_STEPS } from "@/lib/onboarding/onboarding-machine";
import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";

import {
  ROUTE_STEPS,
  cameraTargetForStep,
  headlightSunDirection,
  isArcRevealed,
  isLabelVisible,
  isNodeRevealed,
  offsetXForWidth,
  resolveReducedMotion,
  revealedCityCount,
} from "../scene-helpers";

const ROUTE_LENGTH = 7;

describe("ROUTE_STEPS", () => {
  it("is every onboarding step except the terminal finish, in canonical order", () => {
    expect(ROUTE_STEPS).toEqual(ONBOARDING_STEPS.filter((s) => s !== "finish"));
    expect(ROUTE_STEPS).toHaveLength(6);
    expect(ROUTE_STEPS).not.toContain("finish");
  });
});

describe("cameraTargetForStep", () => {
  it("maps welcome..api_key onto consecutive route indices", () => {
    ROUTE_STEPS.forEach((step, i) => {
      const target = cameraTargetForStep(step, ROUTE_LENGTH);
      expect(target).toEqual({ cityIndex: i, distance: 4.5, pullBack: false });
    });
  });

  it("pulls back at finish, targeting the last city with pullBack true", () => {
    const target = cameraTargetForStep("finish", ROUTE_LENGTH);
    expect(target.pullBack).toBe(true);
    expect(target.cityIndex).toBe(ROUTE_LENGTH - 1);
    expect(target.distance).toBe(8.6);
  });

  it("clamps the city index when the route is shorter than the step sequence", () => {
    const target = cameraTargetForStep("api_key", 3);
    expect(target.cityIndex).toBe(2); // last valid index, not 6
    expect(target.pullBack).toBe(false);
  });

  it("clamps to index 0 for an empty route without throwing", () => {
    expect(() => cameraTargetForStep("welcome", 0)).not.toThrow();
    expect(cameraTargetForStep("welcome", 0).cityIndex).toBe(0);
  });
});

describe("revealedCityCount", () => {
  it("grows by one city per route step", () => {
    ROUTE_STEPS.forEach((step, i) => {
      expect(revealedCityCount(step, ROUTE_LENGTH)).toBe(i + 1);
    });
  });

  it("reveals the entire route at finish", () => {
    expect(revealedCityCount("finish", ROUTE_LENGTH)).toBe(ROUTE_LENGTH);
  });
});

describe("isNodeRevealed", () => {
  it("lights only cities up to and including the current step's target", () => {
    // create_project is index 4 in ROUTE_STEPS -> cities 0..4 revealed, 5 and 6 not yet.
    const step: OnboardingStep = "create_project";
    expect(isNodeRevealed(step, ROUTE_LENGTH, 0)).toBe(true);
    expect(isNodeRevealed(step, ROUTE_LENGTH, 4)).toBe(true);
    expect(isNodeRevealed(step, ROUTE_LENGTH, 5)).toBe(false);
    expect(isNodeRevealed(step, ROUTE_LENGTH, 6)).toBe(false);
  });

  it("lights every city at finish", () => {
    for (let i = 0; i < ROUTE_LENGTH; i++) {
      expect(isNodeRevealed("finish", ROUTE_LENGTH, i)).toBe(true);
    }
  });

  it("lights only the welcome city at the first step", () => {
    expect(isNodeRevealed("welcome", ROUTE_LENGTH, 0)).toBe(true);
    expect(isNodeRevealed("welcome", ROUTE_LENGTH, 1)).toBe(false);
  });
});

describe("isArcRevealed", () => {
  it("draws an arc only once its destination city is revealed", () => {
    const step: OnboardingStep = "create_project"; // reveals cities 0..4
    expect(isArcRevealed(step, ROUTE_LENGTH, 1)).toBe(true); // city0 -> city1
    expect(isArcRevealed(step, ROUTE_LENGTH, 4)).toBe(true); // city3 -> city4
    expect(isArcRevealed(step, ROUTE_LENGTH, 5)).toBe(false); // city4 -> city5, not yet
  });

  it("has no arcs revealed at welcome (only the first city is lit)", () => {
    expect(isArcRevealed("welcome", ROUTE_LENGTH, 1)).toBe(false);
  });

  it("draws every arc at finish", () => {
    for (let i = 1; i < ROUTE_LENGTH; i++) {
      expect(isArcRevealed("finish", ROUTE_LENGTH, i)).toBe(true);
    }
  });
});

describe("isLabelVisible", () => {
  it("requires revealed, front-facing, and sufficiently eased-in all at once", () => {
    expect(isLabelVisible({ revealed: true, frontDot: 0.5, revealAmount: 0.9 })).toBe(true);
    expect(isLabelVisible({ revealed: false, frontDot: 0.5, revealAmount: 0.9 })).toBe(false);
    expect(isLabelVisible({ revealed: true, frontDot: 0.05, revealAmount: 0.9 })).toBe(false);
    expect(isLabelVisible({ revealed: true, frontDot: 0.5, revealAmount: 0.3 })).toBe(false);
  });

  it("sits exactly at the documented thresholds (0.12 dot, 0.5 reveal)", () => {
    expect(isLabelVisible({ revealed: true, frontDot: 0.12, revealAmount: 0.9 })).toBe(false);
    expect(isLabelVisible({ revealed: true, frontDot: 0.13, revealAmount: 0.9 })).toBe(true);
    expect(isLabelVisible({ revealed: true, frontDot: 0.5, revealAmount: 0.5 })).toBe(false);
    expect(isLabelVisible({ revealed: true, frontDot: 0.5, revealAmount: 0.51 })).toBe(true);
  });
});

describe("headlightSunDirection", () => {
  it("returns a unit vector", () => {
    const sun = headlightSunDirection({ x: 0, y: 0, z: 1 });
    const length = Math.sqrt(sun.x * sun.x + sun.y * sun.y + sun.z * sun.z);
    expect(length).toBeCloseTo(1, 10);
  });

  it("offsets the camera direction by (-0.30, +0.45) before normalizing", () => {
    const camDir = { x: 0, y: 0, z: 1 };
    const sun = headlightSunDirection(camDir);
    // Un-normalized offset would be (-0.3, 0.45, 1); check the direction ratios survive.
    const raw = { x: -0.3, y: 0.45, z: 1 };
    const rawLength = Math.sqrt(raw.x * raw.x + raw.y * raw.y + raw.z * raw.z);
    expect(sun.x).toBeCloseTo(raw.x / rawLength, 10);
    expect(sun.y).toBeCloseTo(raw.y / rawLength, 10);
    expect(sun.z).toBeCloseTo(raw.z / rawLength, 10);
  });
});

describe("resolveReducedMotion", () => {
  it("returns the matchMedia result's matches flag", () => {
    expect(resolveReducedMotion({ matches: true })).toBe(true);
    expect(resolveReducedMotion({ matches: false })).toBe(false);
  });

  it("defaults to full motion when matchMedia is unavailable", () => {
    expect(resolveReducedMotion(null)).toBe(false);
  });
});

describe("offsetXForWidth", () => {
  it("pushes the globe left on wide screens", () => {
    expect(offsetXForWidth(1440)).toBe(-1.9);
    expect(offsetXForWidth(760)).toBe(-1.9);
  });

  it("centers the globe under the ~760px breakpoint", () => {
    expect(offsetXForWidth(759)).toBe(0);
    expect(offsetXForWidth(375)).toBe(0);
  });
});
