import { Badge } from "@/components/ui/badge";
import { groupEntriesByYear } from "@/lib/changelog/entries";
import type { ChangelogBlock, ChangelogEntry } from "@/lib/changelog/entries";

import type { ReactNode } from "react";

export function ChangelogHero() {
  return (
    <div className="mx-auto max-w-[52rem] px-6 pt-28 pb-10 text-center">
      <div className="text-muted-foreground font-mono text-[11px] tracking-[0.26em] uppercase">
        Changelog
      </div>
      <h1 className="font-display mt-3.5 text-4xl font-semibold tracking-tight sm:text-5xl">
        Everything we <span className="text-gradient">shipped</span>.
      </h1>
      <p className="text-muted-foreground mx-auto mt-3.5 max-w-[42ch] text-base leading-relaxed">
        The console, the router, and the line between them — as it actually happened.
      </p>
    </div>
  );
}

/** Splits inline text on `code`, **bold**, and [link](href) spans — the only formatting entries use. */
const INLINE_TOKEN = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g;
const LINK_TOKEN = /^\[([^\]]+)\]\(([^)]+)\)$/;

function renderInline(text: string, keyPrefix: string): ReactNode[] {
  return text
    .split(INLINE_TOKEN)
    .filter((part) => part.length > 0)
    .map((part, i) => {
      const key = `${keyPrefix}-${i}`;
      if (part.startsWith("`") && part.endsWith("`")) {
        return (
          <code
            key={key}
            className="text-primary rounded-md bg-[color-mix(in_srgb,var(--accent)_10%,transparent)] px-1.5 py-0.5 font-mono text-[0.9em]"
          >
            {part.slice(1, -1)}
          </code>
        );
      }
      if (part.startsWith("**") && part.endsWith("**")) {
        return (
          <strong key={key} className="text-foreground font-semibold">
            {part.slice(2, -2)}
          </strong>
        );
      }
      const link = LINK_TOKEN.exec(part);
      if (link) {
        return (
          <a key={key} href={link[2]} className="text-primary underline-offset-2 hover:underline">
            {link[1]}
          </a>
        );
      }
      return part;
    });
}

function ChangelogBody({ blocks }: { blocks: ChangelogBlock[] }) {
  return (
    <div className="space-y-3.5">
      {blocks.map((block, i) =>
        block.type === "ul" ? (
          <ul
            key={i}
            className="text-muted-foreground marker:text-muted-foreground/50 list-disc space-y-1.5 pl-5 text-[15px] leading-relaxed"
          >
            {block.items.map((item, j) => (
              <li key={j}>{renderInline(item, `b${i}-i${j}`)}</li>
            ))}
          </ul>
        ) : (
          <p key={i} className="text-muted-foreground text-[15px] leading-relaxed">
            {renderInline(block.text, `b${i}`)}
          </p>
        ),
      )}
    </div>
  );
}

const tagVariant: Partial<Record<ChangelogEntry["tags"][number], string>> = {
  Router: "border-[color-mix(in_srgb,var(--accent)_35%,var(--border))] text-primary",
};

function ChangelogTags({ tags }: { tags: ChangelogEntry["tags"] }) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {tags.map((tag) => (
        <Badge key={tag} variant="mono" className={tagVariant[tag]}>
          {tag}
        </Badge>
      ))}
    </div>
  );
}

/**
 * Each field (date/title/tags/body) is rendered exactly once — never duplicated between a
 * "mobile" and "desktop" copy. Duplicating headings for responsive reflow is a common trick, but
 * it double-announces the title to assistive tech and this repo's Vitest config runs with
 * `css: false` (`vitest.config.ts`), so jsdom cannot hide either copy — a duplicate would be a
 * false "multiple elements" test failure standing in for a real (if muted) production a11y bug.
 * Flexbox + `lg:` utilities reposition this single instance instead: stacked on mobile, a sticky
 * ~1/3-width rail beside the body on desktop. `top-28` clears both the sticky navbar (`h-16`) and
 * the sticky year-jump nav below it (`top-16`) so the two sticky layers never overlap.
 */
function ChangelogEntryArticle({ entry }: { entry: ChangelogEntry }) {
  return (
    <article className="border-border/60 flex flex-col gap-4 border-t py-10 first:border-none first:pt-0 lg:mt-16 lg:flex-row lg:items-start lg:gap-10 lg:border-none lg:py-0 lg:first:mt-0">
      <div className="bg-background/90 sticky top-16 z-10 -mx-6 px-6 py-2.5 backdrop-blur lg:sticky lg:top-28 lg:mx-0 lg:w-1/3 lg:shrink-0 lg:bg-transparent lg:px-0 lg:py-0 lg:backdrop-blur-none">
        <time dateTime={entry.date} className="text-muted-foreground font-mono text-xs">
          {entry.date}
        </time>
        <h2 className="font-display mt-2 text-xl font-semibold tracking-tight">{entry.title}</h2>
        <div className="mt-3">
          <ChangelogTags tags={entry.tags} />
        </div>
      </div>

      {/* border-l is the timeline spine running down the content column. */}
      <div className="lg:border-border lg:w-2/3 lg:border-l lg:pl-10">
        <ChangelogBody blocks={entry.body} />
      </div>
    </article>
  );
}

function YearJumpNav({ years }: { years: string[] }) {
  return (
    <nav
      aria-label="Jump to year"
      className="border-border/60 bg-background/85 sticky top-16 z-20 mx-auto mb-2 flex max-w-[76rem] gap-2 overflow-x-auto border-b px-6 py-3 backdrop-blur"
    >
      {years.map((year) => (
        <a
          key={year}
          href={`#year-${year}`}
          className="border-border text-muted-foreground hover:text-foreground hover:border-foreground/40 shrink-0 rounded-full border px-3 py-1 font-mono text-xs tracking-wide transition-colors"
        >
          {year}
        </a>
      ))}
    </nav>
  );
}

/** The release history: year-grouped, with a sticky ~1/3-width date/title rail beside each entry's body. */
export function ChangelogTimeline({ entries }: { entries: ChangelogEntry[] }) {
  const groups = groupEntriesByYear(entries);

  return (
    <div className="pb-24">
      <YearJumpNav years={groups.map((g) => g.year)} />
      <div className="mx-auto max-w-[76rem] px-6 pt-10">
        {groups.map(({ year, entries: yearEntries }) => (
          <section key={year} className="scroll-mt-24">
            <h2
              id={`year-${year}`}
              className="font-display text-muted-foreground/70 mb-8 scroll-mt-24 text-3xl font-semibold tracking-tight lg:mb-12"
            >
              {year}
            </h2>
            {yearEntries.map((entry) => (
              <ChangelogEntryArticle key={entry.slug} entry={entry} />
            ))}
          </section>
        ))}
      </div>
    </div>
  );
}
