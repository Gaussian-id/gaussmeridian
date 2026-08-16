"use client";

import Link from "next/link";
import { useParams } from "next/navigation";

import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { ProjectDangerZone } from "@/components/orgs";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";

/**
 * Bridge-safe project settings surface. Native Meridian routing controls and customer BYOK are
 * intentionally absent until those capabilities have a qualified product contract.
 */
export default function ProjectSettingsPage() {
  const { orgId, projectId } = useParams<{ orgId: string; projectId: string }>();

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Project"
        title="Settings"
        description="Project lifecycle and bridge-safe configuration destinations."
      />

      <Card className="p-6">
        <CardTitle>Inference configuration</CardTitle>
        <CardDescription className="mt-1">
          Available models, supported request parameters, and retail rates come from the versioned
          GaussMeridian catalog. Native routing controls are not available during the bridge.
        </CardDescription>
        <Link
          href={`/orgs/${orgId}/projects/${projectId}/models`}
          className="text-primary mt-4 inline-block text-sm font-medium"
        >
          View supported models
        </Link>
      </Card>

      <Card className="p-6">
        <CardTitle>Billing and spend</CardTitle>
        <CardDescription className="mt-1">
          Prepaid balance, credit expiry, top-ups, subscriptions, and invoices are owned by the
          organization billing account.
        </CardDescription>
        <Link
          href={`/orgs/${orgId}/billing`}
          className="text-primary mt-4 inline-block text-sm font-medium"
        >
          Manage organization billing
        </Link>
      </Card>

      <ProjectDangerZone />
    </div>
  );
}
