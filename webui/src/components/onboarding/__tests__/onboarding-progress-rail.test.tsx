import { act, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { WORKSPACE_SETUP_STEPS } from "@/lib/onboarding/onboarding-machine";
import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";

import { OnboardingProgressRail } from "../onboarding-progress-rail";

describe("OnboardingProgressRail skipped state", () => {
  it("labels every deferred workspace step as skipped instead of complete", () => {
    render(
      <OnboardingProgressRail
        currentStep="finish"
        completed={new Set<OnboardingStep>(["welcome", "survey", "profile"])}
        skipped={new Set<OnboardingStep>(WORKSPACE_SETUP_STEPS)}
        orientation="vertical"
      />,
    );

    const list = screen.getByRole("list", { name: "Onboarding progress" });
    for (const label of ["Workspace", "First project", "API key"]) {
      const item = within(list).getByText(label).closest("li");
      expect(item).not.toBeNull();
      expect(within(item!).getByText("skipped", { exact: true })).toBeInTheDocument();
      expect(item!.querySelector(".lucide-check")).toBeNull();
    }
  });

  it("renders skipped as authoritative when defensive input overlaps completed state", () => {
    render(
      <OnboardingProgressRail
        currentStep="finish"
        completed={new Set<OnboardingStep>(ONBOARDING_SETUP_AND_PROFILE)}
        skipped={new Set<OnboardingStep>(WORKSPACE_SETUP_STEPS)}
        orientation="vertical"
      />,
    );

    for (const label of ["Workspace", "First project", "API key"]) {
      const item = screen.getByText(label).closest("li");
      expect(item).not.toBeNull();
      expect(within(item!).getByText("skipped", { exact: true })).toBeInTheDocument();
      expect(within(item!).queryByText("completed")).not.toBeInTheDocument();
      expect(item!.querySelector(".lucide-check")).toBeNull();
    }
  });

  it("exposes completed and upcoming states without relying on color or icons", () => {
    render(
      <OnboardingProgressRail
        currentStep="profile"
        completed={new Set<OnboardingStep>(["welcome", "survey"])}
        orientation="vertical"
      />,
    );

    expect(within(screen.getByText("Welcome").closest("li")!).getByText("completed")).toHaveClass(
      "sr-only",
    );
    expect(within(screen.getByText("Workspace").closest("li")!).getByText("upcoming")).toHaveClass(
      "sr-only",
    );
  });

  it("keeps the current horizontal step visible when the active step changes", () => {
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = Element.prototype.scrollIntoView;
    Element.prototype.scrollIntoView = scrollIntoView;

    try {
      const { rerender } = render(
        <OnboardingProgressRail
          currentStep="welcome"
          completed={new Set<OnboardingStep>()}
          orientation="horizontal"
        />,
      );
      scrollIntoView.mockClear();

      rerender(
        <OnboardingProgressRail
          currentStep="finish"
          completed={new Set<OnboardingStep>(["welcome", "survey", "profile"])}
          skipped={new Set<OnboardingStep>(WORKSPACE_SETUP_STEPS)}
          orientation="horizontal"
        />,
      );

      expect(scrollIntoView).toHaveBeenCalledOnce();
      expect(scrollIntoView).toHaveBeenCalledWith({
        behavior: "auto",
        block: "nearest",
        inline: "center",
      });
    } finally {
      Element.prototype.scrollIntoView = originalScrollIntoView;
    }
  });

  it("recenters the current step when the mobile breakpoint becomes active", () => {
    const scrollIntoView = vi.fn();
    const originalScrollIntoView = Element.prototype.scrollIntoView;
    Element.prototype.scrollIntoView = scrollIntoView;
    let breakpointListener: ((event: MediaQueryListEvent) => void) | undefined;
    const addEventListener = vi.fn(
      (_type: string, listener: (event: MediaQueryListEvent) => void) => {
        breakpointListener = listener;
      },
    );
    const removeEventListener = vi.fn();
    const matchMedia = vi.spyOn(window, "matchMedia").mockReturnValue({
      matches: false,
      media: "(max-width: 759px)",
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener,
      removeEventListener,
      dispatchEvent: vi.fn(),
    } as unknown as MediaQueryList);

    try {
      const { unmount } = render(
        <OnboardingProgressRail
          currentStep="finish"
          completed={new Set<OnboardingStep>(["welcome", "survey", "profile"])}
          skipped={new Set<OnboardingStep>(WORKSPACE_SETUP_STEPS)}
          orientation="horizontal"
        />,
      );
      scrollIntoView.mockClear();

      expect(matchMedia).toHaveBeenCalledWith("(max-width: 759px)");
      expect(addEventListener).toHaveBeenCalledWith("change", expect.any(Function));
      act(() => breakpointListener?.({ matches: true } as MediaQueryListEvent));

      expect(scrollIntoView).toHaveBeenCalledOnce();
      expect(scrollIntoView).toHaveBeenCalledWith({
        behavior: "auto",
        block: "nearest",
        inline: "center",
      });

      unmount();
      expect(removeEventListener).toHaveBeenCalledWith("change", breakpointListener);
    } finally {
      matchMedia.mockRestore();
      Element.prototype.scrollIntoView = originalScrollIntoView;
    }
  });
});

const ONBOARDING_SETUP_AND_PROFILE: OnboardingStep[] = [
  "welcome",
  "survey",
  "profile",
  ...WORKSPACE_SETUP_STEPS,
];
