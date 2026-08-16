"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { z } from "zod";

import { useDataQuery, type AuthSession } from "@core/adapters";
import {
  OnboardingCompleteResponseSchema,
  OnboardingProfileResponseSchema,
  OnboardingStateResponseSchema,
} from "@core/adapters/schemas/console.schema";
import type { WorkspaceDisposition } from "@core/adapters/schemas/console.schema";
import {
  ONBOARDING_ADVANCE_RESOURCE,
  ONBOARDING_COMPLETE_RESOURCE,
  ONBOARDING_PROFILE_RESOURCE,
  ONBOARDING_STATE_RESOURCE,
  ONBOARDING_SURVEY_RESOURCE,
} from "@core/config/resources";

import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";

import { useResourceQuery } from "./useResourceQuery";

/** `GET /v1/onboarding/state` — the caller's persisted progress (US O8 resume). */
export function useOnboardingState() {
  return useResourceQuery({
    resource: ONBOARDING_STATE_RESOURCE,
    schema: OnboardingStateResponseSchema,
  });
}

/**
 * `POST /v1/onboarding/advance` — commits `current_step` (+ the full `completed_steps` array,
 * which the wizard's step-machine computes client-side) server-side, so a resuming session
 * lands back on the right step. Invalidates the state query so a refetch (e.g. after a browser
 * back/forward) sees the committed progress.
 */
export function useAdvanceOnboarding() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      currentStep: OnboardingStep;
      completedSteps: OnboardingStep[];
      workspaceDisposition?: WorkspaceDisposition;
    }) =>
      data.query({
        resource: ONBOARDING_ADVANCE_RESOURCE,
        method: "POST",
        body: {
          current_step: input.currentStep,
          completed_steps: input.completedSteps,
          ...(input.workspaceDisposition
            ? { workspace_disposition: input.workspaceDisposition }
            : {}),
        },
        schema: OnboardingStateResponseSchema,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ONBOARDING_STATE_RESOURCE, null] });
    },
  });
}

/** `POST /v1/onboarding/survey` — the transport accepts partial fields, while the onboarding UI
 *  requires interest, team size, and role/title before invoking it. 204 No Content. */
export function useSaveSurvey() {
  const data = useDataQuery();
  return useMutation({
    mutationFn: (input: {
      role_title?: string;
      team_size?: string;
      primary_interest?: string;
      referral?: string;
    }) =>
      data.query({
        resource: ONBOARDING_SURVEY_RESOURCE,
        method: "POST",
        body: input,
        schema: z.unknown(),
      }),
  });
}

/** `PATCH /v1/onboarding/profile` — the transport remains a partial update, while the onboarding
 *  UI requires full name and keeps display name/company optional. */
export function useSaveProfile() {
  const data = useDataQuery();
  return useMutation({
    mutationFn: (input: {
      full_name?: string;
      display_name?: string;
      company?: string;
      timezone?: string;
    }) =>
      data.query({
        resource: ONBOARDING_PROFILE_RESOURCE,
        method: "PATCH",
        body: input,
        schema: OnboardingProfileResponseSchema,
      }),
  });
}

/**
 * `POST /v1/onboarding/complete` is the existing integration boundary for both frontend Finish
 * variants. The server remains the authority on completion policy; any rejection is surfaced by
 * the wizard without changing either UI branch here. A successful response is committed into the
 * existing session cache so the `(app)` gate and navigation observe one atomic transition.
 */
export function useCompleteOnboarding() {
  const data = useDataQuery();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () =>
      data.query({
        resource: ONBOARDING_COMPLETE_RESOURCE,
        method: "POST",
        schema: OnboardingCompleteResponseSchema,
      }),
    onSuccess: (response) => {
      queryClient.setQueryData<AuthSession | null>(["session"], (session) =>
        session ? { ...session, onboardingCompleted: response.onboarding_completed } : session,
      );
    },
  });
}
