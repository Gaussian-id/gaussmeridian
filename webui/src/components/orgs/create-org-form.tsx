"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCreateOrg } from "@/hooks/useConsoleQueries";

import type { FormEvent } from "react";

/**
 * Standalone create-org form — the same field as `OnboardingStepOrg`'s first step, but
 * functional here: it calls `useCreateOrg` and lands the owner inside their new organization.
 * The bounded add-credit journey can instead continue to that organization's Billing page. New
 * organizations remain empty — no default project is created alongside them.
 *
 * DEFERRED (do not add without a fresh design pass): an editable `slug` field. The org
 * identifier scheme (user-chosen slug vs. a system-generated opaque id, Supabase-style) was
 * reopened as a design question — Shelby flagged user-determined slugs as a future risk
 * (collision/enumeration/rename surface). Until that's resolved, this form keeps collecting
 * only `name`; `console-org.adapter.ts` still derives the slug server-side via `slugify()`.
 */
export function CreateOrgForm({
  completion = "organization",
}: {
  completion?: "organization" | "billing";
}) {
  const router = useRouter();
  const createOrg = useCreateOrg();
  const [name, setName] = useState("");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed) return;
    createOrg.mutate(
      { name: trimmed },
      {
        onSuccess: (org) =>
          router.push(`/orgs/${org.id}`),
      },
    );
  }

  return (
    <Card className="mx-auto w-full max-w-md p-6">
      <form onSubmit={handleSubmit} className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="org-name">Organization name</Label>
          <Input
            id="org-name"
            type="text"
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Acme Inc."
            disabled={createOrg.isPending}
          />
        </div>

        {createOrg.isError && (
          <p role="alert" className="text-destructive text-sm">
            Could not create the organization. Try again.
          </p>
        )}

        <Button
          type="submit"
          variant="accent"
          size="lg"
          disabled={createOrg.isPending || !name.trim()}
        >
          {createOrg.isPending ? "Creating…" : "Create organization"}
        </Button>
      </form>
    </Card>
  );
}
