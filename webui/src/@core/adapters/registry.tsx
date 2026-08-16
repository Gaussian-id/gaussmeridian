"use client";

import { createContext, useContext, useMemo, type ReactNode } from "react";

import { createDefaultRegistry } from "./create-default-registry";

import type { AdapterRegistry } from "./types";

const AdapterContext = createContext<AdapterRegistry | null>(null);

/**
 * THE SEAM. Every backend capability is injected here. In production it resolves the
 * default HTTP registry; tests and forks pass `registry` to swap in fakes or alternatives.
 */
export function AdapterProvider({
  children,
  registry,
}: {
  children: ReactNode;
  registry?: AdapterRegistry;
}) {
  const value = useMemo(() => registry ?? createDefaultRegistry(), [registry]);
  return <AdapterContext.Provider value={value}>{children}</AdapterContext.Provider>;
}

export function useAdapters(): AdapterRegistry {
  const ctx = useContext(AdapterContext);
  if (!ctx) {
    throw new Error("useAdapters must be used within an <AdapterProvider>");
  }
  return ctx;
}

export function useLlm() {
  return useAdapters().llm;
}

export function useDataQuery() {
  return useAdapters().data;
}

export function useAuth() {
  return useAdapters().auth;
}
