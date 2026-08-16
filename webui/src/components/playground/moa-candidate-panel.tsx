import type { MoaCandidate } from "@core/adapters/schemas/console.schema";
import { cn } from "@core/lib/utils";

import { Badge } from "@/components/ui/badge";

interface MoaCandidatePanelProps {
  candidates: MoaCandidate[];
}

/**
 * Inline GaussMoA candidate panel for a Playground assistant turn. Same winner/loser idiom as
 * `overview/route-decision-drawer.tsx`'s MoA section: exactly one candidate is marked Winner,
 * every other candidate is stamped `$0.00` — the same "not charged" honesty applied per
 * candidate instead of per request.
 */
export function MoaCandidatePanel({ candidates }: MoaCandidatePanelProps) {
  return (
    <ul className="mt-2 flex flex-col gap-1.5">
      {candidates.map((candidate) => (
        <li
          key={`${candidate.model}-${candidate.provider}`}
          className={cn(
            "border-border flex items-center justify-between gap-3 rounded-lg border p-2.5",
            candidate.is_winner && "border-accent/60 bg-secondary/40",
          )}
        >
          <div className="flex min-w-0 flex-col">
            <span className="truncate font-mono text-xs">
              {candidate.model}{" "}
              <span className="text-muted-foreground">· {candidate.provider}</span>
            </span>
            <span className="text-muted-foreground text-[11px]">
              contribution {Math.round(candidate.contribution * 100)}%
            </span>
          </div>
          {candidate.is_winner ? (
            <Badge variant="outline" className="border-accent text-accent shrink-0 text-[10px]">
              Winner
            </Badge>
          ) : (
            <span className="text-muted-foreground shrink-0 font-mono text-xs">
              ${candidate.stamped_cost.toFixed(2)}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}
