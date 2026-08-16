import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { ThemeProvider } from "next-themes";
import { describe, expect, it, vi } from "vitest";

import { AdapterProvider, type AdapterRegistry } from "@core/adapters";

import { createFakeRegistry } from "@/test/fakes";

import { AppTopbar } from "../app-topbar";

import type { ReactElement, ReactNode } from "react";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

// AppTopbar only needs the palette trigger's `open()` handle, not a mounted palette (that
// dialog's own behavior is covered by command-palette.test.tsx and would otherwise drag in
// TenancyProvider + org/project/model data queries this test has no stake in).
vi.mock("@/components/command", () => ({
  useCommandPaletteTrigger: () => ({ open: vi.fn() }),
}));

function renderTopbar(registry: AdapterRegistry) {
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

  return render((<AppTopbar />) as ReactElement, { wrapper: Wrapper });
}

function sessionRegistry() {
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
        email: "ada@meridianlabs.dev",
      }),
    },
  });
}

describe("AppTopbar", () => {
  it("renders the account menu avatar in place of a standalone theme toggle", async () => {
    renderTopbar(sessionRegistry());

    expect(
      await screen.findByRole("button", { name: /account menu for ada/i }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /toggle color theme/i })).not.toBeInTheDocument();
  });

  it("still renders the command palette search trigger", async () => {
    renderTopbar(sessionRegistry());

    expect(screen.getByRole("button", { name: /open command palette/i })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /open application navigation/i }),
    ).toBeInTheDocument();
  });
});
