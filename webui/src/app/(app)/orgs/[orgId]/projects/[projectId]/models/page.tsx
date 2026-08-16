"use client";

import { useParams } from "next/navigation";
import { useMemo, useState } from "react";

import { DashboardPageHeader } from "@/components/dashboard/dashboard-page-header";
import { ModelCard } from "@/components/dashboard/model-card";
import { buildModelCatalog } from "@/components/dashboard/model-catalog";
import { Card } from "@/components/ui/card";
import { ErrorState } from "@/components/ui/error-state";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useCommerceCatalog } from "@/hooks/useConsoleQueries";
import { useModels } from "@/hooks/useGaussmeridianQueries";

const SKELETON_CARD_COUNT = 6;

/** Configured text-chat models with customer retail rates from the versioned commerce catalog. */
export default function ModelsPage() {
  const { orgId } = useParams<{ orgId: string; projectId: string }>();
  const models = useModels();
  const commerceCatalog = useCommerceCatalog(orgId);
  const [search, setSearch] = useState("");

  const catalog = useMemo(
    () =>
      models.data
        ? buildModelCatalog(models.data.data, commerceCatalog.data?.model_rates ?? [])
        : [],
    [commerceCatalog.data, models.data],
  );
  const filtered = useMemo(() => {
    const normalizedSearch = search.trim().toLowerCase();
    if (!normalizedSearch) return catalog;
    return catalog.filter((model) => model.id.toLowerCase().includes(normalizedSearch));
  }, [catalog, search]);

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
      <DashboardPageHeader
        eyebrow="Project"
        title="Models"
        description="Models currently enabled for text chat in GaussMeridian. Retail rates appear only when published in the versioned billing catalog."
      />

      {!models.isLoading && !models.isError && catalog.length > 0 && (
        <Input
          aria-label="Search models"
          className="w-full sm:w-72"
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Search by model…"
          value={search}
        />
      )}

      {models.isLoading ? (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: SKELETON_CARD_COUNT }).map((_, i) => (
            <Card key={i} className="flex flex-col gap-4 p-6">
              <Skeleton className="h-3 w-16" />
              <Skeleton className="h-6 w-32" />
              <Skeleton className="mt-4 h-10 w-full" />
            </Card>
          ))}
        </div>
      ) : models.isError ? (
        <ErrorState message="Could not load the model catalog. Try again shortly." />
      ) : catalog.length === 0 ? (
        <div className="border-border bg-card text-muted-foreground rounded-xl border p-8 text-center text-sm">
          No models are available yet.
        </div>
      ) : filtered.length === 0 ? (
        <div className="border-border bg-card text-muted-foreground rounded-xl border p-8 text-center text-sm">
          No models match this search.
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {filtered.map((model) => (
            <ModelCard key={model.id} model={model} />
          ))}
        </div>
      )}
    </div>
  );
}
