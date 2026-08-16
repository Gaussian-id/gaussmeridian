/**
 * PRD-24 Wave C — the superadmin revenue/resource observability surface fixtures
 * (`GET /v1/admin/{overview,finance,cost,orgs,projects,watchlist}`). Mirrors the Wave-A backend
 * shapes exactly. Deliberately seeded to demonstrate the three things the console exists to show:
 *
 *  1. **The Bleed** — write-offs (OutcomeGate `r_binary=0`) plus uncollected free-tier usage.
 *     Freebird Collective and Zenith Data are free-tier orgs whose provider cost is never
 *     recovered; every month carries a non-zero `written_off` + `uncollected` split.
 *  2. **Ranking** — provider cost spread across orgs/projects/models so the pivot and Watchlist
 *     have something to rank (Northwind AI is the spend leader; Freebird the bleed leader).
 *  3. **Idle** — Fresh Start Inc and Ghost Ventures have gone quiet (>7d / never), so the
 *     Watchlist idle set is never empty.
 *
 * Reuses the org/project identities already seeded in `orgs.ts`/`projects.ts` where they overlap
 * (`org_meridian`, `org_born_empty`) so the admin console and the tenancy console agree.
 */

export interface BusinessMonthFixture {
  month: string;
  revenue: number;
  provider_cost: number;
  written_off: number;
  uncollected: number;
  bleed: number;
  recovery_rate: number;
  mau_api: number;
  mau_console: number;
  new_users: number;
  new_orgs: number;
  new_projects: number;
  active_orgs: number;
  active_projects: number;
  requests: number;
  tokens: number;
}

export interface CostPivotRowFixture {
  key: string;
  label: string;
  cost: number;
  requests: number;
  recovery_rate: number;
  last_seen: string | null;
}

export interface OrgRowFixture {
  id: string;
  name: string;
  plan: string;
  status: string;
  revenue: number;
  provider_cost: number;
  written_off: number;
  uncollected: number;
  bleed: number;
  write_off_rate: number;
  recovery_rate: number;
  requests: number;
  tokens: number;
  last_activity: string | null;
}

export interface ProjectRowFixture extends OrgRowFixture {
  org_id: string;
  org_name: string;
  key_count: number;
}

export interface IdleRowFixture {
  id: string;
  name: string;
  plan: string;
  status: string;
  last_activity: string | null;
}

export interface AdminAuditEntryFixture {
  id: string;
  actor_email: string;
  action: string;
  target_type: string;
  target_id: string;
  reason: string | null;
  created_at: string;
}

/** 12 months, newest last — `overview.current` is `series.at(-1)`, matching the backend's
 *  `series.last()` convention. Windowing (3/6/12) slices the tail. Bleed climbs faster than
 *  revenue over the year: the console's whole reason to exist. */
