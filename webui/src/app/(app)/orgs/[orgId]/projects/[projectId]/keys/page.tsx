"use client";

import { Check, Copy, KeyRound } from "lucide-react";
import { useParams } from "next/navigation";
import { useMemo, useState } from "react";

import { GaussMeridianAdapterError } from "@core/adapters/gaussmeridian-data.adapter";
import type { ProjectApiKeySchema } from "@core/adapters/schemas/gaussmeridian.schema";

import { createApiKeysColumns } from "@/components/dashboard/api-keys-columns";
import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { ConfirmDestructiveDialog } from "@/components/ui/confirm-destructive-dialog";
import { DataTable } from "@/components/ui/data-table";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  useCreateProjectApiKey,
  useProjectApiKeys,
  useRevokeProjectApiKey,
} from "@/hooks/useProjectApiKeys";

import type { FormEvent } from "react";
import type { z } from "zod";

type ApiKey = z.infer<typeof ProjectApiKeySchema>;

/** Human-readable message for revoke failures. Never surfaces the raw adapter error. */
function revokeErrorMessage(error: unknown): string {
  if (error instanceof GaussMeridianAdapterError && error.status === 404) {
    return "This key was already revoked or removed.";
  }
  return "Could not revoke this key. Try again.";
}

/**
 * Project-scoped API keys. A project has *settings* only, not membership (RBAC lives at the
 * org level — see `console.schema.ts`), so there's no per-key role assignment here.
 *
 * Scope today is represented by real `ApiKeySchema` fields only: rate limits and expiry,
 * shown per key in the "Scope" column (`api-keys-columns.tsx`). Fine-grained allow/deny IAM
 * (restrict a key to specific models, providers, or source IPs) has no backing contract yet —
 * that's flagged below as a deferred Phase-2 item rather than mocked up as a UI with nothing
 * behind it.
 */
export default function ApiKeysPage() {
  const { orgId, projectId } = useParams<{ orgId: string; projectId: string }>();
  const apiKeys = useProjectApiKeys(orgId, projectId);
  const createApiKey = useCreateProjectApiKey(orgId, projectId);
  const revokeApiKey = useRevokeProjectApiKey(orgId, projectId);

  const [isCreating, setIsCreating] = useState(false);
  const [name, setName] = useState("");
  const [copied, setCopied] = useState(false);
  const [revokeTarget, setRevokeTarget] = useState<ApiKey | null>(null);

  function handleCreate(event: FormEvent) {
    event.preventDefault();
    createApiKey.mutate({ name: name.trim() || undefined });
  }

  function handleDismissCreated() {
    createApiKey.reset();
    setName("");
    setIsCreating(false);
    setCopied(false);
  }

  async function handleCopy() {
    if (!createApiKey.data) return;
    await navigator.clipboard.writeText(createApiKey.data.api_key);
    setCopied(true);
  }

  function handleRevokeRequest(key: ApiKey) {
    revokeApiKey.reset();
    setRevokeTarget(key);
  }

  function handleConfirmRevoke() {
    if (!revokeTarget?.id) return;
    revokeApiKey.mutate(revokeTarget.id, { onSuccess: () => setRevokeTarget(null) });
  }

  const columns = useMemo(
    () =>
      createApiKeysColumns({
        onRevoke: handleRevokeRequest,
        pendingKeyId: revokeApiKey.isPending ? (revokeApiKey.variables ?? null) : null,
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- handleRevokeRequest is stable per render intent, deps below cover the values it closes over
    [revokeApiKey.isPending, revokeApiKey.variables],
  );

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <DashboardPageHeader
          eyebrow="Project"
          title="API keys"
          description="Manage the keys used to authenticate requests to this project."
        />
        {!createApiKey.data && !isCreating && (
          <Button type="button" onClick={() => setIsCreating(true)}>
            Create key
          </Button>
        )}
      </div>

      <Card className="border-border bg-secondary/20 p-4">
        <p className="text-muted-foreground text-xs leading-relaxed">
          <span className="text-foreground font-medium">Scope today:</span> rate limits and expiry,
          shown per key below. Fine-grained allow/deny IAM (restricting a key to specific models,
          providers, or source IPs) isn&apos;t backed by a live contract yet — deferred to Phase 2.
        </p>
      </Card>

      {isCreating && !createApiKey.data && (
        <Card className="p-4">
          <form onSubmit={handleCreate} className="flex flex-col gap-4 sm:flex-row sm:items-end">
            <div className="flex flex-1 flex-col gap-1.5">
              <Label htmlFor="api-key-name">Key name (optional)</Label>
              <Input
                id="api-key-name"
                placeholder="e.g. production server"
                value={name}
                onChange={(event) => setName(event.target.value)}
              />
            </div>
            <div className="flex gap-2">
              <Button type="submit" disabled={createApiKey.isPending}>
                {createApiKey.isPending ? "Generating…" : "Generate"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  setIsCreating(false);
                  setName("");
                }}
              >
                Cancel
              </Button>
            </div>
          </form>
          {createApiKey.isError && (
            <p role="alert" className="text-destructive mt-2 text-sm">
              Could not generate a key. Try again.
            </p>
          )}
        </Card>
      )}

      {createApiKey.data && (
        <Card className="border-accent/50 shadow-glow bg-secondary/30 p-4">
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-start gap-3">
              <KeyRound className="text-accent mt-0.5 h-5 w-5 shrink-0" aria-hidden="true" />
              <div>
                <CardTitle className="text-base">{createApiKey.data.message}</CardTitle>
                <CardDescription className="mt-1">
                  Copy this key now — for security, you won&apos;t be able to see it again after you
                  leave this page.
                </CardDescription>
              </div>
            </div>
            <Button type="button" variant="ghost" size="sm" onClick={handleDismissCreated}>
              Dismiss
            </Button>
          </div>
          <div className="mt-3 flex items-center gap-2">
            <code
              aria-label="New API key"
              className="bg-background border-border flex-1 truncate rounded-md border px-3 py-2 font-mono text-sm"
            >
              {createApiKey.data.api_key}
            </code>
            <Button
              type="button"
              variant="outline"
              size="icon"
              onClick={handleCopy}
              aria-label="Copy key to clipboard"
            >
              {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
            </Button>
          </div>
        </Card>
      )}

      <DataTable
        columns={columns}
        data={apiKeys.data ?? []}
        isLoading={apiKeys.isLoading}
        isError={apiKeys.isError}
        errorMessage="Could not load this project's API keys. Try again shortly."
        emptyMessage="No API keys yet. Create one to get started."
      />

      {/* No visible trigger here — "Revoke" lives per-row in the table above (see
       *  `api-keys-columns.tsx`), so this dialog is driven entirely by `revokeTarget`. */}
      <ConfirmDestructiveDialog
        open={revokeTarget !== null}
        onOpenChange={(open) => {
          if (!open) setRevokeTarget(null);
        }}
        title="Revoke API key"
        confirmLabel={revokeApiKey.isPending ? "Revoking…" : "Revoke key"}
        isBusy={revokeApiKey.isPending}
        error={revokeApiKey.isError ? revokeErrorMessage(revokeApiKey.error) : null}
        onConfirm={handleConfirmRevoke}
        description={
          <>
            Requests using{" "}
            <strong className="text-foreground">
              {revokeTarget?.name ?? revokeTarget?.key_prefix}
            </strong>{" "}
            will start failing immediately. This cannot be undone.
          </>
        }
      />
    </div>
  );
}
