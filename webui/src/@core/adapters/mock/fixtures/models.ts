import type { ModelsResponseSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const models: z.infer<typeof ModelsResponseSchema> = {
  data: [
    { id: "gpt-4o-mini", object: "model", created: 1700000000, owned_by: "openai" },
    { id: "gpt-4o", object: "model", created: 1700000400, owned_by: "openai" },
    { id: "claude-3-5-sonnet", object: "model", created: 1700900000, owned_by: "anthropic" },
  ],
};
