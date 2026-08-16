import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { CreateOrgForm } from "@/components/orgs";
import { isAddCreditIntent } from "@/lib/billing/billing-intent";

export default async function CreateOrgPage({
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
        title="Create organization"
        description={
          fundingIntent
            ? "Create the organization that will own this credit balance."
            : "New organizations are born empty — no default project, just you as owner."
        }
      />
      <CreateOrgForm completion={fundingIntent ? "billing" : "organization"} />
    </div>
  );
}
