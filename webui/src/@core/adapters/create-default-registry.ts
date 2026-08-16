import { env } from "@core/lib/env";

import { createConsoleOrgDataAdapter } from "./console-org.adapter";
import { createTransparencyDataAdapter } from "./console-transparency.adapter";
import { createGaussMeridianAuthAdapter } from "./gaussmeridian-auth.adapter";
import { createGaussMeridianDataAdapter } from "./gaussmeridian-data.adapter";
import { createHttpLlmByokAdapter } from "./llm-byok.adapter";
import { createMockRegistry } from "./mock/create-mock-registry";

import type { AdapterRegistry } from "./types";

/**
 * Builds the default HTTP-backed adapter registry from environment config.
 * Fork this to target a specific Gaussian backend or to swap implementations.
 *
 * `NEXT_PUBLIC_USE_MOCKS=1` short-circuits to the in-memory Phase-1 console registry —
 * the console's surfaces (Org/Project/Member/RouteDecision) are built against fixtures
 * until the Phase-2 SurrealDB backend lands. This is the ONLY production file that branches
 * on that flag; every other seam is unaware mocks exist.
 *
 * `data` chains two resource-aware decorators around the plain HTTP adapter:
 * `createConsoleOrgDataAdapter` intercepts the Wave-2 org/project/member/role resources and runs
 * them through the real-DTO anti-corruption mapping (`console.mapper.ts`); `
 * createTransparencyDataAdapter` intercepts the PRD-21 Wave C route-decision/savings resources
 * and rewrites them onto the real, non-project-parameterized backend paths
 * (`console-transparency.adapter.ts`). Every other resource passes straight through unchanged.
 *
 * `llm: createHttpLlmByokAdapter()` streams chat completions through the same same-origin proxy
 * every other resource uses (`/api/gaussmeridian/...`) — no separate base URL, no client-held
 * session token; auth is resolved server-side from the caller's cookie exactly like every other
 * authenticated request (see `llm-byok.adapter.ts`'s doc comment).
 *
 * Wave-2 live cutover: the real backend is now the default (`NEXT_PUBLIC_USE_MOCKS` unset or
 * `"0"`). Mocks are opt-in only, for dev-offline work — set `NEXT_PUBLIC_USE_MOCKS=1`
 * explicitly. `src/middleware.ts`'s dev mock-bypass block has been removed accordingly: the
 * session-cookie guard now applies unconditionally to every guarded route.
 */
export function createDefaultRegistry(): AdapterRegistry {
  if (env.NEXT_PUBLIC_USE_MOCKS === "1") return createMockRegistry();

  return {
    llm: createHttpLlmByokAdapter(),
    data: createConsoleOrgDataAdapter(
      createTransparencyDataAdapter(createGaussMeridianDataAdapter()),
    ),
    auth: createGaussMeridianAuthAdapter(),
  };
}
