import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ChatWidget } from "../chat-widget";

let currentPathname = "/orgs";
vi.mock("next/navigation", () => ({
  usePathname: () => currentPathname,
}));

const launcher = () => screen.queryByRole("link", { name: /open the playground/i });

describe("ChatWidget", () => {
  it("deep-links project pages to that project's Playground", () => {
    currentPathname = "/orgs/org-1/projects/project-1/activity";
    render(<ChatWidget />);

    expect(launcher()).toHaveAttribute("href", "/orgs/org-1/projects/project-1/playground");
  });

  it("hides the launcher while already on the project Playground (pointless self-link)", () => {
    currentPathname = "/orgs/org-1/projects/project-1/playground";
    render(<ChatWidget />);

    expect(launcher()).not.toBeInTheDocument();
  });

  it("hides the launcher on nested Playground routes too", () => {
    currentPathname = "/orgs/org-1/projects/project-1/playground/thread-1";
    render(<ChatWidget />);

    expect(launcher()).not.toBeInTheDocument();
  });

  // The Playground is project-scoped; there is no global one to fall back to.
  it("renders nothing outside a project", () => {
    currentPathname = "/orgs";
    render(<ChatWidget />);

    expect(launcher()).not.toBeInTheDocument();
  });

  it("renders nothing on an org route that is not inside a project", () => {
    currentPathname = "/orgs/org-1/members";
    render(<ChatWidget />);

    expect(launcher()).not.toBeInTheDocument();
  });
});
