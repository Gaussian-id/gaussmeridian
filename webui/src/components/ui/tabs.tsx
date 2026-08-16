"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useId, useState, type KeyboardEvent, type ReactNode } from "react";

import { Button } from "@/components/ui/button";

export interface TabDescriptor {
  value: string;
  label: string;
}

interface TabsProps {
  /** The tab strip, left-to-right. The first entry is the default unless `defaultValue` is set. */
  tabs: TabDescriptor[];
  /** Accessible name for the `role="tablist"` — required (a tablist without a name reads poorly). */
  ariaLabel: string;
  /** Renders the active tab's panel content. Called with the active tab's `value`. */
  children: (active: string) => ReactNode;
  /** Query-string key this tab strip deep-links through. Default `"tab"`. */
  paramKey?: string;
  /** Active tab before the URL says otherwise. Defaults to `tabs[0].value`. */
  defaultValue?: string;
  className?: string;
}

/**
 * A reusable WAI-ARIA "tabs" widget (APG automatic-activation pattern), extracted from the
 * hand-rolled strip in `admin/deletions/page.tsx` and made **deep-linkable**: the active tab is
 * reflected in (and restorable from) the `?<paramKey>=` query string.
 *
 * URL as source of truth (no effect): the active tab is *derived* — the `?<paramKey>=` value when
 * it names a real tab, otherwise a small internal fallback state that a click updates. Every click
 * also writes the URL via `router.replace` (shallow, `scroll:false`), so once the param is set it
 * drives everything and Back/Forward + deep links land on the right tab for free. This avoids
 * syncing URL→state in an effect (a cascading-render smell), and stays instant/testable because the
 * internal fallback covers the no-param render.
 *
 * Because it calls `useSearchParams()`, a consumer that statically prerenders must wrap it in a
 * `<Suspense>` boundary (same requirement the auth pages already satisfy) — see the admin pages.
 */
export function Tabs({
  tabs,
  ariaLabel,
  children,
  paramKey = "tab",
  defaultValue,
  className,
}: TabsProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const baseId = useId();

  const fallback = defaultValue ?? tabs[0]?.value ?? "";
  const isValid = (value: string | null): value is string =>
    value !== null && tabs.some((t) => t.value === value);

  const paramValue = searchParams.get(paramKey);
  const [fallbackActive, setFallbackActive] = useState<string>(
    isValid(paramValue) ? paramValue : fallback,
  );
  const active = isValid(paramValue) ? paramValue : fallbackActive;

  function selectTab(value: string) {
    setFallbackActive(value);
    const next = new URLSearchParams(searchParams.toString());
    next.set(paramKey, value);
    router.replace(`?${next.toString()}`, { scroll: false });
  }

  const tabId = (value: string) => `${baseId}-tab-${value}`;
  const panelId = `${baseId}-panel`;

  // APG automatic activation: Left/Right cycle the roving tab stop and switch immediately;
  // Home/End jump to the ends. Focus follows selection (a ref-free `getElementById`, same idiom
  // the source used) so the keyboard user is never stranded on the old tab stop.
  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    let nextIndex: number | null = null;
    switch (event.key) {
      case "ArrowRight":
        nextIndex = (index + 1) % tabs.length;
        break;
      case "ArrowLeft":
        nextIndex = (index - 1 + tabs.length) % tabs.length;
        break;
      case "Home":
        nextIndex = 0;
        break;
      case "End":
        nextIndex = tabs.length - 1;
        break;
      default:
        return;
    }
    event.preventDefault();
    const next = tabs[nextIndex];
    selectTab(next.value);
    document.getElementById(tabId(next.value))?.focus();
  }

  return (
    <div className={className}>
      <div className="flex flex-wrap gap-2" role="tablist" aria-label={ariaLabel}>
        {tabs.map((tab, index) => (
          <Button
            key={tab.value}
            id={tabId(tab.value)}
            type="button"
            role="tab"
            aria-selected={active === tab.value}
            aria-controls={panelId}
            tabIndex={active === tab.value ? 0 : -1}
            variant={active === tab.value ? "secondary" : "outline"}
            size="sm"
            onClick={() => selectTab(tab.value)}
            onKeyDown={(event) => handleKeyDown(event, index)}
          >
            {tab.label}
          </Button>
        ))}
      </div>

      <div
        id={panelId}
        role="tabpanel"
        aria-labelledby={tabId(active)}
        tabIndex={0}
        className="mt-6 focus:outline-none"
      >
        {children(active)}
      </div>
    </div>
  );
}
