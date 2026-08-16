import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AdapterProvider, type AdapterRegistry, type DataQueryInput } from "@core/adapters";
import { GaussMeridianAdapterError } from "@core/adapters/gaussmeridian-data.adapter";
import type { Member, Org, Project } from "@core/adapters/schemas/console.schema";

import { createFakeRegistry } from "@/test/fakes";
import { byResource } from "@/test/mock-data";

import { TenancyProvider, useTenancy } from "../tenancy-provider";

let mockParams: { orgId?: string; projectId?: string } = {};

vi.mock("next/navigation", () => ({
  useParams: () => mockParams,
}));

const org: Org = {
  id: "org_1",
  name: "Acme Labs",
  slug: "acme-labs",
  plan: "pro",
  created_at: "2026-01-01T00:00:00Z",
  member_count: 2,
  project_count: 1,
};

const project: Project = {
  id: "proj_1",
  org_id: "org_1",
  name: "Production API",
  slug: "production-api",
  environment: "production",
  created_at: "2026-01-02T00:00:00Z",
};

const owner: Member = {
  id: "mem_owner",
  org_id: "org_1",
  user_id: "user_owner",
  email: "owner@acme.dev",
  display_name: "Owner",
  role: "owner",
  status: "active",
  invited_at: null,
  joined_at: "2026-01-01T00:00:00Z",
};

const admin: Member = {
  id: "mem_admin",
  org_id: "org_1",
  user_id: "user_admin",
  email: "admin@acme.dev",
  display_name: "Admin",
  role: "admin",
  status: "active",
  invited_at: "2026-01-01T00:00:00Z",
  joined_at: "2026-01-02T00:00:00Z",
};

function Probe() {
  const { mode, org: resolvedOrg, project: resolvedProject, role, roleStatus } = useTenancy();
  return (
    <dl>
      <dt>mode</dt>
      <dd data-testid="mode">{mode}</dd>
      <dt>org</dt>
      <dd data-testid="org">{resolvedOrg?.name ?? ""}</dd>
      <dt>project</dt>
      <dd data-testid="project">{resolvedProject?.name ?? ""}</dd>
      <dt>role</dt>
      <dd data-testid="role">{role ?? ""}</dd>
      <dt>role status</dt>
      <dd data-testid="role-status">{roleStatus}</dd>
    </dl>
  );
}

function renderTenancy(registry: AdapterRegistry) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <AdapterProvider registry={registry}>
        <TenancyProvider>
          <Probe />
        </TenancyProvider>
      </AdapterProvider>
    </QueryClientProvider>,
  );
}

describe("TenancyProvider", () => {
  it('resolves mode "global" when neither orgId nor projectId is in the URL', () => {
    mockParams = {};
    renderTenancy(createFakeRegistry());
    expect(screen.getByTestId("mode")).toHaveTextContent("global");
    expect(screen.getByTestId("org")).toHaveTextContent("");
  });

  it('resolves mode "org" and fetches the org when only orgId is present', async () => {
    mockParams = { orgId: "org_1" };
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/orgs/org_1": org,
          "v1/orgs/org_1/members": { members: [owner] },
        }),
      },
    });
    renderTenancy(registry);
    expect(screen.getByTestId("mode")).toHaveTextContent("org");
    await waitFor(() => expect(screen.getByTestId("org")).toHaveTextContent("Acme Labs"));
    expect(screen.getByTestId("project")).toHaveTextContent("");
  });

  it('resolves mode "project" and fetches both org and project when orgId + projectId are present', async () => {
    mockParams = { orgId: "org_1", projectId: "proj_1" };
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/orgs/org_1": org,
          "v1/orgs/org_1/projects/proj_1": project,
          "v1/orgs/org_1/members": { members: [owner] },
        }),
      },
    });
    renderTenancy(registry);
    expect(screen.getByTestId("mode")).toHaveTextContent("project");
    await waitFor(() => expect(screen.getByTestId("org")).toHaveTextContent("Acme Labs"));
    await waitFor(() => expect(screen.getByTestId("project")).toHaveTextContent("Production API"));
  });

  it("derives role by matching the session user against the org's member list", async () => {
    mockParams = { orgId: "org_1" };
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/orgs/org_1": org,
          "v1/orgs/org_1/members": { members: [owner, admin] },
        }),
      },
      auth: {
        ...createFakeRegistry().auth,
        getSession: async () => ({
          userId: "user_admin",
          displayName: "Admin",
          token: "tok",
          expiresAt: "2099-01-01T00:00:00Z",
          onboardingCompleted: true,
        }),
      },
    });
    renderTenancy(registry);
    await waitFor(() => expect(screen.getByTestId("role")).toHaveTextContent("admin"));
  });

  it("denies role resolution when the session user has no matching member", async () => {
    mockParams = { orgId: "org_1" };
    const registry = createFakeRegistry({
      data: {
        query: byResource({
          "v1/orgs/org_1": org,
          "v1/orgs/org_1/members": { members: [owner, admin] },
        }),
      },
      auth: {
        ...createFakeRegistry().auth,
        getSession: async () => ({
          userId: "user_unknown",
          displayName: "Unknown",
          token: "tok",
          expiresAt: "2099-01-01T00:00:00Z",
          onboardingCompleted: true,
        }),
      },
    });
    renderTenancy(registry);
    await waitFor(() => expect(screen.getByTestId("role-status")).toHaveTextContent("denied"));
    expect(screen.getByTestId("role")).toHaveTextContent("");
  });

  it("classifies a forbidden membership lookup as denied instead of a retryable outage", async () => {
    mockParams = { orgId: "org_forbidden" };
    const base = createFakeRegistry();
    const registry = createFakeRegistry({
      data: {
        query: async <T,>(input: DataQueryInput<T>) => {
          if (input.resource === "v1/orgs/org_forbidden/members") {
            throw new GaussMeridianAdapterError("Forbidden", 403, "forbidden");
          }
          return base.data.query(input);
        },
      },
    });

    renderTenancy(registry);

    await waitFor(() => expect(screen.getByTestId("role-status")).toHaveTextContent("denied"));
  });
});
