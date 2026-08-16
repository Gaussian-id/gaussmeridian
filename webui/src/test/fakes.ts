import type { AdapterRegistry } from "@core/adapters";

/**
 * In-memory fake adapter registry for tests. Override any slice per test. This is how we
 * exercise the app at the single seam — never against the network.
 */
export function createFakeRegistry(overrides: Partial<AdapterRegistry> = {}): AdapterRegistry {
  return {
    llm: {
      streamChat: async function* ({ messages }) {
        const last = messages.at(-1)?.content ?? "";
        yield `You said: “${last}”. (demo response)`;
      },
    },
    data: {
      query: async () => {
        throw new Error("data.query not stubbed for this test");
      },
    },
    auth: {
      signIn: async ({ email }) => ({
        userId: "user_fake",
        displayName: email,
        token: "tok_fake",
        expiresAt: "2099-01-01T00:00:00Z",
        onboardingCompleted: true,
        email,
      }),
      signUp: async ({ email, username }) => ({
        userId: "user_fake",
        displayName: username || email,
        token: "tok_fake",
        expiresAt: "2099-01-01T00:00:00Z",
        onboardingCompleted: false,
        email,
      }),
      getSession: async () => null,
      signOut: async () => undefined,
      forgotPassword: async () => undefined,
      resetPassword: async () => undefined,
      requestAccountDeletion: async () => undefined,
      cancelAccountDeletion: async () => undefined,
    },
    ...overrides,
  };
}
