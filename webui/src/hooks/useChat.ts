"use client";

import { useCallback, useState } from "react";

import { useLlm, type ChatMessage } from "@core/adapters";

/** The enabled model requested by this project-scoped Playground turn. Supplier identity and
 *  unsupported native-routing evidence are intentionally not browser-facing bridge concepts. */
export interface ChatRoute {
  model: string;
}

export interface UiChatMessage extends ChatMessage {
  id: string;
  /** Only ever set on assistant turns. */
  route?: ChatRoute;
  deliveryState?: "pending" | "settled" | "failed";
  recovery?: "add-credit";
}

export interface ChatThread {
  id: string;
  title: string;
  messages: UiChatMessage[];
}

const TITLE_MAX_LENGTH = 42;

/**
 * Turns a chat-stream failure into an honest, specific message instead of a blanket fallback.
 * The backend has no client-facing "session" concept for chat (see `llm-byok.adapter.ts`'s doc
 * comment) — the failure modes that actually happen here are an unfunded/unreachable provider
 * (this session's known case: Anthropic has a zero balance, only `gpt-4o-mini` is live) or an
 * expired login. Anything else surfaces the backend's own error text verbatim — never a generic
 * "something went wrong" that could be masking a real, checkable cause.
 */
function describeChatError(error: unknown): Pick<UiChatMessage, "content" | "recovery"> {
  const message = error instanceof Error ? error.message : String(error);
  const lower = message.toLowerCase();
  if (lower.includes("add prepaid credit") || lower.includes("payment_required")) {
    return {
      content:
        "This organization needs prepaid credit before Meridian can run model requests. Add credit in Billing, then try again.",
      recovery: "add-credit",
    };
  }
  if (
    lower.includes("balance") ||
    lower.includes("insufficient") ||
    lower.includes("payment") ||
    lower.includes("budget") ||
    lower.includes("quota") ||
    lower.includes("402")
  ) {
    return {
      content:
        "This project does not have enough GaussMeridian credit for this request. Review its spend controls or organization Billing, then try again.",
    };
  }
  if (lower.includes("unauthorized") || lower.includes("401")) {
    return { content: "Your session expired — sign in again to continue chatting." };
  }
  if (lower.includes("model") && (lower.includes("not found") || lower.includes("not enabled"))) {
    return {
      content:
        "This model is not available in GaussMeridian for this project. Choose another model and try again.",
    };
  }
  return { content: "GaussMeridian could not complete this request. Try again shortly." };
}

function createThread(): ChatThread {
  return { id: crypto.randomUUID(), title: "New chat", messages: [] };
}

function titleFrom(text: string): string {
  const trimmed = text.trim();
  return trimmed.length > TITLE_MAX_LENGTH ? `${trimmed.slice(0, TITLE_MAX_LENGTH)}…` : trimmed;
}

/**
 * Streaming, multi-thread chat state over the real `POST /v1/chat/completions/stream` endpoint
 * (`llm.streamChat` — see `llm-byok.adapter.ts`). Auth is resolved server-side from the caller's
 * session, same as every other request; there is no client-held session token. A model whose
 * provider isn't funded (this session: everything except `gpt-4o-mini`, since Anthropic has a
 * zero balance) fails honestly via `describeChatError` rather than a generic fallback message.
 *
 * Threaded so the Playground's conversation-history rail can hold more than one conversation
 * and switch between them with "+ New chat" — a single flat message array can't represent that,
 * so state is keyed by thread and only the active one streams at a time.
 */
export function useChat(projectId: string) {
  const llm = useLlm();
  const [threads, setThreads] = useState<ChatThread[]>(() => [createThread()]);
  const [activeThreadId, setActiveThreadId] = useState<string>(() => threads[0].id);
  const [isStreaming, setIsStreaming] = useState(false);

  const activeThread = threads.find((thread) => thread.id === activeThreadId) ?? threads[0];

  const newChat = useCallback(() => {
    const thread = createThread();
    setThreads((current) => [thread, ...current]);
    setActiveThreadId(thread.id);
  }, []);

  const selectThread = useCallback((id: string) => setActiveThreadId(id), []);

  const send = useCallback(
    async (text: string, route: ChatRoute) => {
      const threadId = activeThreadId;
      const userMessage: UiChatMessage = { id: crypto.randomUUID(), role: "user", content: text };
      const assistantId = crypto.randomUUID();
      const priorMessages = threads.find((thread) => thread.id === threadId)?.messages ?? [];

      setThreads((current) =>
        current.map((thread) =>
          thread.id === threadId
            ? {
                ...thread,
                title: thread.messages.length === 0 ? titleFrom(text) : thread.title,
                messages: [
                  ...thread.messages,
                  userMessage,
                  {
                    id: assistantId,
                    role: "assistant",
                    content: "",
                    route,
                    deliveryState: "pending",
                  },
                ],
              }
            : thread,
        ),
      );
      setIsStreaming(true);

      function patchAssistant(updater: (message: UiChatMessage) => UiChatMessage) {
        setThreads((current) =>
          current.map((thread) =>
            thread.id === threadId
              ? {
                  ...thread,
                  messages: thread.messages.map((message) =>
                    message.id === assistantId ? updater(message) : message,
                  ),
                }
              : thread,
          ),
        );
      }

      try {
        const history = [...priorMessages, userMessage].map(({ role, content }) => ({
          role,
          content,
        }));
        const stream = llm.streamChat({ projectId, model: route.model, messages: history });
        let receivedContent = false;
        for await (const chunk of stream) {
          receivedContent = true;
          patchAssistant((message) => ({ ...message, content: message.content + chunk }));
        }
        if (!receivedContent) throw new Error("empty completion response");
        patchAssistant((message) => ({ ...message, deliveryState: "settled" }));
      } catch (error) {
        const failure = describeChatError(error);
        patchAssistant((message) => ({
          ...message,
          ...failure,
          deliveryState: "failed",
        }));
      } finally {
        setIsStreaming(false);
      }
    },
    [activeThreadId, threads, llm, projectId],
  );

  return { threads, activeThread, isStreaming, send, newChat, selectThread };
}
