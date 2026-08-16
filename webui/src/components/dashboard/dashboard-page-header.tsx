import type { ReactNode } from "react";

interface DashboardPageHeaderProps {
  eyebrow: string;
  title: string;
  description: ReactNode;
}

/** Eyebrow + title + description header shared by every `/dashboard/*` page. */
export function DashboardPageHeader({ eyebrow, title, description }: DashboardPageHeaderProps) {
  return (
    <div>
      <span className="text-muted-foreground font-mono text-xs tracking-[0.2em] uppercase">
        {eyebrow}
      </span>
      <h1 className="font-display mt-1 text-3xl font-semibold tracking-tight">{title}</h1>
      <p className="text-muted-foreground mt-1">{description}</p>
    </div>
  );
}
