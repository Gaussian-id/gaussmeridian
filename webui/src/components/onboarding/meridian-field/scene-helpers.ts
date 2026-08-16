/**
 * Pure logic for `<MeridianField>` (PRD-22 Phase B) — step → route-city mapping, camera-target
 * selection, reveal thresholds, the headlight-sun offset, and the reduced-motion / responsive-
 * offset decisions. No React, no three.js — everything here is data + math so it's independently
 * unit-testable without a WebGL context. `earth-scene.ts` is the imperative three.js consumer;
 * it should call into these rather than re-deriving the same rules inline.
 */

import { ONBOARDING_STEPS } from "@/lib/onboarding/onboarding-machine";
import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";

/**
 * The non-terminal steps that each correspond 1:1 to a city in the route, in flight order (`welcome` through
 * `api_key`, i.e. `ONBOARDING_STEPS` minus the terminal `finish`). `finish` doesn't target a
 * city — it pulls the camera back to reveal the whole route instead.
 */
export const ROUTE_STEPS: readonly OnboardingStep[] = ONBOARDING_STEPS.filter(
  (step) => step !== "finish",
);

const CITY_CAMERA_DISTANCE = 4.5;
const PULLBACK_CAMERA_DISTANCE = 8.6;
const FRONT_FACE_DOT_THRESHOLD = 0.12;
const NODE_REVEAL_LABEL_THRESHOLD = 0.5;
const RESPONSIVE_BREAKPOINT_PX = 760;
const WIDE_OFFSET_X = -1.9;
const NARROW_OFFSET_X = 0;

export interface CameraTarget {
  /** Route index the camera orbits to, clamped to `[0, routeLength - 1]`. */
  cityIndex: number;
  /** Target camera distance from the globe center. */
  distance: number;
  /** True only at `finish` — pull back to reveal the whole route rather than orbit one city. */
  pullBack: boolean;
}

/**
 * Step -> camera target, per PRD-22 Phase B: `welcome..api_key` (`ROUTE_STEPS`)
 * orbit the camera to `route[i]`; `finish` pulls back to reveal the whole route. `routeLength`
 * is clamped defensively (a route shorter than the step sequence still resolves to a valid index) since
 * the route is caller-supplied data, not a compile-time constant.
 */
export function cameraTargetForStep(step: OnboardingStep, routeLength: number): CameraTarget {
  const lastIndex = Math.max(routeLength - 1, 0);

  if (step === "finish") {
    return { cityIndex: lastIndex, distance: PULLBACK_CAMERA_DISTANCE, pullBack: true };
  }

  const stepIndex = ROUTE_STEPS.indexOf(step);
  const cityIndex = stepIndex === -1 ? lastIndex : Math.min(stepIndex, lastIndex);
  return { cityIndex, distance: CITY_CAMERA_DISTANCE, pullBack: false };
}

/**
 * How many leading cities in the route are "revealed" (node lit, arc drawn) at this step: every
 * city up to and including the current orbit target, or the entire route once `finish` pulls
 * back.
 */
export function revealedCityCount(step: OnboardingStep, routeLength: number): number {
  const target = cameraTargetForStep(step, routeLength);
  return target.pullBack ? routeLength : Math.min(target.cityIndex + 1, routeLength);
}

/** Should city node `cityIndex`'s marker/halo be lit at this step? */
export function isNodeRevealed(
  step: OnboardingStep,
  routeLength: number,
  cityIndex: number,
): boolean {
  return cityIndex < revealedCityCount(step, routeLength);
}

/**
 * Should the great-circle arc ending at city `arcEndIndex` (connecting `route[arcEndIndex - 1]`
 * to `route[arcEndIndex]`) be drawn at this step? An arc reveals in lockstep with its
 * destination node — `arcEndIndex` is itself a valid node index, so the same reveal count check
 * applies.
 */
export function isArcRevealed(
  step: OnboardingStep,
  routeLength: number,
  arcEndIndex: number,
): boolean {
  return arcEndIndex < revealedCityCount(step, routeLength);
}

/**
 * Front-face visibility predicate for a capital's projected name label: only show a label for a
 * city that's revealed AND currently facing the camera (`dot(cityDir, camDir)` above threshold,
 * so a city rotated to the far side of the globe doesn't show a label floating mid-sphere) AND
 * far enough through its reveal-in animation to read cleanly rather than flash in at scale zero.
 */
export function isLabelVisible(opts: {
  revealed: boolean;
  frontDot: number;
  revealAmount: number;
}): boolean {
  return (
    opts.revealed &&
    opts.frontDot > FRONT_FACE_DOT_THRESHOLD &&
    opts.revealAmount > NODE_REVEAL_LABEL_THRESHOLD
  );
}

export interface Vec3Like {
  x: number;
  y: number;
  z: number;
}

function normalize(v: Vec3Like): Vec3Like {
  const length = Math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z) || 1;
  return { x: v.x / length, y: v.y / length, z: v.z / length };
}

/**
 * Headlight sun direction: the sun tracks the camera each frame (offset and renormalized) so
 * whichever hemisphere is in view stays sunlit and the terminator sits at a pleasant angle
 * rather than falling dead-on with the view direction. Carried verbatim from the approved
 * prototype's `uSun` update (`x -= 0.30, y += 0.45`, then normalize).
 */
export function headlightSunDirection(cameraDir: Vec3Like): Vec3Like {
  return normalize({ x: cameraDir.x - 0.3, y: cameraDir.y + 0.45, z: cameraDir.z });
}

/**
 * Resolves the `prefers-reduced-motion` decision from a `matchMedia` result (or `null` when
 * unavailable, e.g. non-browser test environments or a server render) — defaults to full motion.
 */
export function resolveReducedMotion(mql: { matches: boolean } | null): boolean {
  return mql?.matches ?? false;
}

/**
 * Responsive globe horizontal offset: pushed left on wide screens so a right-side form panel
 * doesn't overlap it; centered under the ~760px breakpoint where the panel goes full-width.
 */
export function offsetXForWidth(width: number): number {
  return width < RESPONSIVE_BREAKPOINT_PX ? NARROW_OFFSET_X : WIDE_OFFSET_X;
}
