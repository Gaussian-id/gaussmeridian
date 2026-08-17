export type NavbarPosition = "sticky" | "fixed" | "static";

/**
 * Corresponding Source offered under AGPL-3.0 Section 13.
 *
 * Read at module scope so Next.js inlines it at build time. An operator running a
 * MODIFIED build sets `NEXT_PUBLIC_SOURCE_OFFER_URL` to a URL serving their own source;
 * leaving it unset points at upstream, which is correct for an unmodified build and
 * wrong — in the operator's own legal direction — for a modified one.
 */
const SOURCE_OFFER_URL =
  process.env.NEXT_PUBLIC_SOURCE_OFFER_URL?.trim() ||
  "https://github.com/Gaussian-id/gauss-meridian";

/**
 * Boilerplate-level configuration. Per-fork branding and layout behavior live here so a
 * new app can be re-pointed without touching components.
 */
export const siteConfig = {
  name: "GaussMeridian",
  shortName: "Meridian",
  description:
    "Meridian draws the smartest path for every prompt — routing across every model, and charging you only when the answer holds. Bring your keys, self-host, or start free.",
  url: "https://gaussmeridian.io",
  locale: "en",
  navbar: {
    position: "sticky" as NavbarPosition,
    showThemeToggle: true,
  },
  contact: {
    email: "contact@gaussmeridian.io",
  },
  /**
   * Rendered by the site footer on every page. This is a license obligation, not a
   * marketing link — see the repository-root NOTICE before changing or removing it.
   */
  sourceOffer: {
    license: "AGPL-3.0-only",
    url: SOURCE_OFFER_URL,
  },
} as const;

export type SiteConfig = typeof siteConfig;
