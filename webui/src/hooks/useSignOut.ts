"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useRouter } from "next/navigation";

import { useAuth } from "@core/adapters";

/** Signs the current user out and returns them to `/login`. */
export function useSignOut() {
  const auth = useAuth();
  const router = useRouter();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => auth.signOut(),
    onSuccess: () => {
      queryClient.setQueryData(["session"], null);
      router.push("/login");
    },
  });
}
