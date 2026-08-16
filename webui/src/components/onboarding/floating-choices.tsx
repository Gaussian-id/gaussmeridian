"use client";

import { useRef, useState, type KeyboardEvent, type ReactNode } from "react";

import { cn } from "@core/lib/utils";

export interface FloatingChoiceItem {
  value: string;
  label: string;
  icon?: ReactNode;
}

interface FloatingChoicesProps {
  options: FloatingChoiceItem[];
  value: string | null;
  onChange: (value: string | null) => void;
  ariaLabel: string;
  required?: boolean;
  className?: string;
}

/**
 * `<FloatingChoices>` — an accessible single-select rendered as a bare, card-less list that
 * floats directly over the onboarding Earth (PRD-22 follow-up, Shelby: "no cards, so the Earth
 * shows through"). It keeps the same WAI-ARIA radiogroup contract as the retired `<OptionCards>`
 * — container `role="radiogroup"`, each option `role="radio"`, a single roving tab stop, arrow
 * keys move focus+selection, Enter/Space or click select — but drops all box chrome: selection
 * reads as an illuminated marker plus a brightened, weight-shifted label, never a filled tile.
 * Rows reveal with a staggered rise-in on mount (skipped under `prefers-reduced-motion`). Text is
 * light-on-scene by design; the stage forces a dark scene so it stays legible over the globe.
 */
export function FloatingChoices({
  options,
  value,
  onChange,
  ariaLabel,
  required = false,
  className,
}: FloatingChoicesProps) {
  // Which option is the roving tab stop — the selected one, or the first until something is
  // focused/selected, so the group is reachable via a single Tab from outside.
  const [focusedValue, setFocusedValue] = useState<string | null>(value ?? null);
  const tabStop = focusedValue ?? value ?? options[0]?.value ?? null;
  const nodeRefs = useRef<Map<string, HTMLDivElement>>(new Map());

  function selectAt(index: number) {
    const option = options[index];
    if (!option) return;
    setFocusedValue(option.value);
    onChange(option.value);
    nodeRefs.current.get(option.value)?.focus();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>, index: number) {
    switch (event.key) {
      case "ArrowRight":
      case "ArrowDown": {
        event.preventDefault();
        selectAt((index + 1) % options.length);
        break;
      }
      case "ArrowLeft":
      case "ArrowUp": {
        event.preventDefault();
        selectAt((index - 1 + options.length) % options.length);
        break;
      }
      case "Enter":
      case " ": {
        event.preventDefault();
        selectAt(index);
        break;
      }
      default:
        break;
    }
  }

  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      aria-required={required || undefined}
      className={cn("flex flex-col", className)}
    >
      {options.map((option, index) => {
        const selected = option.value === value;
        return (
          <div
            key={option.value}
            ref={(node) => {
              if (node) nodeRefs.current.set(option.value, node);
              else nodeRefs.current.delete(option.value);
            }}
            role="radio"
            aria-checked={selected}
            tabIndex={option.value === tabStop ? 0 : -1}
            onClick={() => selectAt(index)}
            onFocus={() => setFocusedValue(option.value)}
            onKeyDown={(event) => handleKeyDown(event, index)}
            className={cn(
              "group flex min-h-11 cursor-pointer items-center gap-3.5 rounded-md py-2.5 transition-transform outline-none",
              "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-[#5ad1ff]/60",
              selected ? "translate-x-1.5" : "hover:translate-x-1",
            )}
          >
            <span
              aria-hidden="true"
              className={cn(
                "h-2.5 w-2.5 shrink-0 rounded-full transition-all duration-300",
                selected
                  ? "bg-[#5ad1ff] shadow-[0_0_14px_3px_rgba(90,209,255,0.7)]"
                  : "border border-white/40 group-hover:border-white/75",
              )}
            />
            {option.icon && (
              <span aria-hidden="true" className="text-lg leading-none">
                {option.icon}
              </span>
            )}
            <span
              className={cn(
                "text-[15px] transition-colors sm:text-base",
                selected
                  ? "font-semibold text-white [text-shadow:0_1px_16px_rgba(90,209,255,0.35)]"
                  : "font-medium text-white/55 group-hover:text-white/90",
              )}
            >
              {option.label}
            </span>
          </div>
        );
      })}
    </div>
  );
}
