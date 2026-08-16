"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useOrgProjects, useOrgs } from "@/hooks/useConsoleQueries";
import {
  useAdvanceOnboarding,
  useCompleteOnboarding,
  useOnboardingState,
} from "@/hooks/useOnboarding";
import { useSession } from "@/hooks/useSession";
import {
  fromServerState,
  hasSkippedWorkspaceSetup,
  markComplete,
  markWorkspaceSetupSkipped,
  nextIncomplete,
  toCompletedStepsArray,
} from "@/lib/onboarding/onboarding-machine";
import type { OnboardingMachineState, OnboardingStep } from "@/lib/onboarding/onboarding-machine";
import { CAPITAL_POOL, pickRoute } from "@/lib/onboarding/route";

import { ConversationalStage } from "./conversational-stage";
import { OnboardingStepApiKey } from "./onboarding-step-api-key";
import { OnboardingStepCreateOrg } from "./onboarding-step-create-org";
import { OnboardingStepCreateProject } from "./onboarding-step-create-project";
import { OnboardingStepFinish } from "./onboarding-step-finish";
import { OnboardingStepProfile } from "./onboarding-step-profile";
import { OnboardingStepSurvey } from "./onboarding-step-survey";
import { OnboardingStepWelcome } from "./onboarding-step-welcome";

/**
 * The gated bridge onboarding wizard. Owns:
 * - the pure step machine's React-side state (`lib/onboarding/onboarding-machine.ts` does the
 *   actual logic — this component just holds `{ currentStep, completed, skipped }` and persists
 *   transitions via `POST /v1/onboarding/advance`, including the conditional setup branch),
 * - the org/project ids created mid-flow (needed by the project-scoped API-key step).
 *
 * On resume (a returning, not-yet-onboarded user — possibly on a different device), the org/
 * project ids created in an earlier session aren't in local state yet. If the persisted
 * `completed_steps` shows those steps already done, this component resolves the ids by reading
 * them back (`useOrgs`/`useOrgProjects`) rather than getting stuck unable to render a later step.
 */
