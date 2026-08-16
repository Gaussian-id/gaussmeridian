"use client";

import { Menu } from "lucide-react";
import { useState } from "react";

import { siteConfig } from "@core/config";

import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";

import { AppNavigation } from "./app-navigation";

/** Phone and narrow-tablet access to the same product navigation shown in the desktop sidebar. */
export function AppMobileNavigation() {
  const [open, setOpen] = useState(false);

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="shrink-0 md:hidden"
          aria-label="Open application navigation"
        >
          <Menu className="h-5 w-5" aria-hidden="true" />
        </Button>
      </SheetTrigger>
      <SheetContent side="left" className="w-[min(20rem,calc(100vw-2rem))] sm:max-w-xs">
        <SheetHeader>
          <SheetTitle>Product navigation</SheetTitle>
          <SheetDescription>{siteConfig.name} workspace and project destinations.</SheetDescription>
        </SheetHeader>
        <SheetBody className="flex flex-col px-0 py-0">
          <AppNavigation ariaLabel="Mobile application" onNavigate={() => setOpen(false)} />
        </SheetBody>
      </SheetContent>
    </Sheet>
  );
}
