"use client";

import { CheckIcon, CopyIcon } from "lucide-react";
import { useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useCreateProjectApiKey } from "@/hooks/useProjectApiKeys";

import { Prompt } from "./prompt";

interface OnboardingStepApiKeyProps {
  orgId: string;
  projectId: string;
  projectName: string;
  onNext: () => void;
}

/**
 * Bridge step 6 (O7, required) — generates the first API key, scoped to the project just created
 * (DR-012). Shown once with a "you won't see this again" warning (US O7) and a copy-to-clipboard
 * affordance — copy-once: the key is only ever fetched into `createApiKey.data` by the one
 * `handleCreateKey` mutation, never re-requested. `<Prompt>`'s single continue action swaps
 * meaning once the key exists: "Generate key" -> "I've saved it" (`onNext`). The generated-key
 * state requires a Copy attempt before the final acknowledgement can advance.
 */
export function OnboardingStepApiKey({
  orgId,
  projectId,
  projectName,
  onNext,
}: OnboardingStepApiKeyProps) {
  return (
    <OnboardingStepApiKeyForProject
      key={JSON.stringify([orgId, projectId])}
      orgId={orgId}
      projectId={projectId}
      projectName={projectName}
      onNext={onNext}
    />
  );
}

function OnboardingStepApiKeyForProject({
  orgId,
  projectId,
  projectName,
  onNext,
}: OnboardingStepApiKeyProps) {
  const createApiKey = useCreateProjectApiKey(orgId, projectId);
  const [copied, setCopied] = useState(false);
  const [acknowledged, setAcknowledged] = useState(false);
  const [manualCopyRequired, setManualCopyRequired] = useState(false);
  const keyValueRef = useRef<HTMLInputElement>(null);
  const hasKey = Boolean(createApiKey.data);

  function handleCreateKey() {
    createApiKey.mutate({ name: `${projectName} default key` });
  }

  async function handleCopy() {
    if (!createApiKey.data) return;
    setCopied(false);
    setManualCopyRequired(false);
    try {
      const clipboard = navigator.clipboard;
      if (!clipboard?.writeText) throw new Error("Clipboard API unavailable");
      await clipboard.writeText(createApiKey.data.api_key);
      setCopied(true);
    } catch {
      setManualCopyRequired(true);
      keyValueRef.current?.focus();
      keyValueRef.current?.select();
    } finally {
      setAcknowledged(true);
    }
  }

  return (
    <Prompt
      kicker="API key"
      title={hasKey ? "Here's your key." : "Generate your first API key"}
      description={
        hasKey
          ? "Copy it now — for your security, you won't be able to see it again."
          : `Every request to Meridian authenticates with this key, scoped to ${projectName}.`
      }
      onContinue={hasKey ? onNext : handleCreateKey}
      isBusy={createApiKey.isPending}
      continueDisabled={hasKey && !acknowledged}
      continueDisabledReason={hasKey && !acknowledged ? "Copy your key to continue." : undefined}
      continueLabel={
        hasKey ? "I've saved it" : createApiKey.isPending ? "Generating…" : "Generate key"
      }
      error={createApiKey.isError ? "Could not generate a key. Try again." : undefined}
    >
      {createApiKey.data && (
        <div className="border-accent/40 bg-secondary/40 flex flex-col gap-3 rounded-lg border p-4">
          <p role="alert" aria-live="polite" className="text-sm font-medium">
            Copy this key now — you won&apos;t be able to see it again.
          </p>
          <div className="flex items-center gap-2">
            <Input
              ref={keyValueRef}
              aria-label="API key value"
              data-sensitive="true"
              readOnly
              value={createApiKey.data.api_key}
              onFocus={(event) => event.currentTarget.select()}
              autoComplete="off"
              spellCheck={false}
              className="h-11 min-w-0 flex-1 font-mono text-sm"
            />
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="h-11 w-11"
              onClick={handleCopy}
              aria-label="Copy API key"
            >
              {copied ? (
                <CheckIcon className="h-4 w-4" aria-hidden="true" />
              ) : (
                <CopyIcon className="h-4 w-4" aria-hidden="true" />
              )}
            </Button>
          </div>
          {copied && (
            <p aria-live="polite" className="text-muted-foreground text-xs">
              Copied to clipboard.
            </p>
          )}
          {manualCopyRequired && (
            <p aria-live="polite" className="text-muted-foreground text-xs">
              Automatic copy was unavailable. The API key is selected—press Ctrl+C or Command+C to
              copy it.
            </p>
          )}
        </div>
      )}
    </Prompt>
  );
}
