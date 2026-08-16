"use client";

import type { RouteDecision } from "@core/adapters/schemas/console.schema";
import { cn } from "@core/lib/utils";

import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";

import { complexityBand, deliveredBy, guardrailLabel, guardrailTone } from "./route-decision-utils";

import type { ReactNode } from "react";

interface RouteDecisionDrawerProps {
  /** `null` while nothing is selected — the drawer still mounts (controlled by `open`) so its
   *  enter/exit motion runs, but renders nothing until a decision is set. */
  decision: RouteDecision | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

function formatTimestamp(iso: string | null | undefined): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString();
}

/**
 * The route-transparency showcase: renders every field the real `route_decision` row actually
 * carries. Visual template = the marketing hero's trace card (`sections/hero/hero.tsx`) — the
 * same `{k, v, stage}` row idiom over CARROT · xRouter · GaussMoA · OutcomeGate, now showing a
 * real decision. Shared: M4's Activity page reuses this unchanged
 * (`import { RouteDecisionDrawer } from "@/components/overview"`).
 */
export function RouteDecisionDrawer({ decision, open, onOpenChange }: RouteDecisionDrawerProps) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="sm:max-w-xl">
        {decision && (
          <>
            <SheetHeader>
              <span className="text-muted-foreground font-mono text-[11px] tracking-[0.24em] uppercase">
                Route trace · {decision.request_id}
              </span>
              <SheetTitle>
                {(() => {
                  const delivered = deliveredBy(decision);
                  return delivered ? (
                    <>
                      {delivered.model}{" "}
                      <span className="text-muted-foreground font-normal">
                        → {delivered.provider}
                      </span>
                    </>
                  ) : (
                    <span className="text-muted-foreground font-normal">
                      Delivering model unresolved
                    </span>
                  );
                })()}
              </SheetTitle>
              <SheetDescription>Routed {formatTimestamp(decision.created_at)}</SheetDescription>
            </SheetHeader>

            <SheetBody className="flex flex-col gap-6">
              <div className="grid grid-cols-2 gap-4">
                <TraceRow label="guardrail" stage="OutcomeGate">
                  <span
                    className={cn("font-mono text-sm", guardrailTone(decision.guardrail_status))}
                  >
                    {guardrailLabel(decision.guardrail_status)}
                  </span>
                </TraceRow>
                <TraceRow label="cascade" stage="">
                  <span className="font-mono text-sm">
                    {decision.cascade_used ? "Used" : "Not used"}
                  </span>
                </TraceRow>
              </div>

              <TraceRow label="complexity" stage="CARROT">
                <span className="font-mono text-sm">
                  {decision.complexity.toFixed(2)} · {complexityBand(decision.complexity)}
                </span>
              </TraceRow>

              <section>
                <TraceRowLabel label="candidates" stage="xRouter" />
                {decision.candidates.length === 0 ? (
                  <p className="text-muted-foreground mt-2 text-sm">
                    No candidates recorded for this decision.
                  </p>
                ) : (
                  <ol className="mt-2 flex flex-col gap-2">
                    {decision.candidates.map((candidate, index) => (
                      <li
                        key={`${candidate.model}-${candidate.provider}-${index}`}
                        className={cn(
                          "border-border rounded-lg border p-3",
                          candidate.selected && "border-accent/60 bg-secondary/40",
                        )}
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="font-mono text-xs">
                            {candidate.model}{" "}
                            <span className="text-muted-foreground">· {candidate.provider}</span>
                          </span>
                          {candidate.selected && (
                            <Badge variant="mono" className="text-[10px]">
                              Selected
                            </Badge>
                          )}
                        </div>
                        <div className="bg-muted mt-2 h-1.5 overflow-hidden rounded-full">
                          <div
                            className="bg-accent h-full rounded-full"
                            style={{ width: `${Math.round(candidate.score * 100)}%` }}
                          />
                        </div>
                        <p className="text-muted-foreground mt-1.5 font-mono text-xs">
                          score {candidate.score.toFixed(2)}
                        </p>
                      </li>
                    ))}
                  </ol>
                )}
              </section>

              <section>
                <TraceRowLabel label="moa" stage="GaussMoA" />
                {decision.moa.enabled ? (
                  <ul className="mt-2 flex flex-col gap-2">
                    {decision.moa.winner && (
                      <li className="border-accent/60 bg-secondary/40 flex items-center justify-between gap-3 rounded-lg border p-3">
                        <div className="flex min-w-0 flex-col">
                          <span className="font-mono text-xs">{decision.moa.winner.model}</span>
                          <span className="text-muted-foreground text-xs">
                            confidence {Math.round(decision.moa.winner.confidence * 100)}%
                          </span>
                        </div>
                        <Badge
                          variant="outline"
                          className="border-accent text-accent shrink-0 text-[10px]"
                        >
                          Winner
                        </Badge>
                      </li>
                    )}
                    {decision.moa.losers.map((loser, index) => (
                      <li
                        key={`${loser.model}-${index}`}
                        className="border-border flex items-center justify-between gap-3 rounded-lg border p-3"
                      >
                        <div className="flex min-w-0 flex-col">
                          <span className="font-mono text-xs">{loser.model}</span>
                          <span className="text-muted-foreground text-xs">
                            confidence {Math.round(loser.confidence * 100)}%
                          </span>
                        </div>
                        <span className="text-muted-foreground shrink-0 font-mono text-xs">
                          ${loser.cost.toFixed(2)}
                        </span>
                      </li>
                    ))}
                  </ul>
                ) : (
                  <p className="text-muted-foreground mt-2 text-sm">
                    Not used for this route — a single model answered directly.
                  </p>
                )}
              </section>

              <section>
                <TraceRowLabel label="request" stage="" />
                <dl className="mt-2 grid grid-cols-2 gap-3 sm:grid-cols-3">
                  <MetaItem label="baseline cost" value={`$${decision.baseline_cost.toFixed(4)}`} />
                  <MetaItem label="routed at" value={formatTimestamp(decision.created_at)} />
                </dl>
              </section>
            </SheetBody>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}

function TraceRowLabel({ label, stage }: { label: string; stage: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-muted-foreground font-mono text-xs tracking-wide uppercase">
        {label}
      </span>
      {stage && (
        <span className="text-accent font-mono text-[10.5px] tracking-[0.12em] uppercase">
          {stage}
        </span>
      )}
    </div>
  );
}

function TraceRow({
  label,
  stage,
  children,
}: {
  label: string;
  stage: string;
  children: ReactNode;
}) {
  return (
    <div>
      <TraceRowLabel label={label} stage={stage} />
      <div className="mt-1.5">{children}</div>
    </div>
  );
}

function MetaItem({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-muted-foreground font-mono text-[10px] tracking-wide uppercase">
        {label}
      </dt>
      <dd className="mt-0.5 font-mono text-xs">{value}</dd>
    </div>
  );
}
