import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { DataTable } from "@/components/ui/data-table";

import type { ColumnDef } from "@tanstack/react-table";

interface Row {
  name: string;
  value: number;
}

const columns: ColumnDef<Row>[] = [
  { accessorKey: "name", header: "Name" },
  { accessorKey: "value", header: "Value" },
];

const data: Row[] = [
  { name: "a", value: 1 },
  { name: "b", value: 2 },
];

describe("DataTable", () => {
  it("renders rows", () => {
    render(<DataTable columns={columns} data={data} />);
    expect(screen.getByText("a")).toBeInTheDocument();
    expect(screen.getByText("b")).toBeInTheDocument();
  });

  it("renders an empty state when data is empty", () => {
    render(<DataTable columns={columns} data={[]} emptyMessage="No rows yet" />);
    expect(screen.getByText("No rows yet")).toBeInTheDocument();
  });

  it("renders a loading skeleton when isLoading is true", () => {
    render(<DataTable columns={columns} data={[]} isLoading />);
    expect(screen.queryByText("No rows yet")).not.toBeInTheDocument();
  });

  it("renders an error message when isError is true, instead of the empty state", () => {
    render(<DataTable columns={columns} data={[]} isError emptyMessage="No rows yet" />);
    expect(screen.queryByText("No rows yet")).not.toBeInTheDocument();
    expect(
      screen.getByText("Something went wrong loading this data. Try again shortly."),
    ).toBeInTheDocument();
  });

  it("supports a custom errorMessage", () => {
    render(<DataTable columns={columns} data={[]} isError errorMessage="Could not load keys." />);
    expect(screen.getByText("Could not load keys.")).toBeInTheDocument();
  });
});
