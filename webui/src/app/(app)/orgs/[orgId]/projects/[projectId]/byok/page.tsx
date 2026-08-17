import { ByokManager } from "@/components/byok";
import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";

/**
 * Customer BYOK — bring your own provider credentials for this project.
 *
 * This route used to `notFound()`: BYOK was switched off because it sat outside the hosted
 * billing-bridge contract. That reasoning does not hold for a self-hosted gateway, where routing
 * with your own provider key is the normal case rather than a paid add-on, so the route is live.
 *
 * The server still gates it: the account must be allowlisted (`BYOK_ADMIN_EMAILS`) and the vault
 * must be configured (`BYOK_MASTER_KEY`). `ByokManager` reports both states in the operator's own
 * terms rather than surfacing a bare 403 or 503.
 */
export default function ByokPage() {
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Project"
        title="Bring your own key"
        description="Route this project through your own provider credentials."
      />
      <ByokManager />
    </div>
  );
}
