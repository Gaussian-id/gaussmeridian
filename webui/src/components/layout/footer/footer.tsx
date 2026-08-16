import { siteConfig } from "@core/config";

export function Footer() {
  return (
    <footer className="dark border-border bg-background text-foreground border-t">
      <div className="text-muted-foreground mx-auto flex w-full max-w-6xl flex-col items-center justify-between gap-2 px-6 py-8 text-sm sm:flex-row">
        <p>© {siteConfig.name}. Trustworthy autonomy by default.</p>
        <a href={`mailto:${siteConfig.contact.email}`} className="hover:text-foreground">
          {siteConfig.contact.email}
        </a>
      </div>
    </footer>
  );
}
