/**
 * Client-side credential validators for the login + signup forms (PRD-25 auth V&V, part A).
 *
 * Each returns a human error message, or `null` when valid. These are a UX fast-path only — the
 * backend re-validates every rule (never trust the client), and these mirror the backend contract:
 * lowercase handle `^[a-z0-9_-]{3,30}$`, an email shape, and an 8-char minimum password.
 */
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
export const USERNAME_RE = /^[a-z0-9_-]{3,30}$/;
export const MIN_PASSWORD_LENGTH = 8;

export function validateEmail(email: string): string | null {
  if (!email.trim()) return "Enter your email address.";
  if (!EMAIL_RE.test(email)) return "Enter a valid email address.";
  return null;
}

export function validateUsername(username: string): string | null {
  if (!username) return "Choose a username.";
  if (!USERNAME_RE.test(username)) {
    return "Username must be 3–30 lowercase letters, numbers, dashes, or underscores — no spaces or capitals.";
  }
  return null;
}

/**
 * Sign-up passwords must meet the strength floor (≥ 8 chars). For login we only require a
 * non-empty value — an existing account may predate any given rule, so the server is the only
 * authority on whether the password is correct; the client just avoids an empty round-trip.
 */
export function validatePassword(
  password: string,
  opts: { forLogin?: boolean } = {},
): string | null {
  if (!password) return "Enter your password.";
  if (!opts.forLogin && password.length < MIN_PASSWORD_LENGTH) {
    return `Password must be at least ${MIN_PASSWORD_LENGTH} characters.`;
  }
  return null;
}

/** Force lowercase + strip whitespace as the user types a username, so a space or capital can
 *  never land in the field. Remaining invalid chars are caught by `validateUsername` on submit. */
export function normalizeUsername(value: string): string {
  return value.toLowerCase().replace(/\s+/g, "");
}
