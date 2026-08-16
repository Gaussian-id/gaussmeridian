import type { HttpClient } from "./http-client";
import type { DataQueryAdapter } from "./types";

function toQueryString(params?: Record<string, string | number | boolean | undefined>): string {
  if (!params) return "";
  const entries = Object.entries(params).filter(([, value]) => value !== undefined);
  if (entries.length === 0) return "";
  const search = new URLSearchParams(entries.map(([key, value]) => [key, String(value)]));
  return `?${search.toString()}`;
}

/** HTTP reference implementation of the data/query adapter. */
export function createHttpDataQueryAdapter(http: HttpClient): DataQueryAdapter {
  return {
    query: ({ resource, params, schema }) =>
      http.request(`/${resource}${toQueryString(params)}`, {
        method: "GET",
        schema,
      }),
  };
}
