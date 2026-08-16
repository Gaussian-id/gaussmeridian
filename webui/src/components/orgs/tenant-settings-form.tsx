"use client";

import { useTenancy } from "@core/providers";

import { Badge } from "@/components/ui/badge";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";

const PLAN_LABEL: Record<string, string> = { free: "Free", pro: "Pro", enterprise: "Enterprise" };

/**
 * Organization identity — name, slug, plan. Read-only in Phase 1: there is no
 * update-org mutation in the console contract yet (`console.schema.ts` is pending maintainer
 * ratification as the Phase-2 backend spec), so this shows real, live org data rather than a
 * save button that would silently do nothing.
 */
export function TenantSettingsForm() {
  const { org } = useTenancy();

  if (!org) {
    return (
      <Card className="flex flex-col gap-4 p-6">
        <Skeleton className="h-4 w-24" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
      </Card>
    );
  }

  return (
    <Card className="flex flex-col gap-4 p-6">
      <div>
        <CardTitle className="text-base">Organization identity</CardTitle>
        <CardDescription className="mt-1">
          Editing lands with the Phase-2 backend — for now this reflects the organization as
          created.
        </CardDescription>
      </div>

      <div className="flex flex-col gap-1.5">
        <Label htmlFor="tenant-name">Name</Label>
        <Input id="tenant-name" value={org.name} disabled readOnly />
      </div>

      <div className="flex flex-col gap-1.5">
        <Label htmlFor="tenant-slug">Slug</Label>
        <Input id="tenant-slug" value={org.slug} disabled readOnly className="font-mono" />
      </div>

      <div className="flex items-center justify-between">
        <Label>Plan</Label>
        <Badge variant="mono">{PLAN_LABEL[org.plan] ?? org.plan}</Badge>
      </div>
    </Card>
  );
}
