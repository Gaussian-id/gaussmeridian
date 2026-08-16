import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { StatCard } from "../stat-card";

describe("StatCard", () => {
  it("renders label and value", () => {
    render(<StatCard label="Cache hit rate" value="84%" />);
    expect(screen.getByText("Cache hit rate")).toBeInTheDocument();
    expect(screen.getByText("84%")).toBeInTheDocument();
  });

  it("renders a loading skeleton when isLoading is true", () => {
    render(<StatCard label="Cache hit rate" isLoading />);
    expect(screen.queryByText("Cache hit rate")).not.toBeInTheDocument();
  });
});
