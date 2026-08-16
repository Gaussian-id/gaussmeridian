import { cn } from "@core/lib/utils";

import type { RouteDecisionStreamStatus } from "@/hooks/useRouteDecisionStream";

const LABEL: Record<RouteDecisionStreamStatus, string> = {
  connecting: "Connecting…",
  live: "Live",
  disconnected: "Disconnected",
};

// On a light card (`onDark = false`).
const DOT: Record<RouteDecisionStreamStatus, string> = {
  connecting: "bg-muted-foreground",
  live: "bg-[var(--gauss-400)] motion-safe:animate-pulse",
  disconnected: "bg-destructive",
};

// On the hero's dark `bg-brand-gradient` panel — `bg-destructive` reads as near-black there, so
// disconnected uses a tone that stays visible against the gradient.
const DOT_ON_DARK: Record<RouteDecisionStreamStatus, string> = {
  connecting: "bg-white/50",
  live: "bg-[var(--gauss-400)] motion-safe:animate-pulse",
  disconnected: "bg-red-400",
};

interface StreamStatusIndicatorProps {
  status: RouteDecisionStreamStatus;
  /** Style for the hero's dark gradient panel vs. a light card. */
  onDark?: boolean;
  className?: string;
}

/**
 * The single, shared "is the live SSE feed connected?" indicator — one label/dot mapping used by
 * BOTH the Overview hero badge and `RecentRoutesFeed`, both fed the same
 * `useRouteDecisionStream` status lifted to the page. This is deliberately the only place the
 * status maps to a label/color, so the page can never show two "live" badges that disagree
 * (Reviewer F1): the word only ever says "Live" when the connection genuinely is.
 */
export function StreamStatusIndicator({ status, onDark, className }: StreamStatusIndicatorProps) {
  return (
    <span
      className={cn(
        "flex items-center gap-1.5 font-mono uppercase",
        onDark ? "text-white/55" : "text-muted-foreground",
        className,
      )}
    >
      <span
        className={cn(
          "rounded-full",
          onDark ? "size-[7px]" : "size-[6px]",
          onDark ? DOT_ON_DARK[status] : DOT[status],
        )}
      />
      {LABEL[status]}
    </span>
  );
}
