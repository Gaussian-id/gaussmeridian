import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

/**
 * Password change, gated off: the backend has a *logged-out* reset flow (`POST
 * /v1/auth/forgot-password` + `/v1/auth/reset-password`, both unauthenticated, email-token
 * based — `wrapped by `forgotPassword`/`resetPassword` on `AuthAdapter`) but no *logged-in*
 * change-password endpoint (confirmed against `routes.rs`: the only password-adjacent routes
 * under `/v1/auth` are `forgot-password`/`reset-password`; `update_profile` at
 * `/v1/onboarding/profile` never touches the password field). Rather than fake a working form
 * against a real-looking submit button, the fields render disabled with an explicit note — this
 * is a known gap, tracked for a future backend change, not a silent omission.
 */
export function AccountPasswordSection() {
  return (
    <Card className="p-6">
      <div className="flex items-center justify-between gap-4">
        <CardTitle>Password</CardTitle>
        <Badge variant="outline">Coming soon</Badge>
      </div>
      <CardDescription className="mt-1">
        Changing your password from here isn&apos;t available yet — the backend only supports the
        signed-out{" "}
        <a href="/forgot-password" className="text-primary font-medium">
          forgot password
        </a>{" "}
        flow today. This section will activate once a logged-in change-password endpoint ships.
      </CardDescription>

      <div className="mt-6 flex flex-col gap-4" aria-disabled="true">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="account-current-password">Current password</Label>
          <Input id="account-current-password" type="password" disabled placeholder="••••••••" />
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="account-new-password">New password</Label>
            <Input id="account-new-password" type="password" disabled placeholder="••••••••" />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="account-confirm-password">Confirm new password</Label>
            <Input id="account-confirm-password" type="password" disabled placeholder="••••••••" />
          </div>
        </div>
        <Button type="button" variant="outline" disabled className="self-start">
          Change password
        </Button>
      </div>
    </Card>
  );
}
