"use client";

import { createContext, useCallback, useContext, useEffect, useState } from "react";

import { CommandPalette } from "./command-palette";

import type { ReactNode } from "react";

interface CommandPaletteContextValue {
  open: () => void;
}

const CommandPaletteContext = createContext<CommandPaletteContextValue | null>(null);

/** Lets anything — the topbar's search field, in practice — trigger the same palette instance
 *  the global ⌘K/Ctrl+K shortcut opens, instead of each owning its own dialog state. */
export function useCommandPaletteTrigger(): CommandPaletteContextValue {
  const ctx = useContext(CommandPaletteContext);
  if (!ctx) {
    throw new Error("useCommandPaletteTrigger must be used within a <CommandPaletteProvider>");
  }
  return ctx;
}

/** Mounts the ⌘K command palette once, at the app shell, and owns the global keyboard
 *  listener. `⌘K` on macOS, `Ctrl+K` everywhere else. */
export function CommandPaletteProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const isCommandK = (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k";
      if (!isCommandK) return;
      event.preventDefault();
      setOpen((value) => !value);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const openPalette = useCallback(() => setOpen(true), []);

  return (
    <CommandPaletteContext.Provider value={{ open: openPalette }}>
      {children}
      <CommandPalette open={open} onOpenChange={setOpen} />
    </CommandPaletteContext.Provider>
  );
}
