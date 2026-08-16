//! Core router logic for GaussMeridian
//!
//! This crate contains the main routing logic, load balancing, and request
//! processing for the GaussMeridian system.

mod active_instruction;
pub mod advanced_analytics;
pub mod balancer;
pub mod billing;
pub mod cache;
pub mod cascade;
pub mod circuit_breaker;
pub mod classifier;
pub mod guardrails;
pub mod outcome_gate;
pub mod skill;
pub mod connection_pool;
pub mod cost_aware_routing;
pub mod distributed_rate_limiter;
pub mod error;
pub mod health;
pub mod load_balancer;
pub mod model_registry;
pub mod provider_registry;
pub mod rate_limiter;
pub mod request_batcher;
pub mod router;
pub mod routing_evidence;
pub mod routing_policy;
pub mod traits;
pub mod types;
pub mod usage_tracker;
pub mod validator;

pub use active_instruction::{
    ActiveInstructionEvidence, InstructionLanguageProfile, InstructionSpanEvidence,
    InstructionSpanKind, MERIDIAN_ACTIVE_INSTRUCTION_V1_VERSION,
    MERIDIAN_ACTIVE_INSTRUCTION_VERSION,
};
pub use balancer::{AdvancedLoadBalancer, LeastConnectionsLoadBalancer, WeightedLoadBalancer};
pub use billing::{
    BillingManager, BillingSummary, Budget, BudgetStatus, CostBreakdown, ModelPricing,
};
pub use cache::{CacheKey, CacheValue};
pub use classifier::{
    estimate_tokens, ClassificationResult, ComplexityEvidence, ComplexitySignalEvidence,
    ComplexitySignalKind, MeridianComplexityEstimator, MERIDIAN_COMPLEXITY_V2_VERSION,
    MERIDIAN_COMPLEXITY_V3_VERSION, MERIDIAN_COMPLEXITY_VERSION,
};
pub use skill::{extract_skill_vector, provider_capability_matrix, SKILL_DIMS,
    SKILL_NUMERICAL_REASONING, SKILL_CODE_SYNTHESIS, SKILL_TEMPORAL_LOGIC,
    SKILL_LEGAL_INTERPRETATION, SKILL_MEDICAL_KNOWLEDGE, SKILL_SCIENTIFIC_ANALYSIS,
    SKILL_CREATIVE_WRITING, SKILL_ENTITY_EXTRACTION, SKILL_SUMMARISATION,
    SKILL_TRANSLATION, SKILL_MATH_SYMBOLIC, SKILL_DATA_ANALYSIS,
};
pub use circuit_breaker::CircuitBreaker;
pub use connection_pool::{ConnectionGuard, ConnectionPool};
pub use cost_aware_routing::{
    CostAwareRouter, CostAwareRoutingConfig, ModelCapability, QualityTier, RoutingStats,
};
#[cfg(feature = "db")]
pub use distributed_rate_limiter::{
    DistributedRateLimiter, DistributedRateLimiterConfig, RateLimitCheckResult,
};
pub use error::GaussMeridianError;
pub use health::{
    health_score, provider_score, update_quality_ewma, EWMA_ALPHA,
    HealthStatus, ProviderHealth, ProviderInfo,
};
pub use load_balancer::{LoadBalancer, RoundRobinLoadBalancer};
pub use model_registry::ModelRegistry;
pub use cascade::{order_candidates_cheapest_first, should_escalate, CascadeConfig};
pub use guardrails::{GuardrailConfig, GuardrailEngine, GuardrailOutcome, GuardrailViolation};
pub use outcome_gate::{calibrate_confidence, extract_confidence, GateResult, OutcomeGate};
pub use provider_registry::{ProviderEntry, ProviderRegistry, ProviderStatus};
pub use rate_limiter::{RateLimit, RateLimiter};
pub use request_batcher::{BatchRequest, RequestBatcher};
pub use router::{EnterpriseGaussMeridian, GaussMeridian, Router};
pub use traits::LLMProvider;
pub use types::{BalanceInfo, RoutingStrategy, UsageInfo};
#[cfg(feature = "db")]
pub use usage_tracker::{
    ModelUsageSummary, RequestUsage, ResponseUsage, UsageSummary, UsageTracker,
};
pub use validator::RequestValidator;

// Re-export commonly used types
pub use gaussmeridian_models::*;

#[cfg(test)]
mod tests;
