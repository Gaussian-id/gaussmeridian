import { screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { createFakeRegistry } from "@/test/fakes";
import { renderWithProviders } from "@/test/render";

import { Navbar } from "../navbar";

vi.mock("next/navigation", () => ({
  usePathname: () => "/",
  useRouter: () => ({ push: vi.fn(), prefetch: vi.fn() }),
  useSearchParams: () => new URLSearchParams(),
}));

describe("Navbar", () => {
  it("renders the six public routes and the primary CTA", () => {
    renderWithProviders(<Navbar />, createFakeRegistry());

    for (const label of ["Home", "Story", "Solutions", "Pricing", "Changelog", "Docs"]) {
      expect(screen.getAllByRole("link", { name: label }).length).toBeGreaterThan(0);
    }
    expect(screen.getAllByRole("link", { name: /get api key/i }).length).toBeGreaterThan(0);
    // No stale product wordmark
    expect(screen.queryByText(/gaussmeridian/i)).not.toBeInTheDocument();
  });
});
