import {
  FORMATTING_DECIMALS,
  FORMATTING_ROUND_APP,
  FORMATTING_SEPARATOR,
  getRuntimeValue,
} from "./numberFormattingConfig";

function addThousandsSeparator(intStr: string): string {
  return intStr.replace(/\B(?=(\d{3})+(?!\d))/g, FORMATTING_SEPARATOR);
}

function formatWithDecimals(value: number): string {
  if (value === 0) return "0";

  const factor = Math.pow(10, FORMATTING_DECIMALS);
  const rounded = Math.round(value * factor) / factor;

  const str = rounded.toString();
  const parts = str.split(".");

  const integerPart = addThousandsSeparator(parts[0]);

  if (parts.length === 1) return integerPart;

  const decimals = parts[1].substring(0, FORMATTING_DECIMALS).replace(/0+$/, "");

  return decimals ? `${integerPart}.${decimals}` : integerPart;
}

function formatAsInt(value: number): string {
  if (value === 0) return "0";

  const rounded = Math.round(value);

  return addThousandsSeparator(rounded.toString());
}

function formatAsRoundUp(value: number): string {
  if (value === 0) return "0";

  const rounded = Math.ceil(value);

  return addThousandsSeparator(rounded.toString());
}

function formatAsRoundHalfDown(value: number): string {
  if (value === 0) return "0";

  const rounded = Math.ceil(value - 0.5);

  return addThousandsSeparator(rounded.toString());
}

/**
 * * Format number for UI display based on env configuration
 * When NEXT_PUBLIC_ROUND_NUMBER_APP=true, rounds to integer
 * When NEXT_PUBLIC_ROUND_NUMBER_APP=false, rounds to NEXT_PUBLIC_ROUND_DECIMALS decimal places
 * Uses NEXT_PUBLIC_THOUSANDS_SEPARATOR for digit grouping (space or comma)
 */
export function formatToRoundInt(value: number): string {
  return FORMATTING_ROUND_APP ? formatAsInt(value) : formatWithDecimals(value);
}

/**
 * * Always rounds to integer regardless of env configuration
 * Still applies the configured thousands separator (NEXT_PUBLIC_THOUSANDS_SEPARATOR)
 * Use this when the feature requires integer display unconditionally
 */
export function formatToAlwaysRoundInt(value: number): string {
  return formatAsInt(value);
}

/**
 * * Always rounds up (ceiling) regardless of decimal value
 * e.g. 1.1 → 2, 1.9 → 2, 1.0 → 1
 * Still applies the configured thousands separator
 */
export function formatToAlwaysRoundUp(value: number): string {
  return formatAsRoundUp(value);
}

/**
 * * Rounds with "half down" behavior: exactly 0.5 rounds down, above 0.5 rounds up
 * e.g. 1.5 → 1, 1.6 → 2, 1.4 → 1
 * Still applies the configured thousands separator
 */
export function formatToAlwaysRoundHalfDown(value: number): string {
  return formatAsRoundHalfDown(value);
}

function formatAsRaw(value: number): string {
  if (value === 0) return "0";

  const parts = value.toString().split(".");
  const integerPart = addThousandsSeparator(parts[0]);

  return parts.length === 1 ? integerPart : `${integerPart}.${parts[1]}`;
}

// ─── Strategy Pattern ──────────────────────────────────────────────────────────

export type NumberFormatType = "env-default" | "always-int" | "round-up" | "half-down" | "raw";

const VALID_FORMAT_TYPES = new Set<string>([
  "env-default",
  "always-int",
  "round-up",
  "half-down",
  "raw",
]);

