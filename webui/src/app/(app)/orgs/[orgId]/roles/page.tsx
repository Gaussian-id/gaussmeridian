import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { PermissionMatrix } from "@/components/orgs";

export default function RolesPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Organization"
        title="Roles & permissions"
        description="What Owner, Admin, and Developer can each do in this organization."
      />
      <PermissionMatrix />
    </div>
  );
}
