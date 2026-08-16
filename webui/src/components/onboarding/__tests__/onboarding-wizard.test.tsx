import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { DataQueryInput } from "@core/adapters";
import type { Org, Project } from "@core/adapters/schemas/console.schema";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { OnboardingWizard } from "../onboarding-wizard";

const originalClipboardDescriptor = Object.getOwnPropertyDescriptor(navigator, "clipboard");

afterEach(() => {
  if (originalClipboardDescriptor) {
    Object.defineProperty(navigator, "clipboard", originalClipboardDescriptor);
  } else {
    Reflect.deleteProperty(navigator, "clipboard");
  }
});

const push = vi.fn();
vi.mock("next/navigation", () => ({
  useRouter: () => ({ push, replace: push }),
}));

// `<ConversationalStage>` mounts `<MeridianField>`, which dynamically imports `./earth-scene`
// (and, transitively, `three`) on mount. jsdom's canvas has no real WebGL context (see
// vitest.setup.ts — `getContext` is stubbed to return `null`), so the real `MeridianFieldScene`
// would throw trying to construct a `THREE.WebGLRenderer` against it. Stub the scene class with a
// no-op so these tests exercise the wizard's own logic without needing a WebGL context. A real
// `class` (not `vi.fn().mockImplementation(() => ({...}))`) is required here — `MeridianField`
// calls `new SceneCtor(...)`, and an arrow-function implementation isn't constructible.
class MockMeridianFieldScene {
  setStep = vi.fn();
  setReducedMotion = vi.fn();
  resize = vi.fn();
  dispose = vi.fn();
}
vi.mock("@/components/onboarding/meridian-field/earth-scene", () => ({
  MeridianFieldScene: MockMeridianFieldScene,
}));

function brandNewState() {
  return {
    current_step: null,
    completed_steps: [],
    onboarding_completed: false,
    workspace_disposition: "pending",
  };
}

function advanceEcho(input: DataQueryInput<unknown>) {
  const body = input.body as Record<string, unknown>;
  return {
    ...body,
    onboarding_completed: false,
    workspace_disposition: body.workspace_disposition ?? "pending",
  };
}

/**
 * Every step's title renders as an `<h1>` (`<Prompt>`, PRD-22 Phase C) AND, for skippable steps,
 * its `STEP_LABELS` name may also appear as a rail label (`onboarding-progress-rail.tsx`) — the
 * two aren't always the same string post-Phase-C (the rail keeps the short canonical step name;
 * the heading is the conversational copy), but tests still assert against the heading
 * specifically, never bare `findByText`, to stay unambiguous.
 */
function stepHeading(name: RegExp) {
  return screen.findByRole("heading", { name });
}

