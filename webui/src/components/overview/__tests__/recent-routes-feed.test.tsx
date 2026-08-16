import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { GaussMeridianAdapterError } from "@core/adapters/gaussmeridian-data.adapter";
import type { RouteDecision } from "@core/adapters/schemas/console.schema";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { RecentRoutesFeed } from "../recent-routes-feed";

const passedDecision: RouteDecision = {
  id: "route_passed",
  request_id: "req_passed",
  project_id: "proj_1",
  org_id: "org_1",
  candidates: [{ model: "gpt-4o-mini", provider: "openai", score: 0.92, selected: true }],
  moa: { enabled: false, winner: null, losers: [] },
  guardrail_status: "passed",
  cascade_used: false,
  complexity: 0.25,
  baseline_cost: 0.0123,
  created_at: "2026-07-14T09:12:00Z",
};

const disabledDecision: RouteDecision = {
  ...passedDecision,
  id: "route_disabled",
  request_id: "req_disabled",
  guardrail_status: "disabled",
};

describe("RecentRoutesFeed", () => {
  it("shows the delivered model, complexity band, and guardrail status for each decision", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/projects/proj_1/routes": [passedDecision, disabledDecision],
        }),
      },
    });

    renderWithProviders(
      <RecentRoutesFeed
        projectId="proj_1"
        onSelectDecision={vi.fn()}
        streamStatus="live"
        streamEvents={[]}
      />,
      registry,
    );

    await waitFor(() => expect(screen.getAllByText(/gpt-4o-mini/).length).toBeGreaterThan(0));
    expect(screen.getByText("Passed")).toBeInTheDocument();
    expect(screen.getByText("Disabled")).toBeInTheDocument();
    expect(screen.getAllByText("Low").length).toBe(2); // both complexity 0.25 -> low band
    // The connection indicator reflects the passed-in shared status, not a hardcoded "live".
    expect(screen.getByText("Live")).toBeInTheDocument();
  });

  it("shows the shared 'Disconnected' status when the page's SSE connection has dropped", async () => {
    const registry = createFakeRegistry({
      data: { query: byResource({ "v1/projects/proj_1/routes": [passedDecision] }) },
    });

    renderWithProviders(
      <RecentRoutesFeed
        projectId="proj_1"
        onSelectDecision={vi.fn()}
        streamStatus="disconnected"
        streamEvents={[]}
      />,
      registry,
    );

    await waitFor(() => expect(screen.getByText("Disconnected")).toBeInTheDocument());
    expect(screen.queryByText("Live")).not.toBeInTheDocument();
  });

  it("merges a live SSE event in at the top, deduped against the initial fetch", async () => {
    const registry = createFakeRegistry({
      data: { query: byResource({ "v1/projects/proj_1/routes": [passedDecision] }) },
    });
    const liveEvent = {
      request_id: "req_live",
      candidates: [
        { model: "claude-3-5-sonnet", provider: "anthropic", score: 0.8, selected: true },
      ],
      moa: { enabled: false as const, winner: null, losers: [] },
      guardrail_status: "passed",
      cascade_used: false,
      complexity: 0.9,
      baseline_cost: 0.05,
      created_at: "2026-07-15T00:00:00Z", // newer than the fetched row
    };

    renderWithProviders(
      <RecentRoutesFeed
        projectId="proj_1"
        onSelectDecision={vi.fn()}
        streamStatus="live"
        streamEvents={[liveEvent]}
      />,
      registry,
    );

    // Wait on the fetched row (async), then assert the live event merged alongside it.
    await waitFor(() => expect(screen.getAllByText(/gpt-4o-mini/).length).toBeGreaterThan(0));
    expect(screen.getAllByText(/claude-3-5-sonnet/).length).toBeGreaterThan(0);
  });

  it("surfaces the backend's own error message when the fetch fails (Reviewer F2)", async () => {
    const registry = createFakeRegistry({
      data: {
        query: async () => {
          throw new GaussMeridianAdapterError("Route decisions are temporarily unavailable", 503);
        },
      },
    });

    renderWithProviders(
      <RecentRoutesFeed
        projectId="proj_1"
        onSelectDecision={vi.fn()}
        streamStatus="live"
        streamEvents={[]}
      />,
      registry,
    );

    await waitFor(() =>
      expect(screen.getByText("Route decisions are temporarily unavailable")).toBeInTheDocument(),
    );
  });

  it("calls onSelectDecision with the full decision when a row is clicked", async () => {
    const user = userEvent.setup();
    const onSelectDecision = vi.fn();
    const registry = createFakeRegistry({
      data: {
        query: byResource({ "v1/projects/proj_1/routes": [passedDecision] }),
      },
    });

    renderWithProviders(
      <RecentRoutesFeed
        projectId="proj_1"
        onSelectDecision={onSelectDecision}
        streamStatus="live"
        streamEvents={[]}
      />,
      registry,
    );

    const row = await screen.findByRole("button", { name: /gpt-4o-mini/ });
    await user.click(row);

    expect(onSelectDecision).toHaveBeenCalledWith(passedDecision);
  });

  it("shows an empty state when there are no routed requests yet", async () => {
    const registry = createFakeRegistry({
      data: { query: byResource({ "v1/projects/proj_1/routes": [] }) },
    });

    renderWithProviders(
      <RecentRoutesFeed
        projectId="proj_1"
        onSelectDecision={vi.fn()}
        streamStatus="live"
        streamEvents={[]}
      />,
      registry,
    );

    await waitFor(() => expect(screen.getByText(/no routed requests yet/i)).toBeInTheDocument());
  });
});
