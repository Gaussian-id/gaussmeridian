"use client";

import { Trash2 } from "lucide-react";
import { useState } from "react";


import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useByokProviders,
  useDeleteByokKey,
  useRegisterByokKey,
} from "@/hooks/useGaussmeridianQueries";

/**
 * The providers the backend will accept a key for. Mirrors `BYOK_PROVIDERS` in
 * `services/server/src/handlers.rs` — anything outside that set is rejected with a 400, so the
 * form offers a fixed choice rather than a free-text box the server would refuse.
 */
const BYOK_PROVIDERS = [
  { value: "openai", label: "OpenAI" },
  { value: "anthropic", label: "Anthropic" },
  { value: "google", label: "Google (Gemini)" },
  { value: "mistral", label: "Mistral" },
  { value: "cohere", label: "Cohere" },
  { value: "ollama", label: "Ollama" },
] as const;

/** Widened to `string` deliberately: the server may return a provider this build doesn't list. */
const LABEL = new Map<string, string>(BYOK_PROVIDERS.map((p) => [p.value, p.label]));

/**
 * Bring-your-own-key management for one project.
 *
 * The key is sent to the server exactly once and never comes back — `GET /v1/byok/keys` returns
 * provider *names* only, because the encrypted material is not allowed to cross the BFF. So this
 * screen can show you which providers are configured, and let you replace or remove one, but it
 * can never show you a key you already saved.
 *
 * Two server states are worth distinguishing to the reader:
 *   - **403** — this account is not on the BYOK allowlist (`BYOK_ADMIN_EMAILS`).
 *   - **503** — the server has no vault configured (`BYOK_MASTER_KEY` unset), so it cannot
 *     encrypt anything. On a self-hosted install this is the usual cause.
 */
export function ByokManager() {
  const providers = useByokProviders();
  const register = useRegisterByokKey();
  const remove = useDeleteByokKey();

  const [provider, setProvider] = useState<string>(BYOK_PROVIDERS[0].value);
  const [apiKey, setApiKey] = useState("");
  const [notice, setNotice] = useState<string | null>(null);

  const configured = providers.data?.providers ?? [];

  const describeFailure = (error: unknown): string => {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes("403")) {
      return "This account is not on the BYOK allowlist. Add it to BYOK_ADMIN_EMAILS on the server and restart.";
    }
    if (message.includes("503")) {
      return "The server has no key vault configured. Set BYOK_MASTER_KEY and restart the gateway.";
    }
    return message;
  };

  const onSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    setNotice(null);
    register.mutate(
      { provider, api_key: apiKey.trim() },
      {
        onSuccess: () => {
          setApiKey("");
          setNotice(`Stored your ${LABEL.get(provider) ?? provider} key.`);
        },
        onError: (error) => setNotice(describeFailure(error)),
      },
    );
  };

  return (
    <div className="flex flex-col gap-6">
      <Card className="flex flex-col gap-4 p-6">
        <div>
          <CardTitle className="text-base">Your provider keys</CardTitle>
          <CardDescription className="mt-1">
            Route with your own provider credentials instead of the gateway&apos;s. Keys are
            encrypted on the server and never sent back to the browser — you can see which
            providers are configured, replace a key, or remove one.
          </CardDescription>
        </div>

        {providers.isLoading ? (
          <Skeleton className="h-10 w-full" />
        ) : configured.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            No provider keys yet. Add one below and this project will route through it.
          </p>
        ) : (
          <ul className="flex flex-col gap-2">
            {configured.map((name) => (
              <li
                key={name}
                className="border-border flex items-center justify-between rounded-md border px-4 py-3"
              >
                <span className="flex items-center gap-3">
                  <span className="font-medium">{LABEL.get(name) ?? name}</span>
                  <Badge variant="mono">configured</Badge>
                </span>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={remove.isPending}
                  onClick={() => {
                    setNotice(null);
                    remove.mutate(
                      { provider: name },
                      {
                        onSuccess: () => setNotice(`Removed the ${LABEL.get(name) ?? name} key.`),
                        onError: (error) => setNotice(describeFailure(error)),
                      },
                    );
                  }}
                >
                  <Trash2 className="mr-1.5 h-4 w-4" aria-hidden="true" />
                  Remove
                </Button>
              </li>
            ))}
          </ul>
        )}
      </Card>

      <Card className="flex flex-col gap-4 p-6">
        <div>
          <CardTitle className="text-base">Add a provider key</CardTitle>
          <CardDescription className="mt-1">
            Saving a key for a provider you already configured replaces the old one.
          </CardDescription>
        </div>

        <form className="flex flex-col gap-4" onSubmit={onSubmit}>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="byok-provider">Provider</Label>
            <select
              id="byok-provider"
              value={provider}
              onChange={(event) => setProvider(event.target.value)}
              className="border-input bg-background focus-visible:ring-ring h-10 rounded-md border px-3 text-sm focus-visible:ring-2 focus-visible:outline-none"
            >
              {BYOK_PROVIDERS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          <div className="flex flex-col gap-1.5">
            <Label htmlFor="byok-key">API key</Label>
            <Input
              id="byok-key"
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder="Paste the provider key"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
            />
            <p className="text-muted-foreground text-xs">
              Sent once and stored encrypted. It is never shown again, here or anywhere else.
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Button type="submit" disabled={register.isPending || apiKey.trim().length === 0}>
              {register.isPending ? "Saving…" : "Save key"}
            </Button>
            {notice ? (
              <p role="status" className="text-muted-foreground text-sm">
                {notice}
              </p>
            ) : null}
          </div>
        </form>
      </Card>
    </div>
  );
}
