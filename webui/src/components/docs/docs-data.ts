/**
 * Reference tables for the documentation page.
 *
 * Everything here is transcribed from the running system rather than from intent: the endpoint
 * table is generated from `services/server/src/routes.rs`, the header families are the headers a
 * real completion actually returned, and the environment table is what `docker-compose.yml`
 * actually interpolates. When the server changes, these tables are what needs re-checking.
 */

export interface Endpoint {
  method: string;
  path: string;
  auth: "key" | "session" | "admin" | "none";
  summary: string;
}

export interface EndpointGroup {
  id: string;
  title: string;
  blurb: string;
  endpoints: Endpoint[];
}

export const ENDPOINT_GROUPS: EndpointGroup[] = [
  {
    id: "ep-inference",
    title: "Inference",
    blurb:
      "The OpenAI-compatible surface. These are the only endpoints that consume budget, and the only ones that require a project-scoped API key.",
    endpoints: [
      { method: "POST", path: "/v1/chat/completions", auth: "key", summary: "Chat completion." },
      {
        method: "POST",
        path: "/v1/chat/completions/stream",
        auth: "key",
        summary: "Server-sent events variant.",
      },
      {
        method: "POST",
        path: "/v1/chat/completions/batch",
        auth: "key",
        summary: "Array of requests in one call.",
      },
      { method: "POST", path: "/v1/completions", auth: "key", summary: "Legacy text completion." },
      { method: "POST", path: "/v1/embeddings", auth: "key", summary: "Embedding vectors." },
      { method: "GET", path: "/v1/models", auth: "key", summary: "Models available to route to." },
      {
        method: "GET",
        path: "/v1/models/:model",
        auth: "key",
        summary: "One model's capabilities.",
      },
      {
        method: "GET",
        path: "/v1/usage/:request_id",
        auth: "key",
        summary: "Token and cost accounting for one request.",
      },
      { method: "GET", path: "/v1/balance", auth: "key", summary: "Remaining balance." },
    ],
  },
  {
    id: "ep-auth",
    title: "Accounts and sessions",
    blurb: "Console identity. These issue and manage the session token, not API keys.",
    endpoints: [
      { method: "POST", path: "/v1/auth/register", auth: "none", summary: "Create an account." },
      { method: "POST", path: "/v1/auth/login", auth: "none", summary: "Exchange credentials for a token." },
      { method: "POST", path: "/v1/auth/logout", auth: "session", summary: "Revoke the session." },
      { method: "POST", path: "/v1/auth/refresh", auth: "none", summary: "Rotate an expiring session." },
      { method: "GET", path: "/v1/auth/me", auth: "session", summary: "The signed-in user." },
      { method: "POST", path: "/v1/auth/forgot-password", auth: "none", summary: "Begin a reset." },
      { method: "POST", path: "/v1/auth/reset-password", auth: "none", summary: "Complete a reset." },
      {
        method: "POST",
        path: "/v1/auth/me/deletion-request",
        auth: "session",
        summary: "Request account deletion.",
      },
    ],
  },
  {
    id: "ep-keys",
    title: "API keys",
    blurb:
      "Keys are created against a project. The secret is returned once at creation and is never retrievable again — only its prefix is stored in a readable form.",
    endpoints: [
      { method: "GET", path: "/v1/api/keys", auth: "session", summary: "Every key the caller owns." },
      {
        method: "POST",
        path: "/v1/api/keys",
        auth: "session",
        summary: "Issue a key, optionally scoped to a project.",
      },
      { method: "POST", path: "/v1/api/keys/revoke", auth: "session", summary: "Revoke by key id." },
    ],
  },
  {
    id: "ep-byok",
    title: "Bring your own key",
    blurb:
      "Provider credentials supplied by the customer, encrypted at rest. Writes are gated on BYOK_ADMIN_EMAILS.",
    endpoints: [
      { method: "GET", path: "/v1/byok/keys", auth: "session", summary: "Registered providers." },
      { method: "POST", path: "/v1/byok/keys", auth: "admin", summary: "Register a provider key." },
      {
        method: "DELETE",
        path: "/v1/byok/keys/:provider",
        auth: "admin",
        summary: "Remove a provider key.",
      },
    ],
  },
  {
    id: "ep-tenancy",
    title: "Organisations and projects",
    blurb:
      "The tenancy tree. Budgets, guardrail thresholds, and routing parameters live on the project.",
    endpoints: [
      { method: "GET", path: "/v1/orgs", auth: "session", summary: "Orgs you belong to." },
      { method: "POST", path: "/v1/orgs", auth: "session", summary: "Create an org." },
      { method: "GET", path: "/v1/orgs/:id", auth: "session", summary: "One org." },
      { method: "PATCH", path: "/v1/orgs/:id", auth: "session", summary: "Rename or change plan." },
      { method: "DELETE", path: "/v1/orgs/:id", auth: "session", summary: "Delete an org." },
      { method: "GET", path: "/v1/orgs/:id/members", auth: "session", summary: "Members and roles." },
      { method: "POST", path: "/v1/orgs/:id/members", auth: "session", summary: "Invite a member." },
      {
        method: "PATCH",
        path: "/v1/orgs/:id/members/:uid",
        auth: "session",
        summary: "Change a member's role.",
      },
      {
        method: "DELETE",
        path: "/v1/orgs/:id/members/:uid",
        auth: "session",
        summary: "Remove a member.",
      },
      { method: "GET", path: "/v1/orgs/:id/projects", auth: "session", summary: "Projects in an org." },
      { method: "POST", path: "/v1/orgs/:id/projects", auth: "session", summary: "Create a project." },
      {
        method: "PATCH",
        path: "/v1/orgs/:id/projects/:pid",
        auth: "session",
        summary: "Set budget, quality floor, routing parameters.",
      },
      {
        method: "DELETE",
        path: "/v1/orgs/:id/projects/:pid",
        auth: "session",
        summary: "Delete a project.",
      },
      { method: "GET", path: "/v1/project/settings", auth: "session", summary: "Effective settings." },
      { method: "GET", path: "/v1/roles", auth: "session", summary: "Assignable roles." },
    ],
  },
  {
    id: "ep-observability",
    title: "Observability",
    blurb:
      "What routed, what it cost, and why. Route decisions carry the full candidate set and the policy version that produced it.",
    endpoints: [
      { method: "GET", path: "/v1/route-decisions", auth: "session", summary: "Recent routing decisions." },
      {
        method: "GET",
        path: "/v1/route-decisions/:request_id",
        auth: "session",
        summary: "One decision in full.",
      },
      {
        method: "GET",
        path: "/v1/route-decisions/stream",
        auth: "session",
        summary: "Live decision stream.",
      },
      { method: "GET", path: "/v1/routing/config", auth: "session", summary: "Active routing policy." },
      { method: "GET", path: "/v1/routing/stats", auth: "session", summary: "Aggregate routing stats." },
      { method: "GET", path: "/v1/routing/trace", auth: "session", summary: "Per-request trace." },
      { method: "GET", path: "/v1/logs", auth: "session", summary: "Request log." },
      { method: "GET", path: "/v1/analytics/usage", auth: "session", summary: "Usage over a window." },
      { method: "GET", path: "/v1/analytics/cost", auth: "admin", summary: "Cost analytics." },
      { method: "GET", path: "/v1/cache/stats", auth: "session", summary: "Cache hit rates." },
      { method: "POST", path: "/v1/cache/clear", auth: "admin", summary: "Flush the cache." },
    ],
  },
  {
    id: "ep-ops",
    title: "Health and metrics",
    blurb: "Unauthenticated. Point your orchestrator at these.",
    endpoints: [
      { method: "GET", path: "/health", auth: "none", summary: "Liveness." },
      {
        method: "GET",
        path: "/ready",
        auth: "none",
        summary: "Readiness — true when at least one provider is callable.",
      },
      {
        method: "GET",
        path: "/health/providers",
        auth: "session",
        summary: "Per-provider health. Authenticated: it makes live provider calls.",
      },
      { method: "GET", path: "/metrics", auth: "none", summary: "Prometheus exposition." },
    ],
  },
  {
    id: "ep-admin",
    title: "Administration",
    blurb:
      "Superadmin only. Callers outside SUPERADMIN_EMAILS receive 404 rather than 403, so the surface is indistinguishable from routes that do not exist.",
    endpoints: [
      { method: "GET", path: "/v1/admin/me", auth: "admin", summary: "Confirm superadmin status." },
      { method: "GET", path: "/v1/admin/orgs", auth: "admin", summary: "All orgs." },
      { method: "POST", path: "/v1/admin/orgs/:id/suspend", auth: "admin", summary: "Suspend an org." },
      {
        method: "POST",
        path: "/v1/admin/orgs/:id/reactivate",
        auth: "admin",
        summary: "Reinstate an org.",
      },
      { method: "GET", path: "/v1/admin/projects", auth: "admin", summary: "All projects." },
      {
        method: "POST",
        path: "/v1/admin/projects/:id/lock",
        auth: "admin",
        summary: "Freeze a project.",
      },
      { method: "GET", path: "/v1/admin/users", auth: "admin", summary: "All users." },
      { method: "POST", path: "/v1/admin/users/:id/suspend", auth: "admin", summary: "Suspend a user." },
      { method: "GET", path: "/v1/admin/db/api-keys", auth: "admin", summary: "Every issued key." },
      {
        method: "POST",
        path: "/v1/admin/keys/:id/suspend",
        auth: "admin",
        summary: "Suspend one key.",
      },
      { method: "GET", path: "/v1/admin/audit", auth: "admin", summary: "Audit log." },
      { method: "GET", path: "/v1/admin/metrics", auth: "admin", summary: "Platform metrics." },
      { method: "GET", path: "/v1/admin/watchlist", auth: "admin", summary: "Flagged tenants." },
      {
        method: "GET",
        path: "/v1/admin/deletion-requests",
        auth: "admin",
        summary: "Pending deletion requests.",
      },
    ],
  },
];

