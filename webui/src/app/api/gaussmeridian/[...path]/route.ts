import { cookies } from "next/headers";

import { apiBaseUrl } from "@core/lib/env";

import { REFRESH_COOKIE, SESSION_COOKIE, clearedSessionCookies } from "@/app/api/_lib/auth-cookies";
import { reportBackendFailure } from "@/app/api/_lib/backend-failure-reporter";
import { ensureRefreshed } from "@/app/api/_lib/refresh-session";

import { isAllowedBffOperation } from "./route-policy";

// GaussMeridian has no cookie support at all — it only reads an Authorization:
// Bearer header or an x-api-key header. This route is the one place that
// bridges the two: it reads our own httpOnly session cookie server-side
// (never exposed to client JS) and forwards the request to GaussMeridian with
// a Bearer header. GaussMeridianDataAdapter and GaussMeridianAuthAdapter.getSession
// both call through this proxy rather than GaussMeridian directly.
const BACKEND_UNREACHABLE = {
  error: {
    message: "Can't reach the router backend.",
    type: "network_error",
    code: "backend_unreachable",
  },
} as const;

const BFF_ROUTE_NOT_FOUND = {
  error: {
    message: "The requested operation is not available.",
    type: "not_found",
    code: "bff_route_not_found",
  },
} as const;

const SAFE_PROJECT_CONTEXT = /^[A-Za-z0-9][A-Za-z0-9._~:@+-]{0,255}$/;

async function rejectUnlistedOperation(method: string, path: string[]): Promise<Response | null> {
  if (isAllowedBffOperation(method, path)) return null;

  const response = Response.json(BFF_ROUTE_NOT_FOUND, { status: 404 });
  await reportBackendFailure(response, {
    method,
    path: path.join("/"),
    phase: "policy",
  });
  return response;
}

async function forward(request: Request, path: string[], init?: RequestInit): Promise<Response> {
  const cookieStore = await cookies();
  const accessToken = cookieStore.get(SESSION_COOKIE)?.value;
  const refreshToken = cookieStore.get(REFRESH_COOKIE)?.value;

  const url = new URL(request.url);
  const upstreamSearch = new URLSearchParams(url.searchParams);
  const eventSourceProjectId = upstreamSearch.get("project_id");
  upstreamSearch.delete("project_id");
  const query = upstreamSearch.size > 0 ? `?${upstreamSearch.toString()}` : "";
  const upstreamPath = path.join("/");
  const method = init?.method ?? request.method;
  const target = `${apiBaseUrl()}/${upstreamPath}${query}`;
  const isRefreshRoute = upstreamPath === "v1/auth/refresh";
  const requestedProjectId = request.headers.get("x-project-id") ?? eventSourceProjectId;
  const projectId =
    requestedProjectId && SAFE_PROJECT_CONTEXT.test(requestedProjectId)
      ? requestedProjectId
      : undefined;

  const attempt = (bearer?: string): Promise<Response> =>
    fetch(target, {
      ...init,
      headers: {
        ...(init?.headers ?? {}),
        ...(bearer ? { authorization: `Bearer ${bearer}` } : {}),
        ...(projectId ? { "x-project-id": projectId } : {}),
      },
    });

  let res: Response;
  try {
    res = await attempt(accessToken);
  } catch {
    const unreachable = Response.json(BACKEND_UNREACHABLE, { status: 502 });
    await reportBackendFailure(unreachable, {
      method,
      path: upstreamPath,
      phase: "network",
    });
    return unreachable;
  }

  // Silent refresh (PRD-25 Phase 1): an expired access JWT comes back as a small JSON 401
  // *before* any stream body, so branching here never touches a 200 SSE stream. Refresh once
  // (single-flight, shared across concurrent calls) and replay the original request — the
  // POST/PATCH body is the already-read string in `init`, so it is replayable; GET/DELETE
  // have none.
  let refreshedCookies: string[] = [];
  if (res.status === 401 && !isRefreshRoute && refreshToken) {
    try {
      await res.body?.cancel(); // best-effort discard of the 401 error JSON before retrying
    } catch {
      /* a cancel() rejection must not become a 500 */
    }
    const refreshed = await ensureRefreshed(refreshToken);
    if (!refreshed.ok) {
      // Refresh token is dead — end the session cleanly. The next navigation hits the guard
      // with no session cookie and bounces to /login; `getSession` sees this 401 → null now.
      const dead = Response.json(
        {
          error: {
            message: "Your session has expired. Please sign in again.",
            type: "authentication_error",
            code: "session_expired",
          },
        },
        { status: 401 },
      );
      for (const cookie of clearedSessionCookies()) dead.headers.append("set-cookie", cookie);
      return dead;
    }
    refreshedCookies = refreshed.setCookies;
    try {
      res = await attempt(refreshed.accessToken);
    } catch {
      // The refresh already rotated the token server-side (old one revoked). Persist the new
      // pair even on this retry failure — otherwise the client replays the revoked token and
      // trips backend reuse-detection → spurious forced logout of the whole family.
      const unreachable = Response.json(BACKEND_UNREACHABLE, { status: 502 });
      for (const cookie of refreshedCookies) unreachable.headers.append("set-cookie", cookie);
      await reportBackendFailure(unreachable, {
        method,
        path: upstreamPath,
        phase: "retry",
      });
      return unreachable;
    }
  }

  if (!res.ok) {
    await reportBackendFailure(res, {
      method,
      path: upstreamPath,
      phase: refreshedCookies.length > 0 ? "retry" : "upstream",
    });
  }

  // Stream the upstream body through untouched — `res.body` is a `ReadableStream` handed
  // straight to the outgoing `Response`, so SSE (chat completions, the route-decision live
  // feed) streams incrementally rather than buffering into one delayed lump. Any cookies from
  // a successful refresh ride along on this response.
  const headers = new Headers();
  const contentType = res.headers.get("content-type");
  if (contentType) headers.set("content-type", contentType);
  const cacheControl = res.headers.get("cache-control");
  if (cacheControl) headers.set("cache-control", cacheControl);
  const contentDisposition = res.headers.get("content-disposition");
  if (contentDisposition) headers.set("content-disposition", contentDisposition);
  for (const cookie of refreshedCookies) headers.append("set-cookie", cookie);

  return new Response(res.body, { status: res.status, statusText: res.statusText, headers });
}

export async function GET(request: Request, { params }: { params: Promise<{ path: string[] }> }) {
  const { path } = await params;
  const rejection = await rejectUnlistedOperation("GET", path);
  if (rejection) return rejection;
  return forward(request, path);
}

export async function POST(request: Request, { params }: { params: Promise<{ path: string[] }> }) {
  const { path } = await params;
  const rejection = await rejectUnlistedOperation("POST", path);
  if (rejection) return rejection;
  const body = await request.text();
  const idempotencyKey = request.headers.get("idempotency-key");
  return forward(request, path, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(idempotencyKey ? { "idempotency-key": idempotencyKey } : {}),
    },
    body,
  });
}

export async function PATCH(request: Request, { params }: { params: Promise<{ path: string[] }> }) {
  const { path } = await params;
  const rejection = await rejectUnlistedOperation("PATCH", path);
  if (rejection) return rejection;
  const body = await request.text();
  return forward(request, path, {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body,
  });
}

export async function DELETE(
  request: Request,
  { params }: { params: Promise<{ path: string[] }> },
) {
  const { path } = await params;
  const rejection = await rejectUnlistedOperation("DELETE", path);
  if (rejection) return rejection;
  return forward(request, path, { method: "DELETE" });
}