describe("OnboardingWizard — rendering each step", () => {
  it("exposes the initial loading state as a named main landmark", () => {
    const registry = createFakeRegistry({
      data: { query: () => new Promise<never>(() => {}) },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    const main = screen.getByRole("main", { name: "Loading onboarding" });
    expect(within(main).getByRole("status", { name: "Loading onboarding" })).toBeInTheDocument();
  });

  it("renders the welcome step for a brand-new user", async () => {
    const registry = createFakeRegistry({
      data: { query: byResource({ "v1/onboarding/state": brandNewState() }) },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    expect(await stepHeading(/you're in/i)).toBeInTheDocument();
  });

  it("reserves transparent inset and mobile rail space around the conversation", async () => {
    const registry = createFakeRegistry({
      data: { query: byResource({ "v1/onboarding/state": brandNewState() }) },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    await stepHeading(/you're in/i);

    const main = screen.getByRole("main", { name: "Onboarding question" });
    expect(main).toHaveAttribute("data-lenis-prevent");
    expect(main).toHaveClass("max-w-[29rem]", "overflow-y-auto");
    expect(main.firstElementChild).toHaveClass("p-3", "min-[760px]:p-4");
    expect(main.parentElement).toHaveClass(
      "pb-[calc(6rem+env(safe-area-inset-bottom))]",
      "min-[760px]:py-24",
    );
  });

  it("exposes status and progress as landmarks with a keyboard-scrollable mobile rail", async () => {
    const registry = createFakeRegistry({
      data: { query: byResource({ "v1/onboarding/state": brandNewState() }) },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    await stepHeading(/you're in/i);

    expect(screen.getByRole("complementary", { name: "Orbital status" })).toBeInTheDocument();
    expect(
      screen.getByRole("navigation", { name: "Desktop onboarding progress" }),
    ).toBeInTheDocument();
    const mobileProgress = screen.getByRole("navigation", {
      name: "Mobile onboarding progress",
    });
    const scrollSurface = within(mobileProgress).getByLabelText("Scrollable onboarding steps");
    expect(scrollSurface).toHaveAttribute("data-lenis-prevent");
    expect(scrollSurface).toHaveAttribute("tabindex", "0");
    expect(scrollSurface).toHaveClass("focus-visible:ring-2", "focus-visible:ring-inset");
  });

  it("advances from welcome to the survey step and persists the transition", async () => {
    let advanceCalls = 0;
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": brandNewState(),
          "v1/onboarding/advance": (input: DataQueryInput<unknown>) => {
            advanceCalls += 1;
            return advanceEcho(input);
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    fireEvent.click(await screen.findByRole("button", { name: /let's go/i }));
    expect(await stepHeading(/what brings you to meridian/i)).toBeInTheDocument();
    await waitFor(() => expect(advanceCalls).toBe(1));
  });
});

describe("OnboardingWizard — survey/profile are required steps (Shelby, onboarding-refinement)", () => {
  it("requires the minimum identity answers before reaching workspace setup", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "survey",
            completed_steps: ["welcome"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/onboarding/survey": undefined,
          "v1/onboarding/profile": (input: DataQueryInput<unknown>) => input.body,
          "v1/onboarding/advance": advanceEcho,
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    expect(await screen.findByRole("button", { name: "Continue" })).toBeDisabled();
    fireEvent.click(screen.getByRole("radio", { name: "Just exploring" }));
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    fireEvent.click(screen.getByRole("radio", { name: "Just me" }));
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Role \/ title/i), {
      target: { value: "Platform engineer" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    await stepHeading(/a little about you/i);
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Full name/i), { target: { value: "Ada Lovelace" } });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(await stepHeading(/create your first workspace/i)).toBeInTheDocument();
  });
});

describe("OnboardingWizard — create-org is optional", () => {
  it("skips directly to a truthful finish without creating an organization", async () => {
    let orgCreateCalls = 0;
    let advanceBody: Record<string, unknown> | undefined;
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "create_org",
            completed_steps: ["welcome", "survey", "profile"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/onboarding/advance": (input: DataQueryInput<unknown>) => {
            advanceBody = input.body as Record<string, unknown>;
            return advanceEcho(input);
          },
          "v1/orgs": (input: DataQueryInput<unknown>) => {
            if (input.method === "POST") orgCreateCalls += 1;
            return input.method === "POST" ? orgFixture() : { orgs: [] };
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    fireEvent.click(await screen.findByRole("button", { name: "Skip workspace setup" }));

    expect(await stepHeading(/your profile is ready/i)).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: /and your first project/i }),
    ).not.toBeInTheDocument();
    expect(orgCreateCalls).toBe(0);
    expect(advanceBody).toMatchObject({
      current_step: "finish",
      completed_steps: ["welcome", "survey", "profile"],
      workspace_disposition: "skipped",
    });
    expect(screen.getAllByText("skipped", { exact: true })).toHaveLength(6);
  });

  it("stays on Workspace and lets the user retry when skip persistence fails", async () => {
    let advanceCalls = 0;
    let orgCreateCalls = 0;
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "create_org",
            completed_steps: ["welcome", "survey", "profile"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/onboarding/advance": (input: DataQueryInput<unknown>) => {
            advanceCalls += 1;
            if (advanceCalls === 1) throw new Error("advance failed");
            return advanceEcho(input);
          },
          "v1/orgs": (input: DataQueryInput<unknown>) => {
            if (input.method === "POST") orgCreateCalls += 1;
            return input.method === "POST" ? orgFixture() : { orgs: [] };
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    fireEvent.click(await screen.findByRole("button", { name: "Skip workspace setup" }));

    expect(
      await screen.findByText("Could not skip workspace setup. Try again."),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /create your first workspace/i }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("heading", { name: /your profile is ready/i }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Skip workspace setup" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "Skip workspace setup" }));

    expect(await stepHeading(/your profile is ready/i)).toBeInTheDocument();
    expect(advanceCalls).toBe(2);
    expect(orgCreateCalls).toBe(0);
  });

  it("clears a stale skip error when the user switches to creating a workspace", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "create_org",
            completed_steps: ["welcome", "survey", "profile"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/onboarding/advance": () => {
            throw new Error("advance failed");
          },
          "v1/orgs": (input: DataQueryInput<unknown>) => {
            if (input.method === "POST") throw new Error("workspace create failed");
            return { orgs: [] };
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    fireEvent.click(await screen.findByRole("button", { name: "Skip workspace setup" }));
    expect(
      await screen.findByText("Could not skip workspace setup. Try again."),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/workspace name/i), {
      target: { value: "Acme Inc." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create workspace" }));

    expect(
      await screen.findByText("Could not create the workspace. Try again."),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Could not skip workspace setup. Try again."),
    ).not.toBeInTheDocument();
  });
});

describe("OnboardingWizard — resume at next incomplete step (US O8)", () => {
  it("a resuming bridge user with org+project already created lands on the API-key step", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "create_project",
            completed_steps: ["welcome", "survey", "profile", "create_org", "create_project"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/orgs": { orgs: [orgFixture()] },
          "v1/orgs/org_1/projects": { projects: [projectFixture()] },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    expect(await stepHeading(/generate your first api key/i)).toBeInTheDocument();
  });

  it("reflects the resumed position in the progress rail", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "profile",
            completed_steps: ["welcome", "survey"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    await stepHeading(/a little about you/i);
    const rails = screen.getAllByRole("list", { name: /onboarding progress/i });
    for (const rail of rails) {
      const current = within(rail)
        .getByText(/your profile/i)
        .closest("li");
      expect(current).toHaveAttribute("aria-current", "step");
    }
  });
});

describe("OnboardingWizard — required-step gate blocks Finish", () => {
  it("does not render the finish step until org, project, and key are all complete", async () => {
    // api_key isn't complete — nextIncomplete resolves to api_key, not finish, proving the
    // required-step gate holds without the bridge-excluded BYOK password step.
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "api_key",
            completed_steps: ["welcome", "survey", "profile", "create_org", "create_project"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/orgs": { orgs: [orgFixture()] },
          "v1/orgs/org_1/projects": { projects: [projectFixture()] },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    expect(await stepHeading(/generate your first api key/i)).toBeInTheDocument();
    expect(screen.queryByText(/you're all set/i)).not.toBeInTheDocument();
  });
});

describe("OnboardingWizard — recoverable dead-ends (Reviewer F1/F2)", () => {
  it("shows an error + Retry (not an infinite skeleton) when GET /onboarding/state fails", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": () => {
            throw new Error("503 service unavailable");
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    expect(await screen.findByText(/couldn't load your onboarding/i)).toBeInTheDocument();
    expect(screen.getByRole("main", { name: "Onboarding load error" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
    expect(screen.queryByRole("status", { name: /loading onboarding/i })).not.toBeInTheDocument();
  });

  it("recovers when resolving the persisted organization fails", async () => {
    let orgRequests = 0;
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "create_project",
            completed_steps: ["welcome", "survey", "profile", "create_org"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/orgs": () => {
            orgRequests += 1;
            if (orgRequests === 1) throw new Error("organization lookup failed");
            return { orgs: [orgFixture()] };
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    expect(await screen.findByText(/couldn't restore your workspace setup/i)).toBeInTheDocument();
    expect(screen.getByRole("main", { name: "Workspace restoration error" })).toBeInTheDocument();
    expect(screen.queryByRole("status", { name: /loading onboarding/i })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await stepHeading(/and your first project/i)).toBeInTheDocument();
    expect(orgRequests).toBe(2);
  });

  it("recovers when resolving the persisted project fails", async () => {
    let projectRequests = 0;
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "api_key",
            completed_steps: ["welcome", "survey", "profile", "create_org", "create_project"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/orgs": { orgs: [orgFixture()] },
          "v1/orgs/org_1/projects": () => {
            projectRequests += 1;
            if (projectRequests === 1) throw new Error("project lookup failed");
            return { projects: [projectFixture()] };
          },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);

    expect(await screen.findByText(/couldn't restore your workspace setup/i)).toBeInTheDocument();
    expect(screen.getByRole("main", { name: "Workspace restoration error" })).toBeInTheDocument();
    expect(screen.queryByRole("status", { name: /loading onboarding/i })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    expect(await stepHeading(/generate your first api key/i)).toBeInTheDocument();
    expect(projectRequests).toBe(2);
  });

  it("rolls back to create_org when create_org is marked complete but the user has no orgs", async () => {
    // Cross-device resume where the org was deleted out-of-band: completed_steps claims
    // create_org, but the org list comes back empty — the wizard must re-render create_org,
    // not strand the user on an empty card.
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "create_project",
            completed_steps: ["welcome", "survey", "profile", "create_org", "api_key"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/orgs": { orgs: [] },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    expect(await stepHeading(/create your first workspace/i)).toBeInTheDocument();
    for (const label of screen.getAllByText("API key", { exact: true })) {
      const item = label.closest("li");
      expect(item).not.toBeNull();
      expect(within(item!).getByText("upcoming")).toHaveClass("sr-only");
      expect(item!.querySelector(".lucide-check")).toBeNull();
    }
  });

  it("rolls back to create_project when create_project is complete but the org has no projects", async () => {
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "api_key",
            completed_steps: [
              "welcome",
              "survey",
              "profile",
              "create_org",
              "create_project",
              "api_key",
            ],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/orgs": { orgs: [orgFixture()] },
          "v1/orgs/org_1/projects": { projects: [] },
        }),
      },
    });
    renderWithProviders(<OnboardingWizard />, registry);
    expect(await stepHeading(/and your first project/i)).toBeInTheDocument();
    for (const label of screen.getAllByText("API key", { exact: true })) {
      const item = label.closest("li");
      expect(item).not.toBeNull();
      expect(within(item!).getByText("upcoming")).toHaveClass("sr-only");
      expect(item!.querySelector(".lucide-check")).toBeNull();
    }
  });
});

describe("OnboardingWizard — the required happy path through to Finish", () => {
  it("creates an org, a project, a key, then opens that project", async () => {
    let orgs: Org[] = [];
    let projects: Project[] = [];
    let completeCalled = false;
    const advanceBodies: Record<string, unknown>[] = [];
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/state": {
            current_step: "create_org",
            completed_steps: ["welcome", "survey", "profile"],
            onboarding_completed: false,
            workspace_disposition: "pending",
          },
          "v1/onboarding/advance": (input: DataQueryInput<unknown>) => {
            advanceBodies.push(input.body as Record<string, unknown>);
            return advanceEcho(input);
          },
          "v1/orgs": (input: DataQueryInput<unknown>) => {
            if (input.method === "POST") {
              const created = orgFixture();
              orgs = [created];
              return created;
            }
            return { orgs };
          },
          "v1/orgs/org_1/projects": (input: DataQueryInput<unknown>) => {
            if (input.method === "POST") {
              const created = projectFixture();
              projects = [created];
              return created;
            }
            return { projects };
          },
          "v1/orgs/org_1/projects/proj_1/keys": {
            key_id: "key_1",
            api_key: "sk-live-onceonly",
            key_prefix: "sk-live-",
            message: "Store this key securely — it will not be shown again.",
          },
          "v1/onboarding/complete": () => {
            completeCalled = true;
            return { onboarding_completed: true };
          },
        }),
      },
    });

    renderWithProviders(<OnboardingWizard />, registry);

    fireEvent.change(await screen.findByLabelText(/workspace name/i), {
      target: { value: "Acme Inc." },
    });
    fireEvent.click(screen.getByRole("button", { name: /^create workspace$/i }));

    expect(await screen.findByRole("button", { name: "Create project" })).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/project name/i), {
      target: { value: "Production API" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^create project$/i }));

    fireEvent.click(await screen.findByRole("button", { name: /generate key/i }));
    await screen.findByDisplayValue("sk-live-onceonly");
    expect(screen.getByRole("button", { name: /i've saved it/i })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: /copy api key/i }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("sk-live-onceonly"));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: /i've saved it/i })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: /i've saved it/i }));

    fireEvent.click(await screen.findByRole("button", { name: /open dashboard/i }));

    await waitFor(() => expect(completeCalled).toBe(true));
    await waitFor(() => expect(push).toHaveBeenCalledWith("/orgs/org_1/projects/proj_1"));
    expect(advanceBodies).toHaveLength(3);
    expect(advanceBodies.every((body) => !("workspace_disposition" in body))).toBe(true);
  });
});

function orgFixture(): Org {
  return {
    id: "org_1",
    name: "Acme Inc.",
    slug: "acme-inc",
    plan: "free",
    created_at: "2026-07-16T00:00:00Z",
    member_count: 1,
    project_count: 0,
  };
}

function projectFixture(): Project {
  return {
    id: "proj_1",
    org_id: "org_1",
    name: "Production API",
    slug: "production-api",
    environment: "development",
    created_at: "2026-07-16T00:00:00Z",
  };
}
