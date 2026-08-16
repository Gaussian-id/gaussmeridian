"use client";

import { gsap } from "gsap";
import { useEffect, useRef } from "react";

import { cn } from "@core/lib/utils";

import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";
import type { Capital } from "@/lib/onboarding/route";

import { MeridianField } from "./meridian-field";
import { ROUTE_STEPS } from "./meridian-field/scene-helpers";
import { OnboardingProgressRail } from "./onboarding-progress-rail";

import type { ReactNode } from "react";

interface ConversationalStageProps {
  currentStep: OnboardingStep;
  completed: ReadonlySet<OnboardingStep>;
  skipped: ReadonlySet<OnboardingStep>;
  route: Capital[];
  children: ReactNode;
}

/**
 * `<ConversationalStage>` (PRD-22 Phase C) — the one-prompt-at-a-time chrome every onboarding
 * step renders inside. Composes the Phase A/B leaf modules into the approved prototype's layout:
 * - `<MeridianField>` mounted as a fixed, full-bleed background (the reactive Earth, Phase B),
 * - a readability scrim over it so the floating text stays legible against any part of the globe,
 * - `children` (always a `<Prompt>`-based step) as **card-less text floating hard-right** over the
 *   Earth — no panel surface, so the globe reads through (Shelby, PRD-22 follow-up),
 * - a "Satellite over [City]" readout derived from `route` + `currentStep` (top-left, hidden on
 *   mobile, matching the prototype),
 * - the progress rail (`onboarding-progress-rail.tsx`) as a vertical column docked left on
 *   desktop (echoing the landing page's progress line) and a horizontal strip on mobile; both
 *   receive completed and skipped state so deferred resources never appear provisioned,
 * - a gsap fade/rise transition on every step change, and focus-to-heading (the step's `<h1>`,
 *   which `<Prompt>` gives `tabIndex={-1}` for exactly this) so keyboard/screen-reader users land
 *   on the new question rather than a stale focus target.
 *
 * **Forced-dark scene** (Shelby, PRD-22 follow-up): the root carries `.dark`, so every child's
 * theme tokens (`<Prompt>`'s heading/description, the rail, error text) resolve to their dark
 * values regardless of the site theme. This is required, not a preference — the conversation
 * floats directly on the always-dark Earth, and dark text would be invisible against the globe;
 * light-on-scene is the only legible option. The rest of the site stays theme-aware; only this
 * scene is pinned dark, matching the approved prototype. Legibility without a card comes from
 * (a) an edge-weighted scrim that darkens the far left and right — where the rail and floating
 * text sit — while leaving the Earth bright through the centre, and (b) a soft text-shadow on the
 * floating column. Choice inputs use `<FloatingChoices>` (illuminated marker + label, no boxes)
 * and the survey's text fields are transparent underline inputs, so nothing occludes the globe.
 *
 * Below the ~760px breakpoint the globe stays mounted (`MeridianField`/`offsetXForWidth` already
 * recenters it) but a stronger scrim dims it and the floating text goes full-width/centered — the
 * Earth becomes texture, not focus, matching the prototype's mobile collapse.
 */
