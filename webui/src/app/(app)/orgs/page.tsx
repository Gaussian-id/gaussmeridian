import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { OrgChooser } from "@/components/orgs";
import { isAddCreditIntent } from "@/lib/billing/billing-intent";

export default async function OrgChooserPage({
  searchParams,
}: {
  searchParams: Promise<{ intent?: string | string[] }>;
}) {
  const params = await searchParams;
  const fundingIntent = isAddCreditIntent(params.intent);

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Workspace"
        title={fundingIntent ? "Choose where to add credit" : "Organizations"}
        description={
          fundingIntent
            ? "Credit belongs to one organization. Choose the organization that should receive it."
            : "Choose an organization, or create a new one. Login always lands here."
        }
      />
      <OrgChooser fundingIntent={fundingIntent} />
    </div>
  );
}
