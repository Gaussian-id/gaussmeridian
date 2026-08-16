"use client";

import { AlertDialog as AlertDialogPrimitive } from "radix-ui";
import { useId, useRef, useState } from "react";

import { cn } from "@core/lib/utils";

import { Button } from "./button";
import { Input } from "./input";
import { Label } from "./label";

import type { FormEvent, ReactNode } from "react";

export interface ConfirmDestructiveDialogProps {
  /** Renders an `AlertDialog.Trigger` around this element. Omit when the caller drives `open`
   *  from somewhere else (e.g. a per-row "Revoke" button in a table, where the trigger and the
   *  dialog aren't adjacent in the tree) — the dialog stays fully controlled either way. */
  trigger?: ReactNode;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  /** What gets destroyed — the consequences copy shown above the (optional) confirmation
   *  input. Never a placeholder: every caller must spell out the actual blast radius. */
  description: ReactNode;
  /** Exact string the user must type (case-sensitive, leading/trailing whitespace trimmed) to
   *  arm the confirm button — GitHub-style "type the name to confirm". Omit for lower-stakes
   *  destructive actions that still want this dialog's shell + consequences copy but not typed
   *  confirmation (e.g. revoking a single API key); the confirm button arms immediately. */
  resourceName?: string;
  /** Noun used in the typed-confirmation input's accessible name, e.g. "organization",
   *  "project". Only meaningful alongside `resourceName`. */
  resourceLabel?: string;
  confirmLabel: string;
  isBusy?: boolean;
  /** Mapped, human-readable failure message — never a raw backend/adapter error. */
  error?: string | null;
  onConfirm: () => void;
}

/**
 * Shared confirmation shell for every irreversible action in the console (org/project deletion,
 * API-key revocation, BYOK key removal). Built on the `radix-ui` package's `AlertDialog`
 * primitive (the same unified package `sheet.tsx`/`invite-member-dialog.tsx` already depend on
 * via `Dialog`) — `AlertDialog.Content` renders `role="alertdialog"` and wires
 * `aria-labelledby`/`aria-describedby` to `Title`/`Description` automatically; focus trap,
 * Escape-to-close, and focus-return-to-trigger all come from Radix, not reimplemented here.
 *
 * Typed confirmation (`resourceName`) is opt-in: when present, the confirm button stays
 * `disabled` until the input's trimmed value exactly matches `resourceName` (case-sensitive).
 * Paste is never blocked — `Input` is a plain controlled text field. Pressing Enter while the
 * input is focused submits the wrapping `<form>`, which no-ops while the confirm button is
 * still disabled (unarmed) per native HTML implicit-submission rules, and calls `onConfirm`
 * once armed — so "Enter submits when armed" falls out of the form semantics rather than a
 * bespoke keydown handler.
 */
export function ConfirmDestructiveDialog({
  trigger,
  open,
  onOpenChange,
  title,
  description,
  resourceName,
  resourceLabel = "resource",
  confirmLabel,
  isBusy = false,
  error,
  onConfirm,
}: ConfirmDestructiveDialogProps) {
  const [typed, setTyped] = useState("");
  const inputId = useId();
  const errorId = useId();
  const inputRef = useRef<HTMLInputElement>(null);
  const requiresTyping = Boolean(resourceName);
  const armed = requiresTyping ? typed.trim() === resourceName : true;

  function handleOpenChange(next: boolean) {
    onOpenChange(next);
    if (!next) setTyped("");
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!armed || isBusy) return;
    onConfirm();
  }

  const body = (
    <AlertDialogPrimitive.Content
      className={cn(
        "bg-card border-destructive/40 fixed top-1/2 left-1/2 z-50 w-full max-w-md",
        "-translate-x-1/2 -translate-y-1/2 rounded-xl border p-6 shadow-lg focus:outline-none",
      )}
      // Radix's supported way to steer initial focus (not the `autoFocus` DOM prop, which
      // `jsx-a11y/no-autofocus` bans): when this dialog wants typed confirmation, start focus
      // in the input so the user can type immediately, matching GitHub's delete-confirmation UX.
      onOpenAutoFocus={(event) => {
        if (requiresTyping && inputRef.current) {
          event.preventDefault();
          inputRef.current.focus();
        }
      }}
    >
      <AlertDialogPrimitive.Title className="font-display text-destructive text-lg font-semibold tracking-tight">
        {title}
      </AlertDialogPrimitive.Title>
      <AlertDialogPrimitive.Description asChild>
        <div className="text-muted-foreground mt-2 text-sm leading-relaxed">{description}</div>
      </AlertDialogPrimitive.Description>

      <form onSubmit={handleSubmit} className="mt-4 flex flex-col gap-4">
        {requiresTyping && (
          <div className="flex flex-col gap-1.5">
            <Label htmlFor={inputId}>
              Type <span className="text-foreground font-mono font-semibold">{resourceName}</span>{" "}
              to confirm
            </Label>
            <Input
              id={inputId}
              ref={inputRef}
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck={false}
              aria-label={`Type the ${resourceLabel} name to confirm`}
              aria-describedby={error ? errorId : undefined}
              value={typed}
              onChange={(event) => setTyped(event.target.value)}
              disabled={isBusy}
            />
          </div>
        )}

        {error && (
          <p role="alert" id={errorId} className="text-destructive text-sm">
            {error}
          </p>
        )}

        <div className="mt-2 flex justify-end gap-2">
          <AlertDialogPrimitive.Cancel asChild>
            <Button type="button" variant="outline" disabled={isBusy}>
              Cancel
            </Button>
          </AlertDialogPrimitive.Cancel>
          <Button type="submit" variant="destructive" disabled={!armed || isBusy}>
            {confirmLabel}
          </Button>
        </div>
      </form>
    </AlertDialogPrimitive.Content>
  );

  return (
    <AlertDialogPrimitive.Root open={open} onOpenChange={handleOpenChange}>
      {trigger !== undefined && (
        <AlertDialogPrimitive.Trigger asChild>{trigger}</AlertDialogPrimitive.Trigger>
      )}
      <AlertDialogPrimitive.Portal>
        <AlertDialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/50" />
        {body}
      </AlertDialogPrimitive.Portal>
    </AlertDialogPrimitive.Root>
  );
}