/**
 * * Resolve a full per-page row→format map from a single JSON env variable
 * Reads from window.__RUNTIME_CONFIG or process.env.
 * Env value is a JSON object mapping row keys to NumberFormatType.
 * Invalid keys or values are silently ignored — defaults take over.
 *
 * Env naming: NEXT_PUBLIC_FORMAT_{{MODULE}}
 *
 * .env:
 * ```
 * NEXT_PUBLIC_FORMAT_PS={"In-Batch":"round-up","Primary UOM":"raw"}
 * ```
 *
 * Usage in component:
 * ```ts
 * const PS_FORMAT = resolvePageFormatMap('NEXT_PUBLIC_FORMAT_PS', {
 *   'In-Batch': 'always-int',
 *   'Primary UOM': 'env-default',
 * })
 *
 * formatByType(value, PS_FORMAT[row.type] ?? 'env-default')
 * ```
 */
export function resolvePageFormatMap<TRowName extends string>(
  envKey: string,
  defaults: Record<TRowName, NumberFormatType>,
): Record<TRowName, NumberFormatType> {
  const raw = getRuntimeValue(envKey);

  if (!raw) return defaults;

  try {
    const parsed = JSON.parse(raw) as Record<string, string>;
    const result = { ...defaults };

    for (const [key, value] of Object.entries(parsed)) {
      if (key in defaults && VALID_FORMAT_TYPES.has(value)) {
        result[key as TRowName] = value as NumberFormatType;
      }
    }

    return result;
  } catch {
    return defaults;
  }
}

/**
 * * Dispatch formatter based on a format type strategy
 * Use this when different rows/pages require different number formatting behaviors.
 *
 * | Type          | Behavior                                              |
 * | ------------- | ----------------------------------------------------- |
 * | 'env-default' | Respects NEXT_PUBLIC_ROUND_NUMBER_APP env config      |
 * | 'always-int'  | Always rounds to nearest integer (standard rounding)  |
 * | 'round-up'    | Always ceiling (1.1 → 2)                              |
 * | 'half-down'   | 0.5 rounds down, above 0.5 rounds up (1.5 → 1)       |
 * | 'raw'         | No rounding — only applies thousands separator        |
 */
export function formatByType(value: number, type: NumberFormatType): string {
  switch (type) {
    case "env-default":
      return formatToRoundInt(value);
    case "always-int":
      return formatAsInt(value);
    case "round-up":
      return formatAsRoundUp(value);
    case "half-down":
      return formatAsRoundHalfDown(value);
    case "raw":
      return formatAsRaw(value);
  }
}

/**
 * * Create a portable row formatter bound to a resolved page format map
 * Returns a function (value, rowName) => string that any expanded-rows component can use.
 *
 * Usage:
 * ```ts
 * const formatRow = createRowFormatter('NEXT_PUBLIC_FORMAT_INT_DASHBOARD', { ... defaults })
 * formatRow(1234.5, 'Tambah Kurang Plan Order Plan Purch in Batch') // → "1 235"
 * ```
 */
export function createRowFormatter<TRowName extends string>(
  envKey: string,
  defaults: Record<TRowName, NumberFormatType>,
): (value: number, rowName: TRowName) => string {
  const formatMap = resolvePageFormatMap(envKey, defaults);

  return (value: number, rowName: TRowName) =>
    formatByType(value, formatMap[rowName] ?? "env-default");
}

/**
 * * Create a portable page-level formatter with a single format type for all values
 * Use this when a page has no row-type discriminator (e.g. all rows display the same data kind).
 *
 * Usage:
 * ```ts
 * const formatValue = createPageFormatter('NEXT_PUBLIC_FORMAT_MPS_FG_DOI', 'env-default')
 * formatValue(1234.5) // → "1 235"
 * ```
 *
 * .env override:
 * ```
 * NEXT_PUBLIC_FORMAT_MPS_FG_DOI=round-up
 * ```
 */
export function createPageFormatter(
  envKey: string,
  fallback: NumberFormatType,
): (value: number) => string {
  const raw = getRuntimeValue(envKey);
  const type: NumberFormatType = VALID_FORMAT_TYPES.has(raw) ? (raw as NumberFormatType) : fallback;

  return (value: number) => formatByType(value, type);
}
