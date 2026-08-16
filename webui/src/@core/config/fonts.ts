import { Bricolage_Grotesque, Hanken_Grotesk, JetBrains_Mono } from "next/font/google";

/**
 * Font configuration. The CSS variables here feed the `@theme` token layer
 * (`--font-display`, `--font-sans`, `--font-mono`). Swap families to re-brand a fork.
 *
 * - Display: Bricolage Grotesque — characterful, editorial headings.
 * - Body:    Hanken Grotesk — refined, highly legible.
 * - Mono:    JetBrains Mono — the "audit readout" voice (verified/proven/in-perimeter).
 */
export const fontDisplay = Bricolage_Grotesque({
  subsets: ["latin"],
  variable: "--font-display",
  display: "swap",
});

export const fontSans = Hanken_Grotesk({
  subsets: ["latin"],
  variable: "--font-sans",
  display: "swap",
});

export const fontMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-mono",
  display: "swap",
});

/** All font CSS-variable class names for the <html> element. */
export const fontVariables = `${fontDisplay.variable} ${fontSans.variable} ${fontMono.variable}`;
