import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { Prompt } from "../prompt";

describe("Prompt", () => {
  it("renders the kicker, title, and description", () => {
    render(
      <Prompt
        title="About you"
        kicker="Step 2"
        description="Tell us a bit more."
        onContinue={vi.fn()}
      />,
    );

    expect(screen.getByText("Step 2")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "About you" })).toBeInTheDocument();
    expect(screen.getByText("Tell us a bit more.")).toBeInTheDocument();
  });

  it("renders children inside the form body", () => {
    render(
      <Prompt title="Profile" onContinue={vi.fn()}>
        <p>step body</p>
      </Prompt>,
    );

    expect(screen.getByText("step body")).toBeInTheDocument();
  });

  it("calls onContinue when the Continue button is clicked", async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    render(<Prompt title="Profile" onContinue={onContinue} continueLabel="Next" />);

    await user.click(screen.getByRole("button", { name: "Next" }));
    expect(onContinue).toHaveBeenCalledTimes(1);
  });

  it("calls onContinue when Enter is pressed inside the form (native submit)", async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    render(
      <Prompt title="Profile" onContinue={onContinue}>
        <input aria-label="name" />
      </Prompt>,
    );

    await user.type(screen.getByLabelText("name"), "Ada{Enter}");
    expect(onContinue).toHaveBeenCalledTimes(1);
  });

  it("renders a Skip button and calls onSkip, independent of onContinue", async () => {
    const user = userEvent.setup();
    const onSkip = vi.fn();
    const onContinue = vi.fn();
    render(<Prompt title="Survey" onSkip={onSkip} onContinue={onContinue} />);

    await user.click(screen.getByRole("button", { name: "Skip" }));
    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(onContinue).not.toHaveBeenCalled();
  });

  it("renders a custom skipLabel when provided", () => {
    render(
      <Prompt title="Survey" onSkip={vi.fn()} skipLabel="Skip for now" onContinue={vi.fn()} />,
    );
    expect(screen.getByRole("button", { name: "Skip for now" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Skip" })).not.toBeInTheDocument();
  });

  it("does not render a Skip button when onSkip is not provided", () => {
    render(<Prompt title="Create org" onContinue={vi.fn()} />);
    expect(screen.queryByRole("button", { name: "Skip" })).not.toBeInTheDocument();
  });

  it("shows the error in an aria-live alert region when present", () => {
    render(<Prompt title="Create org" onContinue={vi.fn()} error="Could not save." />);

    const alert = screen.getByRole("alert");
    expect(alert).toHaveAttribute("aria-live", "polite");
    expect(alert).toHaveTextContent("Could not save.");
  });

  it("does not render an alert region when there is no error", () => {
    render(<Prompt title="Create org" onContinue={vi.fn()} />);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("disables Skip and Continue while isBusy", () => {
    render(<Prompt title="Create org" onSkip={vi.fn()} onContinue={vi.fn()} isBusy />);

    expect(screen.getByRole("button", { name: "Skip" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
  });

  it("does not call onContinue on submit while isBusy", async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    render(<Prompt title="Create org" onContinue={onContinue} isBusy />);

    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(onContinue).not.toHaveBeenCalled();
  });

  it("gives the heading tabIndex=-1 so it can receive programmatic focus", () => {
    render(<Prompt title="Welcome" onContinue={vi.fn()} />);
    expect(screen.getByRole("heading", { name: "Welcome" })).toHaveAttribute("tabindex", "-1");
  });

  it("disables only Continue for incomplete local input and explains why", async () => {
    const user = userEvent.setup();
    const onSkip = vi.fn();
    const onContinue = vi.fn();
    render(
      <Prompt
        title="Workspace"
        onSkip={onSkip}
        onContinue={onContinue}
        continueDisabled
        continueDisabledReason="Enter a workspace name to continue."
      />,
    );

    const continueButton = screen.getByRole("button", { name: "Continue" });
    const reason = screen.getByText("Enter a workspace name to continue.");
    expect(continueButton).toBeDisabled();
    expect(continueButton).toHaveAttribute("aria-describedby", reason.id);
    expect(screen.getByRole("button", { name: "Skip" })).toBeEnabled();

    await user.click(continueButton);
    expect(onContinue).not.toHaveBeenCalled();
  });

  it("keeps persistent guidance visible and described when Continue becomes enabled", () => {
    const onContinue = vi.fn();
    const { rerender } = render(
      <Prompt
        title="Workspace"
        onContinue={onContinue}
        guidance="Choose one option to continue."
        continueDisabled
        continueDisabledReason="A selection is required."
      />,
    );

    const guidance = screen.getByText("Choose one option to continue.");
    const reason = screen.getByText("A selection is required.");
    const continueButton = screen.getByRole("button", { name: "Continue" });
    expect(continueButton).toHaveAttribute("aria-describedby", `${guidance.id} ${reason.id}`);

    rerender(
      <Prompt
        title="Workspace"
        onContinue={onContinue}
        guidance="Choose one option to continue."
        continueDisabledReason="A selection is required."
      />,
    );

    const persistentGuidance = screen.getByText("Choose one option to continue.");
    expect(persistentGuidance.id).toBe(guidance.id);
    expect(screen.queryByText("A selection is required.")).not.toBeInTheDocument();
    expect(continueButton).toBeEnabled();
    expect(continueButton).toHaveAttribute("aria-describedby", persistentGuidance.id);
  });

  it("adds a local pointer affordance to both enabled Prompt actions", () => {
    const { rerender } = render(<Prompt title="Workspace" onSkip={vi.fn()} onContinue={vi.fn()} />);

    expect(screen.getByRole("button", { name: "Skip" })).toHaveClass("enabled:cursor-pointer");
    expect(screen.getByRole("button", { name: "Continue" })).toHaveClass("enabled:cursor-pointer");

    rerender(<Prompt title="Workspace" onSkip={vi.fn()} onContinue={vi.fn()} isBusy />);
    expect(screen.getByRole("button", { name: "Skip" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
  });

  it("blocks Enter submission while Continue is locally disabled", async () => {
    const user = userEvent.setup();
    const onContinue = vi.fn();
    render(
      <Prompt title="Profile" onContinue={onContinue} continueDisabled>
        <input aria-label="Full name" />
      </Prompt>,
    );

    await user.type(screen.getByLabelText("Full name"), "{Enter}");
    expect(onContinue).not.toHaveBeenCalled();
  });
});
