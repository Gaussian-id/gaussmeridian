"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { navConfig } from "@core/config";
import { cn } from "@core/lib/utils";

export function NavbarMenu() {
  const pathname = usePathname();

  return (
    <nav className="hidden items-center gap-1 md:flex" aria-label="Primary">
      {navConfig.main.map((item) => (
        <Link
          key={item.href}
          href={item.href}
          className={cn(
            "text-muted-foreground hover:text-foreground rounded-md px-3 py-2 text-sm font-medium transition-colors",
            pathname === item.href && "text-foreground",
          )}
        >
          {item.label}
        </Link>
      ))}
    </nav>
  );
}