export const businessMonths: BusinessMonthFixture[] = [
  {
    month: "2025-08",
    revenue: 2140.5,
    provider_cost: 2260.8,
    written_off: 190.4,
    uncollected: 310.2,
    bleed: 500.6,
    recovery_rate: 0.82,
    mau_api: 61,
    mau_console: 138,
    new_users: 22,
    new_orgs: 4,
    new_projects: 9,
    active_orgs: 12,
    active_projects: 21,
    requests: 184_200,
    tokens: 41_800_000,
  },
  {
    month: "2025-09",
    revenue: 2480.75,
    provider_cost: 2610.4,
    written_off: 214.6,
    uncollected: 352.9,
    bleed: 567.5,
    recovery_rate: 0.81,
    mau_api: 68,
    mau_console: 152,
    new_users: 25,
    new_orgs: 3,
    new_projects: 11,
    active_orgs: 13,
    active_projects: 24,
    requests: 205_600,
    tokens: 47_300_000,
  },
  {
    month: "2025-10",
    revenue: 2790.3,
    provider_cost: 2985.1,
    written_off: 246.1,
    uncollected: 401.7,
    bleed: 647.8,
    recovery_rate: 0.8,
    mau_api: 74,
    mau_console: 167,
    new_users: 27,
    new_orgs: 5,
    new_projects: 12,
    active_orgs: 15,
    active_projects: 28,
    requests: 231_900,
    tokens: 53_100_000,
  },
  {
    month: "2025-11",
    revenue: 3105.9,
    provider_cost: 3320.55,
    written_off: 271.8,
    uncollected: 448.3,
    bleed: 720.1,
    recovery_rate: 0.79,
    mau_api: 82,
    mau_console: 184,
    new_users: 30,
    new_orgs: 4,
    new_projects: 13,
    active_orgs: 16,
    active_projects: 31,
    requests: 258_400,
    tokens: 59_600_000,
  },
  {
    month: "2025-12",
    revenue: 3402.15,
    provider_cost: 3690.2,
    written_off: 305.4,
    uncollected: 502.6,
    bleed: 808.0,
    recovery_rate: 0.78,
    mau_api: 89,
    mau_console: 201,
    new_users: 31,
    new_orgs: 6,
    new_projects: 15,
    active_orgs: 18,
    active_projects: 35,
    requests: 284_700,
    tokens: 66_200_000,
  },
  {
    month: "2026-01",
    revenue: 3760.4,
    provider_cost: 4010.9,
    written_off: 334.2,
    uncollected: 561.8,
    bleed: 896.0,
    recovery_rate: 0.78,
    mau_api: 98,
    mau_console: 222,
    new_users: 35,
    new_orgs: 5,
    new_projects: 16,
    active_orgs: 20,
    active_projects: 39,
    requests: 312_500,
    tokens: 73_400_000,
  },
  {
    month: "2026-02",
    revenue: 4088.6,
    provider_cost: 4402.35,
    written_off: 372.9,
    uncollected: 618.4,
    bleed: 991.3,
    recovery_rate: 0.77,
    mau_api: 106,
    mau_console: 240,
    new_users: 34,
    new_orgs: 4,
    new_projects: 17,
    active_orgs: 21,
    active_projects: 42,
    requests: 338_900,
    tokens: 80_100_000,
  },
  {
    month: "2026-03",
    revenue: 4405.2,
    provider_cost: 4760.7,
    written_off: 408.1,
    uncollected: 671.9,
    bleed: 1080.0,
    recovery_rate: 0.77,
    mau_api: 118,
    mau_console: 261,
    new_users: 38,
    new_orgs: 6,
    new_projects: 19,
    active_orgs: 23,
    active_projects: 46,
    requests: 366_200,
    tokens: 87_500_000,
  },
  {
    month: "2026-04",
    revenue: 4712.85,
    provider_cost: 5150.4,
    written_off: 451.6,
    uncollected: 738.2,
    bleed: 1189.8,
    recovery_rate: 0.76,
    mau_api: 131,
    mau_console: 288,
    new_users: 41,
    new_orgs: 5,
    new_projects: 20,
    active_orgs: 25,
    active_projects: 51,
    requests: 397_800,
    tokens: 95_900_000,
  },
  {
    month: "2026-05",
    revenue: 5024.1,
    provider_cost: 5528.9,
    written_off: 496.3,
    uncollected: 805.7,
    bleed: 1302.0,
    recovery_rate: 0.76,
    mau_api: 148,
    mau_console: 312,
    new_users: 44,
    new_orgs: 7,
    new_projects: 22,
    active_orgs: 27,
    active_projects: 56,
    requests: 428_600,
    tokens: 104_200_000,
  },
  {
    month: "2026-06",
    revenue: 5338.7,
    provider_cost: 5946.25,
    written_off: 548.9,
    uncollected: 882.4,
    bleed: 1431.3,
    recovery_rate: 0.75,
    mau_api: 166,
    mau_console: 340,
    new_users: 47,
    new_orgs: 6,
    new_projects: 24,
    active_orgs: 29,
    active_projects: 61,
    requests: 461_300,
    tokens: 113_500_000,
  },
  {
    month: "2026-07",
    revenue: 5602.25,
    provider_cost: 6318.6,
    written_off: 601.7,
    uncollected: 951.8,
    bleed: 1553.5,
    recovery_rate: 0.75,
    mau_api: 182,
    mau_console: 366,
    new_users: 49,
    new_orgs: 8,
    new_projects: 27,
    active_orgs: 31,
    active_projects: 67,
    requests: 494_900,
    tokens: 123_100_000,
  },
];

