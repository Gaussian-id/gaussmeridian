import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { FloatingChoices, type FloatingChoiceItem } from "../floating-choices";

const OPTIONS: FloatingChoiceItem[] = [
  { value: "cost", label: "Cost savings" },
  { value: "quality", label: "Routing quality" },
  { value: "governance", label: "Compliance" },
];

function Controlled({
  onChangeSpy,
  initial = null,
}: {
  onChangeSpy: (v: string | null) => void;
  initial?: string | null;
}) {
  return <FloatingChoicesHarness options={OPTIONS} initial={initial} onChangeSpy={onChangeSpy} />;
}

// A tiny stateful wrapper — FloatingChoices is controlled, so the test owns `value`.
function FloatingChoicesHarness({
  options,
  initial,
  onChangeSpy,
}: {
  options: FloatingChoiceItem[];
  initial: string | null;
  onChangeSpy: (v: string | null) => void;
}) {
  const [value, setValue] = useState(initial);
  return (
    <FloatingChoices
      options={options}
      value={value}
      ariaLabel="Primary interest"
      onChange={(v: string | null) => {
        setValue(v);
        onChangeSpy(v);
      }}
    />
  );
}

describe("FloatingChoices", () => {
  it("renders a radiogroup with one radio per option", () => {
    render(
      <FloatingChoices
        options={OPTIONS}
        value={null}
        onChange={vi.fn()}
        ariaLabel="Primary interest"
      />,
    );

    expect(screen.getByRole("radiogroup", { name: "Primary interest" })).toBeInTheDocument();
    expect(screen.getAllByRole("radio")).toHaveLength(3);
  });

  it("marks the selected option aria-checked=true and the rest false", () => {
    render(
      <FloatingChoices
        options={OPTIONS}
        value="quality"
        onChange={vi.fn()}
        ariaLabel="Primary interest"
      />,
    );

    expect(screen.getByRole("radio", { name: "Routing quality" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("radio", { name: "Cost savings" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });

  it("clicking an option calls onChange with its value", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <FloatingChoices
        options={OPTIONS}
        value={null}
        onChange={onChange}
        ariaLabel="Primary interest"
      />,
    );

    await user.click(screen.getByRole("radio", { name: "Cost savings" }));
    expect(onChange).toHaveBeenCalledWith("cost");
  });

  it("only the selected option is tab-focusable when a value is set (roving tabindex)", () => {
    render(
      <FloatingChoices
        options={OPTIONS}
        value="quality"
        onChange={vi.fn()}
        ariaLabel="Primary interest"
      />,
    );

    expect(screen.getByRole("radio", { name: "Routing quality" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("radio", { name: "Cost savings" })).toHaveAttribute("tabindex", "-1");
    expect(screen.getByRole("radio", { name: "Compliance" })).toHaveAttribute("tabindex", "-1");
  });

  it("only the first option is tab-focusable when nothing is selected yet", () => {
    render(
      <FloatingChoices
        options={OPTIONS}
        value={null}
        onChange={vi.fn()}
        ariaLabel="Primary interest"
      />,
    );

    expect(screen.getByRole("radio", { name: "Cost savings" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("radio", { name: "Routing quality" })).toHaveAttribute(
      "tabindex",
      "-1",
    );
  });

  it("ArrowRight/ArrowDown moves selection and focus to the next option", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <FloatingChoices
        options={OPTIONS}
        value="cost"
        onChange={onChange}
        ariaLabel="Primary interest"
      />,
    );

    screen.getByRole("radio", { name: "Cost savings" }).focus();
    await user.keyboard("{ArrowRight}");

    expect(onChange).toHaveBeenLastCalledWith("quality");
  });

  it("ArrowLeft/ArrowUp moves selection and focus to the previous option, wrapping at the start", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <FloatingChoices
        options={OPTIONS}
        value="cost"
        onChange={onChange}
        ariaLabel="Primary interest"
      />,
    );

    screen.getByRole("radio", { name: "Cost savings" }).focus();
    await user.keyboard("{ArrowLeft}");

    expect(onChange).toHaveBeenLastCalledWith("governance");
  });

  it("Enter selects the focused option", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <FloatingChoices
        options={OPTIONS}
        value={null}
        onChange={onChange}
        ariaLabel="Primary interest"
      />,
    );

    screen.getByRole("radio", { name: "Routing quality" }).focus();
    await user.keyboard("{Enter}");

    expect(onChange).toHaveBeenCalledWith("quality");
  });

  it("Space selects the focused option", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <FloatingChoices
        options={OPTIONS}
        value={null}
        onChange={onChange}
        ariaLabel="Primary interest"
      />,
    );

    screen.getByRole("radio", { name: "Compliance" }).focus();
    await user.keyboard(" ");

    expect(onChange).toHaveBeenCalledWith("governance");
  });

  it("is fully keyboard-operable end to end via a controlled harness", async () => {
    const user = userEvent.setup();
    const onChangeSpy = vi.fn();
    render(<Controlled onChangeSpy={onChangeSpy} />);

    screen.getByRole("radio", { name: "Cost savings" }).focus();
    await user.keyboard("{ArrowRight}{ArrowRight}{Enter}");

    expect(onChangeSpy).toHaveBeenLastCalledWith("governance");
    expect(screen.getByRole("radio", { name: "Compliance" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  });

  it("exposes required semantics on the radiogroup", () => {
    render(
      <FloatingChoices
        options={OPTIONS}
        value={null}
        onChange={vi.fn()}
        ariaLabel="Primary interest"
        required
      />,
    );

    expect(screen.getByRole("radiogroup", { name: "Primary interest" })).toHaveAttribute(
      "aria-required",
      "true",
    );
  });
});
