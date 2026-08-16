import { Footer } from "@/components/layout/footer";
import { Navbar } from "@/components/layout/navbar";

import type { ReactNode } from "react";

/** The app frame: navigation + main + footer. Pages render inside <Shell>. */
export function Shell({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-dvh flex-col">
      <Navbar />
      <main className="flex-1">{children}</main>
      <Footer />
    </div>
  );
}
