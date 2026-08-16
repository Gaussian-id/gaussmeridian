import { describe, expect, it } from "vitest";

import { AuthError } from "@core/adapters/auth-error";

import { mapAuthError } from "../map-auth-error";

const err = (status: number, code?: string) => new AuthError({ message: "raw", status, code });

describe("mapAuthError", () => {
  it("login: 401 shows one anti-enumeration message (no 'account not found')", () => {
    const m = mapAuthError(err(401, "login_failed"), "login");
    expect(m.message).toMatch(/invalid email or password/i);
    expect(m.message).not.toMatch(/not found|no account|doesn't exist/i);
    expect(m.field).toBeUndefined();
  });

  it("login: 403 reports a disabled account", () => {
    expect(mapAuthError(err(403, "login_failed"), "login").message).toMatch(/disabled/i);
  });

  it("network failure (status 0) is distinct from a credential error", () => {
    expect(mapAuthError(err(0), "login").message).toMatch(/can't reach the server/i);
  });

  it("a backend-unreachable proxy response (502) reads as a reachability error", () => {
    expect(mapAuthError(err(502, "backend_unreachable"), "login").message).toMatch(
      /can't reach the server/i,
    );
  });

  it("signup: email_taken maps to the email field", () => {
    expect(mapAuthError(err(409, "email_taken"), "signup")).toEqual({
      field: "email",
      message: "This email is already registered.",
    });
  });

  it("signup: username_taken maps to the username field", () => {
    expect(mapAuthError(err(409, "username_taken"), "signup")).toMatchObject({ field: "username" });
  });

  it("signup: weak_password maps to the password field", () => {
    expect(mapAuthError(err(400, "weak_password"), "signup")).toMatchObject({ field: "password" });
  });

  it("signup: invalid_email maps to the email field", () => {
    expect(mapAuthError(err(400, "invalid_email"), "signup")).toMatchObject({ field: "email" });
  });

  it("reset: any non-network error reads as an invalid/expired link", () => {
    expect(mapAuthError(err(400, "invalid_token"), "reset").message).toMatch(
      /invalid or has expired/i,
    );
  });

  it("forgot: reports a send failure without leaking account existence", () => {
    const m = mapAuthError(err(500, "internal"), "forgot");
    expect(m.message).toMatch(/sending the reset link/i);
    expect(m.message).not.toMatch(/not found|no account/i);
  });

  it("unknown code / non-AuthError falls back to a safe generic message", () => {
    expect(mapAuthError(new Error("boom"), "login").message).toMatch(/something went wrong/i);
    expect(mapAuthError(err(500, "totally_new_code"), "signup").message).toMatch(
      /something went wrong/i,
    );
  });
});
