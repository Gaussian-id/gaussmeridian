"use client";

import { useParams } from "next/navigation";

import { RequestLogSchema } from "@core/adapters/schemas/gaussmeridian.schema";

import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { logsColumns } from "@/components/dashboard/logs-columns";
import { DataTable } from "@/components/ui/data-table";
import { useProjectRequestLogs } from "@/hooks/useGaussmeridianQueries";

import type { z } from "zod";

const ACTIVITY_LIMIT = 100;
type RequestLog = z.infer<typeof RequestLogSchema>;

/**
 * Settled and explicitly pending delivery attempts for this project. The billing bridge uses the
 * outcome-billing ledger as the customer authority; legacy xRouter/GaussMoA trace projections
 * are deliberately absent from this surface.
 */
export default function ActivityPage() {
  const { projectId } = useParams<{ projectId: string }>();
  const requests = useProjectRequestLogs(projectId, { limit: ACTIVITY_LIMIT });

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Project"
        title="Activity"
        description={`The last ${ACTIVITY_LIMIT} settled or explicitly pending requests for this project.`}
      />

      <DataTable<RequestLog, unknown>
        columns={logsColumns}
        data={requests.data ?? []}
        isLoading={requests.isLoading}
        isError={requests.isError}
        errorMessage="Could not load this project's activity. Try again shortly."
        emptyMessage="No settled requests for this project yet."
      />
    </div>
  );
}
