import { describe, expect, it } from "vitest";

import { logsColumns } from "../logs-columns";

describe("logsColumns", () => {
  it("defines a column for every field the ledger-sourced logs table displays", () => {
    const keys = logsColumns.map((c) => ("accessorKey" in c ? c.accessorKey : c.id));
    expect(keys).toEqual(
      expect.arrayContaining([
        "created_at",
        "model",
        "provider",
        "r_binary",
        "validator_result",
        "tokens",
        "latency_ms",
        "cost_charged",
      ]),
    );
  });
});
