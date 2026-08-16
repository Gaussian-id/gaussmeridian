"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useState, useSyncExternalStore, type FormEvent } from "react";

import { isAuthError } from "@core/adapters/auth-error";

import { AuthFormField } from "@/components/auth/auth-form-field";
import { AuthPageHeader } from "@/components/auth/auth-page-header";
import { Button } from "@/components/ui/button";
import { useSignUp } from "@/hooks/useSignUp";
import { mapAuthError } from "@/lib/auth/map-auth-error";
import {
  MIN_PASSWORD_LENGTH,
  normalizeUsername,
  validateEmail,
  validatePassword,
  validateUsername,
} from "@/lib/auth/validate-credentials";
import {
  ADD_CREDIT_LOGIN_HREF,
  ADD_CREDIT_ORG_RESOLVER_HREF,
  isAddCreditIntent,
} from "@/lib/billing/billing-intent";

interface FieldErrors {
  email?: string;
  username?: string;
  password?: string;
}

// Keep the submit button inert in prerendered HTML so a pre-hydration click cannot fall through
// to native form navigation. The server snapshot matches the first hydration render; React then
// switches to the client snapshot without an effect-driven state cascade.
function subscribeToHydration(): () => void {
  return () => {};
}

function clientSnapshot(): boolean {
  return true;
}

function serverSnapshot(): boolean {
  return false;
}

function SignupForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const fundingIntent = isAddCreditIntent(searchParams.get("intent"));
  const signUp = useSignUp();
  const [email, setEmail] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const hydrated = useSyncExternalStore(subscribeToHydration, clientSnapshot, serverSnapshot);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const handle = normalizeUsername(username);
    const errors: FieldErrors = {
      email: validateEmail(email) ?? undefined,
      username: validateUsername(handle) ?? undefined,
      password: validatePassword(password) ?? undefined,
    };
    if (errors.email || errors.username || errors.password) {
      setFieldErrors(errors);
      return;
    }
    setFieldErrors({});
    // First-run signup goes straight into the gated wizard (US O1/O18) — never `/orgs`, which
    // is the returning-user destination once onboarding_completed is true (see OnboardingGate).
    signUp.mutate(
      { email, username: handle, password },
      {
        onSuccess: () =>
          router.push(
            fundingIntent
              ? `/onboarding?redirectTo=${encodeURIComponent(ADD_CREDIT_ORG_RESOLVER_HREF)}`
              : "/onboarding",
          ),
      },
    );
  }

  const mapped = signUp.isError ? mapAuthError(signUp.error, "signup") : null;
  const emailTaken = isAuthError(signUp.error) && signUp.error.code === "email_taken";
  const formError = mapped && !mapped.field ? mapped.message : null;

  // A client validation error wins for a field; otherwise fall back to a server field error
  // (email_taken / username_taken) mapped from the failed mutation.
  const fieldError = (field: keyof FieldErrors) =>
    fieldErrors[field] ?? (mapped?.field === field ? mapped.message : undefined);
  const clearField = (field: keyof FieldErrors) =>
    setFieldErrors((prev) => (prev[field] ? { ...prev, [field]: undefined } : prev));

  return (
    <div className="flex flex-col gap-6">
      <AuthPageHeader title="Create your account" description="Join your Meridian workspace." />

      <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <AuthFormField
            id="email"
            label="Email"
            type="email"
            autoComplete="email"
            value={email}
            onChange={(value) => {
              setEmail(value);
              clearField("email");
            }}
            placeholder="you@company.com"
            error={fieldError("email")}
          />
          {emailTaken && (
            <Link
              href={fundingIntent ? ADD_CREDIT_LOGIN_HREF : "/login"}
              className="text-primary text-sm font-medium"
            >
              Sign in instead →
            </Link>
          )}
        </div>
        <AuthFormField
          id="username"
          label="Username"
          autoComplete="username"
          value={username}
          onChange={(value) => {
            setUsername(normalizeUsername(value));
            clearField("username");
          }}
          placeholder="yourname"
          error={fieldError("username")}
        />
        <AuthFormField
          id="password"
          label="Password"
          type="password"
          autoComplete="new-password"
          minLength={MIN_PASSWORD_LENGTH}
          value={password}
          onChange={(value) => {
            setPassword(value);
            clearField("password");
          }}
          placeholder="••••••••"
          error={fieldError("password")}
        />

        {formError && (
          <p role="alert" className="text-destructive text-sm">
            {formError}
          </p>
        )}

        <Button
          type="submit"
          variant="accent"
          size="lg"
          disabled={!hydrated || signUp.isPending}
          aria-busy={!hydrated || signUp.isPending}
        >
          {!hydrated ? "Preparing form…" : signUp.isPending ? "Creating account…" : "Sign up"}
        </Button>
      </form>

      <p className="text-muted-foreground text-center text-sm">
        Already have an account?{" "}
        <Link
          href={fundingIntent ? ADD_CREDIT_LOGIN_HREF : "/login"}
          className="text-primary font-medium"
        >
          Sign in
        </Link>
      </p>
    </div>
  );
}

export default function SignupPage() {
  return (
    <Suspense fallback={null}>
      <SignupForm />
    </Suspense>
  );
}
