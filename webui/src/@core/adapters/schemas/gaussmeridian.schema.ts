import { z } from "zod";

export const HealthSchema = z.object({
  status: z.string(),
  timestamp: z.string(),
  version: z.string(),
});

const ModelPermissionSchema = z.object({
  id: z.string(),
  object: z.string(),
  created: z.number(),
  allow_create_engine: z.boolean(),
  allow_sampling: z.boolean(),
  allow_logprobs: z.boolean(),
  allow_search_indices: z.boolean(),
  allow_view: z.boolean(),
  allow_fine_tuning: z.boolean(),
  organization: z.string(),
  group: z.string().nullable(),
  is_blocking: z.boolean(),
});

export const ModelsResponseSchema = z.object({
  data: z.array(
    z.object({
      id: z.string(),
      object: z.string(),
      created: z.number(),
      owned_by: z.string(),
      permission: z.array(ModelPermissionSchema).nullable().optional(),
      root: z.string().nullable().optional(),
      parent: z.string().nullable().optional(),
    }),
  ),
});

const CostInfoSchema = z.object({
  input_cost_per_1k_tokens: z.number(),
  output_cost_per_1k_tokens: z.number(),
  currency: z.string(),
  model: z.string(),
});

const ModelCapabilitiesSchema = z.object({
  supports_streaming: z.boolean(),
  supports_functions: z.boolean(),
  supports_vision: z.boolean(),
  supports_embeddings: z.boolean(),
});

// NOTE: `name` is currently always a duplicate of `id` on the backend (router.rs) —
// not a distinct human-readable label. Real backend behavior, not a frontend bug.
export const ModelInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
  context_length: z.number(),
  pricing: CostInfoSchema,
  capabilities: ModelCapabilitiesSchema,
});

// NOTE: backend handler for GET /v1/billing/models is a hardcoded 8-model stub
// today — ignores the live model registry entirely, no auth required. Do not
// present this data as live/authoritative in the UI.
const ModelPricingSchema = z.object({
  model: z.string(),
  provider: z.string(),
  input_cost_per_1k_tokens: z.number(),
  output_cost_per_1k_tokens: z.number(),
  currency: z.string(),
});

export const ModelPricingResponseSchema = z.object({
  models: z.array(ModelPricingSchema),
});

// NOTE: field is `balance`, not `amount`. Backend silently returns
// { balance: 0.0, currency: "USD", ... } when there's no tenant/DB context —
// a 0 balance does not distinguish "really zero" from "not configured."
export const BalanceInfoSchema = z.object({
  balance: z.number(),
  currency: z.string(),
  last_updated: z.string(),
});

export const BudgetStatusSchema = z.object({
  budget_limit: z.number().nullable(), // always null today — backend stub
  current_usage: z.number(),
  remaining: z.number().nullable(), // always null today
  usage_percentage: z.number().nullable(), // always null today
  alert_threshold: z.number().nullable(), // always null today
  is_over_budget: z.boolean(), // always false today
  period: z.string(),
  currency: z.string(),
});

const CostByModelSchema = z.object({ model: z.string(), cost: z.number(), requests: z.number() });
const CostByProviderSchema = z.object({
  provider: z.string(),
  cost: z.number(),
  requests: z.number(),
});

export const BillingSummarySchema = z.object({
  total_cost: z.number(),
  total_requests: z.number(),
  total_input_tokens: z.number(),
  total_output_tokens: z.number(),
  cost_by_model: z.array(CostByModelSchema),
  cost_by_provider: z.array(CostByProviderSchema),
  period_start: z.string(),
  period_end: z.string(),
  currency: z.string(),
});

// NOTE: gated by RBAC permission `analytics.cost.read` on the backend — can
// 403 for a caller whose role doesn't grant it (unlike /v1/analytics/usage,
// which has no such gate). Callers must handle a 403 as an honest
// "not available for your account" state, not assume this always succeeds.
const CostBreakdownEntrySchema = z.object({
  total_cost: z.number(),
  total_tokens: z.number(),
});

export const CostAnalyticsSchema = z.object({
  start_date: z.string(),
  end_date: z.string(),
  total_cost: z.number(),
  total_tokens: z.number(),
  total_requests: z.number(),
  breakdown_by_model: z.record(z.string(), CostBreakdownEntrySchema),
  breakdown_by_provider: z.record(z.string(), CostBreakdownEntrySchema),
});

