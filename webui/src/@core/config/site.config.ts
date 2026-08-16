export type NavbarPosition = "sticky" | "fixed" | "static";

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
} as const;

export type SiteConfig = typeof siteConfig;
