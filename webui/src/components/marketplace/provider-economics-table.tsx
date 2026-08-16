"use client";

import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useModelPricing } from "@/hooks/useGaussmeridianQueries";

interface ProviderEconomicsTableProps {
  modelId: string;
}

/**
 * Per-provider pricing for one model, cross-referenced from `GET /v1/billing/models` (the same
 * pricing stub `model-catalog.ts` uses) by `model === modelId` — every matching row is a real
 * `provider` field on the fixture/response, not an invented multi-provider comparison. The
 * backend note on `ModelPricingResponseSchema` still applies: this is reference data, not a
 * live quote, and today's stub only ever returns one row per model — the table renders
 * whatever it gets rather than assuming more than one.
 */
export function ProviderEconomicsTable({ modelId }: ProviderEconomicsTableProps) {
  const pricing = useModelPricing();

  if (pricing.isLoading) {
    return (
      <Card className="p-6">
        <Skeleton className="h-4 w-40" />
        <Skeleton className="mt-4 h-10 w-full" />
      </Card>
    );
  }

  if (pricing.isError) {
    return (
      <Card className="p-6">
        <CardTitle>Provider economics</CardTitle>
        <p className="text-muted-foreground mt-3 text-sm">
          Could not load reference pricing. Try again shortly.
        </p>
      </Card>
    );
  }

  const rows = (pricing.data?.models ?? []).filter((entry) => entry.model === modelId);

  return (
    <Card className="p-6">
      <CardTitle>Provider economics</CardTitle>
      <CardDescription className="mt-1">
        Reference pricing per 1K tokens, by provider. Not a live quote.
      </CardDescription>

      {rows.length === 0 ? (
        <p className="text-muted-foreground mt-4 text-sm">
          No provider pricing on file for this model yet.
        </p>
      ) : (
        <div className="border-border bg-card mt-4 overflow-hidden rounded-xl border">
          <Table>
            <TableHeader>
              <TableRow className="bg-muted/40 hover:bg-muted/40">
                <TableHead>Provider</TableHead>
                <TableHead>Input / 1K</TableHead>
                <TableHead>Output / 1K</TableHead>
                <TableHead>Currency</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((entry) => (
                <TableRow key={entry.provider}>
                  <TableCell>{entry.provider}</TableCell>
                  <TableCell className="font-mono">
                    {entry.input_cost_per_1k_tokens.toFixed(4)}
                  </TableCell>
                  <TableCell className="font-mono">
                    {entry.output_cost_per_1k_tokens.toFixed(4)}
                  </TableCell>
                  <TableCell>{entry.currency}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </Card>
  );
}
