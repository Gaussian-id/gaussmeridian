import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TenancyProvider } from "@core/providers";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { CommandPaletteProvider } from "../command-palette-provider";

const push = vi.fn();

vi.mock("next/navigation", () => ({
  useParams: () => ({}),
  useRouter: () => ({ push }),
}));

function pressCtrlK() {
  fireEvent.keyDown(window, { key: "k", ctrlKey: true });
}

function setup() {
  const registry = createFakeRegistry({
    data: {
      query: byResource({
        "v1/orgs": { orgs: [] },
        "v1/models": { data: [] },
      }),
    },
  });
  return renderWithProviders(
    <TenancyProvider>
      <CommandPaletteProvider>
        <div>console</div>
      </CommandPaletteProvider>
    </TenancyProvider>,
    registry,
  );
}

describe("CommandPalette", () => {
  it("is closed until the global ⌘K/Ctrl+K shortcut fires, then opens", () => {
    setup();

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    pressCtrlK();

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/search or jump to/i)).toBeInTheDocument();
  });

  it('typing "CARROT" and selecting the entry swaps in the complexity explainer', async () => {
    setup();
    pressCtrlK();

    const input = screen.getByPlaceholderText(/search or jump to/i);
    fireEvent.change(input, { target: { value: "CARROT" } });

    const entry = await screen.findByText(/CARROT — complexity explainer/i);
    fireEvent.click(entry);

    await waitFor(() =>
      expect(screen.getByText(/CARROT is GaussMeridian's complexity scorer/i)).toBeInTheDocument(),
    );
    // The search list is swapped out, not just hidden alongside it.
    expect(screen.queryByPlaceholderText(/search or jump to/i)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /back/i }));
    expect(screen.getByPlaceholderText(/search or jump to/i)).toBeInTheDocument();
  });

  it("navigates and closes when a global item is selected", () => {
    push.mockClear();
    setup();
    pressCtrlK();

    fireEvent.click(screen.getByText("Playground"));

    expect(push).toHaveBeenCalledWith("/playground");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
