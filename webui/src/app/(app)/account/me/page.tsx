import {
  AccountDangerZone,
  AccountPasswordSection,
  AccountProfileForm,
} from "@/components/account";
import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";

/**
 * /account/me — "Account preferences" (Shelby's adjustment 2/4): reached from the navbar's
 * Supabase-style avatar menu (`AccountMenu`, `layout/navbar/account-menu.tsx`). Global — not
 * org/project-scoped — so it sits directly under the `(app)` route group next to `/orgs` and
 * guarded the same way (`src/proxy.ts`'s `GUARDED_PREFIXES`).
 *
 * Three surfaces: editable profile (display name/full name/company/timezone, username/email
 * read-only — `AccountProfileForm`), a password-change section that stays disabled with a
 * "coming soon" note because the backend has no logged-in change-password endpoint yet
 * (`AccountPasswordSection`), and the danger zone's account-deletion *request*
 * (`AccountDangerZone` — reviewed and fulfilled by an administrator, not an immediate delete).
 */
export default function AccountPage() {
  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Account"
        title="Account preferences"
        description="Your profile, password, and account-level actions."
      />

      <AccountProfileForm />
      <AccountPasswordSection />
      <AccountDangerZone />
    </div>
  );
}
