import type { PermissionMatrix } from "@core/adapters/schemas/console.schema";

/**
 * The permission model is identical for every org (RBAC roles are fixed: owner/admin/
 * developer), so the mock returns this same matrix regardless of the requested `orgId`.
 */
export const permissionMatrix: PermissionMatrix = {
  permissions: [
    { key: "org.billing.manage", label: "Manage billing & credits", group: "Organization" },
    { key: "org.members.invite", label: "Invite members", group: "Organization" },
    { key: "org.settings.manage", label: "Manage tenant settings", group: "Organization" },
    { key: "project.create", label: "Create projects", group: "Projects" },
    { key: "project.settings.manage", label: "Manage project settings", group: "Projects" },
    { key: "project.keys.manage", label: "Manage API keys", group: "Projects" },
    { key: "project.byok.manage", label: "Manage BYOK providers", group: "Projects" },
    { key: "project.routes.view", label: "View route decisions", group: "Observability" },
    { key: "project.playground.use", label: "Use the Playground", group: "Observability" },
  ],
  roles: [
    {
      role: "owner",
      grants: [
        "org.billing.manage",
        "org.members.invite",
        "org.settings.manage",
        "project.create",
        "project.settings.manage",
        "project.keys.manage",
        "project.byok.manage",
        "project.routes.view",
        "project.playground.use",
      ],
    },
    {
      role: "admin",
      grants: [
        "org.members.invite",
        "project.create",
        "project.settings.manage",
        "project.keys.manage",
        "project.byok.manage",
        "project.routes.view",
        "project.playground.use",
      ],
    },
    {
      role: "developer",
      grants: ["project.routes.view", "project.playground.use"],
    },
  ],
};
