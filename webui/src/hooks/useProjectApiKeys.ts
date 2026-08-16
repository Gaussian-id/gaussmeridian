"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";

import { useDataQuery } from "@core/adapters";
import {
  CreateApiKeyResponseSchema,
  ProjectApiKeySchema,
} from "@core/adapters/schemas/gaussmeridian.schema";

import { useResourceQuery } from "./useResourceQuery";

/**
 * API keys live at `v1/api/keys`, not under the org/project path.
 *
 * These hooks used to address `v1/orgs/{orgId}/projects/{projectId}/keys`, which the gateway has
 * never served — `routes.rs` nests the key routes under `/api`, and the org routes stop at
 * `/:id/projects/:pid`. Every create, list, and revoke from the project keys page 404'd, so the
 * console could not issue an API key at all.
 *
 * The gateway scopes a key by `project_id` in the request body rather than by URL, and returns the
 * caller's keys as one collection. Project scoping is therefore applied here: the list is filtered
 * to the project being viewed, and creation passes the project through.
 */
const KEYS_RESOURCE = "v1/api/keys";
const REVOKE_RESOURCE = "v1/api/keys/revoke";

export function useProjectApiKeys(orgId: string, projectId: string) {
  return useResourceQuery({
    resource: KEYS_RESOURCE,
    // `GET v1/api/keys` returns every key the caller owns, across projects. Narrow it to the
    // project on screen so the page does not show keys belonging to a different one.
    schema: z
      .array(ProjectApiKeySchema)
      .transform((keys) => keys.filter((key) => key.project_id === projectId)),
    enabled: Boolean(orgId && projectId),
  });
}

export function useCreateProjectApiKey(orgId: string, projectId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (
      input: {
        name?: string;
        rate_limit_per_minute?: number;
        rate_limit_per_day?: number;
        expires_in_days?: number;
      } = {},
    ) =>
      data.query({
        resource: KEYS_RESOURCE,
        method: "POST",
        // The project is carried in the body, not the path. Without it the gateway stores the key
        // unscoped, and generation later fails with `project_scope_required`.
        body: { ...input, project_id: projectId },
        schema: CreateApiKeyResponseSchema,
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: [KEYS_RESOURCE, null] }),
  });
}

export function useRevokeProjectApiKey(orgId: string, projectId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();

  return useMutation({
    // Revocation is a POST to a dedicated endpoint with the key id in the body — there is no
    // DELETE route for an individual key.
    mutationFn: (keyId: string) =>
      data.query({
        resource: REVOKE_RESOURCE,
        method: "POST",
        body: { key_id: keyId },
        schema: z.unknown(),
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: [KEYS_RESOURCE, null] }),
  });
}
