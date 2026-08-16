"use client";

import { useEffect, useRef, useState } from "react";

import { RouteDecisionStreamEventSchema } from "@core/adapters/schemas/console.schema";
import type { RouteDecisionStreamEvent } from "@core/adapters/schemas/console.schema";

export type RouteDecisionStreamStatus = "connecting" | "live" | "disconnected";

const MAX_BUFFERED_EVENTS = 20;
const INITIAL_BACKOFF_MS = 1000;
const MAX_BACKOFF_MS = 30000;

/**
 * Live feed of new route decisions for the caller's project over the real SSE endpoint
 * `GET /v1/route-decisions/stream` — proxied through `/api/gaussmeridian/...` exactly like every
 * other resource (see `gaussmeridian-data.adapter.ts`'s doc comment on why: the backend only ever
 * reads the session via our own Next.js server, never a browser-held cookie directly). Each
 * `data: <json>\n\n` frame is a `RouteDecisionInsert` (see `console.schema.ts`'s
 * `RouteDecisionStreamEventSchema`) — validated the same way every adapter boundary is, so a
 * malformed frame is dropped rather than crashing the feed.
 *
 * Reconnects with capped exponential backoff on any drop. `EventSource` has its own built-in
 * retry, but this hook additionally tracks connection status so the UI can show an honest
 * "disconnected" state instead of silently going stale while the browser retries in the
 * background, and drives its own reconnect (rather than trusting the browser's default retry
 * delay) so a proxy-level failure recovers on a bounded schedule.
 */
export function useRouteDecisionStream(
  projectId: string,
  enabled: boolean,
): {
  status: RouteDecisionStreamStatus;
  events: RouteDecisionStreamEvent[];
} {
  const [status, setStatus] = useState<RouteDecisionStreamStatus>("connecting");
  const [events, setEvents] = useState<RouteDecisionStreamEvent[]>([]);
  const backoffRef = useRef(INITIAL_BACKOFF_MS);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const closedRef = useRef(false);

  useEffect(() => {
    if (!enabled) return undefined;
    closedRef.current = false;
    let source: EventSource | null = null;

    function connect() {
      if (closedRef.current) return;
      setStatus((current) => (current === "live" ? current : "connecting"));
      const context = new URLSearchParams({ project_id: projectId });
      source = new EventSource(`/api/gaussmeridian/v1/route-decisions/stream?${context}`);

      source.onopen = () => {
        backoffRef.current = INITIAL_BACKOFF_MS;
        setStatus("live");
      };

      source.onmessage = (event: MessageEvent<string>) => {
        let json: unknown;
        try {
          json = JSON.parse(event.data);
        } catch {
          return; // Malformed frame — drop it, the stream stays open.
        }
        const parsed = RouteDecisionStreamEventSchema.safeParse(json);
        if (!parsed.success) return;
        setEvents((current) => [parsed.data, ...current].slice(0, MAX_BUFFERED_EVENTS));
      };

      source.onerror = () => {
        source?.close();
        if (closedRef.current) return;
        setStatus("disconnected");
        const delay = backoffRef.current;
        backoffRef.current = Math.min(delay * 2, MAX_BACKOFF_MS);
        timeoutRef.current = setTimeout(connect, delay);
      };
    }

    connect();

    return () => {
      closedRef.current = true;
      source?.close();
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, [enabled, projectId]);

  return { status, events };
}
