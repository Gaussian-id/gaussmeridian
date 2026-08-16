import { gaussMeridianRawRequest } from "./gaussmeridian-data.adapter";

import type { DataQueryAdapter, DataQueryInput } from "./types";

const PROJECT_ROUTES_RE = /^v1\/projects\/([^/]+)\/routes$/;
const PROJECT_SAVINGS_RE = /^v1\/projects\/([^/]+)\/savings$/;
const PROJECT_USAGE_RE = /^v1\/projects\/([^/]+)\/usage-analytics$/;
const PROJECT_REQUEST_LOGS_RE = /^v1\/projects\/([^/]+)\/request-logs$/;

/**
 * Resource-aware decorator for the PRD-21 Wave C transparency resources (route decisions +
 * savings). The FE keeps its project-scoped resource strings (`v1/projects/:id/routes`,
 * `v1/projects/:id/savings`) as stable TanStack Query cache keys — but the REAL backend never
 * takes a project id in these paths: `GET /v1/route-decisions` and `GET /v1/analytics/savings`
 * both resolve the caller's project server-side from the auth context
 * (`resolve_caller_project_id`, handlers.rs). This decorator rewrites the resource string to
 * the real path and forwards query params untouched (`limit`, `start_date`/`end_date`).
 *
 * No DTO reshaping is needed here (unlike `console-org.adapter.ts`'s org/project/member
 * mapping) — the real response bodies already match `console.schema.ts`'s
 * `RouteDecisionListSchema`/`OutcomeSavingsSchema` once those schemas were corrected to the
 * real backend shape (see that file's doc comments). The single `id`/`request_id` in the
 * project-scoped resource string is discarded, not forwarded — the real endpoint has nowhere to
 * put it and doesn't need it (the caller's project is resolved from their session, not a URL
 * segment), so a caller can never address another project's decisions by editing the id in the
 * FE-facing resource string.
 */
export function createTransparencyDataAdapter(base: DataQueryAdapter): DataQueryAdapter {
  return {
    async query<T>(input: DataQueryInput<T>): Promise<T> {
      const { resource, params, schema } = input;

      const routesMatch = resource.match(PROJECT_ROUTES_RE);
      if (routesMatch) {
        const json = await gaussMeridianRawRequest({
          resource: "v1/route-decisions",
          params,
          projectId: routesMatch[1],
        });
        return schema.parse(json) as T;
      }

      const savingsMatch = resource.match(PROJECT_SAVINGS_RE);
      if (savingsMatch) {
        const json = await gaussMeridianRawRequest({
          resource: "v1/analytics/savings",
          params,
          projectId: savingsMatch[1],
        });
        return schema.parse(json) as T;
      }

      const usageMatch = resource.match(PROJECT_USAGE_RE);
      if (usageMatch) {
        const json = await gaussMeridianRawRequest({
          resource: "v1/analytics/usage",
          params,
          projectId: usageMatch[1],
        });
        return schema.parse(json) as T;
      }

      const requestLogsMatch = resource.match(PROJECT_REQUEST_LOGS_RE);
      if (requestLogsMatch) {
        const json = await gaussMeridianRawRequest({
          resource: "v1/logs",
          params,
          projectId: requestLogsMatch[1],
        });
        return schema.parse(json) as T;
      }

      return base.query(input);
    },
  };
}
