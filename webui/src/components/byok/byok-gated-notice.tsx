import { Card, CardDescription, CardTitle } from "@/components/ui/card";

/**
 * The ONE gated-state notice for BYOK key management. BYOK is a paid-tier capability:
 * the backend allows it only for allowlisted accounts (and needs the server vault
 * configured). Reuse this component for any "BYOK not available" state — do not
 * duplicate this notice elsewhere.
 */
export function ByokGatedNotice() {
  return (
    <Card className="p-6">
      <CardTitle>BYOK is a paid-tier feature</CardTitle>
      <CardDescription className="mt-1">
        Bring-your-own-key lets you route with your own provider credentials at zero token markup.
        It&apos;s enabled per account — contact your workspace admin to get access. You can keep
        using platform credits in the meantime.
      </CardDescription>
    </Card>
  );
}
