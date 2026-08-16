import type {
  CommerceCatalogSchema,
  ModelsResponseSchema,
} from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

type ModelListEntry = z.infer<typeof ModelsResponseSchema>["data"][number];
type ModelRateEntry = z.infer<typeof CommerceCatalogSchema>["model_rates"][number];

export type ModelTier = "Flagship" | "Efficient" | "Specialist" | "OSS";

export interface ModelCatalogEntry {
  id: string;
  provider: string;
  tier: ModelTier | null;
  pricing: {
    inputPerMillion: number;
    outputPerMillion: number;
    currency: "idr";
  } | null;
}

/**
 * Joins the configured inference allowlist with the immutable commerce catalog. Supplier-valued
 * `usd_micros` amounts are deliberately excluded: only published customer-facing IDR rates can
 * appear in the Meridian product UI. Missing or mixed-currency rates remain unpublished.
 */
export function buildModelCatalog(
  models: ModelListEntry[],
  rates: ModelRateEntry[],
): ModelCatalogEntry[] {
  const ratesByModelId = new Map(rates.map((entry) => [entry.model_id, entry]));

  return models.map((model) => {
    const matchedRate = ratesByModelId.get(model.id);
    const hasIdrRetailRate =
      matchedRate?.input_per_million_tokens.currency === "idr" &&
      matchedRate.output_per_million_tokens.currency === "idr";

    return {
      id: model.id,
      provider: model.owned_by,
      tier: null,
      pricing: hasIdrRetailRate
        ? {
            inputPerMillion: matchedRate.input_per_million_tokens.minor_units,
            outputPerMillion: matchedRate.output_per_million_tokens.minor_units,
            currency: "idr",
          }
        : null,
    };
  });
}