/** Org directory rows. `bleed = written_off + uncollected`. Free-tier orgs carry the uncollected
 *  arm (usage we never bill); every org carries some write-off (OutcomeGate zero-charges). Status
 *  spans active/locked/suspended so the directory's status column has real variety. */
export const orgRows: OrgRowFixture[] = [
  {
    id: "org_northwind",
    name: "Northwind AI",
    plan: "scale",
    status: "active",
    revenue: 2140.6,
    provider_cost: 2384.2,
    written_off: 214.8,
    uncollected: 96.4,
    bleed: 311.2,
    write_off_rate: 0.09,
    recovery_rate: 0.87,
    requests: 182_400,
    tokens: 46_900_000,
    last_activity: "2026-07-17T15:12:00Z",
  },
  {
    id: "org_freebird",
    name: "Freebird Collective",
    plan: "free",
    status: "active",
    revenue: 0,
    provider_cost: 604.9,
    written_off: 118.3,
    uncollected: 486.6,
    bleed: 604.9,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 58_700,
    tokens: 14_200_000,
    last_activity: "2026-07-17T11:48:00Z",
  },
  {
    id: "org_meridian",
    name: "Meridian Labs",
    plan: "pro",
    status: "active",
    revenue: 1486.25,
    provider_cost: 1602.8,
    written_off: 131.5,
    uncollected: 42.9,
    bleed: 174.4,
    write_off_rate: 0.08,
    recovery_rate: 0.89,
    requests: 121_300,
    tokens: 31_500_000,
    last_activity: "2026-07-17T08:05:00Z",
  },
  {
    id: "org_zenith",
    name: "Zenith Data",
    plan: "free",
    status: "active",
    revenue: 0,
    provider_cost: 372.4,
    written_off: 71.2,
    uncollected: 301.2,
    bleed: 372.4,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 39_100,
    tokens: 9_600_000,
    last_activity: "2026-07-16T22:31:00Z",
  },
  {
    id: "org_acme",
    name: "Acme Robotics",
    plan: "pro",
    status: "active",
    revenue: 962.8,
    provider_cost: 1044.15,
    written_off: 88.7,
    uncollected: 28.4,
    bleed: 117.1,
    write_off_rate: 0.085,
    recovery_rate: 0.89,
    requests: 79_600,
    tokens: 20_100_000,
    last_activity: "2026-07-15T18:20:00Z",
  },
  {
    id: "org_lockbox",
    name: "Lockbox Systems",
    plan: "scale",
    status: "locked",
    revenue: 611.4,
    provider_cost: 812.6,
    written_off: 168.9,
    uncollected: 61.3,
    bleed: 230.2,
    write_off_rate: 0.21,
    recovery_rate: 0.72,
    requests: 62_800,
    tokens: 15_700_000,
    last_activity: "2026-07-14T09:44:00Z",
  },
  {
    id: "org_born_empty",
    name: "Fresh Start Inc",
    plan: "free",
    status: "active",
    revenue: 0,
    provider_cost: 4.2,
    written_off: 4.2,
    uncollected: 0,
    bleed: 4.2,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 320,
    tokens: 74_000,
    last_activity: "2026-06-28T13:10:00Z",
  },
  {
    id: "org_ghost",
    name: "Ghost Ventures",
    plan: "free",
    status: "suspended",
    revenue: 0,
    provider_cost: 141.7,
    written_off: 33.5,
    uncollected: 108.2,
    bleed: 141.7,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 12_400,
    tokens: 3_100_000,
    last_activity: null,
  },
];

/** Project directory rows across the orgs above. `key_count` is the live API-key count per
 *  project. Fresh Start Inc's project is the idle one at the project grain. */
