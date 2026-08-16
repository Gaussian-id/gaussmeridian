"use client";

import { gsap } from "gsap";
import { useEffect, useRef, type ReactNode } from "react";

/** A restrained, on-brand entrance — fade + small rise on mount. */
export function Reveal({ children, delay = 0 }: { children: ReactNode; delay?: number }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const ctx = gsap.context(() => {
      gsap.from(el, { autoAlpha: 0, y: 16, duration: 0.6, ease: "power2.out", delay });
    }, el);
    return () => ctx.revert();
  }, [delay]);

  return <div ref={ref}>{children}</div>;
}
