/**
 * `GET /v1/admin/metrics?months=N` fixture — six months, newest last (`current` is
 * `series.at(-1)`, mirroring `get_admin_metrics`'s real `series.last()` convention). Revenue
 * tracks close to provider cost throughout — there is no billing markup on this backend yet
 * (PRD-23 Wave B doc comment) — so margin is small and sometimes negative by design. The
 * CURRENT month is deliberately negative so the admin dashboard's honest "outcome-gate
 * write-offs" treatment is visible without any interaction, not just reachable in an edge case.
 */
export interface MonthMetricsFixture {
  month: string;
  mau_api: number;
  mau_console: number;
  revenue: number;
  provider_cost: number;
  margin: number;
}

export const adminMetricsSeries: MonthMetricsFixture[] = [
  {
    month: "2026-02",
    mau_api: 96,
    mau_console: 214,
    revenue: 3120.4,
    provider_cost: 3350.1,
    margin: -229.7,
  },
  {
    month: "2026-03",
    mau_api: 118,
    mau_console: 260,
    revenue: 3840.15,
    provider_cost: 3610.9,
    margin: 229.25,
  },
  {
    month: "2026-04",
    mau_api: 142,
    mau_console: 305,
    revenue: 4425.8,
    provider_cost: 4602.35,
    margin: -176.55,
  },
  {
    month: "2026-05",
    mau_api: 165,
    mau_console: 341,
    revenue: 4990.6,
    provider_cost: 4780.2,
    margin: 210.4,
  },
  {
    month: "2026-06",
    mau_api: 189,
    mau_console: 372,
    revenue: 5310.9,
    provider_cost: 5540.75,
    margin: -229.85,
  },
  {
    month: "2026-07",
    mau_api: 204,
    mau_console: 398,
    revenue: 5602.25,
    provider_cost: 6015.5,
    margin: -413.25,
  },
];