export const projectRows: ProjectRowFixture[] = [
  {
    id: "proj_northwind_prod",
    name: "Northwind Production",
    org_id: "org_northwind",
    org_name: "Northwind AI",
    plan: "scale",
    status: "active",
    key_count: 6,
    revenue: 1620.4,
    provider_cost: 1782.9,
    written_off: 151.2,
    uncollected: 62.1,
    bleed: 213.3,
    write_off_rate: 0.085,
    recovery_rate: 0.88,
    requests: 138_700,
    tokens: 35_400_000,
    last_activity: "2026-07-17T15:12:00Z",
  },
  {
    id: "proj_northwind_staging",
    name: "Northwind Staging",
    org_id: "org_northwind",
    org_name: "Northwind AI",
    plan: "scale",
    status: "active",
    key_count: 3,
    revenue: 520.2,
    provider_cost: 601.3,
    written_off: 63.6,
    uncollected: 34.3,
    bleed: 97.9,
    write_off_rate: 0.106,
    recovery_rate: 0.84,
    requests: 43_700,
    tokens: 11_500_000,
    last_activity: "2026-07-16T20:02:00Z",
  },
  {
    id: "proj_freebird_main",
    name: "Freebird Main",
    org_id: "org_freebird",
    org_name: "Freebird Collective",
    plan: "free",
    status: "active",
    key_count: 2,
    revenue: 0,
    provider_cost: 604.9,
    written_off: 118.3,
    uncollected: 486.6,
    bleed: 604.9,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 58_700,
    tokens: 14_200_000,
    last_activity: "2026-07-17T11:48:00Z",
  },
  {
    id: "proj_meridian_router",
    name: "Meridian Router",
    org_id: "org_meridian",
    org_name: "Meridian Labs",
    plan: "pro",
    status: "active",
    key_count: 5,
    revenue: 1180.9,
    provider_cost: 1266.4,
    written_off: 101.8,
    uncollected: 31.7,
    bleed: 133.5,
    write_off_rate: 0.08,
    recovery_rate: 0.9,
    requests: 96_200,
    tokens: 25_100_000,
    last_activity: "2026-07-17T08:05:00Z",
  },
  {
    id: "proj_meridian_sandbox",
    name: "Meridian Sandbox",
    org_id: "org_meridian",
    org_name: "Meridian Labs",
    plan: "pro",
    status: "active",
    key_count: 2,
    revenue: 305.35,
    provider_cost: 336.4,
    written_off: 29.7,
    uncollected: 11.2,
    bleed: 40.9,
    write_off_rate: 0.088,
    recovery_rate: 0.88,
    requests: 25_100,
    tokens: 6_400_000,
    last_activity: "2026-07-13T14:26:00Z",
  },
  {
    id: "proj_zenith_ingest",
    name: "Zenith Ingest",
    org_id: "org_zenith",
    org_name: "Zenith Data",
    plan: "free",
    status: "active",
    key_count: 1,
    revenue: 0,
    provider_cost: 372.4,
    written_off: 71.2,
    uncollected: 301.2,
    bleed: 372.4,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 39_100,
    tokens: 9_600_000,
    last_activity: "2026-07-16T22:31:00Z",
  },
  {
    id: "proj_acme_control",
    name: "Acme Control Plane",
    org_id: "org_acme",
    org_name: "Acme Robotics",
    plan: "pro",
    status: "active",
    key_count: 4,
    revenue: 962.8,
    provider_cost: 1044.15,
    written_off: 88.7,
    uncollected: 28.4,
    bleed: 117.1,
    write_off_rate: 0.085,
    recovery_rate: 0.89,
    requests: 79_600,
    tokens: 20_100_000,
    last_activity: "2026-07-15T18:20:00Z",
  },
  {
    id: "proj_lockbox_api",
    name: "Lockbox API",
    org_id: "org_lockbox",
    org_name: "Lockbox Systems",
    plan: "scale",
    status: "locked",
    key_count: 3,
    revenue: 611.4,
    provider_cost: 812.6,
    written_off: 168.9,
    uncollected: 61.3,
    bleed: 230.2,
    write_off_rate: 0.21,
    recovery_rate: 0.72,
    requests: 62_800,
    tokens: 15_700_000,
    last_activity: "2026-07-14T09:44:00Z",
  },
  {
    id: "proj_fresh_start",
    name: "Fresh Start Trial",
    org_id: "org_born_empty",
    org_name: "Fresh Start Inc",
    plan: "free",
    status: "active",
    key_count: 1,
    revenue: 0,
    provider_cost: 4.2,
    written_off: 4.2,
    uncollected: 0,
    bleed: 4.2,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 320,
    tokens: 74_000,
    last_activity: "2026-06-28T13:10:00Z",
  },
  {
    id: "proj_ghost_legacy",
    name: "Ghost Legacy",
    org_id: "org_ghost",
    org_name: "Ghost Ventures",
    plan: "free",
    status: "suspended",
    key_count: 0,
    revenue: 0,
    provider_cost: 141.7,
    written_off: 33.5,
    uncollected: 108.2,
    bleed: 141.7,
    write_off_rate: 1.0,
    recovery_rate: 0.0,
    requests: 12_400,
    tokens: 3_100_000,
    last_activity: null,
  },
];

