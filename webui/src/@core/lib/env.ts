import { z } from "zod";

/**
 * Public runtime configuration, validated at startup. Only NEXT_PUBLIC_* vars belong here —
 * the front-end is a client and must never hold server secrets.
 */
const envSchema = z.object({
  NEXT_PUBLIC_API_BASE_URL: z.url().default("http://localhost:8000"),
  /** "1" serves the console from in-memory fixtures instead of a live backend (Phase 1). */
  NEXT_PUBLIC_USE_MOCKS: z.enum(["0", "1"]).optional(),
});

export const env = envSchema.parse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
  NEXT_PUBLIC_USE_MOCKS: process.env.NEXT_PUBLIC_USE_MOCKS,
});

export type Env = typeof env;

/**
 * Server-side resolver for the GaussMeridian base URL. `NEXT_PUBLIC_*` values are inlined
 * into the bundles at BUILD time, so a container image cannot override them at run time.
 * Server code (the /api/gaussmeridian proxy and the auth Route Handlers) reads the
 * runtime-only `GAUSSMERIDIAN_API_URL` first, falling back to the build-time value.
 * Never import this from client code — it would always take the fallback.
 */
export function apiBaseUrl(): string {
  return process.env.GAUSSMERIDIAN_API_URL || env.NEXT_PUBLIC_API_BASE_URL;
}
