import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ThemeProvider } from "next-themes";
import { describe, expect, it, vi } from "vitest";

import { AdapterProvider, type AdapterRegistry } from "@core/adapters";

import { createFakeRegistry } from "@/test/fakes";

import { AccountMenu } from "../account-menu";

import type { ReactElement, ReactNode } from "react";

const push = vi.fn();

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
  useRouter: () => ({ push, prefetch: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

/** `AccountMenu` needs both the adapter seam (session, sign-out) and `next-themes`' context
 *  (the inline theme switcher) — `renderWithProviders` only covers the former, and
 *  `theme-toggle.test.tsx` shows the latter is normally wrapped standalone, so this composes
 *  both locally rather than widening the shared helper for one consumer. */
function renderMenu(registry: AdapterRegistry) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <ThemeProvider attribute="class" defaultTheme="light" enableSystem={false}>
        <QueryClientProvider client={queryClient}>
          <AdapterProvider registry={registry}>{children}</AdapterProvider>
        </QueryClientProvider>
      </ThemeProvider>
    );
  }

  return render((<AccountMenu />) as ReactElement, { wrapper: Wrapper });
}

function sessionRegistry(overrides: { email?: string; superadmin?: boolean } = {}) {
  const base = createFakeRegistry();
  return createFakeRegistry({
    auth: {
      ...base.auth,
      getSession: async () => ({
        userId: "user_1",
        displayName: "ada",
        token: "tok_1",
        expiresAt: "2099-01-01T00:00:00Z",
        onboardingCompleted: true,
        email: overrides.email ?? "ada@meridianlabs.dev",
      }),
    },
    data: {
      async query<T>() {
        if (overrides.superadmin) return { superadmin: true } as T;
        throw new Error("mock: v1/admin/me not allowlisted");
      },
    },
  });
}

describe("AccountMenu", () => {
  it("renders nothing while signed out", async () => {
    const registry = createFakeRegistry(); // default getSession resolves null
    renderMenu(registry);

    await waitFor(() => {
      // Give useSession a tick to resolve before asserting absence.
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });
  });

  it("shows the uppercased first letter of the display name as the avatar trigger", async () => {
    renderMenu(sessionRegistry());

    expect(await screen.findByRole("button", { name: /account menu for ada/i })).toHaveTextContent(
      "A",
    );
  });

  it("shows the username and email in the header block when opened", async () => {
    const user = userEvent.setup();
    renderMenu(sessionRegistry({ email: "ada@meridianlabs.dev" }));

    await user.click(await screen.findByRole("button", { name: /account menu/i }));

    expect(await screen.findByText("ada")).toBeInTheDocument();
    expect(screen.getByText("ada@meridianlabs.dev")).toBeInTheDocument();
  });

  it("has an Account preferences item linking to /account/me and a Console item linking to /orgs", async () => {
    const user = userEvent.setup();
    renderMenu(sessionRegistry());

    await user.click(await screen.findByRole("button", { name: /account menu/i }));

    // Radix sets role="menuitem" on the rendered element even with `asChild` around a <Link> —
    // the accessible role is "menuitem", not "link" (the explicit role wins), so these are
    // queried as menu items and asserted on the underlying <a>'s href.
    expect(await screen.findByRole("menuitem", { name: /account preferences/i })).toHaveAttribute(
      "href",
      "/account/me",
    );
    expect(screen.getByRole("menuitem", { name: /^console$/i })).toHaveAttribute("href", "/orgs");
  });

  it("has a Read changelogs item linking to /changelog", async () => {
    const user = userEvent.setup();
    renderMenu(sessionRegistry());

    await user.click(await screen.findByRole("button", { name: /account menu/i }));

    expect(await screen.findByRole("menuitem", { name: /read changelogs/i })).toHaveAttribute(
      "href",
      "/changelog",
    );
  });

  it("offers Light/Dark/System theme options and flips the document theme on selection", async () => {
    const user = userEvent.setup();
    renderMenu(sessionRegistry());

    await user.click(await screen.findByRole("button", { name: /account menu/i }));

    expect(document.documentElement.classList.contains("dark")).toBe(false);

    await user.click(await screen.findByRole("menuitemradio", { name: /dark/i }));

    await waitFor(() => expect(document.documentElement.classList.contains("dark")).toBe(true));
  });

  it("signs out when Sign out is activated", async () => {
    const user = userEvent.setup();
    const signOut = vi.fn().mockResolvedValue(undefined);
    const registry = createFakeRegistry({
      auth: {
        ...sessionRegistry().auth,
        signOut,
      },
    });
    renderMenu(registry);

    await user.click(await screen.findByRole("button", { name: /account menu/i }));
    await user.click(await screen.findByRole("menuitem", { name: /sign out/i }));

    await waitFor(() => expect(signOut).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(push).toHaveBeenCalledWith("/login"));
  });

  it("hides the Admin item for a non-superadmin caller (PRD-23 Wave C)", async () => {
    const user = userEvent.setup();
    renderMenu(sessionRegistry());

    await user.click(await screen.findByRole("button", { name: /account menu/i }));

    expect(await screen.findByText("ada@meridianlabs.dev")).toBeInTheDocument();
    expect(screen.queryByRole("menuitem", { name: /^admin$/i })).not.toBeInTheDocument();
  });

  it("shows an Admin item linking to /admin for an allowlisted superadmin", async () => {
    const user = userEvent.setup();
    renderMenu(sessionRegistry({ superadmin: true }));

    await user.click(await screen.findByRole("button", { name: /account menu/i }));

    expect(await screen.findByRole("menuitem", { name: /^admin$/i })).toHaveAttribute(
      "href",
      "/admin",
    );
  });
});
