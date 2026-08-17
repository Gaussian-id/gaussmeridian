"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";

import { useAuth } from "@core/adapters";

/**
 * Sign-up mutation against the auth adapter. Like `useSignIn`, it **primes the `["session"]` cache**
 * with the returned session on success so the navbar avatar reflects the new session immediately
 * (sign-up also mints the httpOnly session cookie), rather than staying empty until a manual refresh.
 */
export function useSignUp() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { email: string; username: string; password: string }) =>
      auth.signUp(input),
    onSuccess: (session) => {
      queryClient.setQueryData(["session"], session);
    },
  });
}
