import type { ModelPricingResponseSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const modelPricing: z.infer<typeof ModelPricingResponseSchema> = {
  models: [
    {
      model: "gpt-4o-mini",
      provider: "openai",
      input_cost_per_1k_tokens: 0.00015,
      output_cost_per_1k_tokens: 0.0006,
      currency: "USD",
    },
    {
      model: "gpt-4o",
      provider: "openai",
      input_cost_per_1k_tokens: 0.0025,
      output_cost_per_1k_tokens: 0.01,
      currency: "USD",
    },
    {
      model: "claude-3-5-sonnet",
      provider: "anthropic",
      input_cost_per_1k_tokens: 0.003,
      output_cost_per_1k_tokens: 0.015,
      currency: "USD",
    },
  ],
};
