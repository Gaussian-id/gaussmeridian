"use client";

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface PlaygroundHeaderProps {
  model: string;
  models: readonly { id: string }[];
  onModelChange: (model: string) => void;
}

/** The bridge exposes only the enabled GaussMeridian model catalog. Supplier selection and native
 * Meridian routing modes remain behind the provider boundary and are not represented here. */
export function PlaygroundHeader({ model, models, onModelChange }: PlaygroundHeaderProps) {
  return (
    <header className="border-border flex flex-wrap items-center gap-3 border-b px-6 py-4">
      <div>
        <h1 className="font-display text-lg font-semibold tracking-tight">Playground</h1>
        <p className="text-muted-foreground text-xs">Test an enabled model in this project.</p>
      </div>

      <Select value={model} onValueChange={onModelChange}>
        <SelectTrigger aria-label="Model" size="sm" className="ml-auto w-56">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {models.map((entry) => (
            <SelectItem key={entry.id} value={entry.id}>
              {entry.id}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </header>
  );
}
