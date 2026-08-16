"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Suspense, useState, type FormEvent } from "react";

import { isAuthError } from "@core/adapters/auth-error";

import { AuthFormField } from "@/components/auth/auth-form-field";
import { AuthPageHeader } from "@/components/auth/auth-page-header";
import { Button } from "@/components/ui/button";
import { useResetPassword } from "@/hooks/useResetPassword";
import { mapAuthError } from "@/lib/auth/map-auth-error";

function ResetPasswordForm() {
  const searchParams = useSearchParams();
  const token = searchParams.get("token") ?? "";

  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);
  const resetPassword = useResetPassword();

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    // Mirrors the backend's reset_password validation (min 8 chars).
    if (password.length < 8) {
      setValidationError("Password must be at least 8 characters.");
      return;
    }
    if (password !== confirm) {
      setValidationError("Passwords don't match.");
      return;
    }
    setValidationError(null);
    resetPassword.mutate({ token, newPassword: password });
  }

  if (!token) {
    return (
      <p role="alert" className="text-sm">
        This reset link is missing its token. Use the link from your email, or{" "}
        <Link href="/forgot-password" className="text-primary font-medium">
          request a new one
        </Link>
        .
      </p>
    );
  }

  if (resetPassword.isSuccess) {
    return (
      <p role="status" className="text-sm">
        Your password has been reset.{" "}
        <Link href="/login" className="text-primary font-medium">
          Sign in
        </Link>{" "}
        with your new password.
      </p>
    );
  }

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-4">
      <AuthFormField
        id="new-password"
        label="New password"
        type="password"
        autoComplete="new-password"
        value={password}
        onChange={setPassword}
        placeholder="At least 8 characters"
      />
      <AuthFormField
        id="confirm-password"
        label="Confirm password"
        type="password"
        autoComplete="new-password"
        value={confirm}
        onChange={setConfirm}
        placeholder="Repeat the new password"
      />

      {validationError && (
        <p role="alert" className="text-destructive text-sm">
          {validationError}
        </p>
      )}
      {resetPassword.isError && !validationError && (
        <p role="alert" className="text-destructive text-sm">
          {mapAuthError(resetPassword.error, "reset").message}{" "}
          {!(isAuthError(resetPassword.error) && resetPassword.error.status === 0) && (
            <Link href="/forgot-password" className="text-primary font-medium">
              Request a new one
            </Link>
          )}
        </p>
      )}

      <Button type="submit" variant="accent" size="lg" disabled={resetPassword.isPending}>
        {resetPassword.isPending ? "Resetting…" : "Reset password"}
      </Button>
    </form>
  );
}

export default function ResetPasswordPage() {
  return (
    <div className="flex flex-col gap-6">
      <AuthPageHeader
        title="Choose a new password"
        description="Enter a new password for your account."
      />
      {/* useSearchParams requires a Suspense boundary during prerender. */}
      <Suspense fallback={null}>
        <ResetPasswordForm />
      </Suspense>
    </div>
  );
}
