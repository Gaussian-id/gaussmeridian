import Link from "next/link";

import { cn } from "@core/lib/utils";

import { ScrollReveal } from "@/components/motion";
import { buttonVariants } from "@/components/ui/button";

/** Shared brand-gradient closing CTA band, one idea per screen. */
export function CtaBand({
  title,
  subtitle,
  primary,
  secondary,
}: {
  title: string;
  subtitle: string;
  primary: { label: string; href: string };
  secondary?: { label: string; href: string };
}) {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl items-center px-6 py-20">
      <ScrollReveal className="w-full">
        <div className="bg-brand-gradient shadow-glow rounded-[22px] px-6 py-14 text-center text-white">
          <h2 className="font-display text-3xl font-semibold tracking-tight md:text-4xl">
            {title}
          </h2>
          <p className="mx-auto mt-3 max-w-[46ch] text-white/75">{subtitle}</p>
          <div className="mt-7 flex flex-wrap justify-center gap-3">
            <Link
              href={primary.href}
              className={cn(
                buttonVariants({ size: "lg" }),
                "bg-white text-[var(--gauss-900)] hover:bg-white/90",
              )}
            >
              {primary.label}
            </Link>
            {secondary && (
              <Link
                href={secondary.href}
                className={cn(
                  buttonVariants({ variant: "outline", size: "lg" }),
                  "border-white/30 bg-white/10 text-white hover:bg-white/20 hover:text-white",
                )}
              >
                {secondary.label}
              </Link>
            )}
          </div>
        </div>
      </ScrollReveal>
    </section>
  );
}
