import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createFakeRegistry } from "@/test/fakes";
import { renderWithProviders } from "@/test/render";

import { OnboardingGate } from "../onboarding-gate";

const replace = vi.fn();
vi.mock("next/navigation", () => ({
  useRouter: () => ({ replace }),
}));

describe("OnboardingGate (US O9 — the completion gate)", () => {
  it("redirects an un-onboarded signed-in user to /onboarding, without rendering children", async () => {
    replace.mockClear();
    const registry = createFakeRegistry({
      auth: {
        ...createFakeRegistry().auth,
        getSession: async () => ({
          userId: "user_1",
          displayName: "Ada",
          token: "",
          expiresAt: "",
          onboardingCompleted: false,
        }),
      },
    });

    renderWithProviders(
      <OnboardingGate>
        <p>protected content</p>
      </OnboardingGate>,
      registry,
    );

    await waitFor(() => expect(replace).toHaveBeenCalledWith("/onboarding"));
    expect(screen.queryByText("protected content")).not.toBeInTheDocument();
  });

  it("renders children for an onboarded signed-in user, without redirecting", async () => {
    replace.mockClear();
    const registry = createFakeRegistry({
      auth: {
        ...createFakeRegistry().auth,
        getSession: async () => ({
          userId: "user_1",
          displayName: "Ada",
          token: "",
          expiresAt: "",
          onboardingCompleted: true,
        }),
      },
    });

    renderWithProviders(
      <OnboardingGate>
        <p>protected content</p>
      </OnboardingGate>,
      registry,
    );

    expect(await screen.findByText("protected content")).toBeInTheDocument();
    expect(replace).not.toHaveBeenCalled();
  });

  it("does not redirect a signed-out user (no session) — proxy.ts already handles that guard", async () => {
    replace.mockClear();
    const registry = createFakeRegistry(); // default getSession -> null

    renderWithProviders(
      <OnboardingGate>
        <p>protected content</p>
      </OnboardingGate>,
      registry,
    );

    expect(await screen.findByText("protected content")).toBeInTheDocument();
    expect(replace).not.toHaveBeenCalled();
  });
});
