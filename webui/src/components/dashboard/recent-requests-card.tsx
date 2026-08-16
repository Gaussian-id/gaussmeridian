"use client";

import { cn } from "@core/lib/utils";

import { Badge } from "@/components/ui/badge";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { useProjectRequestLogs } from "@/hooks/useGaussmeridianQueries";

const RECENT_LOGS_LIMIT = 5;

export function RecentRequestsCard({ projectId }: { projectId: string }) {
  const recentLogs = useProjectRequestLogs(projectId, { limit: RECENT_LOGS_LIMIT });

  return (
    <Card className="p-6">
      <CardTitle>Recent requests</CardTitle>
      <CardDescription className="mt-1">Last {RECENT_LOGS_LIMIT} requests</CardDescription>
      <div className="mt-4 flex flex-col gap-3 text-sm">
        {recentLogs.isLoading && <p className="text-muted-foreground">Loading…</p>}
        {recentLogs.isError && (
          <p className="text-muted-foreground">Could not load recent requests.</p>
        )}
        {recentLogs.data?.length === 0 && <p className="text-muted-foreground">No requests yet.</p>}
        {recentLogs.data?.map((log, index) => {
          // `r_binary` is the outcome-billing ledger's real charged-vs-not signal:
          // 1 = response validated and charged, 0 = failed validation, charged $0.
          const charged = log.r_binary === 1;
          return (
            <div
              key={log.id ?? index}
              className={cn(
                "border-border flex items-center justify-between gap-3 border-b pb-2 last:border-0 last:pb-0",
                !charged && "opacity-60",
              )}
            >
              <div className="flex flex-col">
                <span className="font-mono text-xs">{log.model}</span>
                <span className="text-muted-foreground text-xs">{log.validator_result}</span>
              </div>
              {charged ? (
                <span className="font-mono text-xs">${log.cost_charged.toFixed(4)}</span>
              ) : (
                <Badge variant="outline" className="text-[10px]">
                  Not charged
                </Badge>
              )}
            </div>
          );
        })}
      </div>
    </Card>
  );
}
