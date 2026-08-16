"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";

import { useDataQuery } from "@core/adapters";
import {
  CreateApiKeyResponseSchema,
  ProjectApiKeySchema,
} from "@core/adapters/schemas/gaussmeridian.schema";

import { useResourceQuery } from "./useResourceQuery";

function projectKeysResource(orgId: string, projectId: string): string {
  return `v1/orgs/${encodeURIComponent(orgId)}/projects/${encodeURIComponent(projectId)}/keys`;
}

export function useProjectApiKeys(orgId: string, projectId: string) {
  return useResourceQuery({
    resource: projectKeysResource(orgId, projectId),
    schema: z.array(ProjectApiKeySchema),
    enabled: Boolean(orgId && projectId),
  });
}

export function useCreateProjectApiKey(orgId: string, projectId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const resource = projectKeysResource(orgId, projectId);

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
        resource,
        method: "POST",
        body: input,
        schema: CreateApiKeyResponseSchema,
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: [resource, null] }),
  });
}

export function useRevokeProjectApiKey(orgId: string, projectId: string) {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  const resource = projectKeysResource(orgId, projectId);

  return useMutation({
    mutationFn: (keyId: string) =>
      data.query({
        resource: `${resource}/${encodeURIComponent(keyId)}`,
        method: "DELETE",
        schema: z.undefined(),
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: [resource, null] }),
  });
}
