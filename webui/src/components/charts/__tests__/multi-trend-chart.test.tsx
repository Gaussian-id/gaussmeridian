import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { MultiTrendChart, type MultiTrendRow, type TrendSeries } from "../multi-trend-chart";

import type { ReactNode } from "react";

// Recharts measures its container to lay out; jsdom reports 0×0, so the real chart renders
// nothing. Stub the primitives with passthroughs that expose each series as a testable node —
// this test is about the component fanning one `<Line>` out per series descriptor, not about
// Recharts' SVG geometry.
vi.mock("recharts", () => {
  const Passthrough = ({ children }: { children?: ReactNode }) => <div>{children}</div>;
  return {
    ResponsiveContainer: Passthrough,
    LineChart: Passthrough,
    CartesianGrid: () => null,
    XAxis: () => null,
    YAxis: () => null,
    Tooltip: () => null,
    Legend: () => null,
    Line: ({ dataKey, name }: { dataKey: string; name: string }) => (
      <div data-testid="series-line" data-key={dataKey}>
        {name}
      </div>
    ),
  };
});

const series: TrendSeries[] = [
  { key: "revenue", label: "Revenue", color: "var(--chart-1)" },
  { key: "provider_cost", label: "Provider cost", color: "var(--chart-4)" },
  { key: "bleed", label: "Bleed", color: "var(--chart-6)" },
];

const data: MultiTrendRow[] = [
  { month: "2026-06", revenue: 100, provider_cost: 120, bleed: 30 },
  { month: "2026-07", revenue: 140, provider_cost: 150, bleed: 40 },
];

describe("MultiTrendChart", () => {
  it("renders one line per series descriptor", () => {
    render(<MultiTrendChart series={series} data={data} />);
    expect(screen.getAllByTestId("series-line")).toHaveLength(series.length);
  });

  it("labels each series by its descriptor label (identity is never color-alone)", () => {
    render(<MultiTrendChart series={series} data={data} />);
    for (const s of series) {
      expect(screen.getByText(s.label)).toBeInTheDocument();
    }
  });

  it("binds each line to its series key", () => {
    render(<MultiTrendChart series={series} data={data} />);
    const keys = screen.getAllByTestId("series-line").map((node) => node.getAttribute("data-key"));
    expect(keys).toEqual(["revenue", "provider_cost", "bleed"]);
  });
});
