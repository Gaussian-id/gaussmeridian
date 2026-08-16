"use client";

import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import type { FormEvent } from "react";

/**
 * Mirrors the backend's `BYOK_PROVIDERS` allowlist (handlers.rs) exactly — the backend
 * rejects any provider outside this set with a 400, so the form must not offer more.
 */
export const BYOK_PROVIDERS = [
  "openai",
  "anthropic",
  "google",
  "mistral",
  "cohere",
  "ollama",
] as const;

export type ByokProvider = (typeof BYOK_PROVIDERS)[number];

const PROVIDER_LABELS: Record<ByokProvider, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  google: "Google",
  mistral: "Mistral",
  cohere: "Cohere",
  ollama: "Ollama",
};

export interface ByokProviderFormValues {
  provider: ByokProvider;
  apiKey: string;
}

interface ByokProviderFormProps {
  /** Disables every control (e.g. while the vault is unavailable). */
  disabled?: boolean;
  /** True while a registration is in flight — disables submit and shows progress text. */
  isPending?: boolean;
  onSubmit?: (values: ByokProviderFormValues) => void;
}

/**
 * Credential form for GaussMeridian's BYOK Manager — NOT the boilerplate's built-in
 * AI-assistant chat widget (`llm-byok.adapter.ts` / `useRegisterKey`), which is an unrelated
 * feature that happens to share the "BYOK" name. Submits `{provider, apiKey}` matching the
 * backend's `RegisterByokKeyRequest`; the key is sent once and never echoed back.
 */
export function ByokProviderForm({
  disabled = false,
  isPending = false,
  onSubmit,
}: ByokProviderFormProps) {
  const [provider, setProvider] = useState<ByokProvider>("openai");
  const [apiKey, setApiKey] = useState("");

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!apiKey.trim()) return;
    onSubmit?.({ provider, apiKey: apiKey.trim() });
    setApiKey(""); // the secret has left the form — don't keep it around in state/DOM
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="byok-provider">Provider</Label>
        <Select
          value={provider}
          onValueChange={(value) => setProvider(value as ByokProvider)}
          disabled={disabled}
        >
          <SelectTrigger id="byok-provider" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {BYOK_PROVIDERS.map((value) => (
              <SelectItem key={value} value={value}>
                {PROVIDER_LABELS[value]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex flex-col gap-1.5">
        <Label htmlFor="byok-api-key">API key</Label>
        <Input
          id="byok-api-key"
          type="password"
          autoComplete="off"
          placeholder="sk-..."
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          disabled={disabled}
        />
      </div>

      <Button
        type="submit"
        variant="accent"
        disabled={disabled || isPending || !apiKey.trim()}
        className="self-start"
      >
        {isPending ? "Saving…" : "Save credentials"}
      </Button>
    </form>
  );
}
