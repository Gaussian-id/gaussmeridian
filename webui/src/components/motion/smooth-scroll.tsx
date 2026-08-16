"use client";

import { ReactLenis } from "lenis/react";

import type { ReactNode } from "react";

/** App-wide smooth scrolling. Restrained, precise defaults — not floaty. */
export function SmoothScroll({ children }: { children: ReactNode }) {
  return (
    <ReactLenis root options={{ lerp: 0.1, duration: 1.1, smoothWheel: true }}>
      {children}
    </ReactLenis>
  );
}
