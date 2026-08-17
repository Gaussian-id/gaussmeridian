import Link from "next/link";

import { BrandLogoResponsive } from "@/components/brand";

export function NavbarLogo() {
  return (
    <Link
      href="/"
      className="focus-visible:ring-ring flex items-center rounded-sm focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none"
    >
      <BrandLogoResponsive markHeight={28} lockupHeight={34} />
    </Link>
  );
}
