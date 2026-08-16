import type { ApiKeySchema } from "@core/adapters/schemas/gaussmeridian.schema";

import type { z } from "zod";

export const apiKeys: z.infer<typeof ApiKeySchema>[] = [
  {
    id: "key_1",
    key_hash: "hashed_abc123",
    key_prefix: "grk_live_ab12",
    user_id: "user_owner",
    tenant_id: "org_meridian",
    name: "Production key",
    rate_limit_per_minute: 600,
    rate_limit_per_day: 100000,
    created_at: "2026-03-02T00:00:00Z",
    expires_at: null,
    last_used_at: "2026-07-14T13:02:00Z",
    active: true,
  },
];
