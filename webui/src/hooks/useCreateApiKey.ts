"use client";

import { useMutation } from "@tanstack/react-query";

import { useDataQuery } from "@core/adapters";
import { CreateApiKeyResponseSchema } from "@core/adapters/schemas/gaussmeridian.schema";

/**
 * Creates a new API key via POST /v1/api/keys. The response's `api_key` is the raw secret,
 * shown once — the backend never returns it again on subsequent reads.
 *
 * `project_id` (DR-012 — API-key project scoping) is optional at the API boundary but the
 * onboarding wizard's "first API key" step always supplies the `project_id` it just created
 * (per PRD-21 Wave B), so the key isn't left unscoped.
 */
export function useCreateApiKey() {
  const data = useDataQuery();
  return useMutation({
    mutationFn: (input: { name?: string; project_id?: string } = {}) =>
      data.query({
        resource: "v1/api/keys",
        method: "POST",
        body: input,
        schema: CreateApiKeyResponseSchema,
      }),
  });
}