/** Cost pivots keyed by `group_by`. Each row: `{ key, label, cost, requests, recovery_rate,
 *  last_seen }`. Model/provider power both `/finance` (`by_model`/`by_provider`) and `/cost`. */
export const costPivots: Record<string, CostPivotRowFixture[]> = {
  org: orgRows.map((o) => ({
    key: o.id,
    label: o.name,
    cost: o.provider_cost,
    requests: o.requests,
    recovery_rate: o.recovery_rate,
    last_seen: o.last_activity,
  })),
  project: projectRows.map((p) => ({
    key: p.id,
    label: `${p.org_name} · ${p.name}`,
    cost: p.provider_cost,
    requests: p.requests,
    recovery_rate: p.recovery_rate,
    last_seen: p.last_activity,
  })),
  user: [
    {
      key: "user_owner",
      label: "ada.meridian",
      cost: 1266.4,
      requests: 96_200,
      recovery_rate: 0.9,
      last_seen: "2026-07-17T08:05:00Z",
    },
    {
      key: "user_north",
      label: "reed.northwind",
      cost: 1782.9,
      requests: 138_700,
      recovery_rate: 0.88,
      last_seen: "2026-07-17T15:12:00Z",
    },
    {
      key: "user_free",
      label: "lena.freebird",
      cost: 604.9,
      requests: 58_700,
      recovery_rate: 0.0,
      last_seen: "2026-07-17T11:48:00Z",
    },
    {
      key: "user_acme",
      label: "kit.acme",
      cost: 1044.15,
      requests: 79_600,
      recovery_rate: 0.89,
      last_seen: "2026-07-15T18:20:00Z",
    },
    {
      key: "user_zen",
      label: "moss.zenith",
      cost: 372.4,
      requests: 39_100,
      recovery_rate: 0.0,
      last_seen: "2026-07-16T22:31:00Z",
    },
    {
      key: "user_lock",
      label: "vale.lockbox",
      cost: 812.6,
      requests: 62_800,
      recovery_rate: 0.72,
      last_seen: "2026-07-14T09:44:00Z",
    },
  ],
  model: [
    {
      key: "gpt-4o",
      label: "gpt-4o",
      cost: 2140.6,
      requests: 168_400,
      recovery_rate: 0.84,
      last_seen: "2026-07-17T15:12:00Z",
    },
    {
      key: "claude-3-7-sonnet",
      label: "claude-3-7-sonnet",
      cost: 1786.3,
      requests: 121_900,
      recovery_rate: 0.86,
      last_seen: "2026-07-17T14:02:00Z",
    },
    {
      key: "gpt-4o-mini",
      label: "gpt-4o-mini",
      cost: 942.1,
      requests: 142_600,
      recovery_rate: 0.79,
      last_seen: "2026-07-17T15:40:00Z",
    },
    {
      key: "gemini-2-5-flash",
      label: "gemini-2-5-flash",
      cost: 711.5,
      requests: 96_200,
      recovery_rate: 0.71,
      last_seen: "2026-07-16T20:11:00Z",
    },
    {
      key: "claude-3-5-haiku",
      label: "claude-3-5-haiku",
      cost: 488.9,
      requests: 78_300,
      recovery_rate: 0.77,
      last_seen: "2026-07-15T18:20:00Z",
    },
    {
      key: "llama-3-3-70b",
      label: "llama-3-3-70b",
      cost: 249.2,
      requests: 41_500,
      recovery_rate: 0.68,
      last_seen: "2026-07-13T09:30:00Z",
    },
  ],
  provider: [
    {
      key: "openai",
      label: "OpenAI",
      cost: 3082.7,
      requests: 311_000,
      recovery_rate: 0.82,
      last_seen: "2026-07-17T15:40:00Z",
    },
    {
      key: "anthropic",
      label: "Anthropic",
      cost: 2275.2,
      requests: 200_200,
      recovery_rate: 0.85,
      last_seen: "2026-07-17T14:02:00Z",
    },
    {
      key: "google",
      label: "Google",
      cost: 711.5,
      requests: 96_200,
      recovery_rate: 0.71,
      last_seen: "2026-07-16T20:11:00Z",
    },
    {
      key: "meta",
      label: "Meta",
      cost: 249.2,
      requests: 41_500,
      recovery_rate: 0.68,
      last_seen: "2026-07-13T09:30:00Z",
    },
  ],
  key: [
    {
      key: "key_north_prod",
      label: "grk_live_north…prod",
      cost: 1782.9,
      requests: 138_700,
      recovery_rate: 0.88,
      last_seen: "2026-07-17T15:12:00Z",
    },
    {
      key: "key_meridian_router",
      label: "grk_live_merid…rtr",
      cost: 1266.4,
      requests: 96_200,
      recovery_rate: 0.9,
      last_seen: "2026-07-17T08:05:00Z",
    },
    {
      key: "key_acme_control",
      label: "grk_live_acme…ctl",
      cost: 1044.15,
      requests: 79_600,
      recovery_rate: 0.89,
      last_seen: "2026-07-15T18:20:00Z",
    },
    {
      key: "key_lockbox_api",
      label: "grk_live_lock…api",
      cost: 812.6,
      requests: 62_800,
      recovery_rate: 0.72,
      last_seen: "2026-07-14T09:44:00Z",
    },
    {
      key: "key_freebird_main",
      label: "grk_live_free…main",
      cost: 604.9,
      requests: 58_700,
      recovery_rate: 0.0,
      last_seen: "2026-07-17T11:48:00Z",
    },
    {
      key: "key_zenith_ingest",
      label: "grk_live_zen…ing",
      cost: 372.4,
      requests: 39_100,
      recovery_rate: 0.0,
      last_seen: "2026-07-16T22:31:00Z",
    },
  ],
};

