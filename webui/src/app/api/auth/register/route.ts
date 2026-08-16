import { apiBaseUrl } from "@core/lib/env";

import { refreshCookie, sessionCookie } from "@/app/api/_lib/auth-cookies";
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
  path: "v1/auth/register",
} as const;

export async function POST(request: Request) {
  const body = await request.json();

  let backendRes: Response;
  try {
    backendRes = await fetch(`${apiBaseUrl()}/v1/auth/register`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    });
  } catch {
    const failure = Response.json(BACKEND_UNREACHABLE, { status: 502 });
    await reportBackendFailure(failure, { ...FAILURE_CONTEXT, phase: "network" });
    return failure;
  }

  if (!backendRes.ok) {
    await reportBackendFailure(backendRes, { ...FAILURE_CONTEXT, phase: "upstream" });
  }

  const contentType = backendRes.headers.get("content-type") ?? "";
  const backendBody = contentType.includes("application/json") ? await backendRes.json() : null;

  if (!backendRes.ok) {
    return Response.json(
      backendBody ?? {
        error: {
          message: "Registration failed",
          type: "registration_error",
          code: "registration_failed",
        },
      },
      { status: backendRes.status },
    );
  }

  const { token, refresh_token, user } = backendBody as {
    token: string;
    refresh_token?: string;
    user: unknown;
  };

  // Access + refresh tokens never leave this Route Handler as readable values —
  // set as httpOnly cookies only. The response body carries the user object,
  // never a token. `.append` (not `.set`) so both Set-Cookie headers survive.
  const response = Response.json(user, { status: 200 });
  response.headers.append("set-cookie", sessionCookie(token));
  if (refresh_token) response.headers.append("set-cookie", refreshCookie(refresh_token));
  return response;
}
