"use client";

import { useEffect, useRef } from "react";

import type { ChatThread } from "@/hooks/useChat";

import { PlaygroundTurn } from "./playground-turn";

interface PlaygroundThreadViewProps {
  thread: ChatThread;
}

/** Centered scrollable thread — the ChatGPT-shaped middle column. Auto-scrolls to the newest
 *  content as the assistant streams. */
export function PlaygroundThreadView({ thread }: PlaygroundThreadViewProps) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const lastMessage = thread.messages.at(-1);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ block: "end" });
  }, [thread.messages.length, lastMessage?.content]);

  if (thread.messages.length === 0) {
    return (
      <div className="text-muted-foreground flex flex-1 flex-col items-center justify-center gap-2 px-6 text-center">
        <p className="font-display text-foreground text-xl font-semibold">
          Ask GaussMeridian anything
        </p>
        <p className="max-w-sm text-sm">
          Send a text prompt to an enabled model. Settled usage is recorded against this project.
        </p>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto px-6 py-6">
      <div className="mx-auto flex w-full max-w-3xl flex-col gap-5">
        {thread.messages.map((message) => (
          <PlaygroundTurn key={message.id} message={message} />
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
