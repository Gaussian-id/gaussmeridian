import Link from "next/link";

import { cn } from "@core/lib/utils";

import { ScrollReveal } from "@/components/motion";
import { buttonVariants } from "@/components/ui/button";

export function HomeCta() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl items-center px-6 py-20">
      <ScrollReveal className="w-full">
        <div className="bg-brand-gradient shadow-glow rounded-[22px] px-6 py-14 text-center text-white">
          <h2 className="font-display text-3xl font-semibold tracking-tight md:text-4xl">
            Point one line at Meridian.
          </h2>
          <p className="mx-auto mt-3 max-w-[44ch] text-white/75">
            Create a project key, add prepaid organization credit, and call a supported model from
            the API you already know.
          </p>
          <div className="mt-7 flex flex-wrap justify-center gap-3">
            <Link
              href="/signup"
              className={cn(
                buttonVariants({ size: "lg" }),
                "bg-white text-[var(--gauss-900)] hover:bg-white/90",
              )}
            >
              Get API key →
            </Link>
          </div>
        </div>
      </ScrollReveal>
    </section>
  );
}