export function ConversationalStage({
  currentStep,
  completed,
  skipped,
  route,
  children,
}: ConversationalStageProps) {
  const panelRef = useRef<HTMLElement>(null);
  const hasMountedRef = useRef(false);

  useEffect(() => {
    const panel = panelRef.current;
    if (!panel) return;

    const reducedMotion =
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const isFirstRender = !hasMountedRef.current;
    hasMountedRef.current = true;
    const heading = panel.querySelector<HTMLElement>("h1");

    if (reducedMotion) {
      // No motion, but focus still moves — a11y should not be gated on prefers-reduced-motion.
      if (!isFirstRender) heading?.focus({ preventScroll: true });
      return;
    }

    // Deliberately `opacity`, not gsap's `autoAlpha` — `autoAlpha` also toggles CSS
    // `visibility`, which hides the panel from the accessibility tree (and therefore from
    // Testing Library's role queries) for the animation's duration. The panel must stay
    // discoverable throughout the transition, not just once it settles.
    const ctx = gsap.context(() => {
      gsap.fromTo(
        panel,
        { opacity: 0, y: 14, filter: "blur(4px)" },
        { opacity: 1, y: 0, filter: "blur(0px)", duration: 0.55, ease: "power2.out" },
      );
    }, panel);

    // Skip stealing focus on the very first paint (nothing to "return to" yet); every subsequent
    // step change moves focus once the enter transition has visibly begun.
    const focusTimer = isFirstRender
      ? undefined
      : window.setTimeout(() => heading?.focus({ preventScroll: true }), 60);

    return () => {
      ctx.revert();
      if (focusTimer !== undefined) window.clearTimeout(focusTimer);
    };
  }, [currentStep]);

  const satellite = satelliteLabel(currentStep, route);

  return (
    <div className="dark relative h-dvh overflow-hidden bg-[#05060c] text-white">
      <MeridianField currentStep={currentStep} route={route} />

      <div
        aria-hidden="true"
        className="pointer-events-none fixed inset-0 z-[5] bg-[#05060c]/65 min-[760px]:bg-gradient-to-r min-[760px]:from-[#05060c]/55 min-[760px]:via-[#05060c]/5 min-[760px]:to-[#05060c]/88"
      />

      {satellite && (
        <aside
          aria-label="Orbital status"
          className="pointer-events-none fixed top-6 left-6 z-10 hidden items-center gap-2.5 min-[760px]:flex"
        >
          <span className="relative flex h-2.5 w-2.5 shrink-0">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#5ad1ff]/50" />
            <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-[#5ad1ff] shadow-[0_0_18px_rgba(90,209,255,0.8)]" />
          </span>
          <span className="text-xs leading-tight">
            <span className="block text-[10px] font-semibold tracking-[0.16em] text-white/50 uppercase">
              Satellite over
            </span>
            <span className="font-semibold text-white">{satellite}</span>
          </span>
        </aside>
      )}

      {/* Progress rail — desktop: a vertical column docked left, echoing the landing page's
          progress line; mobile: a compact horizontal strip along the bottom. Unlike the
          card-less conversation, the rail keeps a faint dark-glass backing: it's a dense stack
          of small labels that needs a steadier surface than free-floating text to stay legible
          over the moving globe. Its tokens resolve dark (the scene forces `.dark`). */}
      <nav
        aria-label="Desktop onboarding progress"
        className="fixed top-1/2 left-5 z-20 hidden max-h-[82vh] -translate-y-1/2 min-[760px]:block"
      >
        <div className="border-border bg-card/70 dark:bg-card/50 rounded-2xl border p-2 shadow-2xl backdrop-blur-xl">
          <OnboardingProgressRail
            currentStep={currentStep}
            completed={completed}
            skipped={skipped}
            orientation="vertical"
          />
        </div>
      </nav>

      <div className="relative z-10 flex h-full items-center justify-center px-2 pt-4 pb-[calc(6rem+env(safe-area-inset-bottom))] min-[760px]:justify-center min-[760px]:px-[8vw] min-[760px]:py-24">
        {/* No card — the conversation floats directly over the Earth (Shelby: "no cards, so the
            Earth shows through"). Legibility is held by the edge scrim + a soft text-shadow, not a
            panel surface. The stage is a forced-dark scene (`.dark` on the root), so text tokens
            resolve light regardless of the site theme — dark text would vanish against the globe.
            The page itself never scrolls (root is `h-dvh overflow-hidden`, Shelby); if a step's
            content is taller than the available height (small laptop, zoomed text, mobile
            landscape) this column scrolls internally instead — `max-h-full` caps it against the
            fixed-height stage and `overflow-y-auto` takes over, with a thin/translucent scrollbar
            (both engines) so it doesn't fight the card-less, no-surface look. The nested inset
            keeps focus rings, helper text, and button shadows away from the scrollport edge. */}
        <main
          ref={panelRef}
          aria-label="Onboarding question"
          data-lenis-prevent
          className={cn(
            "max-h-full w-full max-w-[29rem] overflow-y-auto overscroll-contain text-white",
            "[text-shadow:0_1px_22px_rgba(5,6,12,0.65)]",
            "[scrollbar-width:thin] [scrollbar-color:rgba(255,255,255,0.35)_transparent]",
            "[&::-webkit-scrollbar]:w-1.5 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-white/25 [&::-webkit-scrollbar-track]:bg-transparent",
          )}
        >
          <div className="p-3 min-[760px]:p-4">{children}</div>
        </main>
      </div>

      <nav
        aria-label="Mobile onboarding progress"
        className="fixed inset-x-0 bottom-0 z-20 flex justify-center pb-[calc(1.25rem+env(safe-area-inset-bottom))] min-[760px]:hidden"
      >
        {/* eslint-disable jsx-a11y/no-noninteractive-tabindex -- Axe requires keyboard access to this Safari-scrollable region. */}
        <div
          aria-label="Scrollable onboarding steps"
          data-lenis-prevent
          tabIndex={0}
          className="border-border bg-card/70 dark:bg-card/50 focus-visible:ring-ring pointer-events-auto max-w-[92vw] overflow-x-auto rounded-2xl border px-3 py-2.5 backdrop-blur-xl focus-visible:ring-2 focus-visible:outline-none focus-visible:ring-inset"
        >
          <OnboardingProgressRail
            currentStep={currentStep}
            completed={completed}
            skipped={skipped}
            orientation="horizontal"
          />
        </div>
        {/* eslint-enable jsx-a11y/no-noninteractive-tabindex */}
      </nav>
    </div>
  );
}

/**
 * "Satellite over [City]" — the current orbit target's name, or "the global network" once
 * `finish` pulls the camera back (mirrors `cameraTargetForStep`'s pullback in `scene-helpers.ts`,
 * the single source of truth this reads from rather than re-deriving the step→city mapping).
 */
function satelliteLabel(step: OnboardingStep, route: Capital[]): string | null {
  if (route.length === 0) return null;
  if (step === "finish") return "the global network";
  const stepIndex = ROUTE_STEPS.indexOf(step);
  const cityIndex = stepIndex === -1 ? route.length - 1 : Math.min(stepIndex, route.length - 1);
  return route[cityIndex]?.name ?? null;
}
