"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";

import { useAuth } from "@core/adapters";

/**
 * Sign-in mutation against the auth adapter. On success it **primes the `["session"]` cache** with
 * the returned session (the mirror of `useSignOut`, which clears it to `null`). Without this, the
 * navbar avatar — which reads `useSession()` — stays empty after login until a full page reload:
 * on the login page `useSession` already resolved to `null` (no cookie yet) and cached it, and
 * nothing else refreshes that entry post-login. Seeding it here makes the avatar appear immediately.
 */
export function useSignIn() {
  const auth = useAuth();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: { email: string; password: string }) => auth.signIn(input),
    onSuccess: (session) => {
      queryClient.setQueryData(["session"], session);
    },
  });
}
