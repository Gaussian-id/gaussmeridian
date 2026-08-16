"use client";

import { useMutation } from "@tanstack/react-query";

import { useAuth } from "@core/adapters";

/** Reset-password mutation against the auth adapter. Rejects when the token is
 *  invalid, expired, or already used (single-use), or the password is too short. */
export function useResetPassword() {
  const auth = useAuth();
  return useMutation({
    mutationFn: (input: { token: string; newPassword: string }) => auth.resetPassword(input),
  });
}