export function OnboardingWizard({ completionHref = "/orgs" }: { completionHref?: string }) {
  const router = useRouter();
  const session = useSession();
  const onboardingState = useOnboardingState();
  const advance = useAdvanceOnboarding();
  const complete = useCompleteOnboarding();

  const [machine, setMachine] = useState<OnboardingMachineState | null>(null);
  const [orgId, setOrgId] = useState<string | null>(null);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [projectName, setProjectName] = useState("");
  const [isSkippingWorkspace, setIsSkippingWorkspace] = useState(false);
  const [skipWorkspaceError, setSkipWorkspaceError] = useState<string>();
  // Picked once per session (PRD-22 Phase C) and passed down by identity — MeridianField's mount
  // effect depends on `route` staying referentially stable for the wizard's lifetime.
  const [route] = useState(() => pickRoute(CAPITAL_POOL));

  // Initialize once from the server's persisted state (US O8), and resolve org/project ids
  // once their steps are known-complete but not yet known locally (a cross-device resume).
  // These are computed directly during render — React's documented "derive once, bail out via
  // the already-set guard" pattern — rather than in a `useEffect`, so there's no extra
  // commit-then-re-render round trip and no risk of a stale-refetch race overwriting
  // in-progress local state (the `=== null` guards make each of these fire at most once).
  if (machine === null && onboardingState.data) {
    setMachine(fromServerState(onboardingState.data));
  }

  const needsOrgResolved = Boolean(machine?.completed.has("create_org")) && orgId === null;
  const orgs = useOrgs();
  if (needsOrgResolved && orgId === null && orgs.data?.orgs.length) {
    setOrgId(orgs.data.orgs[0].id);
  }

  const needsProjectResolved =
    Boolean(machine?.completed.has("create_project")) && projectId === null;
  const projects = useOrgProjects(orgId ?? "");
  if (needsProjectResolved && projectId === null && projects.data?.projects.length) {
    const project = projects.data.projects[0];
    setProjectId(project.id);
    setProjectName(project.name);
  }

  // Resume-integrity repair (Reviewer F2): a step marked complete server-side whose backing
  // resource has since vanished — the org/project was deleted out-of-band, or the persisted
  // `completed_steps` is inconsistent with the actual tenant. The resolve blocks above can't
  // set an id from an empty list, so without this the later step guards (which require
  // `orgId`/`projectId`) would all fail and the user would stare at an empty card with no way
  // forward. Instead, roll the wizard back: un-complete the missing resource step and every
  // downstream resource completion, so `nextIncomplete` lands the user back on
  // `create_org`/`create_project` to re-create it. Fires at most once per missing
  // resource — removing the step from `completed` makes its `needs…Resolved` guard false on the
  // next render, so there's no loop.
  const orgMissing =
    needsOrgResolved && !orgs.isLoading && !!orgs.data && orgs.data.orgs.length === 0;
  const projectMissing =
    needsProjectResolved &&
    !projects.isLoading &&
    !!projects.data &&
    projects.data.projects.length === 0;
  if (machine && (orgMissing || projectMissing)) {
    const repaired = new Set<OnboardingStep>(machine.completed);
    if (orgMissing) {
      repaired.delete("create_org");
      repaired.delete("create_project");
      repaired.delete("api_key");
    }
    if (projectMissing) {
      repaired.delete("create_project");
      repaired.delete("api_key");
    }
    setMachine({
      currentStep: nextIncomplete(repaired, machine.skipped),
      completed: repaired,
      skipped: machine.skipped,
    });
  }

  function baseMachine(): OnboardingMachineState {
    return (
      machine ?? {
        currentStep: "welcome",
        completed: new Set<OnboardingStep>(),
        skipped: new Set<OnboardingStep>(),
      }
    );
  }

  function goTo(nextStep: OnboardingStep, completedStep?: OnboardingStep) {
    const base = baseMachine();
    const completed = completedStep ? markComplete(base.completed, completedStep) : base.completed;

    // State updaters must be pure: React may invoke them more than once in development to detect
    // side effects. Persisting from inside the updater duplicated `/onboarding/advance` and could
    // leave the visible step behind a pending mutation. Commit local navigation once, then issue
    // the independent resumability write.
    setMachine({ currentStep: nextStep, completed, skipped: base.skipped });
    advance.mutate({ currentStep: nextStep, completedSteps: toCompletedStepsArray(completed) });
  }

  function advancePast(step: OnboardingStep) {
    const base = baseMachine();
    const completed = markComplete(base.completed, step);
    goTo(nextIncomplete(completed, base.skipped), step);
  }

  function skipWorkspace() {
    const next = markWorkspaceSetupSkipped(baseMachine().completed);
    setSkipWorkspaceError(undefined);
    setIsSkippingWorkspace(true);
    advance.mutate(
      {
        currentStep: next.currentStep,
        completedSteps: toCompletedStepsArray(next.completed),
        workspaceDisposition: "skipped",
      },
      {
        onSuccess: () => setMachine(next),
        onError: () => setSkipWorkspaceError("Could not skip workspace setup. Try again."),
        onSettled: () => setIsSkippingWorkspace(false),
      },
    );
  }

  // Error branch (Reviewer F1): if `GET /v1/onboarding/state` fails (503 when the repo slot is
  // None, a 500, or a network blip), `machine` never initializes and the skeleton would spin
  // forever. Give the user an explicit, recoverable dead-end escape — the same `role="alert"
  // aria-live` pattern the per-step forms use — with a Retry that re-runs the query.
  if (onboardingState.isError) {
    return (
      <main
        aria-label="Onboarding load error"
        className="flex min-h-dvh items-center justify-center px-4"
      >
        <Card className="flex w-full max-w-sm flex-col gap-4 p-6 text-center">
          <div role="alert" aria-live="assertive">
            <CardTitle>We couldn&apos;t load your onboarding</CardTitle>
            <CardDescription className="mt-1">
              Something went wrong reaching the server. Check your connection and try again.
            </CardDescription>
          </div>
          <Button
            type="button"
            variant="accent"
            size="lg"
            onClick={() => onboardingState.refetch()}
            disabled={onboardingState.isFetching}
          >
            {onboardingState.isFetching ? "Retrying…" : "Retry"}
          </Button>
        </Card>
      </main>
    );
  }

  const orgResolutionFailed = needsOrgResolved && orgs.isError;
  const projectResolutionFailed = needsProjectResolved && projects.isError;
  if (orgResolutionFailed || projectResolutionFailed) {
    const isRetrying =
      (orgResolutionFailed && orgs.isFetching) || (projectResolutionFailed && projects.isFetching);

    return (
      <main
        aria-label="Workspace restoration error"
        className="flex min-h-dvh items-center justify-center px-4"
      >
        <Card className="flex w-full max-w-sm flex-col gap-4 p-6 text-center">
          <div role="alert" aria-live="assertive">
            <CardTitle>We couldn&apos;t restore your workspace setup</CardTitle>
            <CardDescription className="mt-1">
              Your saved progress is safe. Retry loading the workspace resources needed to continue.
            </CardDescription>
          </div>
          <Button
            type="button"
            variant="accent"
            size="lg"
            onClick={() => {
              if (orgResolutionFailed) void orgs.refetch();
              if (projectResolutionFailed) void projects.refetch();
            }}
            disabled={isRetrying}
          >
            {isRetrying ? "Retrying…" : "Retry"}
          </Button>
        </Card>
      </main>
    );
  }

  const stillResolvingIds =
    (needsOrgResolved && orgs.isLoading) || (needsProjectResolved && projects.isLoading);

  if (
    onboardingState.isLoading ||
    machine === null ||
    stillResolvingIds ||
    orgMissing ||
    projectMissing
  ) {
    return (
      <main
        aria-label="Loading onboarding"
        className="flex min-h-dvh items-center justify-center px-4"
      >
        <div
          className="flex w-full max-w-sm flex-col gap-3"
          role="status"
          aria-label="Loading onboarding"
        >
          <Skeleton className="h-6 w-2/3" />
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      </main>
    );
  }

  const { currentStep, completed, skipped } = machine;
  const workspaceSkipped = hasSkippedWorkspaceSetup(skipped);

  return (
    <ConversationalStage
      currentStep={currentStep}
      completed={completed}
      skipped={skipped}
      route={route}
    >
      {currentStep === "welcome" && (
        <OnboardingStepWelcome
          displayName={session.data?.displayName}
          onNext={() => advancePast("welcome")}
        />
      )}

      {currentStep === "survey" && <OnboardingStepSurvey onNext={() => advancePast("survey")} />}

      {currentStep === "profile" && <OnboardingStepProfile onNext={() => advancePast("profile")} />}

      {currentStep === "create_org" && (
        <OnboardingStepCreateOrg
          onSkip={skipWorkspace}
          onCreateIntent={() => setSkipWorkspaceError(undefined)}
          isSkipping={isSkippingWorkspace}
          skipError={skipWorkspaceError}
          onNext={(newOrgId) => {
            setOrgId(newOrgId);
            advancePast("create_org");
          }}
        />
      )}

      {currentStep === "create_project" && orgId && (
        <OnboardingStepCreateProject
          orgId={orgId}
          onNext={(newProjectId, newProjectName) => {
            setProjectId(newProjectId);
            setProjectName(newProjectName);
            advancePast("create_project");
          }}
        />
      )}

      {currentStep === "api_key" && orgId && projectId && (
        <OnboardingStepApiKey
          orgId={orgId}
          projectId={projectId}
          projectName={projectName}
          onNext={() => advancePast("api_key")}
        />
      )}

      {currentStep === "finish" && (
        <OnboardingStepFinish
          workspaceSkipped={workspaceSkipped}
          isPending={complete.isPending}
          error={
            complete.isError
              ? workspaceSkipped
                ? "Could not finish onboarding. Try again."
                : "Your organization, project, and API key all need to be set up first."
              : undefined
          }
          onFinish={() =>
            complete.mutate(undefined, {
              onSuccess: () =>
                router.push(
                  completionHref !== "/orgs"
                    ? completionHref
                    : orgId && projectId
                      ? `/orgs/${orgId}/projects/${projectId}`
                      : "/orgs",
                ),
            })
          }
        />
      )}
    </ConversationalStage>
  );
}
