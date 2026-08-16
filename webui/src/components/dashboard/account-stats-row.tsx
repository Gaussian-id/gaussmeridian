"use client";

import { StatCard } from "@/components/dashboard/stat-card";
import { useProjectUsageAnalytics } from "@/hooks/useGaussmeridianQueries";

/** Settled, project-scoped usage facts from the delivery ledger. */
export function AccountStatsRow({ projectId }: { projectId: string }) {
  const usage = useProjectUsageAnalytics(projectId);

  return (
    <div className="grid gap-4 sm:grid-cols-3">
      <StatCard
        label="Requests (period)"
        value={usage.data ? usage.data.summary.total_requests.toLocaleString() : undefined}
        isLoading={usage.isLoading}
      />
      <StatCard
        label="Tokens (period)"
        value={usage.data ? usage.data.summary.total_tokens.toLocaleString() : undefined}
        isLoading={usage.isLoading}
      />
      <StatCard
        label="Settled usage charge"
        value={usage.data ? `$${usage.data.summary.total_cost.toFixed(6)}` : undefined}
        isLoading={usage.isLoading}
      />
    </div>
  );
}
