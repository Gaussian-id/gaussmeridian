import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../dropdown-menu";

function TestMenu({ onSelectItem }: { onSelectItem: () => void }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger>Open menu</DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuLabel>Account</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={onSelectItem}>Do the thing</DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

describe("DropdownMenu", () => {
  it("renders its content only after the trigger is activated", async () => {
    const user = userEvent.setup();
    render(<TestMenu onSelectItem={vi.fn()} />);

    expect(screen.queryByText("Do the thing")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Open menu" }));

    expect(await screen.findByText("Do the thing")).toBeInTheDocument();
    expect(screen.getByText("Account")).toBeInTheDocument();
  });

  it("calls onSelect and closes when an item is activated", async () => {
    const user = userEvent.setup();
    const onSelectItem = vi.fn();
    render(<TestMenu onSelectItem={onSelectItem} />);

    await user.click(screen.getByRole("button", { name: "Open menu" }));
    await user.click(await screen.findByText("Do the thing"));

    expect(onSelectItem).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.queryByText("Do the thing")).not.toBeInTheDocument());
  });

  it("closes on Escape", async () => {
    const user = userEvent.setup();
    render(<TestMenu onSelectItem={vi.fn()} />);

    await user.click(screen.getByRole("button", { name: "Open menu" }));
    expect(await screen.findByText("Do the thing")).toBeInTheDocument();

    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByText("Do the thing")).not.toBeInTheDocument());
  });
});
