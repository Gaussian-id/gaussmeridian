import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { siteConfig } from "@core/config";

import { BrandLogo, BrandLogoResponsive } from "../brand-logo";

const BRAND = `${siteConfig.name} logo`;

/** Every rendered brand image, including the ones a theme class is hiding. */
function images() {
  return Array.from(document.querySelectorAll("img"));
}

describe("BrandLogo", () => {
  it("announces the brand exactly once, whichever ink is visible", () => {
    render(<BrandLogo height={32} />);

    expect(screen.getAllByRole("img", { name: BRAND })).toHaveLength(1);
    // Both inks are in the DOM; neither may carry its own name, or dark mode would announce twice.
    expect(images()).toHaveLength(2);
    for (const img of images()) {
      expect(img).toHaveAttribute("aria-hidden", "true");
      expect(img).toHaveAttribute("alt", "");
    }
  });

  it("never prints the product name as a text node beside the logo", () => {
    render(<BrandLogo height={32} />);

    expect(screen.queryByText(/gaussmeridian/i)).not.toBeInTheDocument();
  });

  it("swaps ink by theme class, not by prefers-color-scheme", () => {
    render(<BrandLogo height={32} />);
    const [darkInk, lightInk] = images();

    expect(darkInk).toHaveAttribute("src", "/logo/meridian-lockup-dark.svg");
    expect(darkInk.className).toContain("dark:hidden");
    expect(lightInk).toHaveAttribute("src", "/logo/meridian-lockup-light.svg");
    expect(lightInk.className).toContain("hidden");
    expect(lightInk.className).toContain("dark:block");

    // A <picture>/<source> media switch would ignore the class-driven toggle entirely.
    expect(document.querySelector("picture")).toBeNull();
    expect(document.querySelector("source")).toBeNull();
  });

  it("renders a single fixed ink for permanently dark surfaces", () => {
    render(<BrandLogo tone="light" height={30} />);

    expect(images()).toHaveLength(1);
    expect(images()[0]).toHaveAttribute("src", "/logo/meridian-lockup-light.svg");
    expect(images()[0]?.className).not.toContain("dark:");
    expect(screen.getAllByRole("img", { name: BRAND })).toHaveLength(1);
  });

  it("uses the mark asset for the mark variant", () => {
    render(<BrandLogo variant="mark" tone="light" height={40} />);

    expect(images()[0]).toHaveAttribute("src", "/logo/meridian-mark-light.svg");
  });

  it("sets explicit dimensions on every image so the logo cannot shift layout", () => {
    render(<BrandLogo height={32} />);

    for (const img of images()) {
      // 32 * 4412 / 1264 = 112
      expect(img).toHaveAttribute("width", "112");
      expect(img).toHaveAttribute("height", "32");
    }
  });

  it("derives the mark width from its own aspect ratio, not the lockup's", () => {
    render(<BrandLogo variant="mark" tone="light" height={40} />);

    // 40 * 316 / 316 = 40
    expect(images()[0]).toHaveAttribute("width", "40");
    expect(images()[0]).toHaveAttribute("height", "40");
  });
});

describe("BrandLogoResponsive", () => {
  it("offers the mark below the sm breakpoint and the lockup above it", () => {
    render(<BrandLogoResponsive markHeight={28} lockupHeight={34} />);

    const sources = images().map((img) => img.getAttribute("src"));
    expect(sources).toContain("/logo/meridian-mark-dark.svg");
    expect(sources).toContain("/logo/meridian-lockup-dark.svg");
  });

  it("keeps exactly one branch in the accessibility tree at a time", () => {
    render(<BrandLogoResponsive markHeight={28} lockupHeight={34} />);

    // Both branches render, but each wrapper is display:none at the breakpoint the other serves,
    // so a screen reader only ever reaches one of them.
    const [mobile, desktop] = Array.from(
      document.querySelectorAll("span.inline-flex.sm\\:hidden, span.hidden.sm\\:inline-flex"),
    );
    expect(mobile).toBeTruthy();
    expect(desktop).toBeTruthy();
    expect(screen.getAllByRole("img", { name: BRAND })).toHaveLength(2);
  });
});
