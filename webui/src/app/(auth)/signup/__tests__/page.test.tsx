import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AuthError } from "@core/adapters/auth-error";

import { createFakeRegistry } from "@/test/fakes";
import { renderWithProviders } from "@/test/render";

import SignupPage from "../page";

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: vi.fn() }),
}));

describe("SignupPage", () => {
  it("submits email/username/password to the register mutation", async () => {
    const registry = createFakeRegistry();
    renderWithProviders(<SignupPage />, registry);
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: "new@user.com" } });
    fireEvent.change(screen.getByLabelText(/username/i), { target: { value: "newuser" } });
    fireEvent.change(screen.getByLabelText(/password/i), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: /sign up/i }));
    await waitFor(() => expect(screen.queryByText(/error/i)).not.toBeInTheDocument());
  });

  it("shows a field-level error + 'sign in instead' link when the email is taken", async () => {
    const registry = createFakeRegistry();
    registry.auth.signUp = async () => {
      throw new AuthError({ status: 409, code: "email_taken", message: "raw" });
    };
    renderWithProviders(<SignupPage />, registry);
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: "taken@user.com" } });
    fireEvent.change(screen.getByLabelText(/username/i), { target: { value: "newuser" } });
    fireEvent.change(screen.getByLabelText(/password/i), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: /sign up/i }));

    expect(await screen.findByText(/this email is already registered/i)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /sign in instead/i })).toHaveAttribute(
      "href",
      "/login",
    );
  });

  it("shows a username-field error when the username is taken", async () => {
    const registry = createFakeRegistry();
    registry.auth.signUp = async () => {
      throw new AuthError({ status: 409, code: "username_taken", message: "raw" });
    };
    renderWithProviders(<SignupPage />, registry);
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: "fresh@user.com" } });
    fireEvent.change(screen.getByLabelText(/username/i), { target: { value: "taken" } });
    fireEvent.change(screen.getByLabelText(/password/i), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: /sign up/i }));

    expect(await screen.findByText(/that username is taken/i)).toBeInTheDocument();
  });
});
