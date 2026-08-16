import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "../sheet";

function TestSheet({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>Trace details</SheetTitle>
          <SheetDescription>The full route decision.</SheetDescription>
        </SheetHeader>
        <SheetBody>Body content</SheetBody>
      </SheetContent>
    </Sheet>
  );
}

describe("Sheet", () => {
  it("renders its content only when open", () => {
    const { rerender } = render(<TestSheet open={false} onOpenChange={vi.fn()} />);
    expect(screen.queryByText("Trace details")).not.toBeInTheDocument();

    rerender(<TestSheet open onOpenChange={vi.fn()} />);
    expect(screen.getByText("Trace details")).toBeInTheDocument();
    expect(screen.getByText("Body content")).toBeInTheDocument();
  });

  it("requests close via onOpenChange when Escape is pressed", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(<TestSheet open onOpenChange={onOpenChange} />);

    await user.keyboard("{Escape}");

    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("requests close via onOpenChange when the close button is clicked", async () => {
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(<TestSheet open onOpenChange={onOpenChange} />);

    await user.click(screen.getByRole("button", { name: /close/i }));

    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
