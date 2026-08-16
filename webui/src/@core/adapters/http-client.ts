import type { ZodType } from "zod";

export class HttpError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "HttpError";
  }
}

export interface HttpClientOptions {
  baseUrl: string;
  /** Supplies the current session token for authenticated requests, if any. */
  getAuthToken?: () => string | null | undefined;
}

/**
 * Minimal fetch wrapper. Every response is parsed through a Zod schema before it is
 * returned, so nothing that crossed the network is trusted by type alone.
 */
export function createHttpClient({ baseUrl, getAuthToken }: HttpClientOptions) {
  async function request<T>(
    path: string,
    init: Omit<RequestInit, "headers"> & {
      schema: ZodType<T>;
      headers?: Record<string, string>;
    },
  ): Promise<T> {
    const { schema, headers, ...rest } = init;
    const token = getAuthToken?.();
    const res = await fetch(`${baseUrl}${path}`, {
      ...rest,
      headers: {
        "content-type": "application/json",
        ...(token ? { authorization: `Bearer ${token}` } : {}),
        ...headers,
      },
    });
    if (!res.ok) {
      throw new HttpError(res.status, `Request to ${path} failed with ${res.status}`);
    }
    return schema.parse(await res.json());
  }

  /** Issue a request and return the raw response body stream (for token streaming). */
  async function stream(
    path: string,
    init: Omit<RequestInit, "headers"> & { headers?: Record<string, string> },
  ): Promise<ReadableStream<Uint8Array>> {
    const { headers, ...rest } = init;
    const token = getAuthToken?.();
    const res = await fetch(`${baseUrl}${path}`, {
      ...rest,
      headers: {
        "content-type": "application/json",
        ...(token ? { authorization: `Bearer ${token}` } : {}),
        ...headers,
      },
    });
    if (!res.ok || !res.body) {
      throw new HttpError(res.status, `Stream from ${path} failed with ${res.status}`);
    }
    return res.body;
  }

  return { request, stream };
}

export type HttpClient = ReturnType<typeof createHttpClient>;
