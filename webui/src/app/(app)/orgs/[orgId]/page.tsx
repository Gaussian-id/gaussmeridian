import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { ProjectList } from "@/components/orgs";

export default function OrgHomePage() {
  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Organization"
        title="Projects"
        description="Every project in this organization. Empty is a first-class state here."
      />
      <ProjectList />
    </div>
  );
}
