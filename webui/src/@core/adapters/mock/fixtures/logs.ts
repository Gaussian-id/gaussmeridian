import type { RequestLogSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const logs: z.infer<typeof RequestLogSchema>[] = [
  {
    id: "log_1",
    model: "gpt-4o-mini",
    provider: "openai",
    tokens_in: 120,
    tokens_out: 80,
    cost_charged: 0.0123,
    r_binary: 1,
    complexity_score: 0.4,
    validator_result: "passed",
    retry_count: 0,
    latency_ms: 380,
    created_at: "2026-07-14T09:12:00Z",
  },
  {
    id: "log_2",
    model: "gpt-4o",
    provider: "openai",
    tokens_in: 200,
    tokens_out: 10,
    cost_charged: 0,
    r_binary: 0,
    complexity_score: 0.9,
    validator_result: "failed:low_confidence",
    retry_count: 1,
    latency_ms: 800,
    created_at: "2026-07-14T10:00:00Z",
  },
];
