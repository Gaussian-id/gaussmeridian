import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TenancyProvider } from "@core/providers";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { AppSidebar } from "../app-sidebar";

vi.mock("next/navigation", () => ({
  useParams: () => ({}),
  usePathname: () => "/orgs",
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
}));

function setup(options: { superadmin?: boolean } = {}) {
  const registry = createFakeRegistry({
    data: {
      query: byResource({
        "v1/orgs": { orgs: [] },
        "v1/admin/me": () => {
          if (options.superadmin) return { superadmin: true };
          throw new Error("mock: v1/admin/me not allowlisted");
        },
      }),
    },
  });
  return renderWithProviders(
    <TenancyProvider>
      <AppSidebar />
    </TenancyProvider>,
    registry,
  );
}

describe("AppSidebar", () => {
  it("no longer renders its own profile/sign-out block (AccountMenu in AppTopbar is the single account surface)", () => {
    setup();

    expect(screen.queryByRole("button", { name: /sign out/i })).not.toBeInTheDocument();
  });

  it("still renders the nav groups", () => {
    setup();

    expect(screen.getByRole("navigation", { name: /application/i })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /organizations/i })).toBeInTheDocument();
  });

  it("hides the Admin nav item for a non-superadmin caller (PRD-23 Wave C)", async () => {
    setup();

    await waitFor(() => {
      expect(screen.queryByRole("link", { name: /^admin$/i })).not.toBeInTheDocument();
    });
  });

  it("shows the Admin nav item, linking to /admin, for an allowlisted superadmin", async () => {
    setup({ superadmin: true });

    expect(await screen.findByRole("link", { name: /^admin$/i })).toHaveAttribute("href", "/admin");
  });
});
