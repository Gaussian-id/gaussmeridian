const INTERNAL_ORIGIN = "http://gaussmeridian.internal";
const UNSAFE_PATH_CHARACTERS = /[\\\u0000-\u001f\u007f]/;

/**
 * Accepts a same-site application path and rejects every external or ambiguous destination.
 * This is deliberately smaller than a generic URL sanitizer: post-auth navigation never needs
 * another origin, credentials, or a protocol-relative URL.
 */
export function safeInternalRedirect(value: string | null | undefined, fallback = "/orgs"): string {
  if (
    !value ||
    !value.startsWith("/") ||
    value.startsWith("//") ||
    UNSAFE_PATH_CHARACTERS.test(value)
  ) {
    return fallback;
  }

  try {
    const parsed = new URL(value, INTERNAL_ORIGIN);
    if (parsed.origin !== INTERNAL_ORIGIN) return fallback;
    return `${parsed.pathname}${parsed.search}${parsed.hash}`;
  } catch {
    return fallback;
  }
}
