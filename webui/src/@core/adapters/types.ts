import type { ZodType } from "zod";

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

/**
 * BYOK provider-key registration/listing/revocation is NOT part of this interface — it's a
 * plain project-scoped resource served by the same `DataQueryAdapter` every other resource
 * uses (`useByokProviders`/`useRegisterByokKey`/`useDeleteByokKey` in
 * `hooks/useGaussmeridianQueries.ts`, hitting the real `GET/POST /v1/byok/keys` and
 * `DELETE /v1/byok/keys/:provider`). There is no client-held "BYOK session token" concept on
 * the real backend — auth for chat, like every other request, is resolved server-side from the
 * caller's session. This interface exists only for the one capability that genuinely needs a
 * long-lived streaming connection outside the request/response `DataQueryAdapter` shape.
 */
export interface LlmByokAdapter {
  /** Stream a chat completion for one explicit, membership-checked project through the
   *  authenticated WebUI transport. Yields text chunks as they arrive and never exposes the
   *  platform-managed supplier credential to the browser. */
  streamChat(input: {
    projectId: string;
    model: string;
    messages: ChatMessage[];
  }): AsyncIterable<string>;
}

export interface DataQueryInput<T> {
  resource: string;
  params?: Record<string, string | number | boolean | undefined>;
  schema: ZodType<T>;
  /** Defaults to GET. Set to POST/PATCH/DELETE for mutations (e.g. creating an API key,
   *  saving project settings, removing a BYOK provider key). */
  method?: "GET" | "POST" | "PATCH" | "DELETE";
  /** JSON-serializable request body. Only meaningful when `method` is not GET. */
  body?: unknown;
  /** Commerce mutations only: forwarded as `Idempotency-Key` by the same-origin BFF. This is
   *  deliberately narrower than accepting arbitrary caller-controlled headers. */
  idempotencyKey?: string;
}

export interface DataQueryAdapter {
  /** Read a resource from a separate Gaussian backend service, validated at the boundary. */
  query<T>(input: DataQueryInput<T>): Promise<T>;
}

export interface AuthSession {
  userId: string;
  displayName: string;
  /** Short-lived session token. The only credential the client ever holds. */
  token: string;
  expiresAt: string;
  /** PRD-21 Wave B / DR-010 D1 — the onboarding gate (US O9). Mirrors the backend's
   *  `PublicUser.onboarding_completed` (`GET /v1/auth/me`). `false` routes a signed-in user
   *  into `/onboarding`; the `(app)` layout gate is the one place this is enforced client-side. */
  onboardingCompleted: boolean;
  /** The caller's account email (`PublicUser.email`, `GET /v1/auth/me`). Optional — adapters
   *  that predate this field (fixtures, the generic HTTP reference adapter) simply omit it.
   *  Surfaced for the navbar account menu's header block (username + email); the account
   *  page's editable profile fields (full name/display name/company/timezone) are a separate,
   *  purpose-built query (`useAccountProfile`, `v1/auth/me` via the data adapter) rather than
   *  living on this shared, broadly-consumed session shape. */
  email?: string;
}

export interface AuthAdapter {
  signIn(input: { email: string; password: string }): Promise<AuthSession>;
  signUp(input: { email: string; username: string; password: string }): Promise<AuthSession>;
  getSession(): Promise<AuthSession | null>;
  signOut(): Promise<void>;
  /** Request a password-reset email. Always resolves (anti-enumeration: the backend
   *  responds identically whether or not the email exists). */
  forgotPassword(input: { email: string }): Promise<void>;
  /** Complete a reset with the emailed token. Rejects on invalid/expired token. */
  resetPassword(input: { token: string; newPassword: string }): Promise<void>;
  /** Request account deletion (`/account/me` danger zone). The real backend has no lifecycle
   *  for this yet — a superadmin PRD will own approval/fulfillment — so the real adapter's
   *  implementation surfaces a 404/405 as a mapped, honest "not enabled yet" `AuthError`
   *  rather than pretending to succeed. */
  requestAccountDeletion(): Promise<void>;
  /** Cancel the caller's own pending deletion request (PRD-23 Wave C —
   *  `DELETE /v1/auth/me/deletion-request`, landed alongside the superadmin surface). Rejects
   *  if there was no pending request to cancel (the backend 404s that case). */
  cancelAccountDeletion(): Promise<void>;
}

/** The full set of backend capabilities, injected at one seam. */
export interface AdapterRegistry {
  llm: LlmByokAdapter;
  data: DataQueryAdapter;
  auth: AuthAdapter;
}
