"use client";

import { useRouter } from "next/navigation";
import { useEffect } from "react";

import { useSession } from "@/hooks/useSession";

import type { ReactNode } from "react";

/**
 * The onboarding gate (US O9 / DR-010 §7-RESOLVED item 2): a signed-in user with
 * `onboarding_completed: false` is bounced into `/onboarding` before seeing any `(app)` route
 * (`/orgs`, `/playground`, `/settings`). The wizard itself owns resuming at the right step
 * (`GET /v1/onboarding/state` → `nextIncomplete()`) — this gate only needs the one boolean, so
 * it reads `useSession()` (already the app's one source of truth for "who is signed in") rather
 * than duplicating a second fetch.
 *
 * Deliberately a data/layout gate, not a `proxy.ts` (middleware) gate: `proxy.ts`'s guard only
 * has the raw session cookie to work with, no server-side call it can cheaply make per request —
 * duplicating the completion check there would mean either an extra network round trip on every
 * guarded request or trusting a client-forgeable cookie claim. This gate runs once per `(app)`
 * navigation, after the session is already being fetched for other reasons (the topbar, RBAC,
 * etc.), so it's free.
 *
 * `/onboarding` itself lives OUTSIDE the `(app)` route group (`src/app/onboarding/page.tsx`) —
 * so this gate never wraps the wizard, and there is no redirect loop to guard against.
 */
export function OnboardingGate({ children }: { children: ReactNode }) {
  const router = useRouter();
  const session = useSession();

  const shouldRedirect =
    session.isSuccess && session.data !== null && session.data.onboardingCompleted === false;

  useEffect(() => {
    if (shouldRedirect) router.replace("/onboarding");
  }, [shouldRedirect, router]);

  // While the session is loading, or a redirect is about to happen, render nothing rather than
  // flash protected `(app)` content — the redirect above fires from the effect on the same tick
  // the session resolves.
  if (session.isLoading || shouldRedirect) return null;

  return children;
}
