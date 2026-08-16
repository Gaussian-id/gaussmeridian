"use client";

import { ThemeProvider as NextThemesProvider } from "next-themes";

import { themeConfig } from "@theme/theme.config";

import type { ReactNode } from "react";

/** Class-based theming. Light is the default; dark is a toggle off the same tokens. */
export function ThemeProvider({ children }: { children: ReactNode }) {
  return (
    <NextThemesProvider
      attribute="class"
      defaultTheme={themeConfig.defaultMode}
      enableSystem={themeConfig.enableSystem}
      storageKey={themeConfig.storageKey}
      disableTransitionOnChange
    >
      {children}
    </NextThemesProvider>
  );
}
