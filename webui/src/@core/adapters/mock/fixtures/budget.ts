import type { BudgetStatusSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const budget: z.infer<typeof BudgetStatusSchema> = {
  budget_limit: 500,
  current_usage: 128.44,
  remaining: 371.56,
  usage_percentage: 25.7,
  alert_threshold: 80,
  is_over_budget: false,
  period: "monthly",
  currency: "USD",
};
