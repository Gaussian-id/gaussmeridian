import { Card, CardTitle } from "@/components/ui/card";

import type { ModelCatalogEntry } from "./model-catalog";

interface ModelCardProps {
  model: ModelCatalogEntry;
}

function formatIdr(value: number): string {
  return `Rp${new Intl.NumberFormat("id-ID", { maximumFractionDigits: 0 }).format(value)}`;
}

/** One entry in the `/dashboard/models` catalog grid — pure prop-rendering, no data fetching. */
export function ModelCard({ model }: ModelCardProps) {
  return (
    <Card className="group hover:border-accent flex flex-col gap-4 p-6 transition-colors">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <span className="text-muted-foreground font-mono text-xs tracking-widest uppercase">
            GaussMeridian model
          </span>
          <CardTitle className="font-display mt-1 truncate text-lg" title={model.id}>
            {model.id}
          </CardTitle>
        </div>
      </div>

      <div className="border-border mt-auto border-t pt-4">
        {model.pricing ? (
          <div className="flex items-center justify-between gap-4 text-sm">
            <div>
              <p className="text-muted-foreground text-xs">Input / 1M tokens</p>
              <p className="font-mono">{formatIdr(model.pricing.inputPerMillion)}</p>
            </div>
            <div className="text-right">
              <p className="text-muted-foreground text-xs">Output / 1M tokens</p>
              <p className="font-mono">{formatIdr(model.pricing.outputPerMillion)}</p>
            </div>
          </div>
        ) : (
          <p className="text-muted-foreground text-xs">Retail rate not published</p>
        )}
      </div>
    </Card>
  );
}
