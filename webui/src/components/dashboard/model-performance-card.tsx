"use client";

import { TrendChart, type TrendPoint } from "@/components/charts";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { useProjectUsageAnalytics } from "@/hooks/useGaussmeridianQueries";

/** Requests-by-model trend for the current period, derived from usage analytics. */
export function ModelPerformanceCard({ projectId }: { projectId: string }) {
  const usage = useProjectUsageAnalytics(projectId);

  const trend: TrendPoint[] =
    usage.data?.model_performance.map((entry) => ({
      label: entry.model,
      value: entry.requests,
    })) ?? [];

  return (
    <Card className="p-6 lg:col-span-2">
      <CardTitle>Model performance</CardTitle>
      <CardDescription className="mt-1">Requests by model, current period</CardDescription>
      <div className="mt-6">
        {usage.isLoading ? (
          <p className="text-muted-foreground text-sm">Loading…</p>
        ) : trend.length === 0 ? (
          <p className="text-muted-foreground text-sm">No usage recorded for this period yet.</p>
        ) : (
          <TrendChart data={trend} />
        )}
      </div>
    </Card>
  );
}