/** Seed rows for the audit trail (`GET /v1/admin/audit`) — a handful of prior control actions so
 *  the Audit surface is never empty on first load. Each control action taken in the running app
 *  prepends a fresh row (`create-mock-registry.ts`). Target ids/statuses agree with `orgRows`/
 *  `projectRows` above (Ghost Ventures is suspended, Lockbox is locked). Newest last here — the
 *  read route sorts newest-first. */
export const adminAuditSeed: AdminAuditEntryFixture[] = [
  {
    id: "aud_seed_reactivate_meridian",
    actor_email: "ceo@gaussmeridian.dev",
    action: "reactivate",
    target_type: "project",
    target_id: "proj_meridian_router",
    reason: null,
    created_at: "2026-07-12T15:03:00Z",
  },
  {
    id: "aud_seed_lock_lockbox",
    actor_email: "ops@gaussmeridian.dev",
    action: "lock",
    target_type: "org",
    target_id: "org_lockbox",
    reason: "Payment dispute — temporary hold pending resolution",
    created_at: "2026-07-14T09:44:00Z",
  },
  {
    id: "aud_seed_suspend_ghost",
    actor_email: "ops@gaussmeridian.dev",
    action: "suspend",
    target_type: "org",
    target_id: "org_ghost",
    reason: "Abuse: scripted signup ring, zero recovery",
    created_at: "2026-07-16T09:12:00Z",
  },
];

/** Idle set for the Watchlist — orgs quiet for longer than the v1 7-day threshold (or never
 *  active). Fresh Start Inc trailed off; Ghost Ventures never used the platform. */
export const watchlistIdle: IdleRowFixture[] = [
  {
    id: "org_born_empty",
    name: "Fresh Start Inc",
    plan: "free",
    status: "active",
    last_activity: "2026-06-28T13:10:00Z",
  },
  {
    id: "org_ghost",
    name: "Ghost Ventures",
    plan: "free",
    status: "suspended",
    last_activity: null,
  },
];
