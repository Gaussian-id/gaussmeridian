import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createFakeRegistry } from "@/test/fakes";
import { renderWithProviders } from "@/test/render";

import { NavbarMobile } from "../navbar-mobile";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

function openMenu() {
  screen.getByRole("button", { name: /toggle menu/i }).click();
}

describe("NavbarMobile", () => {
  it("shows Sign in / Get API key when signed out (Bug #2: mobile navbar not auth-aware)", async () => {
    const registry = createFakeRegistry();
    renderWithProviders(<NavbarMobile />, registry);
    openMenu();

    await waitFor(() => {
      expect(screen.getByRole("link", { name: /sign in/i })).toBeInTheDocument();
    });
    expect(screen.getByRole("link", { name: /get api key/i })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /console/i })).not.toBeInTheDocument();
  });

  it("hides Sign in / Get API key once useSession resolves to a logged-in user — the account menu (rendered by NavbarActions alongside this hamburger) covers navigation instead", async () => {
    const base = createFakeRegistry();
    const registry = createFakeRegistry({
      auth: {
        ...base.auth,
        getSession: async () => ({
          userId: "user_1",
          displayName: "Ada Lovelace",
          token: "tok_1",
          expiresAt: "2099-01-01T00:00:00Z",
          onboardingCompleted: true,
          email: "ada@meridianlabs.dev",
        }),
      },
    });
    renderWithProviders(<NavbarMobile />, registry);
    openMenu();

    await waitFor(() => {
      expect(screen.queryByRole("link", { name: /^sign in$/i })).not.toBeInTheDocument();
      expect(screen.queryByRole("link", { name: /get api key/i })).not.toBeInTheDocument();
    });
    // The panel no longer carries its own "Console" link — that navigation now lives in the
    // account menu avatar, which sits in the same header row at every breakpoint.
    expect(screen.queryByRole("link", { name: /console/i })).not.toBeInTheDocument();
  });
});
