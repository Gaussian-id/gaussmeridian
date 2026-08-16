import { isAuthError } from "@core/adapters/auth-error";

/** Which form the failure happened on — governs the fallback copy. */
export type AuthErrorContext = "login" | "signup" | "reset" | "forgot";

/** The field a message belongs on, when the error is field-specific. */
export type AuthErrorField = "email" | "username" | "password";

export interface MappedAuthError {
  message: string;
  field?: AuthErrorField;
}

const GENERIC = "Something went wrong. Please try again.";
const NETWORK = "Can't reach the server — check your connection and try again.";

/**
 * The single place every auth error becomes user-facing copy. Maps a thrown error to a
 * message (and, when field-specific, the field it belongs on).
 *
 * Two backend signals drive it: register returns distinct `code`s (`email_taken`,
 * `username_taken`, `weak_password`, `invalid_email`); login's code is always `login_failed`,
 * so it's distinguished by HTTP `status` (401 = bad credentials, 403 = disabled). `status: 0`
 * is a network failure (no response). Anti-enumeration is preserved: login collapses
 * unknown-email and wrong-password into one message, and never says "no account found".
 *
 * Copy is kept in sync with the router's TUI table (`services/tui/src/error.rs`).
 */
export function mapAuthError(error: unknown, context: AuthErrorContext): MappedAuthError {
  const code = isAuthError(error) ? error.code : undefined;
  const status = isAuthError(error) ? error.status : undefined;

  // Reachability failure: the client never reached a response (status 0), or our proxy reached us but
  // could not reach the router backend (502/503/504, or an explicit backend_unreachable code).
  if (
    status === 0 ||
    status === 502 ||
    status === 503 ||
    status === 504 ||
    code === "backend_unreachable"
  ) {
    return { message: NETWORK };
  }

  // Field-specific codes the backend returns verbatim (mainly on register).
  switch (code) {
    case "email_taken":
      return { field: "email", message: "This email is already registered." };
    case "username_taken":
      return { field: "username", message: "That username is taken." };
    case "weak_password":
      return { field: "password", message: "Password must be at least 8 characters." };
    case "invalid_email":
      return { field: "email", message: "Enter a valid email address." };
  }

  switch (context) {
    case "login":
      // Login's code is uniformly `login_failed` — the status is the real signal.
      if (status === 401) return { message: "Invalid email or password. Please try again." };
      if (status === 403)
        return { message: "Your account is disabled. Contact support to restore access." };
      return { message: GENERIC };
    case "reset":
      // The reset page appends its own clickable "Request a new one" link, so the copy
      // ends here (network failures short-circuit above with the reachability message).
      return { message: "This reset link is invalid or has expired." };
    case "forgot":
      return { message: "Something went wrong sending the reset link. Try again shortly." };
    case "signup":
      return { message: GENERIC };
  }
}
