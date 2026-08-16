import Link from "next/link";

import { siteConfig } from "@core/config/site.config";

import { MeridianMark } from "@/components/auth/meridian-mark";

import type { ReactNode } from "react";

/** Split auth chrome: a branded Meridian panel on the left (desktop), the form on the right. */
export default function AuthLayout({ children }: { children: ReactNode }) {
  return (
    <div className="grid min-h-dvh lg:grid-cols-2">
      <aside className="bg-brand-gradient relative hidden flex-col justify-between overflow-hidden p-12 text-white lg:flex">
        {/* Atmosphere: a faint grid + a static wireframe meridian globe bleeding off the right edge. */}
        <div className="bg-grid absolute inset-0 opacity-[0.12]" />
        <svg
          viewBox="0 0 400 400"
          aria-hidden="true"
          className="pointer-events-none absolute top-1/2 -right-28 h-[42rem] w-[42rem] -translate-y-1/2 text-white/20"
        >
          <g fill="none" stroke="currentColor" strokeWidth="1">
            <circle cx="200" cy="200" r="159" />
            <ellipse cx="200" cy="200" rx="53" ry="159" />
            <ellipse cx="200" cy="200" rx="106" ry="159" />
            <ellipse cx="200" cy="200" rx="159" ry="53" />
            <ellipse cx="200" cy="200" rx="159" ry="106" />
            <line x1="200" y1="41" x2="200" y2="359" />
            <line x1="41" y1="200" x2="359" y2="200" />
          </g>
          {/* The meridian line. */}
          <ellipse
            cx="200"
            cy="200"
            rx="53"
            ry="159"
            fill="none"
            stroke="var(--gauss-400)"
            strokeWidth="2.5"
            opacity="0.85"
          />
        </svg>

        <Link href="/" className="relative flex items-center gap-2.5">
          <span className="grid h-9 w-9 place-items-center rounded-xl bg-white/10 ring-1 ring-white/15">
            <MeridianMark className="h-5 w-5 text-white" />
          </span>
          <span className="font-display text-xl font-semibold tracking-tight">
            {siteConfig.shortName}
          </span>
        </Link>

        <div className="relative flex flex-col gap-4">
          <p className="font-display text-4xl leading-[1.05] font-semibold tracking-tight text-balance">
            The smartest path for every prompt.
          </p>
          <p className="max-w-sm text-sm leading-relaxed text-white/70">
            One endpoint for every model. Meridian routes each request down the line that fits, and
            only bills for answers that hold.
          </p>
        </div>

        <span className="relative font-mono text-[11px] tracking-[0.2em] text-white/45 uppercase">
          Built by Gaussian
        </span>
      </aside>

      <div className="relative flex items-center justify-center px-6 py-12">
        <div className="bg-radial-glow pointer-events-none absolute inset-0" />
        <div className="relative w-full max-w-sm">{children}</div>
      </div>
    </div>
  );
}
