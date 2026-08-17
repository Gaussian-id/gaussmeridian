import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";
import { renderWithProviders } from "@/test/render";

import { ByokManager } from "../byok-manager";

vi.mock("next/navigation", () => ({
  usePathname: () => "/orgs/org-1/projects/proj-1/byok",
  useParams: () => ({ orgId: "org-1", projectId: "proj-1" }),
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
}));

function setup(options: { providers?: string[]; onMutate?: (body: unknown) => unknown } = {}) {
  const mutate = vi.fn((body: unknown) => options.onMutate?.(body) ?? {});
  const registry = createFakeRegistry({
    data: {
      query: byResource({
        "v1/byok/keys": (input: { method?: string; body?: unknown }) => {
          if (input?.method && input.method !== "GET") return mutate(input.body);
          return { providers: options.providers ?? [] };
        },
      }),
    },
  });
  return { mutate, ...renderWithProviders(<ByokManager />, registry) };
}

describe("ByokManager", () => {
  it("tells a project with no keys what to do next, rather than showing an empty box", async () => {
    setup();

    expect(await screen.findByText(/no provider keys yet/i)).toBeInTheDocument();
  });

  it("lists the providers the server says are configured", async () => {
    setup({ providers: ["openai", "google"] });

    expect(await screen.findByText("OpenAI")).toBeInTheDocument();
    expect(screen.getByText("Google (Gemini)")).toBeInTheDocument();
  });

  it("offers exactly the providers the backend accepts", async () => {
    setup();
    await screen.findByLabelText(/provider/i);

    const options = Array.from(
      screen.getByLabelText<HTMLSelectElement>(/provider/i).options,
      (option) => option.value,
    );
    // Mirrors BYOK_PROVIDERS in services/server/src/handlers.rs — anything else is a 400.
    expect(options).toEqual(["openai", "anthropic", "google", "mistral", "cohere", "ollama"]);
  });

  it("will not submit an empty key", async () => {
    setup();

    expect(await screen.findByRole("button", { name: /save key/i })).toBeDisabled();
  });

  it("sends the provider and key, then clears the field so the secret does not linger", async () => {
    const { mutate } = setup();

    fireEvent.change(await screen.findByLabelText(/api key/i), {
      target: { value: "sk-test-secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: /save key/i }));

    await waitFor(() => expect(mutate).toHaveBeenCalled());
    expect(mutate).toHaveBeenCalledWith({ provider: "openai", api_key: "sk-test-secret" });
    await waitFor(() =>
      expect(screen.getByLabelText<HTMLInputElement>(/api key/i).value).toBe(""),
    );
  });

  it("never renders a stored key — the server only returns provider names", async () => {
    setup({ providers: ["openai"] });
    await screen.findByText("OpenAI");

    expect(document.body.textContent).not.toMatch(/sk-/);
  });

  it("translates a 403 into the allowlist setting an operator has to change", async () => {
    setup({
      onMutate: () => {
        throw new Error("Request failed with status 403");
      },
    });

    fireEvent.change(await screen.findByLabelText(/api key/i), { target: { value: "sk-x" } });
    fireEvent.click(screen.getByRole("button", { name: /save key/i }));

    expect(await screen.findByText(/BYOK_ADMIN_EMAILS/)).toBeInTheDocument();
  });

  it("translates a 503 into the vault variable, the usual self-host cause", async () => {
    setup({
      onMutate: () => {
        throw new Error("Request failed with status 503");
      },
    });

    fireEvent.change(await screen.findByLabelText(/api key/i), { target: { value: "sk-x" } });
    fireEvent.click(screen.getByRole("button", { name: /save key/i }));

    expect(await screen.findByText(/BYOK_MASTER_KEY/)).toBeInTheDocument();
  });
});