export interface HeaderFamily {
  id: string;
  title: string;
  blurb: string;
  rows: [string, string][];
}

export const HEADER_FAMILIES: HeaderFamily[] = [
  {
    id: "hdr-selection",
    title: "What served the request",
    blurb: "Start here when you want to know which model answered and how confident the router was.",
    rows: [
      ["x-gaussmeridian-model-selected", "The model that served. `-model` is the alias you asked for."],
      ["x-gaussmeridian-provider-selected", "The provider it came from, e.g. google."],
      ["x-gaussmeridian-tier", "Capability tier the request landed in, e.g. advanced."],
      ["x-gaussmeridian-score", "Score of the winning candidate."],
      [
        "x-gaussmeridian-candidates",
        'Every candidate considered, as JSON: [{"m":model,"p":provider,"t":tier,"s":score,"c":cost}].',
      ],
      ["x-gaussmeridian-complexity", "Estimated prompt complexity, 0–1. Drives cascade and MoA."],
      ["x-gaussmeridian-skills", "Skill vector matched against the prompt."],
      ["x-gaussmeridian-retry-count", "Provider retries before a response came back."],
    ],
  },
  {
    id: "hdr-band",
    title: "Why that model and not another",
    blurb:
      "When the model you asked for is not the one you got, these say what happened. A request can be moved to a nearby capability band when the desired one has nothing available.",
    rows: [
      ["x-gaussmeridian-desired-band", "The band implied by your request."],
      ["x-gaussmeridian-selected-band", "The band actually used."],
      ["x-gaussmeridian-band-reason", "Why they differ, e.g. nearest_available_band."],
      ["x-gaussmeridian-quality-relaxation", "Whether the quality floor was relaxed to find a route."],
      ["x-gaussmeridian-output-budget", "Output token ceiling applied to this request."],
    ],
  },
  {
    id: "hdr-cost",
    title: "Cost and cache",
    blurb: "What the request cost and whether it was served from cache.",
    rows: [
      ["x-gaussmeridian-cost", "Cost attributed to this request, in USD."],
      ["x-gaussmeridian-r-binary", "Billing outcome flag: 1 charged, 0 not charged."],
      ["x-gaussmeridian-cache-hit", "true when the response came from cache."],
      ["x-gaussmeridian-cache-tier", "Which cache tier answered."],
      ["x-gaussmeridian-budget-used", "Project spend so far."],
      ["x-gaussmeridian-budget-limit", "The project's configured monthly budget."],
      ["x-gaussmeridian-guardrail", "Set when a guardrail acted on the response."],
    ],
  },
  {
    id: "hdr-provenance",
    title: "Provenance",
    blurb:
      "Identifiers that let you reconstruct a decision later. Pair these with GET /v1/route-decisions/:request_id.",
    rows: [
      ["x-gaussmeridian-ballot-id", "Identity of the selection ballot."],
      ["x-gaussmeridian-snapshot-fingerprint", "Fingerprint of the routing inputs at decision time."],
      ["x-gaussmeridian-policy-version", "Hash of the routing policy that produced the decision."],
      [
        "x-gaussmeridian-catalog-version",
        "Catalog version per model. Large — see the note below on header size.",
      ],
      ["x-gaussmeridian-price-version", "Price version per model. Also large."],
    ],
  },
  {
    id: "hdr-experimental",
    title: "Experimental subsystems",
    blurb:
      "Predictive routing components. On a default install these report inactive with a fallback reason, and can be ignored — they are surfaced so an operator running them can see their state per request.",
    rows: [
      ["x-gaussmeridian-predictor-status", "unavailable when no trained state is loaded."],
      ["x-gaussmeridian-predictor-fallback-reason", "Why it did not participate, e.g. no_active_state."],
      ["x-gaussmeridian-predictor-version", "Predictor implementation version."],
      ["x-gaussmeridian-bella-status", "Skill-profiling subsystem state."],
      ["x-gaussmeridian-xrouter-status", "Trajectory router state."],
      ["x-gaussmeridian-r2-status", "Qualification lane state."],
    ],
  },
];

