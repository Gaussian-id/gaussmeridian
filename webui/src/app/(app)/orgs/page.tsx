import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { OrgChooser } from "@/components/orgs";

export default function OrgChooserPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Workspace"
        title="Organizations"
        description="Choose an organization, or create a new one. Login always lands here."
      />
      <OrgChooser />
    </div>
  );
}
