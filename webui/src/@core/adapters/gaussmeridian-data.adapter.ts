import { GaussMeridianErrorSchema } from "./schemas/gaussmeridian-error.schema";

import type { DataQueryAdapter, DataQueryInput } from "./types";

const SAFE_PROJECT_CONTEXT = /^[A-Za-z0-9][A-Za-z0-9._~:@+-]{0,255}$/;

export class GaussMeridianAdapterError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string,
  ) {
    super(message);
    this.name = "GaussMeridianAdapterError";
  }
}

function toQueryString(params?: Record<string, string | number | boolean | undefined>): string {
  if (!params) return "";
  const entries = Object.entries(params).filter(([, value]) => value !== undefined);
  if (entries.length === 0) return "";
  return `?${new URLSearchParams(entries.map(([key, value]) => [key, String(value)])).toString()}`;
}

/**
 * Raw fetch + error-envelope handling against the GaussMeridian proxy, WITHOUT the final
 * schema validation. Extracted so resource-aware decorators (e.g. `console-org.adapter.ts`, which
 * needs to validate against a REAL backend DTO schema, remap the shape, then re-validate
 * against the UI-facing schema) can reuse the exact same fetch/error/empty-body handling
 * instead of re-implementing it. `createGaussMeridianDataAdapter` below adds the caller's response
 * schema and normalizes a contract mismatch into a stable adapter error.
 *
 * GaussMeridian's backend has no cookie support at all — `extract_auth_context`
 * (handlers.rs) only reads an `x-api-key` header or an `Authorization: Bearer`
 * header, never a cookie. So the httpOnly session cookie set by the login
 * Route Handler (D3) can only ever be read by OUR OWN Next.js server, not by
 * GaussMeridian directly. Every data query therefore goes through the local
 * same-origin proxy at `/api/gaussmeridian/[...path]`, which reads the cookie
 * server-side and forwards the request to GaussMeridian with a Bearer header.
 */
export async function gaussMeridianRawRequest(input: {
  resource: string;
  params?: Record<string, string | number | boolean | undefined>;
  method?: "GET" | "POST" | "PATCH" | "DELETE";
  body?: unknown;
  idempotencyKey?: string;
  projectId?: string;
}): Promise<unknown> {
  const { resource, params, method, body, idempotencyKey, projectId } = input;
  const safeProjectId = projectId && SAFE_PROJECT_CONTEXT.test(projectId) ? projectId : undefined;
  const res = await fetch(`/api/gaussmeridian/${resource}${toQueryString(params)}`, {
    method: method ?? "GET",
    credentials: "include", // same-origin — the httpOnly cookie IS sent to our own server
    ...(body !== undefined || idempotencyKey || safeProjectId
      ? {
          headers: {
            ...(body !== undefined ? { "content-type": "application/json" } : {}),
            ...(idempotencyKey ? { "idempotency-key": idempotencyKey } : {}),
            ...(safeProjectId ? { "x-project-id": safeProjectId } : {}),
          },
        }
      : {}),
    ...(body !== undefined ? { body: JSON.stringify(body) } : {}),
  });

  const contentType = res.headers.get("content-type") ?? "";
  const hasJsonBody = contentType.includes("application/json");

  if (!res.ok) {
    if (hasJsonBody) {
      const errorBody = await res.json();
      const parsedError = GaussMeridianErrorSchema.safeParse(errorBody);
      if (parsedError.success) {
        throw new GaussMeridianAdapterError(
          parsedError.data.error.message,
          res.status,
          parsedError.data.error.code,
        );
      }
    }
    // Some backend paths return an empty body with just a status code.
    throw new GaussMeridianAdapterError(`Request failed with status ${res.status}`, res.status);
  }

  // A handful of backend mutation handlers (204 No Content, or any future bare-status 2xx)
  // return no JSON body at all. Returning `undefined` here lets a matching response schema
  // succeed for a caller using `z.unknown()`/`z.undefined()` on a fire-and-forget mutation;
  // anything stricter legitimately fails Zod validation there rather than the raw, unhandled
  // SyntaxError a bare `res.json()` would throw on empty input.
  if (!hasJsonBody) {
    return undefined;
  }

  try {
    return await res.json();
  } catch {
    throw new GaussMeridianAdapterError(`Response from ${resource} was not valid JSON`, res.status);
  }
}

export function createGaussMeridianDataAdapter(): DataQueryAdapter {
  return {
    async query<T>({
      resource,
      params,
      schema,
      method,
      body,
      idempotencyKey,
    }: DataQueryInput<T>): Promise<T> {
      const json = await gaussMeridianRawRequest({
        resource,
        params,
        method,
        body,
        idempotencyKey,
      });
      const parsed = schema.safeParse(json);
      if (!parsed.success) {
        throw new GaussMeridianAdapterError(
          `Response contract validation failed for ${resource}`,
          502,
          "response_contract_invalid",
        );
      }
      return parsed.data;
    },
  };
}
