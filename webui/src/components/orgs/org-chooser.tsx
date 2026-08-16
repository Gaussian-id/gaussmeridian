"use client";

import { Building2, Plus } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { cn } from "@core/lib/utils";

import { buttonVariants } from "@/components/ui/button";
import { ErrorState } from "@/components/ui/error-state";
import { Skeleton } from "@/components/ui/skeleton";
import { useOrgs } from "@/hooks/useConsoleQueries";
import { ADD_CREDIT_INTENT, billingHref } from "@/lib/billing/billing-intent";

import { OrgCard } from "./org-card";

/**
 * The ordinary app entry renders every organization as a card. The bounded add-credit entry is
 * deliberately different: zero organizations continues to creation, one resolves directly to
 * Billing, and many require the user to choose the wallet recipient.
 */
export function OrgChooser({ fundingIntent = false }: { fundingIntent?: boolean }) {
  const router = useRouter();
  const orgs = useOrgs();
  const soleOrg = fundingIntent && orgs.data?.orgs.length === 1 ? orgs.data.orgs[0] : undefined;
  const createHref = fundingIntent ? `/orgs/new?intent=${ADD_CREDIT_INTENT}` : "/orgs/new";

  useEffect(() => {
    if (soleOrg) router.replace(billingHref(soleOrg.id));
  }, [router, soleOrg]);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex justify-end">
        <Link href={createHref} className={cn(buttonVariants({ variant: "accent" }))}>
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
          <Link href={createHref} className={cn(buttonVariants({ variant: "accent" }))}>
            <Plus className="h-4 w-4" aria-hidden="true" />
            Create organization
          </Link>
        </div>
      )}

      {soleOrg ? (
        <div
          className="border-border bg-card rounded-xl border px-6 py-10 text-center"
          role="status"
        >
          <p className="font-display text-lg font-semibold">Opening {soleOrg.name} billing…</p>
          <p className="text-muted-foreground mt-1 text-sm">
            This organization will receive the credit.
          </p>
        </div>
      ) : orgs.isSuccess && orgs.data.orgs.length > 0 ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {orgs.data.orgs.map((org) => (
            <OrgCard
              key={org.id}
              org={org}
              href={fundingIntent ? billingHref(org.id) : undefined}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}
