import { TenancyProvider } from "@core/providers";

import { ChatWidget } from "@/components/chat";
import { CommandPaletteProvider } from "@/components/command";

import { AppSidebar } from "./app-sidebar";
import { AppTopbar } from "./app-topbar";

import type { ReactNode } from "react";

/** Authenticated-app frame: sidebar + topbar + main. Used by the (app) route group.
 *  `TenancyProvider` wraps everything below it so the sidebar and every screen can read
 *  org/project/role context via `useTenancy()`. `CommandPaletteProvider` mounts the ⌘K palette
 *  once here so both the global shortcut and the topbar's search field share one instance. */
export function AppShell({ children }: { children: ReactNode }) {
  return (
    <TenancyProvider>
      <CommandPaletteProvider>
        <div className="flex min-h-dvh">
          <AppSidebar />
          <div className="flex min-w-0 flex-1 flex-col">
            <AppTopbar />
            <main className="flex-1 px-6 py-8">{children}</main>
          </div>
          <ChatWidget />
        </div>
      </CommandPaletteProvider>
    </TenancyProvider>
  );
}
