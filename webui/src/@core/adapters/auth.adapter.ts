import { z } from "zod";

import type { HttpClient } from "./http-client";
import type { AuthAdapter } from "./types";

const authSessionSchema = z.object({
  userId: z.string(),
  displayName: z.string(),
  token: z.string(),
  expiresAt: z.string(),
  onboardingCompleted: z.boolean().default(true),
  email: z.string().optional(),
});

const okSchema = z.object({ ok: z.boolean() });

/** HTTP reference implementation of the auth adapter. */
export function createHttpAuthAdapter(http: HttpClient): AuthAdapter {
  return {
    signIn: ({ email, password }) =>
      http.request("/auth/signin", {
        method: "POST",
        body: JSON.stringify({ email, password }),
        schema: authSessionSchema,
      }),

    signUp: ({ email, username, password }) =>
      http.request("/auth/signup", {
        method: "POST",
        body: JSON.stringify({ email, username, password }),
        schema: authSessionSchema,
      }),

    getSession: async () => {
      try {
        return await http.request("/auth/session", {
          method: "GET",
          schema: authSessionSchema,
        });
      } catch {
        return null;
      }
    },

    signOut: () =>
      http.request("/auth/signout", { method: "POST", schema: okSchema }).then(() => undefined),

    forgotPassword: ({ email }) =>
      http
        .request("/auth/forgot-password", {
          method: "POST",
          body: JSON.stringify({ email }),
          schema: okSchema,
        })
        .then(() => undefined),

    resetPassword: ({ token, newPassword }) =>
      http
        .request("/auth/reset-password", {
          method: "POST",
          body: JSON.stringify({ token, newPassword }),
          schema: okSchema,
        })
        .then(() => undefined),

    requestAccountDeletion: () =>
      http
        .request("/auth/deletion-request", { method: "POST", schema: okSchema })
        .then(() => undefined),

    cancelAccountDeletion: () =>
      http
        .request("/auth/deletion-request", { method: "DELETE", schema: okSchema })
        .then(() => undefined),
  };
}
