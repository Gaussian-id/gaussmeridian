"use client";

import { useParams } from "next/navigation";

import { AccountStatsRow } from "@/components/dashboard/account-stats-row";
import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { ModelPerformanceCard } from "@/components/dashboard/model-performance-card";
import { RecentRequestsCard } from "@/components/dashboard/recent-requests-card";

export default function ProjectOverviewPage() {
  const { projectId } = useParams<{ projectId: string }>();

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Project"
        title="Overview"
        description="Settled requests and usage for this GaussMeridian project."
      />
      <AccountStatsRow projectId={projectId} />
      <div className="grid gap-6 lg:grid-cols-2">
        <RecentRequestsCard projectId={projectId} />
        <ModelPerformanceCard projectId={projectId} />
      </div>
    </div>
  );
}
