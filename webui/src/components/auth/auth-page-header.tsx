import { MeridianMark } from "@/components/auth/meridian-mark";

interface AuthPageHeaderProps {
  title: string;
  description: string;
}

/** Logo mark + title + description shared by every (auth) page. */
export function AuthPageHeader({ title, description }: AuthPageHeaderProps) {
  return (
    <div>
      <span className="bg-brand-gradient shadow-glow grid h-10 w-10 place-items-center rounded-lg text-white lg:hidden">
        <MeridianMark className="h-5 w-5" />
      </span>
      <h1 className="font-display mt-4 text-3xl font-semibold tracking-tight lg:mt-0">{title}</h1>
      <p className="text-muted-foreground mt-1 text-sm">{description}</p>
    </div>
  );
}
