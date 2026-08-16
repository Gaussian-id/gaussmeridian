import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { MembersTable } from "@/components/orgs";

export default function MembersPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Organization"
        title="Team & members"
        description="Everyone with access to this organization, and their role."
      />
      <MembersTable />
    </div>
  );
}
