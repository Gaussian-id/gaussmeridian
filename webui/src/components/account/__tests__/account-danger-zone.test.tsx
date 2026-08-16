import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AuthError } from "@core/adapters/auth-error";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { AccountDangerZone } from "../account-danger-zone";

const profileFixture = {
  id: "user_1",
  email: "ada@meridianlabs.dev",
  username: "ada.lovelace",
  full_name: "Ada Lovelace",
  display_name: "Ada",
  company: "Meridian Labs",
  timezone: "UTC",
};

const pendingProfileFixture = { ...profileFixture, deletion_requested: true };

function registryWith(requestAccountDeletion: () => Promise<void>) {
  const base = createFakeRegistry();
  return createFakeRegistry({
    data: { query: byResource({ "v1/auth/me": profileFixture }) },
    auth: { ...base.auth, requestAccountDeletion },
  });
}

function pendingRegistryWith(cancelAccountDeletion: () => Promise<void>) {
  const base = createFakeRegistry();
  return createFakeRegistry({
    data: { query: byResource({ "v1/auth/me": pendingProfileFixture }) },
    auth: { ...base.auth, cancelAccountDeletion },
  });
}

describe("AccountDangerZone", () => {
  it("requires typing the exact username to arm the confirm button", async () => {
    const user = userEvent.setup();
    renderWithProviders(
      <AccountDangerZone />,
      registryWith(async () => undefined),
    );

    await user.click(await screen.findByRole("button", { name: /request account deletion/i }));

    const confirmButton = screen.getByRole("button", { name: "Request deletion" });
    expect(confirmButton).toBeDisabled();

    const input = screen.getByLabelText(/type the account name to confirm/i);
    await user.type(input, "wrong-username");
    expect(confirmButton).toBeDisabled();

    await user.clear(input);
    await user.type(input, "ada.lovelace");
    expect(confirmButton).toBeEnabled();
  });

  it("submits the deletion request and shows a success message", async () => {
    const user = userEvent.setup();
    const requestAccountDeletion = vi.fn().mockResolvedValue(undefined);
    renderWithProviders(<AccountDangerZone />, registryWith(requestAccountDeletion));

    await user.click(await screen.findByRole("button", { name: /request account deletion/i }));
    await user.type(screen.getByLabelText(/type the account name to confirm/i), "ada.lovelace");
    await user.click(screen.getByRole("button", { name: "Request deletion" }));

    await waitFor(() => expect(requestAccountDeletion).toHaveBeenCalledTimes(1));
    expect(await screen.findByText(/deletion request submitted/i)).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  });

  it("maps the real adapter's 404/405 'not enabled yet' error to honest copy, not a raw status", async () => {
    const user = userEvent.setup();
    const requestAccountDeletion = vi.fn().mockRejectedValue(
      new AuthError({
        message: "Deletion requests aren't enabled on this server yet.",
        status: 404,
        code: "deletion_request_unavailable",
      }),
    );
    renderWithProviders(<AccountDangerZone />, registryWith(requestAccountDeletion));

    await user.click(await screen.findByRole("button", { name: /request account deletion/i }));
    await user.type(screen.getByLabelText(/type the account name to confirm/i), "ada.lovelace");
    await user.click(screen.getByRole("button", { name: "Request deletion" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /deletion requests aren't enabled on this server yet/i,
    );
  });

  it("shows a generic error for any other failure, never a raw error object", async () => {
    const user = userEvent.setup();
    const requestAccountDeletion = vi.fn().mockRejectedValue(new Error("boom"));
    renderWithProviders(<AccountDangerZone />, registryWith(requestAccountDeletion));

    await user.click(await screen.findByRole("button", { name: /request account deletion/i }));
    await user.type(screen.getByLabelText(/type the account name to confirm/i), "ada.lovelace");
    await user.click(screen.getByRole("button", { name: "Request deletion" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      /could not submit your deletion request/i,
    );
  });

  it("shows the pending-review banner and a Cancel request action instead of the request trigger when deletion_requested is true", async () => {
    renderWithProviders(
      <AccountDangerZone />,
      pendingRegistryWith(async () => undefined),
    );

    expect(await screen.findByText(/deletion requested — pending review/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /cancel request/i })).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /request account deletion/i }),
    ).not.toBeInTheDocument();
  });

  it("cancels the pending request", async () => {
    const user = userEvent.setup();
    const cancelAccountDeletion = vi.fn().mockResolvedValue(undefined);
    renderWithProviders(<AccountDangerZone />, pendingRegistryWith(cancelAccountDeletion));

    await user.click(await screen.findByRole("button", { name: /cancel request/i }));

    await waitFor(() => expect(cancelAccountDeletion).toHaveBeenCalledTimes(1));
  });

  it("maps a cancel failure to honest copy, never a raw error", async () => {
    const user = userEvent.setup();
    const cancelAccountDeletion = vi
      .fn()
      .mockRejectedValue(new AuthError({ message: "not found", status: 404 }));
    renderWithProviders(<AccountDangerZone />, pendingRegistryWith(cancelAccountDeletion));

    await user.click(await screen.findByRole("button", { name: /cancel request/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/no pending request to cancel/i);
  });
});
