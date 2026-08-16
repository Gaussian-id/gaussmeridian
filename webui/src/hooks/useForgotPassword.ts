"use client";

import { useMutation } from "@tanstack/react-query";

import { useAuth } from "@core/adapters";

/** Forgot-password mutation against the auth adapter. Always resolves generically —
 *  the backend never reveals whether the email has an account (anti-enumeration). */
export function useForgotPassword() {
  const auth = useAuth();
  return useMutation({
    mutationFn: (input: { email: string }) => auth.forgotPassword(input),
  });
}
