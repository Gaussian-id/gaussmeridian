import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { ModelCard } from "../model-card";

import type { ModelCatalogEntry } from "../model-catalog";

function entry(overrides: Partial<ModelCatalogEntry> = {}): ModelCatalogEntry {
  return {
    id: "openai/gpt-4o-mini",
    provider: "gaussmeridian",
    tier: null,
    pricing: null,
    ...overrides,
  };
}

describe("ModelCard", () => {
  it("renders the enabled model under GaussMeridian branding without a supplier label", () => {
    render(<ModelCard model={entry()} />);
    expect(screen.getByText("openai/gpt-4o-mini")).toBeInTheDocument();
    expect(screen.getByText("GaussMeridian model")).toBeInTheDocument();
    expect(screen.queryByText(/OpenRouter|provider/i)).not.toBeInTheDocument();
  });

  it("shows a truthful state when no immutable retail rate is published", () => {
    render(<ModelCard model={entry({ pricing: null })} />);
    expect(screen.getByText("Retail rate not published")).toBeInTheDocument();
  });

  it("renders integer IDR input and output rates per million tokens", () => {
    render(
      <ModelCard
        model={entry({
          pricing: { inputPerMillion: 3_000, outputPerMillion: 12_000, currency: "idr" },
        })}
      />,
    );
    expect(screen.getByText("Rp3.000")).toBeInTheDocument();
    expect(screen.getByText("Rp12.000")).toBeInTheDocument();
    expect(screen.getAllByText(/1M tokens/i)).toHaveLength(2);
  });
});
