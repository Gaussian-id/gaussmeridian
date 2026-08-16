"use client";

import { useState, type FormEvent } from "react";

import type { AccountProfile } from "@core/adapters/schemas/account.schema";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { ErrorState } from "@/components/ui/error-state";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { useAccountProfile, useUpdateAccountProfile } from "@/hooks/useAccount";

/** Seeded once from the loaded profile via `useState` initializers, matching
 *  `ProjectSettingsForm`'s convention (mounted only once data exists, so no effect-based state
 *  syncing is needed). Username and email are read-only here — there is no backend endpoint to
 *  change either from this page (see the account-page doc comment). */
function ProfileForm({ initial }: { initial: AccountProfile }) {
  const updateProfile = useUpdateAccountProfile();

  const [fullName, setFullName] = useState(initial.full_name ?? "");
  const [displayName, setDisplayName] = useState(initial.display_name ?? "");
  const [company, setCompany] = useState(initial.company ?? "");
  const [timezone, setTimezone] = useState(initial.timezone ?? "");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    // Mirrors `OnboardingStepProfile`'s convention: an emptied field is sent as `undefined`,
    // not `""` — the backend's `update_profile` (handlers.rs) only overwrites a field when the
    // payload key is present (`Option::is_some()`), so omitting it leaves the stored value
    // untouched rather than blanking it out.
    updateProfile.mutate({
      full_name: fullName || undefined,
      display_name: displayName || undefined,
      company: company || undefined,
      timezone: timezone || undefined,
    });
  }

  return (
    <form onSubmit={handleSubmit} className="mt-6 flex flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="account-username">Username</Label>
          <Input id="account-username" value={initial.username} disabled readOnly />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="account-email">Email</Label>
          <Input id="account-email" type="email" value={initial.email} disabled readOnly />
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="account-full-name">Full name</Label>
          <Input
            id="account-full-name"
            value={fullName}
            onChange={(event) => setFullName(event.target.value)}
            placeholder="Ada Lovelace"
            disabled={updateProfile.isPending}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="account-display-name">Display name</Label>
          <Input
            id="account-display-name"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
            placeholder="Ada"
            disabled={updateProfile.isPending}
          />
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="account-company">Company</Label>
          <Input
            id="account-company"
            value={company}
            onChange={(event) => setCompany(event.target.value)}
            placeholder="Acme Inc."
            disabled={updateProfile.isPending}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="account-timezone">Timezone</Label>
          <Input
            id="account-timezone"
            value={timezone}
            onChange={(event) => setTimezone(event.target.value)}
            placeholder="America/New_York"
            disabled={updateProfile.isPending}
          />
        </div>
      </div>

      {updateProfile.isError && (
        <p role="alert" className="text-destructive text-sm">
          Could not save your profile. Try again.
        </p>
      )}
      {updateProfile.isSuccess && !updateProfile.isPending && (
        <p role="status" className="text-accent text-sm">
          Profile saved.
        </p>
      )}

      <Button
        type="submit"
        variant="accent"
        disabled={updateProfile.isPending}
        className="self-start"
      >
        {updateProfile.isPending ? "Saving…" : "Save changes"}
      </Button>
    </form>
  );
}

/** Card shell + the three query states (`useResourceQuery`'s loading/error/data) around
 *  `ProfileForm`. Lives on `/account/me` (`app/(app)/account/me/page.tsx`). */
export function AccountProfileForm() {
  const profile = useAccountProfile();

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between gap-4">
        <CardTitle>Profile</CardTitle>
        <Badge variant="outline">Username &amp; email read-only</Badge>
      </div>
      <CardDescription className="mt-1">
        Your display name, full name, company, and timezone. Changes save immediately.
      </CardDescription>

      {profile.isLoading && (
        <div className="mt-6 flex flex-col gap-4">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-1/3" />
        </div>
      )}
      {profile.isError && (
        <div className="mt-6">
          <ErrorState message="Could not load your account. Try again shortly." />
        </div>
      )}
      {profile.data && <ProfileForm initial={profile.data} />}
    </Card>
  );
}
