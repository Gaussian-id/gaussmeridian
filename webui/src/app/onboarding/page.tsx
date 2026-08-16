import { OnboardingWizard } from "@/components/onboarding";
import { safeInternalRedirect } from "@/lib/navigation/safe-internal-redirect";

export default async function OnboardingPage({
  searchParams,
}: {
  searchParams: Promise<{ redirectTo?: string | string[] }>;
}) {
  const params = await searchParams;
  const redirectValue = Array.isArray(params.redirectTo) ? params.redirectTo[0] : params.redirectTo;
  return <OnboardingWizard completionHref={safeInternalRedirect(redirectValue, "/orgs")} />;
}
