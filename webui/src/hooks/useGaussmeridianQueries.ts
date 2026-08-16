"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";

import { useDataQuery } from "@core/adapters";
import {
  ApiKeySchema,
  BalanceInfoSchema,
  BillingSummarySchema,
  BudgetStatusSchema,
  ByokProvidersSchema,
  CostAnalyticsSchema,
  HealthSchema,
  ModelInfoSchema,
  ModelPricingResponseSchema,
  ModelsResponseSchema,
  ProjectSettingsSchema,
  RequestLogSchema,
  RoutingTraceResponseSchema,
  UsageAnalyticsSchema,
} from "@core/adapters/schemas/gaussmeridian.schema";
import { projectRequestLogsResource, projectUsageAnalyticsResource } from "@core/config/resources";

import { useResourceQuery } from "./useResourceQuery";

const API_KEYS_RESOURCE = "v1/api/keys";
const BYOK_RESOURCE = "v1/byok/keys";
const PROJECT_SETTINGS_RESOURCE = "v1/project/settings";

export function useHealth() {
  return useResourceQuery({ resource: "health", schema: HealthSchema });
}

export function useModels() {
  return useResourceQuery({ resource: "v1/models", schema: ModelsResponseSchema });
}

export function useModelDetail(model: string) {
  return useResourceQuery({
    resource: `v1/models/${model}`,
    schema: ModelInfoSchema,
    enabled: Boolean(model),
  });
}

export function useModelPricing() {
  return useResourceQuery({ resource: "v1/billing/models", schema: ModelPricingResponseSchema });
}

export function useBalance() {
  return useResourceQuery({ resource: "v1/balance", schema: BalanceInfoSchema });
}

export function useBudgetStatus() {
  return useResourceQuery({ resource: "v1/billing/budget", schema: BudgetStatusSchema });
}

export function useBillingSummary() {
  return useResourceQuery({ resource: "v1/billing/summary", schema: BillingSummarySchema });
}

export function useUsageAnalytics(params?: { range?: "24h" | "7d" | "30d" | "90d" }) {
  return useResourceQuery({ resource: "v1/analytics/usage", params, schema: UsageAnalyticsSchema });
}

export function useProjectUsageAnalytics(
  projectId: string,
  params?: { range?: "24h" | "7d" | "30d" | "90d" },
) {
  return useResourceQuery({
    resource: projectUsageAnalyticsResource(projectId),
    params,
    schema: UsageAnalyticsSchema,
    enabled: Boolean(projectId),
  });
}

export function useRequestLogs(params?: { limit?: number }) {
  return useResourceQuery({ resource: "v1/logs", params, schema: z.array(RequestLogSchema) });
}

export function useProjectRequestLogs(projectId: string, params?: { limit?: number }) {
  return useResourceQuery({
    resource: projectRequestLogsResource(projectId),
    params,
    schema: z.array(RequestLogSchema),
    enabled: Boolean(projectId),
  });
}

/**
 * Per-model/per-provider cost breakdown for a date range via GET /v1/analytics/cost.
 * Gated server-side by the `analytics.cost.read` RBAC permission (unlike `/v1/analytics/usage`,
 * which has no such gate) — a caller without it gets a 403, which callers must render as an
 * honest "not available for your account" state rather than assuming this always succeeds.
 */
export function useCostAnalytics(params?: { start_date?: string; end_date?: string }) {
  return useResourceQuery({ resource: "v1/analytics/cost", params, schema: CostAnalyticsSchema });
}

/**
 * The N most recent routed requests (ledger-sourced, same rows as `/v1/logs`) via
 * GET /v1/routing/trace?limit=N — feeds the Overview ops band. `limit` is capped at 100
 * server-side; the response envelope also echoes back `total`/`limit` for that window.
 */
export function useRoutingTrace(params?: { limit?: number }) {
  return useResourceQuery({
    resource: "v1/routing/trace",
    params,
    schema: RoutingTraceResponseSchema,
  });
}

export function useApiKeys() {
  return useResourceQuery({ resource: API_KEYS_RESOURCE, schema: z.array(ApiKeySchema) });
}

/**
 * Revokes an API key via POST /v1/api/keys/revoke. The backend returns an untyped JSON object
 * (`{ message, key_id }`) rather than a validated struct, so the response is accepted loosely —
 * a 2xx status is what actually signals success. Invalidates the key list on success so the
 * revoked row's status refreshes.
 */
export function useRevokeApiKey() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { key_id: string }) =>
      data.query({
        resource: "v1/api/keys/revoke",
        method: "POST",
        body: input,
        schema: z.unknown(),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [API_KEYS_RESOURCE, null] });
    },
  });
}

/** Provider names with a BYOK key registered on the caller's project — names only,
 *  the encrypted key material never returns to the client. */
export function useByokProviders() {
  return useResourceQuery({ resource: BYOK_RESOURCE, schema: ByokProvidersSchema });
}

/**
 * Registers (encrypts + stores) a provider API key via POST /v1/byok/keys. The key is sent
 * to the backend exactly once and never comes back. 403 means the account isn't on the BYOK
 * allowlist; 503 means the server vault isn't configured (`BYOK_MASTER_KEY` unset).
 */
export function useRegisterByokKey() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { provider: string; api_key: string }) =>
      data.query({
        resource: BYOK_RESOURCE,
        method: "POST",
        body: input,
        schema: z.unknown(),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [BYOK_RESOURCE, null] });
    },
  });
}

/** The caller's project routing/billing settings, resolved server-side from their session. */
export function useProjectSettings() {
  return useResourceQuery({ resource: PROJECT_SETTINGS_RESOURCE, schema: ProjectSettingsSchema });
}

/**
 * Partial update via PATCH /v1/project/settings — only the provided fields change.
 * The backend validates lambda/quality_floor into [0,1], budget_monthly >= 0, and
 * requires an https alert webhook; the settings form mirrors those rules client-side.
 */
export function useUpdateProjectSettings() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      lambda?: number;
      quality_floor?: number;
      budget_monthly?: number;
      hard_limit?: boolean;
      alert_webhook_url?: string;
    }) =>
      data.query({
        resource: PROJECT_SETTINGS_RESOURCE,
        method: "PATCH",
        body: input,
        schema: ProjectSettingsSchema,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [PROJECT_SETTINGS_RESOURCE, null] });
    },
  });
}

/** Removes a registered provider key via DELETE /v1/byok/keys/:provider. */
export function useDeleteByokKey() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { provider: string }) =>
      data.query({
        resource: `${BYOK_RESOURCE}/${input.provider}`,
        method: "DELETE",
        schema: z.unknown(),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [BYOK_RESOURCE, null] });
    },
  });
}
