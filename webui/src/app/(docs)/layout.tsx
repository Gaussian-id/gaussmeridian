import { Shell } from "@/components/layout/shell";

import type { ReactNode } from "react";

/**
 * Docs chrome: the same Navbar/Footer shell as the marketing pages, deliberately without the
 * meridian globe.
 *
 * The globe is an animated WebGL canvas fixed behind the whole page. That is right for a landing
 * page someone scrolls through once, and wrong behind documentation someone reads for twenty
 * minutes while copying commands out of it — it moves under the text and competes with the code
 * blocks for attention.
 *
 * `/docs` lives in its own route group rather than under `(marketing)` so it can opt out of that
 * background entirely. Route groups do not affect the URL, so the page is still served at `/docs`.
 */
export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="relative z-10">
      <Shell>{children}</Shell>
    </div>
  );
}
