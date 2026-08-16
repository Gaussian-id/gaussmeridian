import type { Project } from "@core/adapters/schemas/console.schema";

/** `org_born_empty` intentionally has no projects here — see `orgs.ts`. */
export const projects: Project[] = [
  {
    id: "proj_prod",
    org_id: "org_meridian",
    name: "Production API",
    slug: "production-api",
    environment: "production",
    created_at: "2026-03-02T00:00:00Z",
  },
  {
    id: "proj_dev",
    org_id: "org_meridian",
    name: "Dev Sandbox",
    slug: "dev-sandbox",
    environment: "development",
    created_at: "2026-03-05T00:00:00Z",
  },
];
