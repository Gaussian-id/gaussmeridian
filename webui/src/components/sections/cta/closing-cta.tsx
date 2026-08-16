import { Button } from "@/components/ui/button";

export function ClosingCta() {
  return (
    <section className="px-6 py-20">
      <div className="bg-brand-gradient shadow-glow relative mx-auto w-full max-w-6xl overflow-hidden rounded-3xl px-8 py-16 text-center text-white sm:px-16">
        <div className="bg-grid absolute inset-0 opacity-20" />
        <div className="relative mx-auto flex max-w-2xl flex-col items-center gap-6">
          <h2 className="font-display text-3xl font-semibold tracking-tight md:text-4xl">
            Switch on the automation you were once too afraid to trust.
          </h2>
          <p className="text-white/80">
            Trustworthy autonomy, made the default — an agility no amount of hiring could buy.
          </p>
          <Button
            variant="accent"
            size="lg"
            className="bg-white text-[var(--gauss-800)] hover:bg-white/90"
          >
            Request access
          </Button>
        </div>
      </div>
    </section>
  );
}
