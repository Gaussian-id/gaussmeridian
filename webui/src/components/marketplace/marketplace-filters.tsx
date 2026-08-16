"use client";

import { DataTableToolbar } from "@/components/ui/data-table";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

import { ALL_PROVIDERS_VALUE, ALL_TIERS_VALUE, UNTIERED_VALUE } from "./filter-catalog";

import type { MarketplaceFilterState, PriceSort } from "./filter-catalog";

interface MarketplaceFiltersProps {
  filters: MarketplaceFilterState;
  onFiltersChange: (next: MarketplaceFilterState) => void;
  providers: string[];
  tiers: string[];
}

const PRICE_SORT_OPTIONS: { value: PriceSort; label: string }[] = [
  { value: "none", label: "Default order" },
  { value: "price-asc", label: "Price: low to high" },
  { value: "price-desc", label: "Price: high to low" },
];

/**
 * Filter/sort bar above the model marketplace grid. Every option is derived from the loaded
 * catalog (`collectProviders`/`collectTiers`) rather than hardcoded — there is no capability
 * filter because the list-level `/v1/models` response carries no capability field (that only
 * exists on the single-model detail response), so one isn't faked here.
 */
export function MarketplaceFilters({
  filters,
  onFiltersChange,
  providers,
  tiers,
}: MarketplaceFiltersProps) {
  return (
    <DataTableToolbar>
      <Input
        placeholder="Search by model or provider…"
        value={filters.search}
        onChange={(event) => onFiltersChange({ ...filters, search: event.target.value })}
        className="w-full sm:w-64"
        aria-label="Search models"
      />

      <Select
        value={filters.provider}
        onValueChange={(value) => onFiltersChange({ ...filters, provider: value })}
      >
        <SelectTrigger className="w-40" aria-label="Filter by provider">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={ALL_PROVIDERS_VALUE}>All providers</SelectItem>
          {providers.map((provider) => (
            <SelectItem key={provider} value={provider}>
              {provider}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Select
        value={filters.tier}
        onValueChange={(value) => onFiltersChange({ ...filters, tier: value })}
      >
        <SelectTrigger className="w-40" aria-label="Filter by tier">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={ALL_TIERS_VALUE}>All tiers</SelectItem>
          <SelectItem value={UNTIERED_VALUE}>Untiered</SelectItem>
          {tiers.map((tier) => (
            <SelectItem key={tier} value={tier}>
              {tier}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Select
        value={filters.priceSort}
        onValueChange={(value) => onFiltersChange({ ...filters, priceSort: value as PriceSort })}
      >
        <SelectTrigger className="w-44" aria-label="Sort by price">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {PRICE_SORT_OPTIONS.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </DataTableToolbar>
  );
}