export interface EnvVar {
  name: string;
  required: boolean;
  def?: string;
  purpose: string;
}

export const ENV_CORE: EnvVar[] = [
  {
    name: "JWT_SECRET",
    required: true,
    purpose:
      "Signs console session tokens. Use a long random string. Startup refuses a known placeholder value in production.",
  },
  {
    name: "GAUSSMERIDIAN_API_KEY",
    required: true,
    purpose: "Bootstrap key, so the gateway is callable before you have created one in the console.",
  },
  { name: "SURREALDB_PASSWORD", required: true, purpose: "Root password for the bundled database." },
  {
    name: "REDIS_PASSWORD",
    required: true,
    purpose:
      "Redis auth. Compose builds REDIS_URL from it. Redis rejects unauthenticated clients, so this is not optional.",
  },
  { name: "GRAFANA_PASSWORD", required: true, purpose: "Grafana admin password, observability profile." },
];

export const ENV_PROVIDERS: EnvVar[] = [
  { name: "OPENAI_API_KEY", required: false, purpose: "OpenAI credential." },
  { name: "ANTHROPIC_API_KEY", required: false, purpose: "Anthropic credential." },
  { name: "GEMINI_API_KEY", required: false, purpose: "Google credential." },
  {
    name: "GAUSSMERIDIAN__PROVIDERS__OPENAI__BASE_URL",
    required: false,
    def: "mock provider",
    purpose:
      "Redirects the OpenAI provider. Defaults to the bundled mock so a fresh clone runs with no credentials. Double underscores are the config-path separator.",
  },
  {
    name: "ANTHROPIC_API_BASE",
    required: false,
    purpose: "Point Anthropic at a mock or proxy instead of the real API.",
  },
];

