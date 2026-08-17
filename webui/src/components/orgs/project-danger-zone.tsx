"use client";

import { Trash2 } from "lucide-react";
import { useParams, useRouter } from "next/navigation";
import { useState } from "react";

import { GaussMeridianAdapterError } from "@core/adapters/gaussmeridian-data.adapter";
import { canManageProjects } from "@core/lib/rbac";
import { useTenancy } from "@core/providers";

import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { ConfirmDestructiveDialog } from "@/components/ui/confirm-destructive-dialog";
import { useDeleteProject } from "@/hooks/useConsoleQueries";

/** Human-readable message for the project-deletion failure modes the backend actually returns.
 *  Never surfaces the raw adapter error — mirrors `deleteOrgErrorMessage` in `danger-zone.tsx`. */
function deleteProjectErrorMessage(error: unknown): string {
  if (error instanceof GaussMeridianAdapterError) {
    if (error.status === 403) return "Only organization owners and admins can delete a project.";
    if (error.status === 404) return "This project was already deleted.";
  }
  return "Could not delete the project. Try again.";
}

/**
 * Irreversible project deletion, mirroring `orgs/danger-zone.tsx`'s org-level component:
 * `ConfirmDestructiveDialog` with typed confirmation (the caller must type the exact project
 * name to arm the delete button), gated to Owner/Admin (`canManageProjects` — the real backend's
 * `DELETE /v1/orgs/:id/projects/:pid` is Admin+, see `handlers.rs::delete_project`).
 *
 * Lives on the project Settings page (`.../projects/[projectId]/settings/page.tsx`) — the
 * project's only settings surface, so its danger zone belongs there rather than a bespoke new
 * route, matching how the org's danger zone lives on the org Settings page.
 *
 * Consequences copy: the backend's `delete_project` cascades — `ProjectRepository::
 * delete_cascade` deletes every `api_keys` row whose `project_id` links to this project in the
 * same transaction (DR-012 made `api_keys.project_id` a real `record<project>` link; commit
 * 3198466). Keys created WITHOUT a project scope (`project_id = NONE` — org-fallback keys)
 * survive project deletion, so the copy claims the scoped-key cascade and carves out exactly
 * that exception. (The FE-side `ApiKeySchema` doesn't expose `project_id`, so the mock registry
 * can't mirror the scoped cascade precisely — acceptable: mocks err toward keys surviving,
 * never toward claiming a revocation that didn't happen.)
 */
export function ProjectDangerZone() {
  const { orgId, projectId } = useParams<{ orgId: string; projectId: string }>();
  const { project, role } = useTenancy();
  const router = useRouter();
  const deleteProject = useDeleteProject(orgId, projectId);
  const permitted = canManageProjects(role);

  const [open, setOpen] = useState(false);

  function handleConfirm() {
    deleteProject.mutate(undefined, {
      onSuccess: () => {
        setOpen(false);
        router.push(`/orgs/${orgId}`);
      },
    });
  }

  return (
    <Card className="border-destructive/40 flex flex-col gap-4 p-6">
      <div>
        <CardTitle className="text-destructive text-base">Danger zone</CardTitle>
        <CardDescription className="mt-1">
          Deleting this project also removes every API key scoped to it. There is no undo.
        </CardDescription>
      </div>

      {!permitted && role && (
        <p className="text-muted-foreground text-sm">
          Only organization owners and admins can delete this project.
        </p>
      )}

      {project && (
        <ConfirmDestructiveDialog
          open={open}
          onOpenChange={setOpen}
          title="Delete project"
          resourceName={project.name}
          resourceLabel="project"
          confirmLabel={deleteProject.isPending ? "Deleting…" : "Delete project"}
          isBusy={deleteProject.isPending}
          error={deleteProject.isError ? deleteProjectErrorMessage(deleteProject.error) : null}
          onConfirm={handleConfirm}
          description={
            <>
              This permanently deletes <strong className="text-foreground">{project.name}</strong>,
              and every API key scoped to it. This cannot be undone.
            </>
          }
          trigger={
            <Button
              type="button"
              variant="outline"
              className="border-destructive text-destructive hover:bg-destructive/10 hover:text-destructive self-start"
              disabled={!permitted || deleteProject.isPending}
            >
              <Trash2 className="h-4 w-4" aria-hidden="true" />
              Delete project
            </Button>
          }
        />
      )}
    </Card>
  );
}
