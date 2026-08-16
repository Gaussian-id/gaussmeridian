"use client";

import { useQuery } from "@tanstack/react-query";

import { useAuth } from "@core/adapters";

/** The current authenticated session, or null if signed out. */
export function useSession() {
  const auth = useAuth();
  return useQuery({
    queryKey: ["session"],
    queryFn: () => auth.getSession(),
  });
}
