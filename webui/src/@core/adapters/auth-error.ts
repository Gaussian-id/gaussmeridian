/**
 * A typed auth failure. Unlike a bare `Error`, it preserves the backend's error `code`
 * (from the `{ error: { code, message } }` envelope) and the HTTP `status`, so the UI can
 * map a failure to a specific, human message. `status: 0` means the request never got a
 * response (network failure / server unreachable).
 */
export class AuthError extends Error {
  readonly code?: string;
  readonly status: number;

  constructor(params: { message: string; status: number; code?: string }) {
    super(params.message);
    this.name = "AuthError";
    this.status = params.status;
    this.code = params.code;
  }
}

export function isAuthError(value: unknown): value is AuthError {
  return value instanceof AuthError;
}
