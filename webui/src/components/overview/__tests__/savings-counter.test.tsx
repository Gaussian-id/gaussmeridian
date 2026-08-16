import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SavingsCounter } from "../savings-counter";

/** Forces `prefers-reduced-motion: reduce` so the counter renders its final value on the very
 *  next animation frame instead of animating — see `savings-counter.tsx`'s duration-0 path. */
function mockReducedMotion(matches: boolean) {
  window.matchMedia = ((query: string) => ({
    matches,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })) as unknown as typeof window.matchMedia;
}

describe("SavingsCounter", () => {
  afterEach(() => {
    mockReducedMotion(false);
  });

  it("renders the not_charged_total figure under reduced motion", async () => {
    mockReducedMotion(true);
    render(<SavingsCounter total={4.82} count={37} />);

    await waitFor(() => expect(screen.getByText("$4.82")).toBeInTheDocument());
    expect(screen.getByText(/37 calls failed OutcomeGate/)).toBeInTheDocument();
  });

  it("shows a loading placeholder instead of a stale figure while loading", () => {
    mockReducedMotion(true);
    render(<SavingsCounter total={4.82} count={37} isLoading />);

    expect(screen.getByText("—")).toBeInTheDocument();
    expect(screen.queryByText("$4.82")).not.toBeInTheDocument();
  });

  it("singularizes the call count copy for exactly one failed call", async () => {
    mockReducedMotion(true);
    render(<SavingsCounter total={0.01} count={1} />);

    await waitFor(() => expect(screen.getByText(/1 call failed OutcomeGate/)).toBeInTheDocument());
  });
});
