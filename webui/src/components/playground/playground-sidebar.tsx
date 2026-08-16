"use client";

import { Plus } from "lucide-react";

import { cn } from "@core/lib/utils";

import { Button } from "@/components/ui/button";
import type { ChatThread } from "@/hooks/useChat";

interface PlaygroundSidebarProps {
  threads: ChatThread[];
  activeThreadId: string;
  onSelectThread: (id: string) => void;
  onNewChat: () => void;
}

/** Conversation-history rail: "+ New chat" plus every thread from this session, newest first. */
export function PlaygroundSidebar({
  threads,
  activeThreadId,
  onSelectThread,
  onNewChat,
}: PlaygroundSidebarProps) {
  return (
    <aside className="border-border bg-card hidden w-64 shrink-0 flex-col border-r md:flex">
      <div className="border-border border-b p-3">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full justify-start gap-2"
          onClick={onNewChat}
          aria-label="Start a new chat"
        >
          <Plus className="h-4 w-4" aria-hidden="true" />
          New chat
        </Button>
      </div>
      <nav
        className="flex flex-1 flex-col gap-0.5 overflow-y-auto p-2"
        aria-label="Conversation history"
      >
        {threads.map((thread) => {
          const active = thread.id === activeThreadId;
          return (
            <button
              key={thread.id}
              type="button"
              onClick={() => onSelectThread(thread.id)}
              aria-current={active ? "true" : undefined}
              className={cn(
                "truncate rounded-md px-3 py-2 text-left text-sm transition-colors",
                active
                  ? "bg-secondary text-secondary-foreground"
                  : "text-muted-foreground hover:bg-secondary/50 hover:text-foreground",
              )}
            >
              {thread.title}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