export const ENV_ROUTING: EnvVar[] = [
  {
    name: "GAUSSMERIDIAN_GUARDRAIL_PII",
    required: false,
    def: "1",
    purpose: "Scan responses for personal data and block on a hit.",
  },
  {
    name: "GAUSSMERIDIAN_GUARDRAIL_INJECTION",
    required: false,
    def: "1",
    purpose: "Scan for prompt-injection patterns.",
  },
  {
    name: "GAUSSMERIDIAN_CASCADE",
    required: false,
    def: "1",
    purpose: "Try a cheaper model first, escalate when confidence is low.",
  },
  {
    name: "GAUSSMERIDIAN_CASCADE_THRESHOLD",
    required: false,
    def: "0.7",
    purpose: "Confidence below which the cascade escalates.",
  },
  {
    name: "GAUSSMERIDIAN_MOA",
    required: false,
    def: "1",
    purpose: "Run several models on complex prompts and reconcile the answers.",
  },
  {
    name: "GAUSSMERIDIAN_MOA_AGENTS",
    required: false,
    def: "gpt-4o-mini,gpt-4o",
    purpose: "Comma-separated roster used by mixture-of-agents.",
  },
  {
    name: "GAUSSMERIDIAN_TAU_MOA",
    required: false,
    def: "0.7",
    purpose: "Complexity threshold. 0.7 = complex prompts only, 0.0 = every request, 1.0 = never.",
  },
  {
    name: "GAUSSMERIDIAN_MOA_TIMEOUT_SECS",
    required: false,
    def: "30",
    purpose: "Ceiling on a mixture-of-agents round.",
  },
];

