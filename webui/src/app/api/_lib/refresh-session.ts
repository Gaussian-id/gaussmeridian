import { apiBaseUrl } from "@core/lib/env";

import { refreshCookie, sessionCookie } from "./auth-cookies";
import { reportBackendFailure } from "./backend-failure-reporter";

const REFRESH_CONTEXT = {
  method: "POST",
  path: "v1/auth/refresh",
} as const;

const BACKEND_UNREACHABLE = {
  error: {
    message: "Can't reach the router backend.",
    type: "network_error",
    code: "backend_unreachable",
  },
} as const;

const INVALID_REFRESH_RESPONSE = {
  error: {
    message: "The router backend returned an invalid refresh response.",
    type: "protocol_error",
    code: "invalid_refresh_response",
  },
} as const;

/**
 * Server-side silent-refresh for the BFF (PRD-25 Phase 1). When a proxied call
 * comes back 401 because the access JWT expired, the proxy calls `ensureRefreshed`
 * with the opaque refresh token from the cookie; on success it retries the original
 * request with the new access token and re-sets both cookies.
 *
 * Single-flight: N concurrent requests from one browser all carry the same refresh
 * token and would otherwise each rotate it — a self-inflicted reuse-detection storm.
 * We coalesce concurrent refreshes of the SAME token into one backend round-trip,
 * keyed by the token value and cleared in `finally`. (Correctness under multiple
 * Next instances is the backend's atomic rotate + grace window, not this map.)
 */
export type RefreshResult = { ok: true; accessToken: string; setCookies: string[] } | { ok: false };

const inFlight = new Map<string, Promise<RefreshResult>>();

async function doRefresh(refreshToken: string): Promise<RefreshResult> {
  let res: Response;
  try {
    res = await fetch(`${apiBaseUrl()}/v1/auth/refresh`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
  } catch {
    const failure = Response.json(BACKEND_UNREACHABLE, { status: 502 });
    await reportBackendFailure(failure, { ...REFRESH_CONTEXT, phase: "network" });
    return { ok: false };
  }

  if (!res.ok) {
    await reportBackendFailure(res, { ...REFRESH_CONTEXT, phase: "refresh" });
    return { ok: false };
  }

  let body: { token?: string; refresh_token?: string };
  try {
    body = (await res.json()) as { token?: string; refresh_token?: string };
  } catch {
    const failure = Response.json(INVALID_REFRESH_RESPONSE, { status: 502 });
    await reportBackendFailure(failure, { ...REFRESH_CONTEXT, phase: "refresh" });
    return { ok: false };
  }
  if (!body.token) {
    const failure = Response.json(INVALID_REFRESH_RESPONSE, { status: 502 });
    await reportBackendFailure(failure, { ...REFRESH_CONTEXT, phase: "refresh" });
    return { ok: false };
  }

  const setCookies = [sessionCookie(body.token)];
  if (body.refresh_token) setCookies.push(refreshCookie(body.refresh_token));
  return { ok: true, accessToken: body.token, setCookies };
}

export function ensureRefreshed(refreshToken: string): Promise<RefreshResult> {
  const existing = inFlight.get(refreshToken);
  if (existing) return existing;

  const pending = doRefresh(refreshToken).finally(() => inFlight.delete(refreshToken));
  inFlight.set(refreshToken, pending);
  return pending;
}
