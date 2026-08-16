"use client";

import { Check, Minus } from "lucide-react";
import { useParams } from "next/navigation";
import { Fragment } from "react";

import type { Role } from "@core/adapters/schemas/console.schema";

import { Badge } from "@/components/ui/badge";
import { ErrorState } from "@/components/ui/error-state";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { usePermissionMatrix } from "@/hooks/useConsoleQueries";

const ROLE_LABEL: Record<Role, string> = { owner: "Owner", admin: "Admin", developer: "Developer" };

/**
 * What Owner, Admin, and Developer can each do in this organization — rows are permissions
 * (grouped), columns are the 3 fixed RBAC roles. Read-only: role -> permission grants are
 * defined by the platform, not editable per org.
 */
export function PermissionMatrix() {
  const { orgId } = useParams<{ orgId: string }>();
  const matrix = usePermissionMatrix(orgId);

  if (matrix.isLoading) {
    return (
      <div className="border-border bg-card flex flex-col gap-2 rounded-xl border p-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <Skeleton key={i} className="h-8 w-full" />
        ))}
      </div>
    );
  }

  if (matrix.isError || !matrix.data) {
    return <ErrorState message="Could not load the permission matrix. Try again shortly." />;
  }

  const { permissions, roles } = matrix.data;
  const groups = [...new Set(permissions.map((permission) => permission.group))];

  function hasGrant(role: Role, permissionKey: string): boolean {
    return roles.find((entry) => entry.role === role)?.grants.includes(permissionKey) ?? false;
  }

  return (
    <div className="border-border bg-card overflow-hidden rounded-xl border">
      <Table>
        <TableHeader>
          <TableRow className="bg-muted/40 hover:bg-muted/40">
            <TableHead>Permission</TableHead>
            {roles.map((entry) => (
              <TableHead key={entry.role} className="text-center">
                <Badge variant="mono">{ROLE_LABEL[entry.role]}</Badge>
              </TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {groups.map((group) => (
            <Fragment key={group}>
              <TableRow className="hover:bg-transparent">
                <TableCell
                  colSpan={roles.length + 1}
                  className="text-muted-foreground bg-secondary/30 py-2 font-mono text-xs tracking-[0.15em] uppercase"
                >
                  {group}
                </TableCell>
              </TableRow>
              {permissions
                .filter((permission) => permission.group === group)
                .map((permission) => (
                  <TableRow key={permission.key}>
                    <TableCell className="whitespace-normal">{permission.label}</TableCell>
                    {roles.map((entry) => (
                      <TableCell key={entry.role} className="text-center">
                        {hasGrant(entry.role, permission.key) ? (
                          <Check
                            className="text-accent mx-auto h-4 w-4"
                            aria-label={`${ROLE_LABEL[entry.role]} can ${permission.label.toLowerCase()}`}
                          />
                        ) : (
                          <Minus
                            className="text-muted-foreground/40 mx-auto h-4 w-4"
                            aria-label={`${ROLE_LABEL[entry.role]} cannot ${permission.label.toLowerCase()}`}
                          />
                        )}
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
            </Fragment>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
