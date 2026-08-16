"use client";

import { gsap } from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import { useEffect, useRef } from "react";

export interface PipelineStage {
  /** Two-digit order, e.g. "01". */
  num: string;
  /** Short rail label, e.g. "CARROT". */
  short: string;
  /** Full stage name, e.g. "CARROT · Complexity". */
  name: string;
  desc: string;
  /** Small label before the signal, e.g. "emits". */
  signalLabel: string;
  /** The real signal, e.g. "x-gaussmeridian-complexity: 0.34". */
  signal: string;
}

/**
 * The signature deep-dive: a sticky meridian rail whose nodes turn blue as a scrubbed pulse
 * passes them, while each stage sharpens into focus. On desktop it's a scroll-scrubbed
 * scrollytelling; on mobile (or reduced-motion) it degrades to a static left-border timeline,
 * fully readable. Every stage shows the real `x-gaussmeridian-*` signal it emits.
 */
export function PipelineScrollytelling({
  eyebrow,
  heading,
  stages,
}: {
  eyebrow: string;
  heading: string;
  stages: PipelineStage[];
}) {
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    if (!window.matchMedia("(min-width: 821px)").matches) return; // mobile = static timeline

    gsap.registerPlugin(ScrollTrigger);
    const ctx = gsap.context(() => {
      const nodes = gsap.utils.toArray<HTMLElement>(".pl-node", root);
      const n = Math.max(nodes.length - 1, 1);
      const st = { trigger: root, start: "top 30%", end: "bottom 75%", scrub: 0.4 } as const;

      gsap.to(".pl-fill", { scaleY: 1, ease: "none", scrollTrigger: st });
      gsap.to(".pl-pulse", {
        top: "100%",
        ease: "none",
        scrollTrigger: {
          ...st,
          onUpdate: (self) => {
            const p = self.progress;
            nodes.forEach((node, i) => node.classList.toggle("pl-lit", p >= i / n - 0.0001));
          },
        },
      });

      gsap.utils.toArray<HTMLElement>(".pl-card", root).forEach((card) => {
        ScrollTrigger.create({
          trigger: card,
          start: "top 55%",
          end: "bottom 45%",
          onToggle: (self) => card.classList.toggle("pl-active", self.isActive),
        });
      });
    }, root);

    return () => ctx.revert();
  }, [stages.length]);

  return (
    <section
      ref={rootRef}
      className="relative mx-auto grid max-w-6xl gap-10 px-6 py-16 md:grid-cols-[300px_1fr]"
    >
      <div className="top-0 hidden h-screen flex-col justify-center gap-8 py-24 md:sticky md:flex">
        <div>
          <div className="text-muted-foreground font-mono text-[11px] tracking-[0.22em] uppercase">
            {eyebrow}
          </div>
          <h2 className="font-display mt-2.5 max-w-[16ch] text-3xl font-semibold tracking-tight">
            {heading}
          </h2>
        </div>
        <div className="relative flex h-[54vh] w-full flex-col justify-between">
          <span className="pl-rail-track" />
          <span className="pl-fill" />
          <span className="pl-pulse" />
          {stages.map((s, i) => (
            <div key={s.num} className="pl-node relative z-[2] flex items-center gap-3" data-i={i}>
              <span className="pl-bullet" />
              <span className="pl-label font-mono text-xs">
                {s.num} · {s.short}
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="md:pt-[14vh]">
        {stages.map((s) => (
          <div
            key={s.num}
            className="pl-card border-border flex min-h-[70vh] items-center border-l-2 pl-6 md:min-h-screen md:border-l-0 md:pl-0"
          >
            <div className="pl-body">
              <span className="font-mono text-xs tracking-[0.2em] text-[var(--gauss-500)]">
                {s.num}
              </span>
              <span className="font-display mt-2.5 block text-2xl font-semibold tracking-tight md:text-3xl">
                {s.name}
              </span>
              <p className="text-muted-foreground mt-3.5 max-w-[46ch] text-[17px] leading-relaxed">
                {s.desc}
              </p>
              <div
                className="text-primary mt-4 inline-flex items-center gap-2.5 rounded-lg border px-3 py-2 font-mono text-[12.5px]"
                style={{
                  background: "color-mix(in srgb, var(--accent) 8%, transparent)",
                  borderColor: "color-mix(in srgb, var(--accent) 22%, transparent)",
                }}
              >
                <span className="text-muted-foreground text-[10px] tracking-[0.14em] uppercase">
                  {s.signalLabel}
                </span>
                <code>{s.signal}</code>
              </div>
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
