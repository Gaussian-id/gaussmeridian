import Link from "next/link";

import { siteConfig } from "@core/config";

export function NavbarLogo() {
  return (
    <Link href="/" className="flex items-center gap-2.5 font-semibold tracking-tight">
      <span aria-hidden="true" className="bg-brand-gradient shadow-glow size-[18px] rounded-full" />
      <span className="font-display text-lg">{siteConfig.shortName}</span>
    </Link>
  );
}
