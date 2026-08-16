"use client";

import { useMemo } from "react";

import type {
  RouteDecision,
  RouteDecisionStreamEvent,
} from "@core/adapters/schemas/console.schema";
import { cn } from "@core/lib/utils";

import { Badge } from "@/components/ui/badge";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { useRouteDecisions } from "@/hooks/useConsoleQueries";
import type { RouteDecisionStreamStatus } from "@/hooks/useRouteDecisionStream";

import {
  COMPLEXITY_BAND_LABEL,
  complexityBand,
  deliveredBy,
  guardrailLabel,
  guardrailTone,
  queryErrorMessage,
} from "./route-decision-utils";
import { StreamStatusIndicator } from "./stream-status-indicator";

const RECENT_ROUTES_LIMIT = 8;

interface RecentRoutesFeedProps {
  projectId: string;
  onSelectDecision: (decision: RouteDecision) => void;
  /** SSE connection state + buffered new decisions — lifted to the page so this feed and the
   *  hero badge share one connection and one status (Reviewer F1). */
  streamStatus: RouteDecisionStreamStatus;
  streamEvents: RouteDecisionStreamEvent[];
}

/**
 * Live feed of recent routing decisions for this project, sourced from `GET /v1/route-decisions`
 * (see `useRouteDecisions`) and kept current by the page-level `useRouteDecisionStream` SSE feed
 * (`GET /v1/route-decisions/stream`, passed in via `streamEvents`) — new decisions merge in at
 * the top by `request_id` as they arrive, deduped against the initial fetch. The connection
 * status shown is real, not decorative: it's the same `streamStatus` the hero badge renders,
 * including a genuine "Disconnected" while the connection backs off and retries.
 *
 * The real `route_decision` row carries no charged-cost/`r_binary` signal of its own (that lives
 * on the separate ledger, see `console.schema.ts`'s doc comment), so this shows the real
 * guardrail outcome instead of a fabricated charged/free pill. Clicking a row opens the full
 * transparency drawer (`route-decision-drawer.tsx`).
 */
export function RecentRoutesFeed({
  projectId,
  onSelectDecision,
  streamStatus,
  streamEvents,
}: RecentRoutesFeedProps) {
  const routes = useRouteDecisions(projectId, { limit: RECENT_ROUTES_LIMIT });

  const decisions = useMemo(() => {
    const byRequestId = new Map<string, RouteDecision>();
    for (const decision of routes.data ?? []) byRequestId.set(decision.request_id, decision);
    for (const event of streamEvents) {
      if (!byRequestId.has(event.request_id)) byRequestId.set(event.request_id, event);
    }
    return [...byRequestId.values()]
      .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
      .slice(0, RECENT_ROUTES_LIMIT);
  }, [routes.data, streamEvents]);

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between gap-2">
        <CardTitle>Recent routes</CardTitle>
        <StreamStatusIndicator status={streamStatus} className="text-[10px] tracking-[0.14em]" />
      </div>
      <CardDescription className="mt-1">
        Last {RECENT_ROUTES_LIMIT} routing decisions — select a row for the full trace.
      </CardDescription>
      <div className="mt-4 flex flex-col text-sm">
        {routes.isLoading && <p className="text-muted-foreground py-2">Loading…</p>}
        {routes.isError && (
          <p className="text-muted-foreground py-2">
            {queryErrorMessage(routes.error, "Could not load recent routes.")}
          </p>
        )}
        {!routes.isLoading && !routes.isError && decisions.length === 0 && (
          <p className="text-muted-foreground py-2">No routed requests yet.</p>
        )}
        {decisions.map((decision) => {
          const delivered = deliveredBy(decision);
          const band = complexityBand(decision.complexity);
          return (
            <button
              key={decision.id ?? decision.request_id}
              type="button"
              onClick={() => onSelectDecision(decision)}
              className="border-border hover:bg-secondary/60 flex items-center justify-between gap-3 border-b px-2 py-2.5 text-left last:border-0"
            >
              <div className="flex min-w-0 flex-col">
                <span className="truncate font-mono text-xs">
                  {delivered ? (
                    <>
                      {delivered.model}{" "}
                      <span className="text-muted-foreground">→ {delivered.provider}</span>
                    </>
                  ) : (
                    <span className="text-muted-foreground">Delivering model unresolved</span>
                  )}
                </span>
                <span className="text-muted-foreground truncate font-mono text-[11px]">
                  {decision.request_id}
                </span>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                <Badge variant="mono" className="text-[10px]">
                  {COMPLEXITY_BAND_LABEL[band]}
                </Badge>
                <span
                  className={cn(
                    "font-mono text-[10.5px] tracking-wide uppercase",
                    guardrailTone(decision.guardrail_status),
                  )}
                >
                  {guardrailLabel(decision.guardrail_status)}
                </span>
              </div>
            </button>
          );
        })}
      </div>
    </Card>
  );
}
