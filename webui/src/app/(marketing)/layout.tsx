import { Shell } from "@/components/layout/shell";
import { MeridianGlobe, ScrollTriggerBridge } from "@/components/motion";

import type { ReactNode } from "react";

/** Public marketing chrome: the meridian globe background + Navbar/Footer shell. */
export default function MarketingLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <MeridianGlobe />
      <ScrollTriggerBridge />
      <div className="relative z-10">
        <Shell>{children}</Shell>
      </div>
    </>
  );
}
