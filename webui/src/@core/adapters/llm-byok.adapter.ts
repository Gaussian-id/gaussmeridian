import { z } from "zod";

import { GaussMeridianErrorSchema } from "./schemas/gaussmeridian-error.schema";

import type { LlmByokAdapter } from "./types";

const chatChunkSchema = z.object({
  choices: z
    .array(
      z.object({
        delta: z.object({ content: z.string().nullable().optional() }).nullable().optional(),
        finish_reason: z.string().nullable().optional(),
      }),
    )
    .default([]),
});

/** `handlers.rs::stream_chat_completions`'s mid-stream error frame — a bare `{"error": "..."}`
 *  object, NOT an OpenAI-shaped chunk. Sent when the underlying provider call fails after the
 *  stream has already started (e.g. the provider account has no funded balance), so it can't be
 *  surfaced as an HTTP status — the connection is already a 200 SSE stream by that point. */
const chatStreamErrorFrameSchema = z.object({ error: z.string() });
const SAFE_PROJECT_CONTEXT = /^[A-Za-z0-9][A-Za-z0-9._~:@+-]{0,255}$/;

/**
 * Parses the `text/event-stream` body of `POST /v1/chat/completions/stream` into plain text
 * chunks (OpenAI-shaped `ChatCompletionChunk.choices[0].delta.content`). A malformed or
 * unrecognized `data:` line is skipped, not thrown — a single bad frame can't kill an otherwise
 * healthy stream. A `{"error": "..."}` frame IS thrown, as an `Error` carrying the backend's own
 * message, so the caller (`useChat`) can show the real failure instead of going silently empty.
 */
async function* parseChatStream(body: ReadableStream<Uint8Array>): AsyncIterable<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const frames = buffer.split("\n\n");
      buffer = frames.pop() ?? "";
      for (const frame of frames) {
        const dataLine = frame.split("\n").find((line) => line.startsWith("data:"));
        if (!dataLine) continue;
        const payload = dataLine.slice("data:".length).trim();
        if (!payload || payload === "[DONE]") continue;

        let json: unknown;
        try {
          json = JSON.parse(payload);
        } catch {
          continue;
        }

        const errorFrame = chatStreamErrorFrameSchema.safeParse(json);
        if (errorFrame.success) throw new Error(errorFrame.data.error);

        const chunk = chatChunkSchema.safeParse(json);
        const content = chunk.success ? chunk.data.choices[0]?.delta?.content : undefined;
        if (content) yield content;
      }
    }
  } finally {
    reader.releaseLock();
  }
}

/**
 * HTTP reference implementation of chat streaming — goes through the same same-origin proxy
 * (`/api/gaussmeridian/...`) every other resource query uses, NOT a separate `baseUrl` fetch: the
 * real backend only ever reads the session cookie via our own Next.js server (see
 * `gaussmeridian-data.adapter.ts`'s doc comment). There is no client-held session token — auth is
 * resolved server-side from the forwarded cookie, exactly like every authenticated
 * `DataQueryAdapter` call.
 */
export function createHttpLlmByokAdapter(): LlmByokAdapter {
  return {
    streamChat: async function* ({ projectId, model, messages }) {
      if (!SAFE_PROJECT_CONTEXT.test(projectId)) {
        throw new Error("A valid project context is required to use the Playground.");
      }
      const res = await fetch("/api/gaussmeridian/v1/chat/completions/stream", {
        method: "POST",
        credentials: "include",
        headers: {
          "content-type": "application/json",
          "x-project-id": projectId,
        },
        body: JSON.stringify({ model, messages, stream: true }),
      });

      if (!res.ok || !res.body) {
        const contentType = res.headers.get("content-type") ?? "";
        if (contentType.includes("application/json")) {
          const parsedError = GaussMeridianErrorSchema.safeParse(await res.json().catch(() => null));
          if (parsedError.success) throw new Error(parsedError.data.error.message);
        }
        throw new Error(`Chat stream failed with status ${res.status}`);
      }

      yield* parseChatStream(res.body);
    },
  };
}
