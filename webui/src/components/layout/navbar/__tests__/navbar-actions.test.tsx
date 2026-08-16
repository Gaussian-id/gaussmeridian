import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { createFakeRegistry } from "@/test/fakes";
import { renderWithProviders } from "@/test/render";

import { NavbarActions } from "../navbar-actions";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

describe("NavbarActions", () => {
  it("shows Sign in / Get API key when signed out (Bug #2: navbar not auth-aware)", async () => {
    const registry = createFakeRegistry();
    renderWithProviders(<NavbarActions />, registry);

    await waitFor(() => {
      expect(screen.getByRole("link", { name: /sign in/i })).toBeInTheDocument();
    });
    expect(screen.getByRole("link", { name: /get api key/i })).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /console/i })).not.toBeInTheDocument();
    // The account menu only exists once a session is confirmed.
    expect(screen.queryByRole("button", { name: /account menu/i })).not.toBeInTheDocument();
  });

  it("shows the account menu avatar and hides Sign in when useSession resolves to a logged-in user", async () => {
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
    renderWithProviders(<NavbarActions />, registry);

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /account menu for ada lovelace/i }),
      ).toBeInTheDocument();
    });
    expect(screen.queryByRole("link", { name: /^sign in$/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /get api key/i })).not.toBeInTheDocument();
  });

  it("opening the account menu surfaces Console and Account preferences (the old Console link's replacement)", async () => {
    const user = userEvent.setup();
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
    renderWithProviders(<NavbarActions />, registry);

    await user.click(await screen.findByRole("button", { name: /account menu/i }));

    expect(screen.getByRole("menuitem", { name: /^console$/i })).toHaveAttribute("href", "/orgs");
    expect(screen.getByRole("menuitem", { name: /account preferences/i })).toHaveAttribute(
      "href",
      "/account/me",
    );
  });
});
