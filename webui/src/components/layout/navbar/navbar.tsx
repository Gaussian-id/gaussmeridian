import { siteConfig, type NavbarPosition } from "@core/config";
import { cn } from "@core/lib/utils";

import { NavbarActions } from "./navbar-actions";
import { NavbarLogo } from "./navbar-logo";
import { NavbarMenu } from "./navbar-menu";
import { NavbarMobile } from "./navbar-mobile";

const positionClasses: Record<NavbarPosition, string> = {
  sticky: "sticky top-0",
  fixed: "fixed inset-x-0 top-0",
  static: "static",
};

/** App navigation. Reads declaratively; subparts are co-located in this folder. */
export function Navbar() {
  return (
    <header
      className={cn(
        "glass border-border z-50 w-full border-b",
        positionClasses[siteConfig.navbar.position],
      )}
    >
      <div className="mx-auto flex h-16 w-full max-w-6xl items-center justify-between px-6">
        <NavbarLogo />
        <NavbarMenu />
        <div className="flex items-center gap-2">
          <NavbarActions />
          <NavbarMobile />
        </div>
      </div>
    </header>
  );
}
