import type { UsageAnalyticsSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const usage: z.infer<typeof UsageAnalyticsSchema> = {
  summary: {
    total_requests: 512,
    successful_requests: 493,
    total_tokens: 812000,
    total_cost: 128.44,
    average_latency_ms: 640,
    p95_latency_ms: 1580,
    p99_latency_ms: 2210,
    success_rate: 0.963,
    error_rate: 0.037,
  },
  model_performance: [
    { model: "gpt-4o-mini", requests: 310, tokens: 210000, cost: 24.1 },
    { model: "gpt-4o", requests: 140, tokens: 402000, cost: 78.2 },
    { model: "claude-3-5-sonnet", requests: 62, tokens: 200000, cost: 26.14 },
  ],
  time_range: { start: "2026-06-14T00:00:00Z", end: "2026-07-14T00:00:00Z" },
};
