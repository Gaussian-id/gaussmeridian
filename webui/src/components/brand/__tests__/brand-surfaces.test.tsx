import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { siteConfig } from "@core/config";

import AuthLayout from "@/app/(auth)/layout";
import { AuthPageHeader } from "@/components/auth/auth-page-header";
import { Footer } from "@/components/layout/footer/footer";
import { NavbarLogo } from "@/components/layout/navbar/navbar-logo";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

const BRAND = `${siteConfig.name} logo`;

/**
 * Every surface that identifies the product must announce the brand, and must announce it once.
 * These are the assertions that would have caught the pre-PRD-29 state, where the navbar showed a
 * coloured dot and the sidebar showed a bare text string.
 */
describe("brand surfaces", () => {
  it("marketing navbar shows the brand and links home", () => {
    render(<NavbarLogo />);

    // Mark below `sm`, lockup from `sm` up — one accessible name per branch, one branch visible.
    expect(screen.getAllByRole("img", { name: BRAND })).toHaveLength(2);
    expect(screen.getByRole("link")).toHaveAttribute("href", "/");
  });

  // The console sidebar needs the tenancy/registry harness, so its brand assertions live with the
  // rest of its suite in `layout/app-shell/__tests__/app-sidebar.test.tsx`.

  it("footer shows the brand in fixed light ink, because its ground is always dark", () => {
    render(<Footer />);

    expect(screen.getAllByRole("img", { name: BRAND })).toHaveLength(1);
    const img = document.querySelector("img");
    expect(img).toHaveAttribute("src", "/logo/meridian-lockup-light.svg");
    // A themed pair here would render the dark ink on the footer's permanently dark ground.
    expect(document.querySelectorAll("img")).toHaveLength(1);
  });

  it("auth split panel shows the brand in fixed light ink over the gradient", () => {
    render(
      <AuthLayout>
        <p>form</p>
      </AuthLayout>,
    );

    expect(screen.getAllByRole("img", { name: BRAND })).toHaveLength(1);
    expect(document.querySelector("img")).toHaveAttribute(
      "src",
      "/logo/meridian-lockup-light.svg",
    );
  });

  it("auth split panel keeps its atmospheric globe", () => {
    render(
      <AuthLayout>
        <p>form</p>
      </AuthLayout>,
    );

    // The decorative wireframe globe is not a logo and is explicitly preserved by PRD-29.
    const decorative = document.querySelector('svg[viewBox="0 0 400 400"]');
    expect(decorative).toBeTruthy();
  });

  it("auth page header shows the mark where the split panel is hidden", () => {
    render(<AuthPageHeader title="Sign in" description="Welcome back." />);

    expect(screen.getAllByRole("img", { name: BRAND })).toHaveLength(1);
    expect(document.querySelector("img")?.getAttribute("src")).toContain("meridian-mark");
    expect(screen.getByRole("heading", { name: "Sign in" })).toBeInTheDocument();
  });
});
