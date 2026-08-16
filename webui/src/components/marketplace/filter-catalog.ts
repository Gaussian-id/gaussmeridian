import type { ModelCatalogEntry, ModelTier } from "@/components/dashboard/model-catalog";

/** Sentinel `Select` value meaning "no filter applied" — Radix `Select` requires a non-empty
 *  string value, so `undefined`/`null` can't be used directly as an option value. */
export const ALL_PROVIDERS_VALUE = "all-providers";
export const ALL_TIERS_VALUE = "all-tiers";
/** Sentinel for `ModelCatalogEntry.tier === null` (every model without a derived tier). */
export const UNTIERED_VALUE = "untiered";

export type PriceSort = "none" | "price-asc" | "price-desc";

export interface MarketplaceFilterState {
  search: string;
  provider: string;
  tier: string;
  priceSort: PriceSort;
}

export const DEFAULT_MARKETPLACE_FILTERS: MarketplaceFilterState = {
  search: "",
  provider: ALL_PROVIDERS_VALUE,
  tier: ALL_TIERS_VALUE,
  priceSort: "none",
};

/** Distinct providers present in the catalog, sorted — never a hardcoded list, so the filter
 *  can never offer a provider that doesn't actually appear in `/v1/models`. */
export function collectProviders(catalog: ModelCatalogEntry[]): string[] {
  return Array.from(new Set(catalog.map((model) => model.provider))).sort();
}

/** Distinct tiers present in the catalog. `deriveTier` (model-catalog.ts) only ever assigns
 *  "OSS" or `null` today — this stays derived rather than hardcoding every `ModelTier` so the
 *  filter never offers a tier no model actually has. */
export function collectTiers(catalog: ModelCatalogEntry[]): ModelTier[] {
  const tiers = new Set<ModelTier>();
  for (const model of catalog) {
    if (model.tier) tiers.add(model.tier);
  }
  return Array.from(tiers).sort();
}

function inputPrice(model: ModelCatalogEntry): number {
  return model.pricing?.inputPerMillion ?? 0;
}

/**
 * Pure filter + sort over a catalog already built by `buildModelCatalog`. Deliberately only
 * covers fields the catalog actually carries (provider, tier, price) — there is no
 * capability field at the list level (`ModelsResponseSchema` has none; capabilities only
 * exist on the single-model detail response), so a "capability" filter is not offered here
 * rather than faked against data that doesn't exist yet.
 */
export function filterModelCatalog(
  catalog: ModelCatalogEntry[],
  filters: MarketplaceFilterState,
): ModelCatalogEntry[] {
  const search = filters.search.trim().toLowerCase();

  const filtered = catalog.filter((model) => {
    if (
      search &&
      !model.id.toLowerCase().includes(search) &&
      !model.provider.toLowerCase().includes(search)
    ) {
      return false;
    }
    if (filters.provider !== ALL_PROVIDERS_VALUE && model.provider !== filters.provider) {
      return false;
    }
    if (filters.tier === UNTIERED_VALUE && model.tier !== null) return false;
    if (
      filters.tier !== ALL_TIERS_VALUE &&
      filters.tier !== UNTIERED_VALUE &&
      model.tier !== filters.tier
    ) {
      return false;
    }
    return true;
  });

  if (filters.priceSort === "none") return filtered;

  // Models with no reference pricing always sort last, in either direction — "unknown" isn't
  // meaningfully cheaper or more expensive than a priced model, so it shouldn't rank as $0.
  const priced = filtered.filter((model) => model.pricing !== null);
  const unpriced = filtered.filter((model) => model.pricing === null);
  priced.sort((a, b) =>
    filters.priceSort === "price-asc"
      ? inputPrice(a) - inputPrice(b)
      : inputPrice(b) - inputPrice(a),
  );
  return [...priced, ...unpriced];
}
