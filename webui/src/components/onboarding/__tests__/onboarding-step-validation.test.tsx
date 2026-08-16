import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { DataQueryInput } from "@core/adapters";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { OnboardingStepApiKey } from "../onboarding-step-api-key";
import { OnboardingStepCreateOrg } from "../onboarding-step-create-org";
import { OnboardingStepCreateProject } from "../onboarding-step-create-project";
import { OnboardingStepProfile } from "../onboarding-step-profile";
import { OnboardingStepSurvey } from "../onboarding-step-survey";

const originalClipboardDescriptor = Object.getOwnPropertyDescriptor(navigator, "clipboard");

afterEach(() => {
  if (originalClipboardDescriptor) {
    Object.defineProperty(navigator, "clipboard", originalClipboardDescriptor);
  } else {
    Reflect.deleteProperty(navigator, "clipboard");
  }
});

describe("OnboardingStepSurvey required answers", () => {
  it("requires an interest before opening team size", async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <OnboardingStepSurvey onNext={vi.fn()} />,
      createFakeRegistry({ data: { query: byResource({ "v1/onboarding/survey": undefined }) } }),
    );

    const continueButton = screen.getByRole("button", { name: "Continue" });
    const guidance = screen.getByText("Choose one option to continue.");
    expect(continueButton).toBeDisabled();
    expect(continueButton).toHaveAttribute("aria-describedby", guidance.id);
    expect(screen.getByRole("radiogroup", { name: "Primary interest" })).toHaveAttribute(
      "aria-required",
      "true",
    );

    await user.click(screen.getByRole("radio", { name: "Just exploring" }));
    expect(continueButton).toBeEnabled();
    expect(screen.getByText("Choose one option to continue.").id).toBe(guidance.id);
    expect(continueButton).toHaveAttribute("aria-describedby", guidance.id);
    await user.click(continueButton);
    expect(screen.getByRole("heading", { name: "How big is your team?" })).toBeInTheDocument();
  });

  it("requires team size before opening details", async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <OnboardingStepSurvey onNext={vi.fn()} />,
      createFakeRegistry({ data: { query: byResource({ "v1/onboarding/survey": undefined }) } }),
    );

    await user.click(screen.getByRole("radio", { name: "Just exploring" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    const continueButton = screen.getByRole("button", { name: "Continue" });
    const guidance = screen.getByText("Choose your team size to continue.");
    expect(continueButton).toBeDisabled();
    expect(continueButton).toHaveAttribute("aria-describedby", guidance.id);

    await user.click(screen.getByRole("radio", { name: "Just me" }));
    expect(continueButton).toBeEnabled();
    expect(screen.getByText("Choose your team size to continue.").id).toBe(guidance.id);
    expect(continueButton).toHaveAttribute("aria-describedby", guidance.id);
    await user.click(continueButton);
    expect(screen.getByRole("heading", { name: "One last thing." })).toBeInTheDocument();
  });

  it("requires a trimmed role while leaving referral optional", async () => {
    const user = userEvent.setup();
    const onNext = vi.fn();
    const requests: DataQueryInput<unknown>[] = [];
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/survey": (input: DataQueryInput<unknown>) => {
            requests.push(input);
            return undefined;
          },
        }),
      },
    });
    renderWithProviders(<OnboardingStepSurvey onNext={onNext} />, registry);

    await user.click(screen.getByRole("radio", { name: "Just exploring" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await user.click(screen.getByRole("radio", { name: "Just me" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));

    const role = screen.getByLabelText(/Role \/ title/i);
    expect(screen.getByLabelText(/How did you hear about us\? \(optional\)/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    await user.click(role);
    await user.tab();
    expect(screen.getByText("Enter your role to continue.")).toBeInTheDocument();

    await user.type(role, "   ");
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(requests).toHaveLength(0);

    await user.clear(role);
    await user.type(role, "  Platform engineer  ");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(onNext).toHaveBeenCalledTimes(1));
    expect(requests[0].body).toMatchObject({
      role_title: "Platform engineer",
      team_size: "Just me",
      primary_interest: "Just exploring",
      referral: undefined,
    });
  });
});

describe("OnboardingStepProfile required answer", () => {
  it("requires full name while display name and company remain optional", async () => {
    const user = userEvent.setup();
    const onNext = vi.fn();
    const requests: DataQueryInput<unknown>[] = [];
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/onboarding/profile": (input: DataQueryInput<unknown>) => {
            requests.push(input);
            return input.body;
          },
        }),
      },
    });
    renderWithProviders(<OnboardingStepProfile onNext={onNext} />, registry);

    const fullName = screen.getByLabelText(/Full name/i);
    expect(screen.getByLabelText(/Display name \(optional\)/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/Company \(optional\)/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    await user.click(fullName);
    await user.tab();
    expect(screen.getByText("Enter your full name to continue.")).toBeInTheDocument();

    await user.type(fullName, "   ");
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(requests).toHaveLength(0);

    await user.clear(fullName);
    await user.type(fullName, "  Ada Lovelace  ");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await waitFor(() => expect(onNext).toHaveBeenCalledTimes(1));
    expect(requests[0].body).toMatchObject({
      full_name: "Ada Lovelace",
      display_name: undefined,
      company: undefined,
    });
  });
});

