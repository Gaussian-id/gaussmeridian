"use client";

import { ChatBubble } from "@/components/chat";
import { Badge } from "@/components/ui/badge";
import type { UiChatMessage } from "@/hooks/useChat";

interface PlaygroundTurnProps {
  message: UiChatMessage;
}

/** One factual Playground turn. The selected GaussMeridian model is shown only after the stream
 * settles; pending and failed attempts never masquerade as completed model output. */
export function PlaygroundTurn({ message }: PlaygroundTurnProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <ChatBubble message={message} />
      {message.role === "assistant" && message.route && (
        <div className="flex justify-start">
          {message.deliveryState === "settled" ? (
            <Badge variant="mono" className="text-[10px]">
              GaussMeridian · {message.route.model}
            </Badge>
          ) : message.deliveryState === "pending" ? (
            <Badge variant="outline" role="status" aria-live="polite">
              GaussMeridian is responding…
            </Badge>
          ) : (
            <Badge variant="outline" className="text-destructive">
              Request failed
            </Badge>
          )}
        </div>
      )}
    </div>
  );
}
