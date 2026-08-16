/**
 * Runtime configuration for number formatting (consumed by formatNumber.ts).
 *
 * Values resolve via getRuntimeValue, which prefers a runtime-injected `window.__RUNTIME_CONFIG`
 * (so a container can override without a rebuild) and falls back to build-time `process.env`.
 *
 * Env contract:
 *   - NEXT_PUBLIC_ROUND_DECIMALS       decimals used when not rounding to int (default 2)
 *   - NEXT_PUBLIC_ROUND_NUMBER_APP     "true" => round to integer app-wide (default false)
 *   - NEXT_PUBLIC_THOUSANDS_SEPARATOR  digit-group separator, space or comma (default space)
 */
declare global {
  interface Window {
    __RUNTIME_CONFIG?: Record<string, string | undefined>;
  }
}

/**
 * Read an env value, preferring a runtime-injected `window.__RUNTIME_CONFIG` over build-time
 * `process.env`. Returns "" when the key is unset. Note: Next only inlines `process.env` for
 * statically-referenced `NEXT_PUBLIC_*` keys, so dynamic client-side lookups rely on the runtime
 * config; server-side reads see the full `process.env`.
 */
export function getRuntimeValue(key: string): string {
  if (typeof window !== "undefined") {
    const runtime = window.__RUNTIME_CONFIG?.[key];
    if (runtime != null && runtime !== "") return runtime;
  }
  return process.env[key] ?? "";
}

const rawDecimals = getRuntimeValue("NEXT_PUBLIC_ROUND_DECIMALS");
const parsedDecimals = Number(rawDecimals);

/** Decimal places used when {@link FORMATTING_ROUND_APP} is false. Default 2. */
export const FORMATTING_DECIMALS: number =
  rawDecimals !== "" && Number.isFinite(parsedDecimals) && parsedDecimals >= 0 ? parsedDecimals : 2;

/** When true, numbers are rounded to integers app-wide. Default false. */
export const FORMATTING_ROUND_APP: boolean =
  getRuntimeValue("NEXT_PUBLIC_ROUND_NUMBER_APP").toLowerCase() === "true";

/** Thousands separator for digit grouping (space or comma). Default space. */
export const FORMATTING_SEPARATOR: string =
  getRuntimeValue("NEXT_PUBLIC_THOUSANDS_SEPARATOR") || " ";
