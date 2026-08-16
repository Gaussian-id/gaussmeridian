import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, describe, expect, it, vi } from "vitest";

import { BYOK_PROVIDERS, ByokProviderForm } from "../byok-provider-form";

// Radix Select drives its popup with pointer-capture + scroll APIs jsdom doesn't implement.
beforeAll(() => {
  Element.prototype.hasPointerCapture ??= () => false;
  Element.prototype.setPointerCapture ??= () => {};
  Element.prototype.releasePointerCapture ??= () => {};
  Element.prototype.scrollIntoView ??= () => {};
});

async function selectProvider(user: ReturnType<typeof userEvent.setup>, label: string) {
  await user.click(screen.getByRole("combobox", { name: /provider/i }));
  await user.click(await screen.findByRole("option", { name: label }));
}

describe("ByokProviderForm", () => {
  it("offers exactly the backend's provider allowlist", async () => {
    const user = userEvent.setup();
    render(<ByokProviderForm />);

    await user.click(screen.getByRole("combobox", { name: /provider/i }));
    const options = await screen.findAllByRole("option");

    expect(options).toHaveLength(BYOK_PROVIDERS.length);
  });

  it("submits the chosen provider and key, then clears the key field", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<ByokProviderForm onSubmit={onSubmit} />);

    await selectProvider(user, "Anthropic");
    const keyInput = screen.getByLabelText("API key");
    await user.type(keyInput, "sk-ant-secret");
    await user.click(screen.getByRole("button", { name: /save credentials/i }));

    expect(onSubmit).toHaveBeenCalledWith({ provider: "anthropic", apiKey: "sk-ant-secret" });
    expect(keyInput).toHaveValue(""); // the secret must not linger in the field after submit
  });

  it("does not submit with an empty key", async () => {
    const onSubmit = vi.fn();
    render(<ByokProviderForm onSubmit={onSubmit} />);

    expect(screen.getByRole("button", { name: /save credentials/i })).toBeDisabled();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it("shows progress text and blocks resubmission while pending", async () => {
    const user = userEvent.setup();
    render(<ByokProviderForm isPending />);

    await user.type(screen.getByLabelText("API key"), "sk-x");

    expect(screen.getByRole("button", { name: /saving/i })).toBeDisabled();
  });

  it("disables every control when disabled is true", () => {
    render(<ByokProviderForm disabled />);

    expect(screen.getByRole("combobox", { name: /provider/i })).toBeDisabled();
    expect(screen.getByLabelText("API key")).toBeDisabled();
    expect(screen.getByRole("button", { name: /save credentials/i })).toBeDisabled();
  });
});
