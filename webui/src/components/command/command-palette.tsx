"use client";

import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "cmdk";
import {
  BarChart3,
  Boxes,
  Building2,
  CreditCard,
  KeyRound,
  LayoutDashboard,
  ListOrdered,
  Package,
  Search,
  Settings,
  ShieldCheck,
  Sparkles,
  Users,
  type LucideIcon,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { Dialog as DialogPrimitive } from "radix-ui";
import { useEffect, useState } from "react";

import { useTenancy } from "@core/providers";

import { useOrgProjects, useOrgs } from "@/hooks/useConsoleQueries";
import { useModels } from "@/hooks/useGaussmeridianQueries";

import { CarrotExplainer } from "./carrot-explainer";

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type PaletteMode = "root" | "carrot";

/**
 * ⌘K / Ctrl+K command palette: navigate the org/project tree, jump to a model, or type "CARROT"
 * for a short complexity-scoring explainer. Built on `cmdk` (accessible filtering/keyboard nav)
 * inside the same `radix-ui` package's Dialog primitive `sheet.tsx`/`invite-member-dialog.tsx`
 * already use — no extra dialog dependency needed. Deliberately unanimated: this is a calm,
 * utility surface (see the Playground for where cinematic motion belongs instead), so there is
 * no motion to make reduced-motion-safe in the first place.
 */
export function CommandPalette({ open, onOpenChange }: CommandPaletteProps) {
  const router = useRouter();
  const { org, project, mode: tenancyMode } = useTenancy();
  const orgs = useOrgs();
  const orgProjects = useOrgProjects(org?.id ?? "");
  const models = useModels();
  const [mode, setMode] = useState<PaletteMode>("root");
  const [search, setSearch] = useState("");

  function handleOpenChange(next: boolean) {
    onOpenChange(next);
    if (!next) {
      setMode("root");
      setSearch("");
    }
  }

  // Escape steps back out of the explainer before it closes the whole palette.
  useEffect(() => {
    if (!open) return;
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && mode === "carrot") {
        event.stopPropagation();
        setMode("root");
      }
    }
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [open, mode]);

  function go(href: string) {
    handleOpenChange(false);
    router.push(href);
  }

  return (
    <DialogPrimitive.Root open={open} onOpenChange={handleOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/50" />
        <DialogPrimitive.Content className="bg-card border-border fixed top-[18%] left-1/2 z-50 w-full max-w-lg -translate-x-1/2 overflow-hidden rounded-xl border shadow-lg focus:outline-none">
          <DialogPrimitive.Title className="sr-only">Command palette</DialogPrimitive.Title>
          <DialogPrimitive.Description className="sr-only">
            Search or jump to a page, project, model, or the CARROT explainer.
          </DialogPrimitive.Description>

          {mode === "carrot" ? (
            <CarrotExplainer onBack={() => setMode("root")} />
          ) : (
            <Command shouldFilter label="Search the console" className="flex flex-col">
              <div className="border-border flex items-center gap-2 border-b px-4">
                <Search className="text-muted-foreground h-4 w-4 shrink-0" aria-hidden="true" />
                <CommandInput
                  value={search}
                  onValueChange={setSearch}
                  placeholder="Search or jump to…"
                  className="placeholder:text-muted-foreground flex h-12 w-full bg-transparent text-sm outline-none"
                />
              </div>
              <CommandList className="max-h-80 overflow-y-auto p-2">
                <CommandEmpty className="text-muted-foreground py-6 text-center text-sm">
                  No results.
                </CommandEmpty>

                <CommandGroup heading="Global">
                  <PaletteItem
                    icon={Building2}
                    label="Organizations"
                    onSelect={() => go("/orgs")}
                  />
                  <PaletteItem
                    icon={Sparkles}
                    label="CARROT — complexity explainer"
                    keywords={["carrot", "complexity", "score", "routing"]}
                    onSelect={() => setMode("carrot")}
                  />
                </CommandGroup>

                {orgs.data && orgs.data.orgs.length > 0 && (
                  <CommandGroup heading="Organizations">
                    {orgs.data.orgs.map((candidate) => (
                      <PaletteItem
                        key={candidate.id}
                        icon={Building2}
                        label={candidate.name}
                        onSelect={() => go(`/orgs/${candidate.id}`)}
                      />
                    ))}
                  </CommandGroup>
                )}

                {org && (tenancyMode === "org" || tenancyMode === "project") && (
                  <CommandGroup heading={org.name}>
                    <PaletteItem
                      icon={LayoutDashboard}
                      label="Overview"
                      onSelect={() => go(`/orgs/${org.id}`)}
                    />
                    <PaletteItem
                      icon={Users}
                      label="Members"
                      onSelect={() => go(`/orgs/${org.id}/members`)}
                    />
                    <PaletteItem
                      icon={ShieldCheck}
                      label="Roles"
                      onSelect={() => go(`/orgs/${org.id}/roles`)}
                    />
                    <PaletteItem
                      icon={Settings}
                      label="Org settings"
                      onSelect={() => go(`/orgs/${org.id}/settings`)}
                    />
                  </CommandGroup>
                )}

                {org && orgProjects.data && orgProjects.data.projects.length > 0 && (
                  <CommandGroup heading="Projects">
                    {orgProjects.data.projects.map((candidate) => (
                      <PaletteItem
                        key={candidate.id}
                        icon={Package}
                        label={candidate.name}
                        onSelect={() => go(`/orgs/${org.id}/projects/${candidate.id}`)}
                      />
                    ))}
                  </CommandGroup>
                )}

                {org && project && tenancyMode === "project" && (
                  <CommandGroup heading={project.name}>
                    <PaletteItem
                      icon={LayoutDashboard}
                      label="Overview"
                      onSelect={() => go(`/orgs/${org.id}/projects/${project.id}`)}
                    />
                    <PaletteItem
                      icon={ListOrdered}
                      label="Activity"
                      onSelect={() => go(`/orgs/${org.id}/projects/${project.id}/activity`)}
                    />
                    <PaletteItem
                      icon={BarChart3}
                      label="Usage"
                      onSelect={() => go(`/orgs/${org.id}/projects/${project.id}/usage`)}
                    />
                    <PaletteItem
                      icon={Package}
                      label="Models"
                      onSelect={() => go(`/orgs/${org.id}/projects/${project.id}/models`)}
                    />
                    <PaletteItem
                      icon={KeyRound}
                      label="API keys"
                      onSelect={() => go(`/orgs/${org.id}/projects/${project.id}/keys`)}
                    />
                    <PaletteItem
                      icon={Boxes}
                      label="BYOK"
                      onSelect={() => go(`/orgs/${org.id}/projects/${project.id}/byok`)}
                    />
                    <PaletteItem
                      icon={Settings}
                      label="Project settings"
                      onSelect={() => go(`/orgs/${org.id}/projects/${project.id}/settings`)}
                    />
                  </CommandGroup>
                )}

                {org &&
                  project &&
                  tenancyMode === "project" &&
                  models.data &&
                  models.data.data.length > 0 && (
                    <CommandGroup heading="Models">
                      {models.data.data.map((entry) => (
                        <PaletteItem
                          key={entry.id}
                          icon={Package}
                          label={entry.id}
                          onSelect={() =>
                            go(`/orgs/${org.id}/projects/${project.id}/models/${entry.id}`)
                          }
                        />
                      ))}
                    </CommandGroup>
                  )}
              </CommandList>
            </Command>
          )}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

function PaletteItem({
  icon: Icon,
  label,
  keywords,
  onSelect,
}: {
  icon: LucideIcon;
  label: string;
  keywords?: string[];
  onSelect: () => void;
}) {
  return (
    <CommandItem
      value={label}
      keywords={keywords}
      onSelect={onSelect}
      className="data-[selected=true]:bg-secondary data-[selected=true]:text-secondary-foreground flex cursor-pointer items-center gap-2.5 rounded-md px-3 py-2 text-sm"
    >
      <Icon className="text-muted-foreground h-4 w-4 shrink-0" aria-hidden="true" />
      {label}
    </CommandItem>
  );
}
