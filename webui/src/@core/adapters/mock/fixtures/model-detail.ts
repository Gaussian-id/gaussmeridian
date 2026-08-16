import type { ModelInfoSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

type ModelInfo = z.infer<typeof ModelInfoSchema>;

/**
 * Per-model detail (`GET /v1/models/:modelId`), keyed by id. Covers exactly the three ids
 * present in `models.ts` — pricing figures match `model-pricing.ts` so the marketplace card
 * and the detail page never disagree about the same model's price. Not every model in the
 * catalog needs an entry here for the mock to be honest: an id with no entry 404s, same as a
 * real backend would for an unknown model.
 */
export const modelDetails: Record<string, ModelInfo> = {
  "gpt-4o-mini": {
    id: "gpt-4o-mini",
    name: "gpt-4o-mini",
    context_length: 128000,
    pricing: {
      input_cost_per_1k_tokens: 0.00015,
      output_cost_per_1k_tokens: 0.0006,
      currency: "USD",
      model: "gpt-4o-mini",
    },
    capabilities: {
      supports_streaming: true,
      supports_functions: true,
      supports_vision: true,
      supports_embeddings: false,
    },
  },
  "gpt-4o": {
    id: "gpt-4o",
    name: "gpt-4o",
    context_length: 128000,
    pricing: {
      input_cost_per_1k_tokens: 0.0025,
      output_cost_per_1k_tokens: 0.01,
      currency: "USD",
      model: "gpt-4o",
    },
    capabilities: {
      supports_streaming: true,
      supports_functions: true,
      supports_vision: true,
      supports_embeddings: false,
    },
  },
  "claude-3-5-sonnet": {
    id: "claude-3-5-sonnet",
    name: "claude-3-5-sonnet",
    context_length: 200000,
    pricing: {
      input_cost_per_1k_tokens: 0.003,
      output_cost_per_1k_tokens: 0.015,
      currency: "USD",
      model: "claude-3-5-sonnet",
    },
    capabilities: {
      supports_streaming: true,
      supports_functions: true,
      supports_vision: true,
      supports_embeddings: false,
    },
  },
};
