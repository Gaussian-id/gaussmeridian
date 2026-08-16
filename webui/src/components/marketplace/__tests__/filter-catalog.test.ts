import { describe, expect, it } from "vitest";

import type { ModelCatalogEntry } from "@/components/dashboard/model-catalog";

import {
  ALL_PROVIDERS_VALUE,
  ALL_TIERS_VALUE,
  collectProviders,
  collectTiers,
  DEFAULT_MARKETPLACE_FILTERS,
  filterModelCatalog,
  UNTIERED_VALUE,
} from "../filter-catalog";

const catalog: ModelCatalogEntry[] = [
  {
    id: "gpt-4o-mini",
    provider: "openai",
    tier: null,
    pricing: { inputPerMillion: 3_000, outputPerMillion: 12_000, currency: "idr" },
  },
  {
    id: "gpt-4o",
    provider: "openai",
    tier: null,
    pricing: { inputPerMillion: 50_000, outputPerMillion: 200_000, currency: "idr" },
  },
  {
    id: "claude-3-5-sonnet",
    provider: "anthropic",
    tier: null,
    pricing: { inputPerMillion: 60_000, outputPerMillion: 300_000, currency: "idr" },
  },
  {
    id: "llama-3-70b",
    provider: "meta-llama",
    tier: "OSS",
    pricing: null,
  },
];

describe("collectProviders", () => {
  it("returns the distinct providers actually present in the catalog, sorted", () => {
    expect(collectProviders(catalog)).toEqual(["anthropic", "meta-llama", "openai"]);
  });
});

describe("collectTiers", () => {
  it("returns only tiers that appear on at least one model — never a fabricated tier list", () => {
    expect(collectTiers(catalog)).toEqual(["OSS"]);
  });
});

describe("filterModelCatalog", () => {
  it("returns every model when no filters are applied", () => {
    expect(filterModelCatalog(catalog, DEFAULT_MARKETPLACE_FILTERS)).toHaveLength(4);
  });

  it("filters by search against id and provider (case-insensitive)", () => {
    const result = filterModelCatalog(catalog, { ...DEFAULT_MARKETPLACE_FILTERS, search: "GPT" });
    expect(result.map((m) => m.id)).toEqual(["gpt-4o-mini", "gpt-4o"]);
  });

  it("filters by provider", () => {
    const result = filterModelCatalog(catalog, {
      ...DEFAULT_MARKETPLACE_FILTERS,
      provider: "anthropic",
    });
    expect(result.map((m) => m.id)).toEqual(["claude-3-5-sonnet"]);
  });

  it("filters by tier, including the untiered sentinel", () => {
    const oss = filterModelCatalog(catalog, { ...DEFAULT_MARKETPLACE_FILTERS, tier: "OSS" });
    expect(oss.map((m) => m.id)).toEqual(["llama-3-70b"]);

    const untiered = filterModelCatalog(catalog, {
      ...DEFAULT_MARKETPLACE_FILTERS,
      tier: UNTIERED_VALUE,
    });
    expect(untiered.map((m) => m.id)).toEqual(["gpt-4o-mini", "gpt-4o", "claude-3-5-sonnet"]);
  });

  it("combines search + provider + tier filters (AND, not OR)", () => {
    const result = filterModelCatalog(catalog, {
      search: "gpt",
      provider: "openai",
      tier: UNTIERED_VALUE,
      priceSort: "none",
    });
    expect(result.map((m) => m.id)).toEqual(["gpt-4o-mini", "gpt-4o"]);
  });

  it("sorts by price ascending, keeping unpriced models last", () => {
    const result = filterModelCatalog(catalog, {
      ...DEFAULT_MARKETPLACE_FILTERS,
      priceSort: "price-asc",
    });
    expect(result.map((m) => m.id)).toEqual([
      "gpt-4o-mini",
      "gpt-4o",
      "claude-3-5-sonnet",
      "llama-3-70b",
    ]);
  });

  it("sorts by price descending, keeping unpriced models last in either direction", () => {
    const result = filterModelCatalog(catalog, {
      ...DEFAULT_MARKETPLACE_FILTERS,
      priceSort: "price-desc",
    });
    expect(result.map((m) => m.id)).toEqual([
      "claude-3-5-sonnet",
      "gpt-4o",
      "gpt-4o-mini",
      "llama-3-70b",
    ]);
  });

  it("ALL_PROVIDERS_VALUE / ALL_TIERS_VALUE apply no filter", () => {
    const result = filterModelCatalog(catalog, {
      search: "",
      provider: ALL_PROVIDERS_VALUE,
      tier: ALL_TIERS_VALUE,
      priceSort: "none",
    });
    expect(result).toHaveLength(4);
  });
});
