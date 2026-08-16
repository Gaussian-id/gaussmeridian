/**
 * TypeScript mirror of the Gaussian theme.
 *
 * The CSS in `tokens.css` is the source of truth for what renders. This file exposes the
 * brand values to JavaScript contexts that cannot read CSS variables ergonomically —
 * primarily Three.js materials and GSAP — and centralizes theme behavior flags.
 */
export const themeConfig = {
  /** Default color mode when the user hasn't chosen one. Light is canonical. */
  defaultMode: "light",
  /** Follow the OS `prefers-color-scheme` when the user picks "System" in the theme switcher.
   *  (The switcher offers a System option, so this must be enabled for it to resolve at all.) */
  enableSystem: true,
  /** Storage key for the persisted user choice. */
  storageKey: "gauss-theme",

  /** Brand palette for canvas/3D/motion contexts (hex mirrors tokens.css). */
  palette: {
    gauss900: "#0a2a6b",
    gauss800: "#0b3c8c",
    gauss700: "#1456c7",
    gauss500: "#2e7bff",
    gauss400: "#4da3ff",
    ink: "#0b1220",
  },

  radius: "0.75rem",
} as const;

export type ThemeMode = "light" | "dark";
export type ThemeConfig = typeof themeConfig;
