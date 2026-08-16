import type { DataQueryInput } from "@core/adapters";

type ResourceEntry = unknown | ((input: DataQueryInput<unknown>) => unknown);

/**
 * Builds a `data.query` implementation from a `resource -> fixture` map, for surface tests
 * that need several distinct resources served by one fake registry:
 *
 *   createFakeRegistry({ data: { query: byResource({ "v1/orgs": { orgs: [...] } }) } })
 *
 * An entry may be a static fixture or a function of the full query input (for mutation
 * echoes, or a fixture that varies by params/body). Always validates through the caller's
 * schema, same parity contract as the real and mock adapters — a test fixture that doesn't
 * match its schema fails the same way a bad response would.
 */
export function byResource(map: Record<string, ResourceEntry>) {
  return async function query<T>(input: DataQueryInput<T>): Promise<T> {
    if (!(input.resource in map)) {
      throw new Error(`byResource: no fixture registered for resource "${input.resource}"`);
    }
    const entry = map[input.resource];
    const value =
      typeof entry === "function"
        ? (entry as (i: DataQueryInput<unknown>) => unknown)(input)
        : entry;
    return input.schema.parse(value);
  };
}
