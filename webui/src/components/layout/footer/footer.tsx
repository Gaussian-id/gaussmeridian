import { siteConfig } from "@core/config";

import { BrandLogo } from "@/components/brand";

export function Footer() {
  return (
    <footer className="dark border-border bg-background text-foreground border-t">
      <div className="text-muted-foreground mx-auto flex w-full max-w-6xl flex-col items-center justify-between gap-4 px-6 py-8 text-sm sm:flex-row">
        {/* The footer hard-codes `dark`, so its ground never follows the theme — fixed light ink. */}
        <div className="flex flex-col items-center gap-2 sm:items-start">
          <BrandLogo tone="light" height={30} />
          <p>© {siteConfig.name}. Trustworthy autonomy by default.</p>
        </div>
        <div className="flex flex-col items-center gap-2 sm:items-end">
          <a href={`mailto:${siteConfig.contact.email}`} className="hover:text-foreground">
            {siteConfig.contact.email}
          </a>
          {/*
            AGPL-3.0 §13 source offer. This link is the mechanism the license itself
            suggests for a web application, and it is a compliance obligation rather
            than a design choice — it appears on every page because the footer does,
            and it must not be removed or hidden behind an interaction. Operators of a
            MODIFIED build repoint it at their own Corresponding Source via
            NEXT_PUBLIC_SOURCE_OFFER_URL; see the repository-root NOTICE.
          */}
          <a
            href={siteConfig.sourceOffer.url}
            target="_blank"
            rel="noreferrer"
            className="hover:text-foreground"
          >
            Source · {siteConfig.sourceOffer.license}
          </a>
        </div>
      </div>
    </footer>
  );
}
