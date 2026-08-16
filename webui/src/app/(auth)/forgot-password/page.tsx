"use client";

import Link from "next/link";
import { useState, type FormEvent } from "react";

import { AuthFormField } from "@/components/auth/auth-form-field";
import { AuthPageHeader } from "@/components/auth/auth-page-header";
import { Button } from "@/components/ui/button";
import { useForgotPassword } from "@/hooks/useForgotPassword";
import { mapAuthError } from "@/lib/auth/map-auth-error";

export default function ForgotPasswordPage() {
  const [email, setEmail] = useState("");
  const forgotPassword = useForgotPassword();

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    forgotPassword.mutate({ email });
  }

  return (
    <div className="flex flex-col gap-6">
      <AuthPageHeader
        title="Reset your password"
        description="Enter your email and we'll send you a reset link."
      />

      {forgotPassword.isSuccess ? (
        <p role="status" className="text-sm">
          If an account exists for that email, we&apos;ve sent a reset link. The link expires in one
          hour.
        </p>
      ) : (
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <AuthFormField
            id="email"
            label="Email"
            type="email"
            autoComplete="email"
            value={email}
            onChange={setEmail}
            placeholder="you@company.com"
          />

          {forgotPassword.isError && (
            <p role="alert" className="text-destructive text-sm">
              {mapAuthError(forgotPassword.error, "forgot").message}
            </p>
          )}

          <Button type="submit" variant="accent" size="lg" disabled={forgotPassword.isPending}>
            {forgotPassword.isPending ? "Sending…" : "Send reset link"}
          </Button>
        </form>
      )}

      <p className="text-muted-foreground text-center text-sm">
        Remembered your password?{" "}
        <Link href="/login" className="text-primary font-medium">
          Sign in
        </Link>
      </p>
    </div>
  );
}
