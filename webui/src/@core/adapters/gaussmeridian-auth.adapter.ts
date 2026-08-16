import { z } from "zod";

import { AuthError } from "./auth-error";
import { GaussMeridianErrorSchema } from "./schemas/gaussmeridian-error.schema";

import type { AuthAdapter, AuthSession } from "./types";

const UserSchema = z.object({
  id: z.string(),
  email: z.string(),
  username: z.string(),
  tenant_id: z.string().nullable(),
  roles: z.array(z.string()),
  created_at: z.string(),
  active: z.boolean(),
  // PRD-21 Wave B / DR-010 D1 — surfaced on PublicUser (handlers.rs) so the onboarding gate
  // reads off the same `GET /v1/auth/me` call every other session check already makes.
  // Defaulted `true` for any legacy/mocked response that predates this field, so a stale
  // fixture never wrongly bounces an already-onboarded session back into the wizard.
  onboarding_completed: z.boolean().default(true),
});

export type GaussMeridianUser = z.infer<typeof UserSchema>;

/**
 * `body` present -> POST with a JSON body; `body` omitted -> bare `method` request (used by
 * `cancelAccountDeletion`'s DELETE, which sends no body). Handles an empty-body 2xx response
 * (e.g. `DELETE /v1/auth/me/deletion-request` returns 204 No Content) by resolving `undefined`
 * rather than letting `res.json()` throw a raw `SyntaxError` on empty input — same empty-body
 * handling `gaussMeridianRawRequest` (`gaussmeridian-data.adapter.ts`) already does for the data
 * seam.
 */
async function sendJson(path: string, method: "POST" | "DELETE", body?: unknown): Promise<unknown> {
  let res: Response;
  try {
    res = await fetch(path, {
      method,
      credentials: "include",
      ...(body !== undefined
        ? { headers: { "content-type": "application/json" }, body: JSON.stringify(body) }
        : {}),
    });
  } catch {
    // The request never reached a response — offline, DNS failure, server down.
    // status 0 lets the UI show a "can't reach the server" message, distinct from
    // a credential error.
    throw new AuthError({ message: `Network request to ${path} failed`, status: 0 });
  }

  const contentType = res.headers.get("content-type") ?? "";
  const hasJsonBody = contentType.includes("application/json");

  if (!res.ok) {
    const rawBody = hasJsonBody ? await res.json().catch(() => null) : null;
    // The backend emits `{ error: { message, type, code } }` for errors with a body;
    // some paths return an empty body with just a status. Preserve the code when present,
    // fall back to status-only so the UI can still map by HTTP status.
    const parsed = GaussMeridianErrorSchema.safeParse(rawBody);
    if (parsed.success) {
      throw new AuthError({
        message: parsed.data.error.message,
        code: parsed.data.error.code,
        status: res.status,
      });
    }
    throw new AuthError({
      message: `Request to ${path} failed with ${res.status}`,
      status: res.status,
    });
  }

  return hasJsonBody ? res.json() : undefined;
}

function postJson(path: string, body: unknown): Promise<unknown> {
  return sendJson(path, "POST", body);
}

// AuthSession.token is required by the shared interface (documented there as
// "the only credential the client ever holds") but this app deliberately
// never populates it with anything meaningful — the real session lives in the
// httpOnly gaussmeridian_session cookie, set by the Route Handler proxies below,
// which client JS can never read. Nothing in this app reads .token off an
// AuthSession (confirmed by grep before this task started) — kept empty only
// to satisfy the shared interface.
function toSession(user: GaussMeridianUser): AuthSession {
  return {
    userId: user.id,
    displayName: user.username,
    token: "",
    expiresAt: "",
    onboardingCompleted: user.onboarding_completed,
    email: user.email,
  };
}

export function createGaussMeridianAuthAdapter(): AuthAdapter {
  return {
    async signIn(credentials: { email: string; password: string }) {
      const raw = await postJson("/api/auth/login", credentials);
      return toSession(UserSchema.parse(raw));
    },
    async signUp(credentials: { email: string; username: string; password: string }) {
      const raw = await postJson("/api/auth/register", credentials);
      return toSession(UserSchema.parse(raw));
    },
    async getSession() {
      try {
        const res = await fetch("/api/gaussmeridian/v1/auth/me", { credentials: "include" });
        if (!res.ok) return null;
        return toSession(UserSchema.parse(await res.json()));
      } catch {
        return null;
      }
    },
    async signOut() {
      await fetch("/api/auth/logout", { method: "POST", credentials: "include" });
    },
    // Forgot/reset go through the generic GaussMeridian proxy rather than dedicated
    // Route Handlers: unlike login/register they never mint a session cookie, so
    // there is nothing for a dedicated handler to do beyond forwarding.
    async forgotPassword({ email }) {
      await postJson("/api/gaussmeridian/v1/auth/forgot-password", { email });
    },
    async resetPassword({ token, newPassword }) {
      await postJson("/api/gaussmeridian/v1/auth/reset-password", {
        token,
        new_password: newPassword,
      });
    },
    // The real backend has no deletion-request lifecycle yet (`routes.rs` has no
    // `/v1/auth/me/deletion-request` route as of this writing — a superadmin PRD will own
    // approval/fulfillment). `postJson` throws a typed `AuthError` on any non-2xx; a 404
    // (unregistered route) or 405 (method not allowed on an existing but differently-shaped
    // route) both mean "not enabled on this server", so both are remapped to one honest
    // message here rather than letting the raw "Request failed with 404" leak into the UI.
    // Any other failure (network, 401, 500…) passes through unchanged for the caller to map.
    async requestAccountDeletion() {
      try {
        await postJson("/api/gaussmeridian/v1/auth/me/deletion-request", {});
      } catch (err) {
        if (err instanceof AuthError && (err.status === 404 || err.status === 405)) {
          throw new AuthError({
            message: "Deletion requests aren't enabled on this server yet.",
            status: err.status,
            code: "deletion_request_unavailable",
          });
        }
        throw err;
      }
    },
    // PRD-23 Wave C — `DELETE /v1/auth/me/deletion-request`, landed alongside the superadmin
    // surface (unlike `requestAccountDeletion` above, no "not enabled yet" remap: this route
    // exists on the backend from day one of this feature, there is no legacy pre-PRD-23 gap to
    // paper over). 404 (no pending request to cancel) passes through unchanged for the caller.
    async cancelAccountDeletion() {
      await sendJson("/api/gaussmeridian/v1/auth/me/deletion-request", "DELETE");
    },
  };
}
