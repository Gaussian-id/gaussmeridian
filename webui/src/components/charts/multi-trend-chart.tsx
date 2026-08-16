"use client";

import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

/** One line in a `MultiTrendChart`. `color` is any CSS color — pass a theme token
 *  (`var(--chart-1)`, `var(--accent)`, `var(--destructive)`) so it follows light/dark. */
export interface TrendSeries {
  /** Row property this line reads its y-value from. */
  key: string;
  /** Legend + tooltip label. */
  label: string;
  color: string;
}

/** A row of the chart, keyed by month. Every series `key` is a numeric column on the row.
 *  Consumers whose row type already has `month: string` (e.g. `BusinessMonth`) can pass their
 *  arrays directly — the component is generic over the row shape, constrained only to `month`. */
export type MultiTrendRow = { month: string } & Record<string, number | string>;

interface MultiTrendChartProps<Row extends { month: string }> {
  series: TrendSeries[];
  data: Row[];
  /** Formats y-axis ticks + tooltip values (e.g. currency, compact counts). Defaults to a plain
   *  locale number. */
  valueFormatter?: (value: number) => string;
  height?: number;
}

const defaultFormatter = (value: number) => value.toLocaleString();

/**
 * A multi-series trend chart — a net-new sibling to the single-series `TrendChart` (not a fork:
 * `TrendChart` has five consumers whose one-`Area` shape must not change). Renders N themed
 * `<Line>`s over a shared month axis, a legend (identity is never color-alone), and a
 * crosshair tooltip. Follows the /dataviz discipline: thin 2px lines, recessive grid/axes in
 * muted-foreground ink, one y-axis (never dual-axis — pass separately-scaled measures as two
 * charts), and a validated colorblind-safe series palette supplied by the caller via theme
 * tokens.
 */
export function MultiTrendChart<Row extends { month: string }>({
  series,
  data,
  valueFormatter = defaultFormatter,
  height = 260,
}: MultiTrendChartProps<Row>) {
  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart data={data} margin={{ top: 8, right: 12, bottom: 0, left: -8 }}>
        <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" vertical={false} />
        <XAxis
          dataKey="month"
          stroke="var(--muted-foreground)"
          fontSize={12}
          tickLine={false}
          axisLine={false}
        />
        <YAxis
          stroke="var(--muted-foreground)"
          fontSize={12}
          tickLine={false}
          axisLine={false}
          width={56}
          tickFormatter={(value: number) => valueFormatter(value)}
        />
        <Tooltip
          contentStyle={{
            background: "var(--popover)",
            border: "1px solid var(--border)",
            borderRadius: "0.5rem",
            color: "var(--popover-foreground)",
            fontSize: "0.8rem",
          }}
          formatter={(value) => valueFormatter(Number(value))}
        />
        <Legend wrapperStyle={{ fontSize: 12, color: "var(--muted-foreground)" }} />
        {series.map((s) => (
          <Line
            key={s.key}
            type="monotone"
            dataKey={s.key}
            name={s.label}
            stroke={s.color}
            strokeWidth={2}
            dot={false}
            activeDot={{ r: 4 }}
          />
        ))}
      </LineChart>
    </ResponsiveContainer>
  );
}
