import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { RouteDecision } from "@core/adapters/schemas/console.schema";

import { RouteDecisionDrawer } from "../route-decision-drawer";

const moaDecision: RouteDecision = {
  id: "route_3",
  request_id: "req_3",
  project_id: "proj_1",
  org_id: "org_1",
  candidates: [
    { model: "gpt-4o", provider: "openai", score: 0.89, selected: false },
    { model: "claude-3-5-sonnet", provider: "anthropic", score: 0.85, selected: false },
    { model: "gemini-1.5-pro", provider: "google", score: 0.79, selected: false },
  ],
  moa: {
    enabled: true,
    winner: { model: "gpt-4o", confidence: 0.89, cost: 0.0672 },
    losers: [
      { model: "claude-3-5-sonnet", confidence: 0.85, cost: 0 },
      { model: "gemini-1.5-pro", confidence: 0.79, cost: 0 },
    ],
  },
  guardrail_status: "skipped",
  cascade_used: false,
  complexity: 0.87,
  baseline_cost: 0.0672,
  created_at: "2026-07-14T11:47:00Z",
};

const singleModelDecision: RouteDecision = {
  id: "route_single",
  request_id: "req_single",
  project_id: "proj_1",
  org_id: "org_1",
  candidates: [{ model: "gpt-4o-mini", provider: "openai", score: 0.9, selected: true }],
  moa: { enabled: false, winner: null, losers: [] },
  guardrail_status: "disabled",
  cascade_used: true,
  complexity: 0.5,
  baseline_cost: 0.02,
  created_at: "2026-07-14T09:00:00Z",
};

describe("RouteDecisionDrawer", () => {
  it("renders every RouteDecisionSchema field for a GaussMoA-routed decision", () => {
    render(<RouteDecisionDrawer decision={moaDecision} open onOpenChange={vi.fn()} />);

    // Header: request_id, delivered model (from moa.winner, since no candidate is selected)
    expect(screen.getByText(/req_3/)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /gpt-4o.*gaussmoa/ })).toBeInTheDocument();

    // Guardrail + complexity
    expect(screen.getAllByText("Skipped").length).toBeGreaterThan(0);
    expect(screen.getByText("0.87 · high")).toBeInTheDocument();

    // Candidates: model/provider/score, none marked selected (real MoA-dispatch shape)
    expect(screen.getAllByText(/claude-3-5-sonnet/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/gemini-1\.5-pro/).length).toBeGreaterThan(0);
    expect(screen.queryByText("Selected")).not.toBeInTheDocument();

    // GaussMoA panel: exactly one winner, losers stamped $0.00
    expect(screen.getByText("Winner")).toBeInTheDocument();
    expect(screen.getAllByText("$0.00").length).toBe(2); // claude + gemini losers

    // Cascade + baseline cost
    expect(screen.getByText("Not used")).toBeInTheDocument(); // cascade_used: false
    expect(screen.getByText("$0.0672")).toBeInTheDocument(); // baseline cost
  });

  it("marks exactly one GaussMoA winner and stamps every loser $0.00", () => {
    render(<RouteDecisionDrawer decision={moaDecision} open onOpenChange={vi.fn()} />);

    const winners = screen.getAllByText("Winner");
    expect(winners).toHaveLength(1);
    expect(screen.getAllByText("$0.00")).toHaveLength(moaDecision.moa.losers.length);
  });

  it("shows 'not used' GaussMoA copy and a Selected badge for a single-model, cascade-used route", () => {
    render(<RouteDecisionDrawer decision={singleModelDecision} open onOpenChange={vi.fn()} />);

    expect(
      screen.getByText("Not used for this route — a single model answered directly."),
    ).toBeInTheDocument();
    expect(screen.getByText("Disabled")).toBeInTheDocument(); // guardrail_status
    expect(screen.getByText("Used")).toBeInTheDocument(); // cascade_used: true
    expect(screen.getByText("Selected")).toBeInTheDocument();
  });

  it("renders nothing inside the sheet when no decision is selected", () => {
    render(<RouteDecisionDrawer decision={null} open onOpenChange={vi.fn()} />);
    expect(screen.queryByText(/route trace/i)).not.toBeInTheDocument();
  });
});
