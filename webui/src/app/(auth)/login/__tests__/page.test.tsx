import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AuthError } from "@core/adapters/auth-error";

import { createFakeRegistry } from "@/test/fakes";
import { renderWithProviders } from "@/test/render";

import LoginPage from "../page";

const push = vi.fn();
const replace = vi.fn();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push, replace }),
  useSearchParams: () => new URLSearchParams(),
}));

const mockFetch = vi.fn();

beforeEach(() => {
  push.mockClear();
  replace.mockClear();
  mockFetch.mockReset();
  // Default: the caller is NOT an allowlisted superadmin — the real backend 404s `/v1/admin/me`.
  mockFetch.mockResolvedValue(new Response(null, { status: 404 }));
  vi.stubGlobal("fetch", mockFetch);
});

describe("LoginPage", () => {
  it("redirects a normal user to /orgs (the Org Chooser) after a successful sign-in", async () => {
    renderWithProviders(<LoginPage />, createFakeRegistry());
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: "user@example.com" } });
    fireEvent.change(screen.getByLabelText(/password/i), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: /sign in/i }));
    await waitFor(() => expect(push).toHaveBeenCalledWith("/orgs"));
  });

  it("sends an allowlisted superadmin straight to the admin console (never the tenant app)", async () => {
    mockFetch.mockResolvedValueOnce(
      new Response(JSON.stringify({ superadmin: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    );
    renderWithProviders(<LoginPage />, createFakeRegistry());
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: "admin@example.com" } });
    fireEvent.change(screen.getByLabelText(/password/i), { target: { value: "hunter22" } });
    fireEvent.click(screen.getByRole("button", { name: /sign in/i }));

    await waitFor(() => expect(replace).toHaveBeenCalledWith("/admin"));
    expect(push).not.toHaveBeenCalledWith("/orgs");
  });

  it("links to sign up and forgot password (full cross-nav)", () => {
    renderWithProviders(<LoginPage />, createFakeRegistry());
    expect(screen.getByRole("link", { name: /sign up/i })).toHaveAttribute("href", "/signup");
    expect(screen.getByRole("link", { name: /forgot password/i })).toHaveAttribute(
      "href",
      "/forgot-password",
    );
  });

  it("shows one anti-enumeration message when sign-in fails (401)", async () => {
    const registry = createFakeRegistry();
    registry.auth.signIn = async () => {
      throw new AuthError({ status: 401, code: "login_failed", message: "raw" });
    };
    renderWithProviders(<LoginPage />, registry);
    fireEvent.change(screen.getByLabelText(/email/i), { target: { value: "user@example.com" } });
    fireEvent.change(screen.getByLabelText(/password/i), { target: { value: "wrongpass" } });
    fireEvent.click(screen.getByRole("button", { name: /sign in/i }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(/invalid email or password/i);
    expect(alert).not.toHaveTextContent(/not found|no account/i);
  });
});
