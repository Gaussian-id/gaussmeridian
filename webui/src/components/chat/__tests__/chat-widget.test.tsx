import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ChatWidget } from "../chat-widget";

let currentPathname = "/orgs";
vi.mock("next/navigation", () => ({
  usePathname: () => currentPathname,
}));

describe("ChatWidget", () => {
  it("renders the floating Playground launcher on other authed routes", () => {
    currentPathname = "/orgs";
    render(<ChatWidget />);

    expect(screen.getByRole("link", { name: /open the playground/i })).toHaveAttribute(
      "href",
      "/playground",
    );
  });

  it("hides the launcher while already on the Playground route (pointless self-link)", () => {
    currentPathname = "/playground";
    render(<ChatWidget />);

    expect(screen.queryByRole("link", { name: /open the playground/i })).not.toBeInTheDocument();
  });

  it("hides the launcher on nested Playground routes too", () => {
    currentPathname = "/playground/thread-1";
    render(<ChatWidget />);

    expect(screen.queryByRole("link", { name: /open the playground/i })).not.toBeInTheDocument();
  });

  it("deep-links project pages to that project's Playground", () => {
    currentPathname = "/orgs/org-1/projects/project-1/activity";
    render(<ChatWidget />);

    expect(screen.getByRole("link", { name: /open the playground/i })).toHaveAttribute(
      "href",
      "/orgs/org-1/projects/project-1/playground",
    );
  });

  it("does not cover controls on the project-scoped Playground", () => {
    currentPathname = "/orgs/org-1/projects/project-1/playground";
    render(<ChatWidget />);

    expect(screen.queryByRole("link", { name: /open the playground/i })).not.toBeInTheDocument();
  });
});
