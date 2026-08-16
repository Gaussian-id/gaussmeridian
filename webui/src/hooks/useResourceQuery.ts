"use client";

import { useQuery } from "@tanstack/react-query";

import { useDataQuery } from "@core/adapters";

import type { ZodType } from "zod";

/**
 * Reads a resource through the data adapter via TanStack Query. This is the standard way
 * components consume server data — never call an adapter or `fetch` directly in a component.
 */
export function useResourceQuery<T>(options: {
  resource: string;
  params?: Record<string, string | number | boolean | undefined>;
  schema: ZodType<T>;
  enabled?: boolean;
}) {
  const data = useDataQuery();
  return useQuery({
    queryKey: [options.resource, options.params ?? null],
    queryFn: () =>
      data.query({ resource: options.resource, params: options.params, schema: options.schema }),
    enabled: options.enabled,
  });
}
