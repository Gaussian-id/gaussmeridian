"use client";

import { useParams } from "next/navigation";
import { useState } from "react";

import { TrendChart, type TrendPoint } from "@/components/charts";
import { AccountStatsRow } from "@/components/dashboard/account-stats-row";
import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { ModelPerformanceCard } from "@/components/dashboard/model-performance-card";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useProjectUsageAnalytics } from "@/hooks/useGaussmeridianQueries";

type UsageRange = NonNullable<NonNullable<Parameters<typeof useProjectUsageAnalytics>[1]>["range"]>;

const RANGE_OPTIONS: { value: UsageRange; label: string }[] = [
  { value: "24h", label: "Last 24 hours" },
  { value: "7d", label: "Last 7 days" },
  { value: "30d", label: "Last 30 days" },
  { value: "90d", label: "Last 90 days" },
];

/**
 * Trend + per-model usage for this project. `useUsageAnalytics` stays server-resolved in
 * Phase 1 (see `console.schema.ts` header note) — id-parameterizing it to the active
 * `projectId` is a Phase-2 backend decision, not made here.
 */
export default function UsagePage() {
  const { projectId } = useParams<{ projectId: string }>();
  const [range, setRange] = useState<UsageRange>("7d");
  const usage = useProjectUsageAnalytics(projectId, { range });

  const costTrend: TrendPoint[] =
    usage.data?.model_performance.map((entry) => ({
      label: entry.model,
      value: entry.cost,
    })) ?? [];

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <DashboardPageHeader
          eyebrow="Project"
          title="Usage"
          description="Requests, spend, and per-model breakdown for this project."
        />

        <Select value={range} onValueChange={(value) => setRange(value as UsageRange)}>
          <SelectTrigger className="w-[160px]" aria-label="Time range">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {RANGE_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <AccountStatsRow projectId={projectId} />

      <Card className="p-6">
        <CardTitle>Cost by model</CardTitle>
        <CardDescription className="mt-1">Spend breakdown for the selected period</CardDescription>
        <div className="mt-6">
          {usage.isLoading ? (
            <p className="text-muted-foreground text-sm">Loading…</p>
          ) : costTrend.length === 0 ? (
            <p className="text-muted-foreground text-sm">No usage recorded for this period yet.</p>
          ) : (
            <TrendChart data={costTrend} />
          )}
        </div>
      </Card>

      <ModelPerformanceCard projectId={projectId} />
    </div>
  );
}
