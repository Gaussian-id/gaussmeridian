import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { RecentRequestsCard } from "../recent-requests-card";

describe("RecentRequestsCard", () => {
  it("reads only the selected project's settled delivery ledger", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/projects/proj_27/request-logs": [
            {
              id: "ledger_27",
              model: "openai/gpt-4o-mini",
              provider: "openrouter",
              tokens_in: 12,
              tokens_out: 3,
              cost_charged: 0.000_02,
              r_binary: 1,
              complexity_score: 0,
              validator_result: "passed",
              retry_count: 0,
              latency_ms: 500,
              created_at: "2026-08-10T00:00:00Z",
            },
          ],
        }),
      },
    });

    renderWithProviders(<RecentRequestsCard projectId="proj_27" />, registry);

    await waitFor(() => expect(screen.getByText("openai/gpt-4o-mini")).toBeInTheDocument());
    expect(screen.queryByText(/GaussMoA|OutcomeGate|xRouter/i)).not.toBeInTheDocument();
  });
});
