import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { ConfirmDestructiveDialog } from "../confirm-destructive-dialog";

/** Controlled harness — the dialog is fully controlled (`open`/`onOpenChange`), so tests own
 *  the open state, matching `floating-choices.test.tsx`'s house style. */
function Harness({
  resourceName,
  onConfirm,
  isBusy,
  error,
}: {
  resourceName?: string;
  onConfirm: () => void;
  isBusy?: boolean;
  error?: string | null;
}) {
  const [open, setOpen] = useState(true);
  return (
    <ConfirmDestructiveDialog
      open={open}
      onOpenChange={setOpen}
      title="Delete organization"
      resourceName={resourceName}
      resourceLabel="organization"
      confirmLabel="Delete organization"
      isBusy={isBusy}
      error={error}
      onConfirm={onConfirm}
      description="This permanently deletes Acme Labs. This cannot be undone."
      trigger={<button type="button">Open</button>}
    />
  );
}

describe("ConfirmDestructiveDialog", () => {
  it("renders as an alertdialog with the title/description wired via aria-labelledby/describedby", () => {
    render(<Harness resourceName="Acme Labs" onConfirm={vi.fn()} />);

    const dialog = screen.getByRole("alertdialog", { name: "Delete organization" });
    expect(dialog).toHaveAccessibleName("Delete organization");
    expect(dialog).toHaveAccessibleDescription(
      "This permanently deletes Acme Labs. This cannot be undone.",
    );
  });

  it("shows an explicit label naming the exact string to type", () => {
    render(<Harness resourceName="Acme Labs" onConfirm={vi.fn()} />);
    expect(screen.getByLabelText(/type the organization name to confirm/i)).toBeInTheDocument();
    expect(screen.getByText("Acme Labs")).toBeInTheDocument();
  });

  it("keeps the confirm button disabled until the typed value exactly matches (case-sensitive)", async () => {
    const user = userEvent.setup();
    render(<Harness resourceName="Acme Labs" onConfirm={vi.fn()} />);

    const confirmButton = screen.getByRole("button", { name: "Delete organization" });
    const input = screen.getByLabelText(/type the organization name to confirm/i);

    expect(confirmButton).toBeDisabled();

    await user.type(input, "acme labs"); // wrong case
    expect(confirmButton).toBeDisabled();

    await user.clear(input);
    await user.type(input, "Acme Lab"); // partial
    expect(confirmButton).toBeDisabled();

    await user.clear(input);
    await user.type(input, "Acme Labs"); // exact match
    expect(confirmButton).toBeEnabled();
  });

  it("trims leading/trailing whitespace on the typed value before comparing", async () => {
    const user = userEvent.setup();
    render(<Harness resourceName="Acme Labs" onConfirm={vi.fn()} />);

    const input = screen.getByLabelText(/type the organization name to confirm/i);
    await user.type(input, "  Acme Labs  ");

    expect(screen.getByRole("button", { name: "Delete organization" })).toBeEnabled();
  });

  it("calls onConfirm when the armed confirm button is clicked", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(<Harness resourceName="Acme Labs" onConfirm={onConfirm} />);

    await user.type(screen.getByLabelText(/type the organization name to confirm/i), "Acme Labs");
    await user.click(screen.getByRole("button", { name: "Delete organization" }));

    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("Enter submits once armed", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(<Harness resourceName="Acme Labs" onConfirm={onConfirm} />);

    const input = screen.getByLabelText(/type the organization name to confirm/i);
    await user.type(input, "Acme Labs{Enter}");

    expect(onConfirm).toHaveBeenCalledTimes(1);
  });

  it("Enter does nothing while unarmed (mismatched or empty input)", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(<Harness resourceName="Acme Labs" onConfirm={onConfirm} />);

    const input = screen.getByLabelText(/type the organization name to confirm/i);
    await user.type(input, "wrong name{Enter}");

    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("allows pasting the resource name (paste is never blocked)", async () => {
    const user = userEvent.setup();
    render(<Harness resourceName="Acme Labs" onConfirm={vi.fn()} />);

    const input = screen.getByLabelText(/type the organization name to confirm/i);
    await user.click(input);
    await user.paste("Acme Labs");

    expect(screen.getByRole("button", { name: "Delete organization" })).toBeEnabled();
  });

  it("arms immediately with no typed-confirmation input when resourceName is omitted", () => {
    render(<Harness onConfirm={vi.fn()} />);
    expect(screen.queryByLabelText(/type the .* name to confirm/i)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Delete organization" })).toBeEnabled();
  });

  it("disables the confirm and cancel buttons while busy", () => {
    render(<Harness resourceName="Acme Labs" onConfirm={vi.fn()} isBusy />);
    expect(screen.getByRole("button", { name: /delete organization/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
  });

  it("surfaces a mapped error via role=alert, never a raw error object", () => {
    render(
      <Harness
        resourceName="Acme Labs"
        onConfirm={vi.fn()}
        error="Could not delete the organization. Try again."
      />,
    );
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Could not delete the organization. Try again.",
    );
  });

  it("Escape closes the dialog and returns focus to the trigger", async () => {
    const user = userEvent.setup();
    render(<Harness resourceName="Acme Labs" onConfirm={vi.fn()} />);

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    await user.keyboard("{Escape}");

    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Open" })).toHaveFocus();
  });

  it("Cancel closes the dialog without calling onConfirm", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(<Harness resourceName="Acme Labs" onConfirm={onConfirm} />);

    await user.click(screen.getByRole("button", { name: "Cancel" }));

    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("resets the typed value after closing, so reopening starts unarmed again", async () => {
    const user = userEvent.setup();

    function ReopenHarness() {
      const [open, setOpen] = useState(true);
      return (
        <ConfirmDestructiveDialog
          open={open}
          onOpenChange={setOpen}
          title="Delete organization"
          resourceName="Acme Labs"
          resourceLabel="organization"
          confirmLabel="Delete organization"
          onConfirm={vi.fn()}
          description="Consequences."
          trigger={<button type="button">Open</button>}
        />
      );
    }

    render(<ReopenHarness />);
    await user.type(screen.getByLabelText(/type the organization name to confirm/i), "Acme Labs");
    expect(screen.getByRole("button", { name: "Delete organization" })).toBeEnabled();

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());

    await user.click(screen.getByRole("button", { name: "Open" }));
    expect(screen.getByLabelText(/type the organization name to confirm/i)).toHaveValue("");
    expect(screen.getByRole("button", { name: "Delete organization" })).toBeDisabled();
  });

  it("is fully keyboard-operable end to end: tab to input, type, tab to confirm, Enter activates", async () => {
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(<Harness resourceName="Acme Labs" onConfirm={onConfirm} />);

    const input = screen.getByLabelText(/type the organization name to confirm/i);
    expect(input).toHaveFocus(); // autoFocus

    await user.keyboard("Acme Labs");
    await user.tab(); // -> Cancel
    await user.tab(); // -> Confirm
    expect(screen.getByRole("button", { name: "Delete organization" })).toHaveFocus();

    await user.keyboard("{Enter}");
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });
});
