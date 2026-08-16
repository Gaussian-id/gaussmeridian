import { describe, expect, it } from "vitest";

import { buildModelCatalog } from "../model-catalog";

function model(overrides: Partial<{ id: string; owned_by: string }> = {}) {
  return {
    id: "openai/gpt-4o-mini",
    object: "model",
    created: 1700000000,
    owned_by: "gaussmeridian",
    ...overrides,
  };
}

function rate(overrides: Partial<{ model_id: string }> = {}) {
  return {
    model_id: "openai/gpt-4o-mini",
    input_per_million_tokens: { minor_units: 3_000, currency: "idr" as const },
    output_per_million_tokens: { minor_units: 12_000, currency: "idr" as const },
    ...overrides,
  };
}

describe("buildModelCatalog", () => {
  it("attaches only the matching immutable IDR retail rate", () => {
    const [entry] = buildModelCatalog([model()], [rate()]);

    expect(entry.pricing).toEqual({
      inputPerMillion: 3_000,
      outputPerMillion: 12_000,
      currency: "idr",
    });
  });

  it("leaves retail pricing unpublished when no versioned rate matches", () => {
    const [entry] = buildModelCatalog([model()], [rate({ model_id: "other-model" })]);

    expect(entry.pricing).toBeNull();
  });

  it("never converts supplier-valued USD micros into customer retail pricing", () => {
    const [entry] = buildModelCatalog(
      [model()],
      [
        {
          ...rate(),
          input_per_million_tokens: { minor_units: 150_000, currency: "usd_micros" },
          output_per_million_tokens: { minor_units: 600_000, currency: "usd_micros" },
        },
      ],
    );

    expect(entry.pricing).toBeNull();
  });

  it("retains the GaussMeridian catalog owner without deriving supplier claims", () => {
    const [entry] = buildModelCatalog([model()], []);

    expect(entry.provider).toBe("gaussmeridian");
    expect(entry.tier).toBeNull();
  });
});