describe("OnboardingStepCreateOrg", () => {
  it("keeps Create disabled when blank while leaving the explicit skip available", async () => {
    const user = userEvent.setup();
    const onSkip = vi.fn();
    const onNext = vi.fn();
    renderWithProviders(
      <OnboardingStepCreateOrg onNext={onNext} onSkip={onSkip} />,
      createFakeRegistry({
        data: {
          query: byResource({
            "v1/orgs": {
              id: "org_1",
              name: "Acme Inc.",
              slug: "acme-inc",
              plan: "free",
              created_at: "2026-07-16T00:00:00Z",
              member_count: 1,
              project_count: 0,
            },
          }),
        },
      }),
    );

    expect(screen.getByRole("button", { name: "Create workspace" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Skip workspace setup" }));
    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(onNext).not.toHaveBeenCalled();
  });
});

describe("OnboardingStepCreateProject", () => {
  it("does not create a project until a trimmed name exists", async () => {
    const user = userEvent.setup();
    const onNext = vi.fn();
    const requests: DataQueryInput<unknown>[] = [];
    renderWithProviders(
      <OnboardingStepCreateProject orgId="org_1" onNext={onNext} />,
      createFakeRegistry({
        data: {
          query: byResource({
            "v1/orgs/org_1/projects": (input: DataQueryInput<unknown>) => {
              requests.push(input);
              return {
                id: "proj_1",
                org_id: "org_1",
                name: "Production API",
                slug: "production-api",
                environment: "development",
                created_at: "2026-07-16T00:00:00Z",
              };
            },
          }),
        },
      }),
    );

    const createButton = screen.getByRole("button", { name: "Create project" });
    expect(createButton).toBeDisabled();
    expect(screen.getByText("Enter a project name to continue.")).toBeInTheDocument();
    await user.type(screen.getByLabelText(/Project name/i), "   ");
    await user.tab();
    expect(createButton).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("Enter a project name to continue.");
    expect(requests).toHaveLength(0);

    await user.clear(screen.getByLabelText(/Project name/i));
    await user.type(screen.getByLabelText(/Project name/i), "  Production API  ");
    await user.click(createButton);
    await waitFor(() => expect(onNext).toHaveBeenCalledWith("proj_1", "Production API"));
    expect(requests[0].body).toEqual({ name: "Production API" });
  });
});

describe("OnboardingStepApiKey copy gate", () => {
  it("keeps the final acknowledgement disabled until Copy is attempted", async () => {
    const user = userEvent.setup();
    const onNext = vi.fn();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    renderWithProviders(
      <OnboardingStepApiKey
        orgId="org_1"
        projectId="proj_1"
        projectName="Production API"
        onNext={onNext}
      />,
      createFakeRegistry({
        data: {
          query: byResource({
            "v1/orgs/org_1/projects/proj_1/keys": {
              key_id: "key_1",
              api_key: "sk-live-onceonly",
              key_prefix: "sk-live-",
              message: "Store this key securely — it will not be shown again.",
            },
          }),
        },
      }),
    );

    await user.click(screen.getByRole("button", { name: "Generate key" }));
    const savedButton = await screen.findByRole("button", { name: "I've saved it" });
    const keyValue = screen.getByRole("textbox", { name: "API key value" });
    expect(keyValue).toHaveAttribute("readonly");
    expect(keyValue).toHaveAttribute("data-sensitive", "true");
    await user.click(keyValue);
    expect(keyValue).toHaveFocus();
    expect(keyValue).toHaveProperty("selectionStart", 0);
    expect(keyValue).toHaveProperty("selectionEnd", "sk-live-onceonly".length);
    expect(savedButton).toBeDisabled();
    expect(screen.getByText("Copy your key to continue.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Copy API key" }));
    expect(writeText).toHaveBeenCalledWith("sk-live-onceonly");
    expect(await screen.findByText("Copied to clipboard.")).toBeInTheDocument();
    await waitFor(() => expect(savedButton).toBeEnabled());
    await user.click(savedButton);
    expect(onNext).toHaveBeenCalledTimes(1);
  });

  it("does not trap the user when clipboard permission is denied", async () => {
    const user = userEvent.setup();
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
    });
    renderWithProviders(
      <OnboardingStepApiKey
        orgId="org_1"
        projectId="proj_1"
        projectName="Production API"
        onNext={vi.fn()}
      />,
      createFakeRegistry({
        data: {
          query: byResource({
            "v1/orgs/org_1/projects/proj_1/keys": {
              key_id: "key_1",
              api_key: "sk-live-onceonly",
              key_prefix: "sk-live-",
              message: "Store this key securely — it will not be shown again.",
            },
          }),
        },
      }),
    );

    await user.click(screen.getByRole("button", { name: "Generate key" }));
    await user.click(await screen.findByRole("button", { name: "Copy API key" }));
    expect(await screen.findByText(/Automatic copy was unavailable/i)).toBeInTheDocument();
    const keyValue = screen.getByRole("textbox", { name: "API key value" });
    expect(keyValue).toHaveFocus();
    expect(keyValue).toHaveProperty("selectionStart", 0);
    expect(keyValue).toHaveProperty("selectionEnd", "sk-live-onceonly".length);
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "I've saved it" })).toBeEnabled(),
    );
    expect(screen.queryByText("Copied to clipboard.")).not.toBeInTheDocument();
  });

  it("provides the same manual fallback when the Clipboard API is absent", async () => {
    const user = userEvent.setup();
    Reflect.deleteProperty(navigator, "clipboard");
    renderWithProviders(
      <OnboardingStepApiKey
        orgId="org_1"
        projectId="proj_1"
        projectName="Production API"
        onNext={vi.fn()}
      />,
      createFakeRegistry({
        data: {
          query: byResource({
            "v1/orgs/org_1/projects/proj_1/keys": {
              key_id: "key_1",
              api_key: "sk-live-onceonly",
              key_prefix: "sk-live-",
              message: "Store this key securely — it will not be shown again.",
            },
          }),
        },
      }),
    );

    await user.click(screen.getByRole("button", { name: "Generate key" }));
    await user.click(await screen.findByRole("button", { name: "Copy API key" }));

    expect(await screen.findByText(/Automatic copy was unavailable/i)).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "API key value" })).toHaveFocus();
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "I've saved it" })).toBeEnabled(),
    );
    expect(screen.queryByText("Copied to clipboard.")).not.toBeInTheDocument();
  });

  it("clears the revealed key and acknowledgement when project identity changes", async () => {
    const user = userEvent.setup();
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/orgs/org_1/projects/proj_1/keys": {
            key_id: "key_1",
            api_key: "sk-live-project-one",
            key_prefix: "sk-live-",
            message: "Store this key securely — it will not be shown again.",
          },
          "v1/orgs/org_2/projects/proj_2/keys": {
            key_id: "key_2",
            api_key: "sk-live-project-two",
            key_prefix: "sk-live-",
            message: "Store this key securely — it will not be shown again.",
          },
        }),
      },
    });
    const { rerender } = renderWithProviders(
      <OnboardingStepApiKey
        orgId="org_1"
        projectId="proj_1"
        projectName="Project one"
        onNext={vi.fn()}
      />,
      registry,
    );

    await user.click(screen.getByRole("button", { name: "Generate key" }));
    expect(await screen.findByDisplayValue("sk-live-project-one")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copy API key" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "I've saved it" })).toBeEnabled(),
    );

    rerender(
      <OnboardingStepApiKey
        orgId="org_2"
        projectId="proj_2"
        projectName="Project two"
        onNext={vi.fn()}
      />,
    );

    expect(
      screen.getByRole("heading", { name: "Generate your first API key" }),
    ).toBeInTheDocument();
    expect(screen.queryByDisplayValue("sk-live-project-one")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Generate key" }));
    expect(await screen.findByDisplayValue("sk-live-project-two")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "I've saved it" })).toBeDisabled();
  });
});
