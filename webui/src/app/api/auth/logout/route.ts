import { cookies } from "next/headers";

import { apiBaseUrl } from "@core/lib/env";

import { REFRESH_COOKIE, SESSION_COOKIE, clearedSessionCookies } from "@/app/api/_lib/auth-cookies";
import { reportBackendFailure } from "@/app/api/_lib/backend-failure-reporter";

const BACKEND_UNREACHABLE = {
  error: {
    message: "Can't reach the router backend.",
    type: "network_error",
    code: "backend_unreachable",
  },
} as const;

const FAILURE_CONTEXT = {
  method: "POST",
  path: "v1/auth/logout",
} as const;

// Clears both local httpOnly cookies (the primary boundary) and, best-effort,
// revokes server-side via POST /v1/auth/logout — the backend revokes the JWT
// (Redis list) and the refresh-token FAMILY (DB, so it works even when Redis is
// down). Revocation failure never blocks logout: clearing the cookies below is
// what ends the browser session. Known P1 limitation: if the access JWT is
// already expired the backend logout may 401 before the family revoke runs, so
// the family then lapses only at its natural TTL — the browser session still
// ends immediately via the cookie clears.
export async function POST() {
  const store = await cookies();
  const token = store.get(SESSION_COOKIE)?.value;
  const refreshToken = store.get(REFRESH_COOKIE)?.value;

  if (token) {
    try {
      const backendResponse = await fetch(`${apiBaseUrl()}/v1/auth/logout`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
        body: JSON.stringify(refreshToken ? { refresh_token: refreshToken } : {}),
      });
      if (!backendResponse.ok) {
        await reportBackendFailure(backendResponse, {
          ...FAILURE_CONTEXT,
          phase: "upstream",
        });
      }
    } catch {
      const failure = Response.json(BACKEND_UNREACHABLE, { status: 502 });
      await reportBackendFailure(failure, { ...FAILURE_CONTEXT, phase: "network" });
    }
  }

  const response = Response.json({ ok: true }, { status: 200 });
  for (const cookie of clearedSessionCookies()) {
    response.headers.append("set-cookie", cookie);
  }
  return response;
}
