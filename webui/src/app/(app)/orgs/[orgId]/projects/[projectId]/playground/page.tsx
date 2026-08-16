import { Playground } from "@/components/playground";

export default async function ProjectPlaygroundPage({
  params,
}: {
  params: Promise<{ orgId: string; projectId: string }>;
}) {
  const { orgId, projectId } = await params;
  return <Playground orgId={orgId} projectId={projectId} />;
}
