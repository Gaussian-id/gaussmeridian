import type { BalanceInfoSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const balance: z.infer<typeof BalanceInfoSchema> = {
  balance: 842.17,
  currency: "USD",
  last_updated: "2026-07-15T00:00:00Z",
};
