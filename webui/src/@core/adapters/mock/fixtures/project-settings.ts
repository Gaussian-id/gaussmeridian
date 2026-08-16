import type { ProjectSettings } from "@core/adapters/schemas/gaussmeridian.schema";

export const projectSettings: ProjectSettings = {
  lambda: 0.5,
  quality_floor: 0.75,
  tau_moa: 0.62,
  budget_monthly: 500,
  hard_limit: false,
  alert_webhook_url: null,
  validator_type: "semantic_similarity",
};
