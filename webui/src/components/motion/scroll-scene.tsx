"use client";

import { gsap } from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { useLenis } from "lenis/react";
import { useEffect, useRef, type ReactNode } from "react";

/**
 * Bridges the app-wide Lenis smooth scroll to GSAP ScrollTrigger — every Lenis scroll updates
 * ScrollTrigger so scroll-driven reveals and scrubs stay in sync. Mount once in the marketing layout.
 */
export function ScrollTriggerBridge() {
  useEffect(() => {
    gsap.registerPlugin(ScrollTrigger);
  }, []);
  useLenis(() => ScrollTrigger.update());
  return null;
}

/**
 * Below-the-fold reveal: fades + rises the wrapped content when it scrolls into view. Under
 * reduced-motion it renders visible with no animation. The content is always in the DOM
 * regardless of JS (progressive enhancement) — this only animates it in.
 */
export function ScrollReveal({
  children,
  y = 24,
  delay = 0,
  className,
}: {
  children: ReactNode;
  y?: number;
  delay?: number;
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    gsap.registerPlugin(ScrollTrigger);
    const ctx = gsap.context(() => {
      gsap.from(el, {
        y,
        duration: 0.6,
        ease: "power2.out",
        delay,
        scrollTrigger: { trigger: el, start: "top 85%" },
      });
    }, el);
    return () => ctx.revert();
  }, [y, delay]);

  return (
    <div ref={ref} className={className}>
      {children}
    </div>
  );
}
