const SLUG_RE = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;

/**
 * Derives a lowercase alnum+hyphen slug from a display name, matching the real backend's
 * `CreateOrgRequest.slug` / project-name constraint (lowercase alnum+hyphen, <= 63 chars, no
 * leading/trailing hyphen). Runs of non-alnum characters collapse to a single hyphen.
 */
export function slugify(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 63)
    .replace(/-+$/g, "");
}

/**
 * True when `value` satisfies the backend's slug constraint: lowercase alnum+hyphen, 1-63
 * characters, no leading or trailing hyphen.
 */
export function isValidSlug(value: string): boolean {
  return SLUG_RE.test(value);
}
