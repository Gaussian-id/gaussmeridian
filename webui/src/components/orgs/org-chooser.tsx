"use client";

import { Building2, Plus } from "lucide-react";
import Link from "next/link";

import { cn } from "@core/lib/utils";

import { buttonVariants } from "@/components/ui/button";
import { ErrorState } from "@/components/ui/error-state";
import { Skeleton } from "@/components/ui/skeleton";
import { useOrgs } from "@/hooks/useConsoleQueries";

import { OrgCard } from "./org-card";

/**
 * The app entry: every organization the caller belongs to, rendered as a card.
 *
 * There used to be a second mode here — an "add credit" entry that auto-forwarded a sole
 * organization straight to `/orgs/:id/billing`. That route was removed with the billing surfaces,
 * so the redirect landed users on a 404 without them clicking anything named billing. The mode is
 * gone with it.
 */
export function OrgChooser() {
  const orgs = useOrgs();

  return (
    <div className="flex flex-col gap-6">
      <div className="flex justify-end">
        <Link href="/orgs/new" className={cn(buttonVariants({ variant: "accent" }))}>
          <Plus className="h-4 w-4" aria-hidden="true" />
          Create organization
        </Link>
      </div>

      {orgs.isLoading && (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Skeleton key={i} className="h-40 w-full rounded-xl" />
          ))}
        </div>
      )}

      {orgs.isError && (
        <ErrorState message="Could not load your organizations. Try again shortly." />
      )}

      {orgs.isSuccess && orgs.data.orgs.length === 0 && (
        <div className="border-border bg-card flex flex-col items-center gap-4 rounded-xl border border-dashed px-8 py-16 text-center">
          <div className="bg-secondary flex h-14 w-14 items-center justify-center rounded-full">
            <Building2 className="text-accent h-7 w-7" aria-hidden="true" />
          </div>
          <div>
            <h2 className="font-display text-xl font-semibold tracking-tight">
              No organizations yet
            </h2>
            <p className="text-muted-foreground mt-1 max-w-sm text-sm">
              Create your first organization to invite teammates, spin up projects, and start
              routing requests through GaussMeridian.
            </p>
          </div>
          <Link href="/orgs/new" className={cn(buttonVariants({ variant: "accent" }))}>
            <Plus className="h-4 w-4" aria-hidden="true" />
            Create organization
          </Link>
        </div>
      )}

      {orgs.isSuccess && orgs.data.orgs.length > 0 ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {orgs.data.orgs.map((org) => (
            <OrgCard
              key={org.id}
              org={org}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}
