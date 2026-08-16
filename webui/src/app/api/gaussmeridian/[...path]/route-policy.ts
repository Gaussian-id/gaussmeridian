type BffMethod = "GET" | "POST" | "PATCH" | "DELETE";

interface BffRouteRule {
  readonly methods: readonly BffMethod[];
  readonly path: RegExp;
}

const SEGMENT = String.raw`[A-Za-z0-9][A-Za-z0-9._~:@+-]{0,255}`;

function exact(pattern: string): RegExp {
  return new RegExp(`^${pattern}$`);
}

/**
 * Browser-session routes owned by the authenticated Meridian WebUI.
 *
 * This is intentionally narrower than the Rust service's route table. In particular, provider
 * webhooks, operational endpoints, admin database helpers, and API-key-authenticated inference
 * are never reachable through the cookie-to-bearer bridge. New WebUI capabilities must make an
 * explicit method/path addition here and add a policy test before they can cross the BFF.
 */
const BFF_ROUTE_POLICY: readonly BffRouteRule[] = [
  { methods: ["GET"], path: exact("health") },

  { methods: ["GET"], path: exact("v1/auth/me") },
  { methods: ["POST"], path: exact("v1/auth/forgot-password") },
  { methods: ["POST"], path: exact("v1/auth/reset-password") },
  { methods: ["POST", "DELETE"], path: exact("v1/auth/me/deletion-request") },

  { methods: ["GET", "POST"], path: exact("v1/api/keys") },
  { methods: ["POST"], path: exact("v1/api/keys/revoke") },
  { methods: ["GET", "PATCH"], path: exact("v1/project/settings") },

  // The browser playground has its own session-authenticated streaming endpoint. Customer API
  // traffic uses POST /v1/chat/completions directly with a Meridian project key and bypasses BFF.
  { methods: ["POST"], path: exact("v1/chat/completions/stream") },
  { methods: ["GET"], path: exact("v1/models") },
  { methods: ["GET"], path: exact(`v1/usage/${SEGMENT}`) },
  { methods: ["GET"], path: exact("v1/balance") },
  { methods: ["GET"], path: exact("v1/analytics/cost") },
  { methods: ["GET"], path: exact("v1/analytics/usage") },
  { methods: ["GET"], path: exact("v1/analytics/savings") },
  { methods: ["GET"], path: exact("v1/logs") },
  { methods: ["GET"], path: exact("v1/routing/trace") },
  { methods: ["GET"], path: exact("v1/route-decisions") },
  { methods: ["GET"], path: exact("v1/route-decisions/stream") },
  { methods: ["GET"], path: exact(`v1/route-decisions/${SEGMENT}`) },
  { methods: ["GET"], path: exact("v1/billing/summary") },
  { methods: ["GET"], path: exact("v1/billing/budget") },
  { methods: ["GET"], path: exact("v1/moa-candidates") },

  { methods: ["GET", "POST"], path: exact("v1/orgs") },
  { methods: ["GET", "PATCH", "DELETE"], path: exact(`v1/orgs/${SEGMENT}`) },
  { methods: ["GET", "POST"], path: exact(`v1/orgs/${SEGMENT}/projects`) },
  {
    methods: ["GET", "PATCH", "DELETE"],
    path: exact(`v1/orgs/${SEGMENT}/projects/${SEGMENT}`),
  },
  { methods: ["GET", "POST"], path: exact(`v1/orgs/${SEGMENT}/projects/${SEGMENT}/keys`) },
  {
    methods: ["DELETE"],
    path: exact(`v1/orgs/${SEGMENT}/projects/${SEGMENT}/keys/${SEGMENT}`),
  },
  { methods: ["GET", "POST"], path: exact(`v1/orgs/${SEGMENT}/members`) },
  { methods: ["PATCH", "DELETE"], path: exact(`v1/orgs/${SEGMENT}/members/${SEGMENT}`) },

  { methods: ["GET"], path: exact(`v1/orgs/${SEGMENT}/billing/catalog`) },
  { methods: ["GET"], path: exact(`v1/orgs/${SEGMENT}/billing/payment-methods`) },
  { methods: ["GET"], path: exact(`v1/orgs/${SEGMENT}/billing/wallet`) },
  { methods: ["GET"], path: exact(`v1/orgs/${SEGMENT}/billing/invoices`) },
  {
    methods: ["GET"],
    path: exact(`v1/orgs/${SEGMENT}/billing/invoices/${SEGMENT}/document`),
  },
  { methods: ["GET", "POST"], path: exact(`v1/orgs/${SEGMENT}/billing/topups`) },
  { methods: ["GET"], path: exact(`v1/orgs/${SEGMENT}/billing/topups/${SEGMENT}`) },
  {
    methods: ["GET"],
    path: exact(`v1/orgs/${SEGMENT}/billing/topups/${SEGMENT}/payment-action`),
  },
  {
    methods: ["GET"],
    path: exact(`v1/orgs/${SEGMENT}/billing/topups/${SEGMENT}/receipt`),
  },
  {
    methods: ["POST"],
    path: exact(`v1/orgs/${SEGMENT}/billing/topups/${SEGMENT}/reconcile`),
  },
  { methods: ["GET", "POST"], path: exact(`v1/orgs/${SEGMENT}/billing/subscriptions`) },
  {
    methods: ["GET"],
    path: exact(`v1/orgs/${SEGMENT}/billing/subscriptions/${SEGMENT}`),
  },
  {
    methods: ["POST"],
    path: exact(`v1/orgs/${SEGMENT}/billing/subscriptions/${SEGMENT}/cancel`),
  },

  { methods: ["GET"], path: exact("v1/roles") },
  { methods: ["GET"], path: exact("v1/onboarding/state") },
  { methods: ["POST"], path: exact("v1/onboarding/advance") },
  { methods: ["POST"], path: exact("v1/onboarding/survey") },
  { methods: ["PATCH"], path: exact("v1/onboarding/profile") },
  { methods: ["POST"], path: exact("v1/onboarding/complete") },
  { methods: ["POST"], path: exact(`v1/projects/${SEGMENT}/password`) },
  { methods: ["POST"], path: exact(`v1/projects/${SEGMENT}/password/verify`) },

  { methods: ["GET"], path: exact("v1/admin/me") },
  { methods: ["GET"], path: exact("v1/admin/metrics") },
  { methods: ["GET"], path: exact("v1/admin/overview") },
  { methods: ["GET"], path: exact("v1/admin/finance") },
  { methods: ["GET"], path: exact("v1/admin/cost") },
  { methods: ["GET"], path: exact("v1/admin/orgs") },
  { methods: ["GET"], path: exact(`v1/admin/orgs/${SEGMENT}`) },
  { methods: ["GET"], path: exact("v1/admin/projects") },
  { methods: ["GET"], path: exact(`v1/admin/projects/${SEGMENT}`) },
  { methods: ["GET"], path: exact("v1/admin/watchlist") },
  { methods: ["GET"], path: exact("v1/admin/users") },
  { methods: ["GET"], path: exact("v1/admin/audit") },
  { methods: ["GET"], path: exact("v1/admin/deletion-requests") },
  {
    methods: ["POST"],
    path: exact(`v1/admin/deletion-requests/${SEGMENT}/(?:fulfill|reject)`),
  },
  {
    methods: ["GET"],
    path: exact(`v1/admin/finance/topups/${SEGMENT}/${SEGMENT}/timeline`),
  },
  {
    methods: ["POST"],
    path: exact(`v1/admin/finance/topups/${SEGMENT}/${SEGMENT}/repair`),
  },
  { methods: ["GET"], path: exact(`v1/admin/(?:orgs|projects)/${SEGMENT}/impact`) },
  {
    methods: ["POST"],
    path: exact(`v1/admin/(?:orgs|projects)/${SEGMENT}/(?:lock|suspend|reactivate)`),
  },
  {
    methods: ["POST"],
    path: exact(`v1/admin/(?:users|keys)/${SEGMENT}/(?:suspend|reactivate)`),
  },
] as const;

const SAFE_SEGMENT = new RegExp(`^${SEGMENT}$`);

export function isAllowedBffOperation(method: string, path: readonly string[]): boolean {
  if (!(["GET", "POST", "PATCH", "DELETE"] as const).includes(method as BffMethod)) return false;
  if (path.length === 0 || path.some((segment) => !SAFE_SEGMENT.test(segment))) return false;

  const normalizedPath = path.join("/");
  return BFF_ROUTE_POLICY.some(
    (rule) => rule.methods.includes(method as BffMethod) && rule.path.test(normalizedPath),
  );
}
