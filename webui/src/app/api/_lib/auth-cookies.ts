/**
 * Server-side helpers for the two httpOnly session cookies (PRD-25 Phase 1).
 *
 * The backend now issues a short-lived access JWT + a long-lived opaque refresh
 * token. Both live only in httpOnly cookies set by the auth Route Handlers and are
 * never exposed to client JS. Both carry a 30-day `Max-Age` (the refresh-token
 * lifetime): the *cookie* is a container whose presence keeps the page guard happy,
 * while the *access JWT inside it* expires quickly and is renewed via
 * `/v1/auth/refresh` (see `refresh-session.ts`). This is the shape that closes
 * BUG-05 without changing `proxy.ts`.
 */
export const SESSION_COOKIE = "gaussmeridian_session";
export const REFRESH_COOKIE = "gaussmeridian_refresh";

const THIRTY_DAYS_SECONDS = 60 * 60 * 24 * 30; // 2592000 — matches the backend refresh TTL default
// Hosted payment providers return through a cross-site top-level GET. `Lax` includes these
// cookies on that safe navigation while still withholding them from cross-site subrequests and
// unsafe methods. State-changing BFF routes additionally fail closed on Origin in `proxy.ts`.
const ATTRS = "HttpOnly; Secure; SameSite=Lax; Path=/";

/** `Set-Cookie` string for the access-JWT cookie. */
export function sessionCookie(token: string): string {
  return `${SESSION_COOKIE}=${token}; ${ATTRS}; Max-Age=${THIRTY_DAYS_SECONDS}`;
}

/** `Set-Cookie` string for the opaque refresh-token cookie. */
export function refreshCookie(token: string): string {
  return `${REFRESH_COOKIE}=${token}; ${ATTRS}; Max-Age=${THIRTY_DAYS_SECONDS}`;
}

/** The two `Set-Cookie` strings that clear both cookies (logout / dead-refresh). */
export function clearedSessionCookies(): string[] {
  return [`${SESSION_COOKIE}=; ${ATTRS}; Max-Age=0`, `${REFRESH_COOKIE}=; ${ATTRS}; Max-Age=0`];
}
