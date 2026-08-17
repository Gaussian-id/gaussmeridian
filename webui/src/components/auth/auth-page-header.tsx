import { BrandLogo } from "@/components/brand";

interface AuthPageHeaderProps {
  title: string;
  description: string;
}

/** Logo mark + title + description shared by every (auth) page. */
export function AuthPageHeader({ title, description }: AuthPageHeaderProps) {
  return (
    <div>
      {/* Only shown where the branded split panel is not: below `lg` the form is the whole page. */}
      <span className="flex lg:hidden">
        <BrandLogo variant="mark" height={40} />
      </span>
      <h1 className="font-display mt-4 text-3xl font-semibold tracking-tight lg:mt-0">{title}</h1>
      <p className="text-muted-foreground mt-1 text-sm">{description}</p>
    </div>
  );
}