export const UsageAnalyticsSchema = z.object({
  summary: z.object({
    total_requests: z.number(),
    successful_requests: z.number(),
    total_tokens: z.number(),
    total_cost: z.number(),
    average_latency_ms: z.number(),
    p95_latency_ms: z.number(),
    p99_latency_ms: z.number(),
    success_rate: z.number(),
    error_rate: z.number(),
  }),
  model_performance: z.array(
    z.object({
      model: z.string(),
      requests: z.number(),
      tokens: z.number(),
      cost: z.number(),
    }),
  ),
  time_range: z.object({ start: z.string(), end: z.string() }),
});

// Traced against the real backend LedgerEntryView (ledger_repository.rs) — /v1/logs is
// sourced from the outcome-billing ledger, scoped to the caller's own project. `r_binary`
// is the outcome-gate result: 1 = response validated, cost charged; 0 = validation failed,
// $0 charged (`cost_charged` is 0). This is the truthful charged-vs-not signal.
export const RequestLogSchema = z.object({
  id: z.string().nullable(),
  model: z.string(),
  provider: z.string(),
  tokens_in: z.number(),
  tokens_out: z.number(),
  cost_charged: z.number(),
  r_binary: z.number(),
  complexity_score: z.number(),
  validator_result: z.string(),
  retry_count: z.number(),
  latency_ms: z.number(),
  created_at: z.string().nullable(),
});

// Traced against the real backend RoutingTraceResponse (handlers.rs, get_routing_trace):
// GET /v1/routing/trace?limit=N wraps the same ledger-sourced rows as /v1/logs (identical
// LedgerEntryView shape — reuse RequestLogSchema) in an { entries, total, limit } envelope.
// `total`/`limit` describe the trace window returned (limit is capped at 100 server-side),
// not a count across all history.
export const RoutingTraceResponseSchema = z.object({
  entries: z.array(RequestLogSchema),
  total: z.number(),
  limit: z.number(),
});
export type RoutingTraceResponse = z.infer<typeof RoutingTraceResponseSchema>;

// Traced against the real backend ApiKey struct for the self-service
// GET /v1/api/keys endpoint (NOT the admin-only /v1/admin/db/api-keys —
// both happen to share this same struct shape, but this schema is used
// against the self-service route). key_hash is a hash, not the raw secret,
// but IS present in the JSON payload.
export const ApiKeySchema = z.object({
  id: z.string().nullable(),
  key_hash: z.string(),
  key_prefix: z.string(),
  user_id: z.string(),
  tenant_id: z.string().nullable(),
  name: z.string().nullable(),
  rate_limit_per_minute: z.number().nullable(),
  rate_limit_per_day: z.number().nullable(),
  created_at: z.string(),
  expires_at: z.string().nullable(),
  last_used_at: z.string().nullable(),
  active: z.boolean(),
});

export const CommerceMoneySchema = z.object({
  minor_units: z.number().int().nonnegative().safe(),
  currency: z.enum(["idr", "usd_micros"]),
});

const TopUpPackageSchema = z.object({
  package_id: z.string(),
  customer_price: CommerceMoneySchema,
  granted_credit: CommerceMoneySchema,
});

const SubscriptionPlanSchema = z.object({
  plan_id: z.string(),
  display_name: z.string().trim().min(1),
  summary: z.string().trim().min(1),
  cycle_price: CommerceMoneySchema,
  included_credit: CommerceMoneySchema,
  credit_policy: z.enum(["reset", "rollover"]),
});

const ModelRateSchema = z.object({
  model_id: z.string(),
  input_per_million_tokens: CommerceMoneySchema,
  output_per_million_tokens: CommerceMoneySchema,
});

export const CommerceCatalogSchema = z.object({
  version_id: z.string(),
  effective_at: z.string(),
  top_up_packages: z.array(TopUpPackageSchema),
  subscription_plans: z.array(SubscriptionPlanSchema),
  model_rates: z.array(ModelRateSchema),
});

