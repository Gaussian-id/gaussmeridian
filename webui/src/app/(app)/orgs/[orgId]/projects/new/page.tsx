import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { CreateProjectForm } from "@/components/orgs";

export default function CreateProjectPage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Organization"
        title="New project"
        description="Name a project and pick its environment to get started."
      />
      <CreateProjectForm />
    </div>
  );
}
