import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { DangerZone, TenantSettingsForm } from "@/components/orgs";

export default function OrgSettingsPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Organization"
        title="Settings"
        description="Tenant-level settings, including the danger zone."
      />
      <TenantSettingsForm />
      <DangerZone />
    </div>
  );
}
