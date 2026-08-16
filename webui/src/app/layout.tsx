import "@theme/globals.css";

import { siteConfig } from "@core/config";
import { fontVariables } from "@core/config/fonts";
import { RootProviders } from "@core/providers";

import { SmoothScroll } from "@/components/motion";

import type { Metadata } from "next";

export const metadata: Metadata = {
  title: { default: siteConfig.name, template: `%s · ${siteConfig.name}` },
  description: siteConfig.description,
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang={siteConfig.locale} className={fontVariables} suppressHydrationWarning>
      <body className="min-h-dvh">
        <RootProviders>
          <SmoothScroll>{children}</SmoothScroll>
        </RootProviders>
      </body>
    </html>
  );
}
