import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { CreateOrgForm } from "@/components/orgs";

export default function CreateOrgPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Workspace"
        title="Create organization"
        description="New organizations are born empty — no default project, just you as owner."
      />
      <CreateOrgForm />
    </div>
  );
}
