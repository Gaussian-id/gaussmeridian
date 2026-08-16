"use client";

import { AdapterProvider } from "@core/adapters";

import { QueryProvider } from "./query-provider";
import { ThemeProvider } from "./theme-provider";

import type { ReactNode } from "react";

/** Composes every app-wide provider in one place. Wrapped once in the root layout. */
export function RootProviders({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider>
      <QueryProvider>
        <AdapterProvider>{children}</AdapterProvider>
      </QueryProvider>
    </ThemeProvider>
  );
}
