import { screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { OutcomeSavings, RouteDecision } from "@core/adapters/schemas/console.schema";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { OverviewHero } from "../overview-hero";

/** Forces `prefers-reduced-motion: reduce` so `Reveal`/`useParallax`/`useTilt` no-op and the
 *  savings counter settles on its next frame instead of animating — see
 *  `savings-counter.test.tsx` for the same pattern applied at the unit level. */
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

const savingsFixture: OutcomeSavings = {
  total_requests: 549,
  total_cost_charged: 128.44,
  total_baseline_cost: 133.26,
  total_saved: 4.82,
  zero_charge_count: 37,
  zero_charge_saved: 4.82,
  avg_r_binary: 0.93,
  cascade_adoption_pct: 0.24,
  moa_adoption_pct: 0.18,
};

function decisionWithComplexity(id: string, complexity: number): RouteDecision {
  return {
    id,
    request_id: id,
    candidates: [{ model: "gpt-4o-mini", provider: "openai", score: 0.9, selected: true }],
    moa: { enabled: false, winner: null, losers: [] },
    guardrail_status: "passed",
    cascade_used: false,
    complexity,
    baseline_cost: 0.01,
    created_at: "2026-07-14T00:00:00Z",
  };
}

describe("OverviewHero", () => {
  afterEach(() => {
    mockReducedMotion(false);
  });

  it("renders the OutcomeGate savings counter, complexity mix, and adoption tiles", async () => {
    mockReducedMotion(true);
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/projects/proj_1/savings": savingsFixture,
          "v1/projects/proj_1/routes": [
            decisionWithComplexity("d1", 0.1),
            decisionWithComplexity("d2", 0.1),
          ],
        }),
      },
    });

    renderWithProviders(<OverviewHero projectId="proj_1" streamStatus="live" />, registry);

    await waitFor(() => expect(screen.getByText("$4.82")).toBeInTheDocument());
    expect(screen.getByText(/37 calls failed OutcomeGate/)).toBeInTheDocument();

    expect(screen.getByText("18%")).toBeInTheDocument(); // GaussMoA adoption
    expect(screen.getByText("24%")).toBeInTheDocument(); // Cascade adoption
    expect(screen.getByText("93%")).toBeInTheDocument(); // Reliability (avg_r_binary)

    await waitFor(() => expect(screen.getByText("2")).toBeInTheDocument()); // low-complexity count
    expect(screen.getByText(/last 2 routed/)).toBeInTheDocument();
  });

  it("shows the shared stream status in the hero badge, not a hardcoded 'live' (Reviewer F1)", async () => {
    mockReducedMotion(true);
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/projects/proj_1/savings": savingsFixture,
          "v1/projects/proj_1/routes": [decisionWithComplexity("d1", 0.1)],
        }),
      },
    });

    renderWithProviders(<OverviewHero projectId="proj_1" streamStatus="disconnected" />, registry);

    // The badge reflects the real connection state passed from the page — when the feed is
    // disconnected the hero must NOT still claim "Live".
    await waitFor(() => expect(screen.getByText("Disconnected")).toBeInTheDocument());
    expect(screen.queryByText("Live")).not.toBeInTheDocument();
  });
});
