"use client";

import { SendHorizontal } from "lucide-react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";

import type { FormEvent, KeyboardEvent } from "react";

interface PlaygroundComposerProps {
  onSend: (text: string) => void;
  disabled?: boolean;
}

/** Bottom composer: a textarea + Send, Enter-to-submit (Shift+Enter for a newline).
 *  Deliberately no attachment / file-upload control — the Playground is text-only. */
export function PlaygroundComposer({ onSend, disabled }: PlaygroundComposerProps) {
  const [value, setValue] = useState("");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const text = value.trim();
    if (!text) return;
    onSend(text);
    setValue("");
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
    }
  }

  return (
    <form onSubmit={handleSubmit} className="border-border border-t px-6 py-4">
      <div className="mx-auto flex w-full max-w-3xl items-end gap-2">
        <Textarea
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Message GaussMeridian…"
          aria-label="Message GaussMeridian"
          rows={1}
          className="min-h-11 flex-1 resize-none"
          disabled={disabled}
        />
        <Button
          type="submit"
          size="icon"
          variant="accent"
          disabled={disabled || !value.trim()}
          aria-label="Send"
        >
          <SendHorizontal className="h-4 w-4" />
        </Button>
      </div>
    </form>
  );
}
