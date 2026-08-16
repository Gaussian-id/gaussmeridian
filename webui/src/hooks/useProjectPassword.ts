"use client";

import { useMutation } from "@tanstack/react-query";
import { z } from "zod";

import { useDataQuery } from "@core/adapters";
import { projectPasswordResource, projectPasswordVerifyResource } from "@core/config/resources";

/**
 * `POST /v1/projects/:id/password` — sets/replaces the project's optional "project password"
 * (`access_secret`, DR-010 D4), a second factor guarding that project's BYOK vault (O6/P5).
 * Argon2id-hashed server-side; the plaintext is sent once and never stored or echoed back.
 * 204 No Content on success.
 */
export function useSetProjectPassword(projectId: string) {
  const data = useDataQuery();
  return useMutation({
    mutationFn: (input: { password: string }) =>
      data.query({
        resource: projectPasswordResource(projectId),
        method: "POST",
        body: input,
        schema: z.unknown(),
      }),
  });
}

/** `POST /v1/projects/:id/password/verify` — 200 on match, throws (401) on mismatch or when
 *  no password has been set. Not used by the wizard itself (which only ever sets a fresh
 *  password), but the recovery path (US #48) needs it — kept alongside the setter. */
// Reserved for the US#48 project-password recovery UI (project settings) — tracked in
// Team-Decision-Log 2026-07-16, not wired in Wave B.
export function useVerifyProjectPassword(projectId: string) {
  const data = useDataQuery();
  return useMutation({
    mutationFn: (input: { password: string }) =>
      data.query({
        resource: projectPasswordVerifyResource(projectId),
        method: "POST",
        body: input,
        schema: z.unknown(),
      }),
  });
}
