import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { AdminUser } from "@core/adapters/schemas/admin.schema";

import { DataTable } from "@/components/ui/data-table";

import { adminUsersColumns } from "../admin-users-columns";

const baseUser: AdminUser = {
  id: "user_1",
  email: "jordan.churn@meridianlabs.dev",
  username: "jordan.churn",
  created_at: "2026-05-14T00:00:00Z",
  active: true,
  onboarding_completed: true,
  orgs: [{ org_id: "org_meridian", org_name: "Meridian Labs", role: "owner" }],
  deletion_status: null,
  last_active_api: null,
  last_active_console: null,
};

function renderRows(rows: AdminUser[]) {
  return render(<DataTable columns={adminUsersColumns} data={rows} />);
}

describe("adminUsersColumns", () => {
  it("renders the user's email and username", () => {
    renderRows([baseUser]);
    expect(screen.getByText("jordan.churn@meridianlabs.dev")).toBeInTheDocument();
    expect(screen.getByText("jordan.churn")).toBeInTheDocument();
  });

  it("renders an org badge per membership", () => {
    renderRows([baseUser]);
    expect(screen.getByText("Meridian Labs · owner")).toBeInTheDocument();
  });

  it("renders a dash when the user has no org memberships", () => {
    renderRows([{ ...baseUser, orgs: [] }]);
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("renders relative last-active labels, and 'Never' for a null timestamp", () => {
    renderRows([
      { ...baseUser, last_active_api: new Date().toISOString(), last_active_console: null },
    ]);
    expect(screen.getByText(/API: Just now/)).toBeInTheDocument();
    expect(screen.getByText(/Console: Never/)).toBeInTheDocument();
  });

  it("shows a Deactivated badge for an inactive user", () => {
    renderRows([{ ...baseUser, active: false }]);
    expect(screen.getByText("Deactivated")).toBeInTheDocument();
  });

  it("shows an Onboarding badge when onboarding isn't complete", () => {
    renderRows([{ ...baseUser, onboarding_completed: false }]);
    expect(screen.getByText("Onboarding")).toBeInTheDocument();
  });

  it("shows a Deletion pending badge when deletion_status is pending", () => {
    renderRows([{ ...baseUser, deletion_status: "pending" }]);
    expect(screen.getByText("Deletion pending")).toBeInTheDocument();
  });

  it("does not show a deletion badge when there is no pending request", () => {
    renderRows([baseUser]);
    expect(screen.queryByText("Deletion pending")).not.toBeInTheDocument();
  });
});
