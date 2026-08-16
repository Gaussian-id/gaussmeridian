"use client";

import { useEffect, useRef } from "react";

import { cn } from "@core/lib/utils";

import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";
import type { Capital } from "@/lib/onboarding/route";

import { resolveReducedMotion } from "./scene-helpers";

import type { MeridianFieldScene } from "./earth-scene";

export interface MeridianFieldProps {
  currentStep: OnboardingStep;
  route: Capital[];
  className?: string;
}

/**
 * `<MeridianField>` — the self-contained, lazy-loaded reactive-Earth background for onboarding
 * (PRD-22 Phase B). A thin React shell: it owns the canvas/labels DOM, dynamically imports
 * `./earth-scene` (and, transitively, `three`) inside an effect so neither ships in the initial
 * bundle, and forwards `currentStep` / reduced-motion changes into the imperative scene. All the
 * actual three.js work — the day/night Earth, camera choreography, nodes/arcs/labels — lives in
 * `earth-scene.ts`; this component renders nothing meaningful until that import resolves
 * client-side, so it's safe under SSR.
 *
 * The canvas is decorative: `aria-hidden`, `pointer-events-none`, and it never receives focus —
 * it must never compete with the onboarding form rendered above it.
 */
export function MeridianField({ currentStep, route, className }: MeridianFieldProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const labelsRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<MeridianFieldScene | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const labelsContainer = labelsRef.current;
    if (!canvas || !labelsContainer || route.length === 0) return;

    let cancelled = false;
    const reducedMotionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    // Captured once at mount — the step-forwarding effect below keeps the live scene in sync,
    // this only seeds the scene's starting orbit target before the first `setStep` call lands.
    const initialStep = currentStep;

    void import("./earth-scene").then(({ MeridianFieldScene: SceneCtor }) => {
      if (cancelled) return;
      sceneRef.current = new SceneCtor({
        canvas,
        labelsContainer,
        route,
        initialStep,
        reducedMotion: resolveReducedMotion(reducedMotionQuery),
      });
    });

    const onMotionChange = () => {
      sceneRef.current?.setReducedMotion(resolveReducedMotion(reducedMotionQuery));
    };
    reducedMotionQuery.addEventListener("change", onMotionChange);

    const onResize = () => {
      sceneRef.current?.resize(window.innerWidth, window.innerHeight);
    };
    window.addEventListener("resize", onResize);

    return () => {
      cancelled = true;
      reducedMotionQuery.removeEventListener("change", onMotionChange);
      window.removeEventListener("resize", onResize);
      sceneRef.current?.dispose();
      sceneRef.current = null;
    };
    // route identity is expected to be stable for the lifetime of the wizard (Phase C picks it
    // once per session); currentStep is deliberately omitted and forwarded via the effect below
    // instead, so a step change doesn't tear down and rebuild the whole scene.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route]);

  useEffect(() => {
    sceneRef.current?.setStep(currentStep);
  }, [currentStep]);

  return (
    <div
      className={cn("pointer-events-none fixed inset-0 z-0 overflow-hidden", className)}
      aria-hidden="true"
    >
      <canvas ref={canvasRef} className="h-full w-full" />
      <div ref={labelsRef} className="pointer-events-none absolute inset-0" />
    </div>
  );
}
