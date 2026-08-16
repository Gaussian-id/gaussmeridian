"use client";

import { X } from "lucide-react";
import { Dialog as DialogPrimitive } from "radix-ui";
import { useState } from "react";

import type { Role } from "@core/adapters/schemas/console.schema";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import type { FormEvent, ReactNode } from "react";

const ROLE_OPTIONS: { value: Role; label: string; description: string }[] = [
  {
    value: "developer",
    label: "Developer",
    description: "Can use project APIs and view project activity.",
  },
  { value: "admin", label: "Admin", description: "Can also manage projects, keys, and members." },
  { value: "owner", label: "Owner", description: "Full control, including billing and deletion." },
];

interface InviteMemberDialogProps {
  trigger: ReactNode;
  /** Controlled — the parent owns `open` so it can close the dialog itself once the invite
   *  mutation actually succeeds, rather than closing optimistically on submit. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
  isPending: boolean;
  isError: boolean;
  onInvite: (input: { email: string; role: Role }) => void;
  assignableRoles: readonly Role[];
}

/**
 * Modal invite form, built directly on the `radix-ui` package's Dialog primitive (already a
 * dependency via `ui/select.tsx`) — a shared `ui/dialog.tsx` isn't introduced here since M3
 * plans a distinct `ui/sheet.tsx` (slide-in drawer) for the transparency drawer; this stays a
 * small, self-contained centered modal scoped to Team & Members.
 */
export function InviteMemberDialog({
  trigger,
  open,
  onOpenChange,
  isPending,
  isError,
  onInvite,
  assignableRoles,
}: InviteMemberDialogProps) {
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<Role>("developer");

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const trimmed = email.trim();
    if (!trimmed) return;
    onInvite({ email: trimmed, role });
  }

  function handleOpenChange(next: boolean) {
    onOpenChange(next);
    if (!next) {
      setEmail("");
      setRole("developer");
    }
  }

  return (
    <DialogPrimitive.Root open={open} onOpenChange={handleOpenChange}>
      <DialogPrimitive.Trigger asChild>{trigger}</DialogPrimitive.Trigger>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/50" />
        <DialogPrimitive.Content className="bg-card border-border fixed top-1/2 left-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-xl border p-6 shadow-lg focus:outline-none">
          <div className="flex items-start justify-between gap-4">
            <div>
              <DialogPrimitive.Title className="font-display text-lg font-semibold tracking-tight">
                Invite a member
              </DialogPrimitive.Title>
              <DialogPrimitive.Description className="text-muted-foreground mt-1 text-sm">
                They&apos;ll receive an invite for this organization at the role you choose.
              </DialogPrimitive.Description>
            </div>
            <DialogPrimitive.Close asChild>
              <Button type="button" variant="ghost" size="icon" aria-label="Close">
                <X className="h-4 w-4" aria-hidden="true" />
              </Button>
            </DialogPrimitive.Close>
          </div>

          <form onSubmit={handleSubmit} className="mt-4 flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="invite-email">Email</Label>
              <Input
                id="invite-email"
                type="email"
                required
                placeholder="teammate@example.com"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                disabled={isPending}
              />
            </div>

            <div className="flex flex-col gap-1.5">
              <Label htmlFor="invite-role">Role</Label>
              <Select
                value={role}
                onValueChange={(value) => setRole(value as Role)}
                disabled={isPending}
              >
                <SelectTrigger id="invite-role" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {ROLE_OPTIONS.filter((option) => assignableRoles.includes(option.value)).map(
                    (option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ),
                  )}
                </SelectContent>
              </Select>
              <p className="text-muted-foreground text-xs">
                {ROLE_OPTIONS.find((option) => option.value === role)?.description}
              </p>
            </div>

            {isError && (
              <p role="alert" className="text-destructive text-sm">
                Could not send the invite. Try again.
              </p>
            )}

            <div className="mt-2 flex justify-end gap-2">
              <DialogPrimitive.Close asChild>
                <Button type="button" variant="outline" disabled={isPending}>
                  Cancel
                </Button>
              </DialogPrimitive.Close>
              <Button type="submit" disabled={isPending || !email.trim()}>
                {isPending ? "Sending…" : "Send invite"}
              </Button>
            </div>
          </form>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
