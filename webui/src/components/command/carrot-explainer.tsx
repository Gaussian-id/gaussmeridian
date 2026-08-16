import { ArrowLeft } from "lucide-react";

import { Button } from "@/components/ui/button";

interface CarrotExplainerProps {
  onBack: () => void;
}

/** Swapped in for the palette's search list when the operator types "CARROT" and selects the
 *  explainer entry — same complexity-scoring story `docs/docs.tsx`'s Routing section tells,
 *  condensed to command-palette length. */
export function CarrotExplainer({ onBack }: CarrotExplainerProps) {
  return (
    <div className="flex flex-col gap-3 p-4">
      <div className="flex items-center justify-between">
        <span className="text-accent font-mono text-[10.5px] tracking-[0.16em] uppercase">
          CARROT
        </span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 gap-1.5 px-2"
          onClick={onBack}
        >
          <ArrowLeft className="h-3.5 w-3.5" aria-hidden="true" />
          Back
        </Button>
      </div>
      <p className="text-foreground text-sm leading-relaxed">
        CARROT is GaussMeridian&apos;s complexity scorer. Every prompt sent with{" "}
        <code className="text-primary rounded bg-[color-mix(in_srgb,var(--accent)_10%,transparent)] px-1 py-0.5 font-mono text-[0.9em]">
          model:&quot;auto&quot;
        </code>{" "}
        is scored 0–1 before routing — low scores stay on cheap, fast models; high scores can
        trigger a cascade escalation or fan out across models with GaussMoA.
      </p>
      <p className="text-muted-foreground text-xs leading-relaxed">
        The score rides back on every response as{" "}
        <code className="text-primary">x-gaussmeridian-complexity</code>, alongside the ranked
        candidates xRouter considered — see the route-transparency drawer on any Overview or
        Activity row for a live example.
      </p>
    </div>
  );
}