export const ENV_ACCESS: EnvVar[] = [
  {
    name: "BYOK_MASTER_KEY",
    required: false,
    purpose:
      "Base64 of 32 random bytes; encrypts stored provider keys. Compose injects a development-only default — generate your own with `openssl rand -base64 32` before storing anything real.",
  },
  {
    name: "BYOK_ADMIN_EMAILS",
    required: false,
    purpose: "Emails allowed to register or delete BYOK keys. Unset means every BYOK write returns 403.",
  },
  {
    name: "SUPERADMIN_EMAILS",
    required: false,
    purpose:
      "Emails with access to /v1/admin/*. Unset means the entire admin surface answers 404 for everyone.",
  },
];

export const ENV_INFRA: EnvVar[] = [
  { name: "GAUSSMERIDIAN_DB_URL", required: false, def: "ws://surrealdb:8000", purpose: "Database endpoint." },
  { name: "GAUSSMERIDIAN_DB_NAMESPACE", required: false, def: "gaussmeridian", purpose: "SurrealDB namespace." },
  { name: "GAUSSMERIDIAN_DB_DATABASE", required: false, def: "main", purpose: "SurrealDB database." },
  {
    name: "REDIS_URL",
    required: false,
    purpose:
      "Cache and rate-limit backend. Deliberately unprefixed — the server reads this name and nothing else. GAUSSMERIDIAN_REDIS_URL is read by no code.",
  },
  { name: "RUST_LOG", required: false, def: "info", purpose: "Log filter, e.g. info,gaussmeridian=debug." },
];

export const ERROR_ROWS: [string, string, string][] = [
  ["400", "empty_messages", "The messages array was empty."],
  [
    "400",
    "project_scope_required",
    "The key is not attached to a project, so there is nothing to bill. Recreate it from a project's Keys page.",
  ],
  [
    "401",
    "unauthorized",
    "No credential, or one the gateway does not recognise. Check you used x-api-key rather than Authorization.",
  ],
  ["402", "payment_required", "The project has no budget or has spent it. Set budget_monthly above zero."],
  ["403", "project_access_denied", "The caller is not a member of that project's organisation."],
  ["403", "—", "On BYOK writes: the caller's email is not in BYOK_ADMIN_EMAILS."],
  ["404", "—", "On /v1/admin/*: the caller is not a superadmin. Deliberately not 403."],
  ["409", "—", "Registration conflict: that email or username already exists."],
  ["429", "rate_limited", "The key's per-minute or per-day limit was reached."],
  ["502", "—", "The upstream provider failed after retries. See x-gaussmeridian-retry-count."],
];
