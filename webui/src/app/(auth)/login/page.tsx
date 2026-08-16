"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useState, type FormEvent } from "react";

import { AuthFormField } from "@/components/auth/auth-form-field";
import { AuthPageHeader } from "@/components/auth/auth-page-header";
import { Button } from "@/components/ui/button";
import { useSignIn } from "@/hooks/useSignIn";
import { mapAuthError } from "@/lib/auth/map-auth-error";
import { validateEmail, validatePassword } from "@/lib/auth/validate-credentials";
import { safeInternalRedirect } from "@/lib/navigation/safe-internal-redirect";

function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const signIn = useSignIn();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [fieldErrors, setFieldErrors] = useState<{ email?: string; password?: string }>({});

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const emailError = validateEmail(email);
    const passwordError = validatePassword(password, { forLogin: true });
    if (emailError || passwordError) {
      setFieldErrors({ email: emailError ?? undefined, password: passwordError ?? undefined });
      return;
    }
    setFieldErrors({});
    signIn.mutate(
      { email, password },
      {
        onSuccess: async (session) => {
          const requestedDestination = safeInternalRedirect(
            searchParams.get("redirectTo"),
            "/orgs",
          );
          // A user who signed up but never finished onboarding must land in the gated wizard,
          // not `/orgs`. Route on the same `onboardingCompleted` boolean OnboardingGate uses, so
          // first sign-in goes straight to `/onboarding` instead of flashing a protected route
          // until a refresh lets the gate re-bounce it (US O9 / DR-010). A `redirectTo` deep-link
          // still yields to onboarding — the gate would override it anyway.
          if (session.onboardingCompleted === false) {
            const onboardingDestination =
              requestedDestination === "/orgs"
                ? "/onboarding"
                : `/onboarding?redirectTo=${encodeURIComponent(requestedDestination)}`;
            router.replace(onboardingDestination);
            return;
          }

          // An explicit deep-link wins for everyone (they were trying to reach a specific page).
          if (requestedDestination !== "/orgs") {
            router.push(requestedDestination);
            return;
          }

          // No explicit destination: a superadmin's home is the admin console, never the tenant
          // app — checked through the same `GET /v1/admin/me` allowlist probe SuperadminGate uses
          // (a non-allowlisted caller 404s, so this falls through to the tenant landing).
          try {
            const res = await fetch("/api/gaussmeridian/v1/admin/me", { credentials: "include" });
            if (res.ok && ((await res.json()) as { superadmin?: boolean })?.superadmin === true) {
              router.replace("/admin");
              return;
            }
          } catch {
            // network/probe failure → treat as a normal user and land in the app
          }
          router.push("/orgs");
        },
      },
    );
  }

  const errorMessage = signIn.isError ? mapAuthError(signIn.error, "login").message : null;

  return (
    <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-4">
      <AuthFormField
        id="email"
        label="Email"
        type="email"
        autoComplete="email"
        value={email}
        onChange={(value) => {
          setEmail(value);
          if (fieldErrors.email) setFieldErrors((prev) => ({ ...prev, email: undefined }));
        }}
        placeholder="you@company.com"
        error={fieldErrors.email}
      />
      <AuthFormField
        id="password"
        label="Password"
        type="password"
        autoComplete="current-password"
        value={password}
        onChange={(value) => {
          setPassword(value);
          if (fieldErrors.password) setFieldErrors((prev) => ({ ...prev, password: undefined }));
        }}
        placeholder="••••••••"
        error={fieldErrors.password}
      />

      <div className="-mt-1 flex justify-end">
        <Link href="/forgot-password" className="text-primary text-sm font-medium">
          Forgot password?
        </Link>
      </div>

      {errorMessage && (
        <p role="alert" className="text-destructive text-sm">
          {errorMessage}
        </p>
      )}

      <Button type="submit" variant="accent" size="lg" disabled={signIn.isPending}>
        {signIn.isPending ? "Signing in…" : "Sign in"}
      </Button>
    </form>
  );
}

export default function LoginPage() {
  return (
    <div className="flex flex-col gap-6">
      <AuthPageHeader title="Welcome back" description="Sign in to your Meridian workspace." />

      {/* useSearchParams (redirectTo) requires a Suspense boundary during prerender. */}
      <Suspense fallback={null}>
        <LoginForm />
      </Suspense>

      <p className="text-muted-foreground text-center text-sm">
        New to Meridian?{" "}
        <Link href="/signup" className="text-primary font-medium">
          Sign up
        </Link>
      </p>
    </div>
  );
}
