"use client";

import { useEffect, useRef } from "react";

/** Motion is on only for fine pointers without a reduced-motion preference. */
export function cursorMotionEnabled(): boolean {
  if (typeof window === "undefined") return false;
  return (
    !window.matchMedia("(prefers-reduced-motion: reduce)").matches &&
    window.matchMedia("(pointer: fine)").matches
  );
}

/**
 * Lerped parallax — the element drifts with the cursor. Attach the returned ref to a layer.
 * Disabled (no-op) under reduced-motion / coarse pointers.
 */
export function useParallax<T extends HTMLElement>(strength = 8) {
  const ref = useRef<T>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el || !cursorMotionEnabled()) return;

    let tx = 0;
    let ty = 0;
    let x = 0;
    let y = 0;
    let frame = 0;

    const onMove = (e: PointerEvent) => {
      tx = (e.clientX / window.innerWidth - 0.5) * 2;
      ty = (e.clientY / window.innerHeight - 0.5) * 2;
    };
    const loop = () => {
      x += (tx - x) * 0.06;
      y += (ty - y) * 0.06;
      el.style.transform = `translate3d(${x * strength}px, ${y * strength}px, 0)`;
      frame = requestAnimationFrame(loop);
    };

    window.addEventListener("pointermove", onMove, { passive: true });
    frame = requestAnimationFrame(loop);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("pointermove", onMove);
      el.style.transform = "";
    };
  }, [strength]);

  return ref;
}

/**
 * 3D tilt toward the cursor on hover. Attach the returned ref to a card.
 * Disabled (no-op) under reduced-motion / coarse pointers.
 */
export function useTilt<T extends HTMLElement>(max = 8) {
  const ref = useRef<T>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el || !cursorMotionEnabled()) return;

    const onMove = (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      const px = (e.clientX - r.left) / r.width - 0.5;
      const py = (e.clientY - r.top) / r.height - 0.5;
      el.style.transition = "transform .1s ease";
      el.style.transform = `perspective(820px) rotateY(${px * max}deg) rotateX(${-py * max}deg)`;
    };
    const reset = () => {
      el.style.transition = "transform .45s ease";
      el.style.transform = "";
    };

    el.addEventListener("pointermove", onMove);
    el.addEventListener("pointerleave", reset);
    return () => {
      el.removeEventListener("pointermove", onMove);
      el.removeEventListener("pointerleave", reset);
      el.style.transform = "";
    };
  }, [max]);

  return ref;
}
