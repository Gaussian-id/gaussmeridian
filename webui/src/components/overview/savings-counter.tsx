"use client";

import { useEffect, useRef, useState } from "react";

interface SavingsCounterProps {
  /** Total dollars OutcomeGate did not charge for failed calls, over the period. */
  total: number;
  /** How many individual calls were not charged. */
  count: number;
  isLoading?: boolean;
}

const COUNT_UP_DURATION_MS = 900;

function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/**
 * The OutcomeGate hero number: "$X not charged for failed calls." Animated count-up on load —
 * gated behind `prefers-reduced-motion`, which renders the final value immediately. This is
 * the single most load-bearing figure on the Overview page, so it must never be illegible
 * mid-animation for a user who has asked for less motion.
 */
export function SavingsCounter({ total, count, isLoading }: SavingsCounterProps) {
  const [display, setDisplay] = useState(0);
  const frameRef = useRef<number>(0);

  useEffect(() => {
    if (isLoading) return;

    // Reduced motion completes in a single frame — indistinguishable from "instant" to the
    // user, while still only ever calling `setDisplay` from inside the rAF callback (an event),
    // never synchronously in the effect body.
    const durationMs = prefersReducedMotion() ? 0 : COUNT_UP_DURATION_MS;
    const start = performance.now();
    function tick(now: number) {
      const progress = durationMs === 0 ? 1 : Math.min((now - start) / durationMs, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      setDisplay(total * eased);
      if (progress < 1) {
        frameRef.current = requestAnimationFrame(tick);
      }
    }
    frameRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frameRef.current);
  }, [total, isLoading]);

  return (
    <div>
      <p className="font-mono text-[11px] tracking-[0.28em] text-white/55 uppercase">
        Not charged for failed calls
      </p>
      <p className="font-display mt-2 text-5xl font-semibold tracking-tight text-white sm:text-6xl">
        {isLoading ? "—" : `$${display.toFixed(2)}`}
      </p>
      <p className="mt-2 text-sm text-white/65">
        {isLoading
          ? "Loading…"
          : `${count.toLocaleString()} ${count === 1 ? "call" : "calls"} failed OutcomeGate — $0.00 charged`}
      </p>
    </div>
  );
}
