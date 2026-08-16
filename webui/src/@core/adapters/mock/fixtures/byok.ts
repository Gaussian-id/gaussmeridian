import type { ByokProvidersSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const byokProviders: z.infer<typeof ByokProvidersSchema> = {
  providers: ["openai", "anthropic"],
};
