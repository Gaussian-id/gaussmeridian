/**
 * The public changelog's content — hand-authored, in-repo, typed. Each entry is sourced from
 * this product's actual commit history (the GaussMeridian console and router repos); see the
 * `date` on each entry against `git log --date=short` in either repo to trace it back.
 *
 * Why hand-authored instead of generated from commits: conventional-commit subjects are written
 * for engineers, not customers — "fix(auth): map login user-not-found to 401" needs a human pass
 * to become "sign-in errors now tell you what actually happened." A typed array keeps that pass
 * reviewable and type-checked without adding a build-time generation step.
 *
 * `body` is a tiny constrained shape — not markdown — on purpose: entries are all authored here,
 * never user-submitted, so there is no case for a real parser (or its dependency weight / string-
 * injection surface). `p` and `ul` blocks cover every entry below; inline text supports the three
 * spans product writing actually needs — `code`, **bold**, and [links](https://example.com) — via
 * the tiny tokenizer in `changelog.tsx`.
 *
 * Keep this array sorted newest-first — `__tests__/entries.test.ts` asserts it.
 */

export type ChangelogTag = "Console" | "Router" | "Onboarding" | "Auth" | "Marketing";

export type ChangelogBlock = { type: "p"; text: string } | { type: "ul"; items: string[] };

export interface ChangelogEntry {
  /** Unique, URL-safe; not currently routed to, but stable for anchors/keys/future deep-links. */
  slug: string;
  title: string;
  /** ISO date, YYYY-MM-DD. */
  date: string;
  tags: ChangelogTag[];
  body: ChangelogBlock[];
}

export const changelogEntries: ChangelogEntry[] = [
  {
    slug: "org-project-deletion-cascade",
    title: "Deleting an org or project now cleans up after itself",
    date: "2026-07-16",
    tags: ["Console"],
    body: [
      {
        type: "p",
        text: "Removing an organization or project now takes everything underneath it with it — memberships, API keys, BYOK credentials — in one cascade, instead of leaving orphaned records behind. You confirm by typing the resource's name, not just clicking a red button.",
      },
      {
        type: "p",
        text: "Fixed a serialization bug in the delete path that could corrupt the parent record mid-cascade, and added a type-to-confirm dialog (`ConfirmDestructiveDialog`) shared by both org and project settings.",
      },
    ],
  },
  {
    slug: "auth-error-clarity",
    title: "Sign-up and sign-in errors that actually tell you what happened",
    date: "2026-07-16",
    tags: ["Auth"],
    body: [
      {
        type: "p",
        text: 'Registering with an email or username already in use now returns a clear conflict instead of a generic failure. Signing in maps unknown accounts to "not found" and disabled accounts to "forbidden" — not one flat, unhelpful "unauthorized."',
      },
      {
        type: "ul",
        items: [
          "Register: duplicate email/username → `409 Conflict`",
          "Login: unknown account → `401`, inactive account → `403`",
        ],
      },
    ],
  },
  {
    slug: "route-transparency-live",
    title: "See exactly how every request was routed, live",
    date: "2026-07-16",
    tags: ["Router", "Console"],
    body: [
      {
        type: "p",
        text: "The console's transparency drawer now streams the real routing decision as it happens — which model served the call, why, and what the road not taken would have cost — over a live feed, not a mock.",
      },
      {
        type: "p",
        text: 'Backed by persisted route-decision data (DR-009) and an SSE feed replacing the old placeholder "charged" pill with an honest, real-data status.',
      },
    ],
  },
  {
    slug: "onboarding-meridian-earth",
    title: "Onboarding, rebuilt as Meridian Earth",
    date: "2026-07-16",
    tags: ["Onboarding"],
    body: [
      {
        type: "p",
        text: "First run is now a conversational, 7-step wizard set against a reactive Earth backdrop instead of a static form — gated so you can't wander into a dead end, and themed for AA contrast in both light and dark mode.",
      },
      {
        type: "p",
        text: "New `MeridianField` background and conversational wizard shell on the frontend, backed by a real onboarding schema, endpoints, and survey persistence (DR-010) on the router.",
      },
    ],
  },
  {
    slug: "multi-tenant-console",
    title: "A real multi-tenant console: orgs, projects, roles, keys",
    date: "2026-07-15",
    tags: ["Console"],
    body: [
      {
        type: "p",
        text: "The console is now a real multi-tenant workspace, not a demo shell. Create organizations, spin up projects inside them, invite teammates with scoped roles, and issue project-scoped API keys — all backed by live data.",
      },
      {
        type: "ul",
        items: [
          "Org / membership / role schema, repositories, and seed data",
          "Rank-bounded role assignment — closes an Admin-to-Owner privilege escalation",
          "Project-scoped API keys (DR-012)",
        ],
      },
    ],
  },
  {
    slug: "public-site-meridian-first",
    title: "The public site, rebuilt Meridian-first",
    date: "2026-07-14",
    tags: ["Marketing"],
    body: [
      {
        type: "p",
        text: "Pricing, story, docs, and this changelog now live under one coherent Meridian identity, in one route group, instead of the old GaussMeridian shell.",
      },
    ],
  },
  {
    slug: "cross-provider-fallback",
    title: "A provider outage doesn't have to be your outage",
    date: "2026-07-12",
    tags: ["Router"],
    body: [
      {
        type: "p",
        text: "If a provider times out or goes down mid-request, Meridian now reroutes to another servable model instead of erroring — the call completes, and the fallback chain shows up in your response headers.",
      },
    ],
  },
  {
    slug: "mixture-of-agents",
    title: "Mixture-of-Agents: hard prompts get more brains",
    date: "2026-07-12",
    tags: ["Router"],
    body: [
      {
        type: "p",
        text: "Genuinely hard prompts now fan out across several models in parallel and merge into the strongest answer automatically — one aggregated bill, no manual orchestration on your end.",
      },
      {
        type: "p",
        text: "CARROT-triggered gate dispatches to an in-process fan-out engine with a latency budget and single-model fallback, so MoA never becomes the slow path.",
      },
    ],
  },
];

/** The 4-digit year an entry's ISO date falls in — the sole source of year group headers. */
export function entryYear(entry: ChangelogEntry): string {
  return entry.date.slice(0, 4);
}

/**
 * Groups entries by year, newest year first, preserving each group's existing (newest-first)
 * entry order. Years are derived entirely from `entry.date` — never hardcoded.
 */
export function groupEntriesByYear(
  entries: ChangelogEntry[],
): { year: string; entries: ChangelogEntry[] }[] {
  const groups = new Map<string, ChangelogEntry[]>();
  for (const entry of entries) {
    const year = entryYear(entry);
    const bucket = groups.get(year);
    if (bucket) bucket.push(entry);
    else groups.set(year, [entry]);
  }
  return [...groups.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([year, yearEntries]) => ({
      year,
      entries: yearEntries,
    }));
}