export const CommerceWalletSchema = z.object({
  org_id: z.string(),
  available_credit: CommerceMoneySchema,
  journal_version: z.number().int().nonnegative().safe(),
  as_of: z.string(),
  activity: z
    .array(
      z.object({
        journal_sequence: z.number().int().positive().safe(),
        event_type: z.enum([
          "credit_granted",
          "credit_reserved",
          "reservation_settled",
          "reservation_released",
        ]),
        amount: CommerceMoneySchema.nullable(),
        reference_id: z.string(),
        occurred_at: z.string(),
      }),
    )
    .max(20),
});

const TopUpOrderProjectionSchema = z.object({
  order_id: z.string(),
  org_id: z.string(),
  package_id: z.string(),
  catalog_version: z.string(),
  customer_price: CommerceMoneySchema,
  granted_credit: CommerceMoneySchema,
  payment_status: z.enum([
    "created",
    "pending",
    "creation_unknown",
    "paid",
    "failed",
    "expired",
    "cancelled",
    "refunded",
  ]),
  fulfillment_status: z.enum(["not_ready", "ready", "granted", "reversed", "exception"]),
  payment_method: z
    .enum(["qris", "card", "dana", "gopay", "shopeepay", "bri_direct_debit"])
    .nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const TopUpOrderSchema = TopUpOrderProjectionSchema.extend({
  checkout_url: z.string().url().nullable(),
  payment_action: z
    .discriminatedUnion("kind", [
      z.object({
        kind: z.literal("qr_code"),
        qr_string: z.string().min(1).max(8192),
        expires_at: z.string().nullable(),
      }),
      z.object({
        kind: z.literal("redirect"),
        web_url: z.string().nullable(),
        mobile_deep_link: z.string().nullable(),
        expires_at: z.string().nullable(),
      }),
      z.object({
        kind: z.literal("card_components"),
        components_sdk_key: z.string().min(1).max(16384),
        expires_at: z.string().nullable(),
      }),
    ])
    .optional(),
});

export const PaymentMethodOptionSchema = z.object({
  method: z.enum(["qris", "card", "dana", "gopay", "shopeepay", "bri_direct_debit"]),
  label: z.string().trim().min(1).max(64),
  description: z.string().trim().min(1).max(240),
});

export const PaymentMethodOptionsSchema = z.array(PaymentMethodOptionSchema).max(12);

export const PaymentActionSchema = TopUpOrderSchema.shape.payment_action.unwrap();

export const TopUpOrderSummarySchema = TopUpOrderProjectionSchema.strict();
export const TopUpOrderListSchema = z.array(TopUpOrderSummarySchema).max(20);

export const PaymentReceiptSchema = z.object({
  order_id: z.string(),
  org_id: z.string(),
  invoice_id: z.string(),
  invoice_number: z.string(),
  payment_method: z
    .enum(["qris", "card", "dana", "gopay", "shopeepay", "bri_direct_debit"])
    .nullable(),
  customer_price: CommerceMoneySchema,
  granted_credit: CommerceMoneySchema,
  payment_status: TopUpOrderProjectionSchema.shape.payment_status,
  fulfillment_status: TopUpOrderProjectionSchema.shape.fulfillment_status,
  proof_state: z.enum([
    "awaiting_payment",
    "payment_verified",
    "credit_granted",
    "needs_review",
    "not_completed",
  ]),
  verified_at: z.string().nullable(),
  credit_granted_at: z.string().nullable(),
});

const InvoiceLineSchema = z.object({
  description: z.string(),
  quantity: z.number().int().positive(),
  unit_price: CommerceMoneySchema,
  line_total: CommerceMoneySchema,
});

export const CommerceInvoiceSchema = z.object({
  invoice_id: z.string(),
  invoice_number: z.string(),
  org_id: z.string(),
  brand_name: z.literal("GaussMeridian"),
  source_type: z.enum(["topup", "subscription_cycle"]),
  source_id: z.string(),
  catalog_version: z.string(),
  issued_at: z.string(),
  paid_at: z.string().nullable(),
  status: z.enum(["open", "paid", "void", "refunded"]),
  lines: z.array(InvoiceLineSchema).min(1),
  total: CommerceMoneySchema,
});

export const CommerceInvoiceListSchema = z.array(CommerceInvoiceSchema);

const SubscriptionPlanChangeSchema = z.object({
  change_id: z.string(),
  target_plan_id: z.string(),
  target_display_name: z.string().trim().min(1),
  target_cycle_price: CommerceMoneySchema,
  target_included_credit: CommerceMoneySchema,
  target_credit_policy: z.enum(["reset", "rollover"]),
  status: z.enum(["requested", "scheduled", "provider_unknown"]),
  effective_at: z.string(),
  requested_at: z.string(),
});

export const CommerceSubscriptionSchema = z.object({
  subscription_id: z.string(),
  org_id: z.string(),
  plan_id: z.string(),
  plan_display_name: z.string().trim().min(1),
  plan_summary: z.string().trim().min(1),
  catalog_version: z.string(),
  cycle_price: CommerceMoneySchema,
  included_credit: CommerceMoneySchema,
  credit_policy: z.enum(["reset", "rollover"]),
  status: z.enum([
    "created",
    "setup_pending",
    "creation_unknown",
    "setup_failed",
    "active",
    "past_due",
    "cancellation_requested",
    "cancellation_unknown",
    "cancel_pending",
    "cancelled",
  ]),
  checkout_url: z.string().url().nullable(),
  current_period_start: z.string().nullable(),
  current_period_end: z.string().nullable(),
  cancel_effective_at: z.string().nullable(),
  pending_change: SubscriptionPlanChangeSchema.nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

export const CommerceSubscriptionListSchema = z.array(CommerceSubscriptionSchema);

export type CommerceCatalog = z.infer<typeof CommerceCatalogSchema>;
export type CommerceWallet = z.infer<typeof CommerceWalletSchema>;
export type TopUpOrder = z.infer<typeof TopUpOrderSchema>;
export type PaymentMethodOption = z.infer<typeof PaymentMethodOptionSchema>;
export type PaymentAction = z.infer<typeof PaymentActionSchema>;
export type PaymentReceipt = z.infer<typeof PaymentReceiptSchema>;
export type TopUpOrderSummary = z.infer<typeof TopUpOrderSummarySchema>;
export type CommerceInvoice = z.infer<typeof CommerceInvoiceSchema>;
export type CommerceSubscription = z.infer<typeof CommerceSubscriptionSchema>;

/**
 * Customer-safe project key metadata returned by the project-scoped routes. The stored
 * key hash and owning user id are intentionally absent: neither is required to operate a key,
 * and neither belongs in a browser response.
 */
export const ProjectApiKeySchema = z.object({
  id: z.string(),
  // Nullable: `create_api_key` takes `project_id` as optional, so a key created outside the
  // project flow (the onboarding wizard's "first API key" step, or a direct API call) is stored
  // unscoped. A single unscoped key in the list would otherwise fail the whole array parse.
  project_id: z.string().nullable(),
  key_prefix: z.string(),
  name: z.string().nullable(),
  rate_limit_per_minute: z.number().nullable(),
  rate_limit_per_day: z.number().nullable(),
  created_at: z.string(),
  expires_at: z.string().nullable(),
  last_used_at: z.string().nullable(),
  active: z.boolean(),
});

// Traced against the real backend CreateApiKeyResponse (handlers.rs, create_api_key).
// `api_key` is the raw secret — returned ONCE on creation only, never re-fetchable via
// GET /v1/api/keys (which only ever returns key_hash).
export const CreateApiKeyResponseSchema = z.object({
  key_id: z.string(),
  api_key: z.string(),
  key_prefix: z.string(),
  message: z.string(),
});

// Traced against the real backend list_byok_keys response (handlers.rs): provider
// NAMES only — the encrypted key material never leaves the server, by design.
export const ByokProvidersSchema = z.object({
  providers: z.array(z.string()),
});

// Traced against the real backend ProjectSettingsResponse (handlers.rs,
// get_project_settings / update_project_settings). tau_moa and validator_type are
// read-only surfaces — the PATCH endpoint does not accept them.
export const ProjectSettingsSchema = z.object({
  lambda: z.number(),
  quality_floor: z.number(),
  tau_moa: z.number(),
  budget_monthly: z.number().nullable(),
  hard_limit: z.boolean(),
  alert_webhook_url: z.string().nullable(),
  validator_type: z.string(),
});

export type ProjectSettings = z.infer<typeof ProjectSettingsSchema>;
