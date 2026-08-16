import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { createFakeRegistry } from "@/test/fakes";
import { renderWithProviders } from "@/test/render";

import { AppMobileNavigation } from "../app-mobile-navigation";

vi.mock("next/navigation", () => ({
  useParams: () => ({}),
  usePathname: () => "/orgs",
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
}));

vi.mock("@core/providers", () => ({
  useTenancy: () => ({ mode: "global" }),
}));

vi.mock("@/hooks/useAdminQueries", () => ({
  useIsSuperadmin: () => false,
}));

vi.mock("@/hooks/useConsoleQueries", () => ({
  useOrgs: () => ({ data: { orgs: [] } }),
  useOrgProjects: () => ({ data: { projects: [] } }),
}));

describe("AppMobileNavigation", () => {
  it("opens authenticated product navigation and closes it after choosing a destination", async () => {
    const user = userEvent.setup();
    renderWithProviders(<AppMobileNavigation />, createFakeRegistry());

    const trigger = screen.getByRole("button", { name: /open application navigation/i });
    expect(trigger).toHaveClass("md:hidden");
    expect(
      screen.queryByRole("navigation", { name: /mobile application/i }),
    ).not.toBeInTheDocument();

    await user.click(trigger);

    expect(screen.getByRole("dialog", { name: /product navigation/i })).toBeInTheDocument();
    const navigation = screen.getByRole("navigation", { name: /mobile application/i });
    expect(navigation).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /organizations/i })).toBeInTheDocument();

    await user.click(screen.getByRole("link", { name: /organizations/i }));
    expect(screen.queryByRole("dialog", { name: /product navigation/i })).not.toBeInTheDocument();
  });
});
