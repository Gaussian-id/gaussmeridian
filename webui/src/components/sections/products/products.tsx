import { ArrowUpRight } from "lucide-react";

import { cn } from "@core/lib/utils";

import { Card } from "@/components/ui/card";

const products = [
  { name: "GaussMeridian", tagline: "Intelligent LLM API gateway.", featured: false },
  {
    name: "GaussMeridian",
    tagline: "Web console for routing, analytics, and key management.",
    featured: true,
  },
  { name: "Developer SDK", tagline: "OpenAI-compatible API for your apps.", featured: false },
];

export function Products() {
  return (
    <section aria-labelledby="products-heading" className="border-border border-t">
      <div className="mx-auto w-full max-w-6xl px-6 py-20">
        <div className="flex items-end justify-between gap-6">
          <h2
            id="products-heading"
            className="font-display text-3xl font-semibold tracking-tight md:text-4xl"
          >
            Open-source LLM infrastructure.
          </h2>
          <span className="text-muted-foreground hidden font-mono text-xs tracking-widest uppercase sm:block">
            the GaussMeridian suite
          </span>
        </div>

        <div className="mt-12 grid gap-4 md:grid-cols-3">
          {products.map((product) => (
            <Card
              key={product.name}
              className={cn(
                "group relative overflow-hidden p-6",
                product.featured && "bg-brand-gradient border-transparent text-white",
              )}
            >
              {product.featured && <div className="bg-grid absolute inset-0 opacity-20" />}
              <div className="relative flex h-full flex-col">
                <span
                  className={cn(
                    "font-mono text-xs tracking-widest uppercase",
                    product.featured ? "text-white/60" : "text-muted-foreground",
                  )}
                >
                  {product.featured ? "flagship" : "module"}
                </span>
                <h3 className="font-display mt-4 text-2xl font-semibold">{product.name}</h3>
                <p
                  className={cn(
                    "mt-2 text-sm",
                    product.featured ? "text-white/80" : "text-muted-foreground",
                  )}
                >
                  {product.tagline}
                </p>
                <span className="mt-8 inline-flex items-center gap-1 text-sm font-medium">
                  Learn more
                  <ArrowUpRight
                    className="h-4 w-4 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5"
                    aria-hidden="true"
                  />
                </span>
              </div>
            </Card>
          ))}
        </div>
      </div>
    </section>
  );
}
