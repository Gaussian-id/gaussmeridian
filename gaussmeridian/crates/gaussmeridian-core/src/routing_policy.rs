//! Deterministic routing-policy seam.
//!
//! This module deliberately contains no I/O. Server adapters freeze credentials,
//! catalog support, policy, prices, and evidence before calling [`select`].

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use serde::{Deserialize, Serialize};

use crate::classifier::ComplexityEvidence;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub mod bella;
pub mod compound;
pub mod predictors;
pub mod r2;
pub mod requirements;
pub mod snapshot;

use bella::{BellaCapabilityDecision, FrozenBellaEvidence};
use compound::{CompoundShadowDecision, FrozenCompoundEvidence};
use predictors::{FrozenPredictionEvidence, PredictionEstimate, PredictionFallbackReason};
use r2::{
    FrozenR2Evidence, R2ActionDisposition, R2ActionIdentity, R2Decision, R2EvaluatedAction,
    R2EvaluationDisposition, R2HeadPrediction, R2InstructionInputEstimate,
};

pub const SKILL_DIMENSIONS: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CapabilityBand {
    Baseline,
    Advanced,
    Frontier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeploymentKind {
    Managed,
    BringYourOwnKey,
    Local,
    AirGapped,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Price {
    /// Provider input-token price in billing currency units per million tokens.
    pub input_per_million: f64,
    /// Provider output-token price in billing currency units per million tokens.
    pub output_per_million: f64,
    /// Expected non-token provider charge used by deterministic risk ranking.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub expected_fixed_cost: f64,
    /// Conservative non-token provider-liability bound used by reservation.
    pub fixed_cost_upper_bound: f64,
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogModel {
    pub model_id: String,
    pub provider_id: String,
    /// Immutable provider/catalog metadata version used for execution evidence.
    pub model_version: String,
    pub capability_band: CapabilityBand,
    pub deployment_kind: DeploymentKind,
    pub price: Price,
    pub semantic_quality_prior: f64,
    pub transport_success_probability: f64,
    pub credential_available: bool,
    pub adapter_registered: bool,
    pub adapter_supports_model: bool,
    pub tenant_allowed: bool,
    pub compliant: bool,
    pub skill_proficiency: [f64; SKILL_DIMENSIONS],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    models: Vec<CatalogModel>,
}

impl CatalogSnapshot {
    pub fn new(models: Vec<CatalogModel>) -> Self {
        Self { models }
    }

    pub fn models(&self) -> &[CatalogModel] {
        &self.models
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillRequirement {
    pub skill_index: usize,
    pub minimum_proficiency: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutingContext {
    pub complexity: f32,
    pub estimated_input_tokens: u32,
    /// Ratified ceiling used for hard-budget authorization, distinct from the ranking estimate.
    pub input_token_upper_bound: u32,
    pub output_token_budget: u32,
    pub hard_skills: Vec<SkillRequirement>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectPolicy {
    /// Canonical risk weight: zero is quality-only; one is cost-only.
    pub cost_weight: f64,
    pub quality_floor: f64,
    pub max_band: CapabilityBand,
    pub moderate_complexity_threshold: f32,
    pub high_complexity_threshold: f32,
    /// Maximum ballot-prefix length that execution may dispatch.
    pub max_provider_attempts: u32,
    /// Maximum router inference or orchestration liability for the trajectory.
    pub router_cost_upper_bound: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSnapshot {
    pub policy_version: String,
    pub catalog_version: String,
    pub price_version: String,
    pub evaluator_version: String,
    /// Versioned normalization bounds. These are not recomputed from the candidate set.
    pub normalized_cost_floor: f64,
    pub normalized_cost_ceiling: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complexity: Option<ComplexityEvidence>,
    #[serde(default)]
    pub predictions: FrozenPredictionEvidence,
    #[serde(default, skip_serializing_if = "FrozenBellaEvidence::is_inactive")]
    pub bella: FrozenBellaEvidence,
    /// Request-scoped joint model-output-budget evidence; inactive preserves historical snapshots.
    #[serde(default, skip_serializing_if = "FrozenR2Evidence::is_inactive")]
    pub r2: FrozenR2Evidence,
    /// Bounded P5 compound candidates evaluated in shadow mode after the P4 ballot is final.
    #[serde(default, skip_serializing_if = "FrozenCompoundEvidence::is_inactive")]
    pub compound: FrozenCompoundEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ExclusionReason {
    CredentialUnavailable,
    AdapterNotRegistered,
    AdapterDoesNotSupportModel,
    TenantDenied,
    ComplianceDenied,
    AboveAbsoluteBandCeiling,
    HardSkillBelowThreshold { skill_index: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModelExclusion {
    pub model_id: String,
    pub provider_id: String,
    pub reasons: Vec<ExclusionReason>,
}

/// Ordered, bounded relaxations permitted after hard eligibility.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelaxationReason {
    /// Admit hard-eligible candidates below the configured advisory quality floor.
    QualityFloorRelaxed,
}

/// Typed explanation for the selected capability band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandSelectionReason {
    /// At least one qualified candidate exists in the desired band.
    DesiredBand,
    /// The deterministic nearest qualified band was selected.
    NearestAvailableBand,
}

/// Observable relationship between complexity's desired band and the selected band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandDecision {
    /// Capability band requested by the frozen complexity score.
    pub desired: CapabilityBand,
    /// Capability band selected from hard-eligible, quality-qualified candidates.
    pub selected: CapabilityBand,
    /// Typed explanation for equality or divergence.
    pub reason: BandSelectionReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BallotEntry {
    pub model_id: String,
    pub provider_id: String,
    pub capability_band: CapabilityBand,
    pub deployment_kind: DeploymentKind,
    pub output_token_budget: u32,
    /// Frozen justification for retaining the P3 ceiling or selecting a tested R2 budget.
    #[serde(default, skip_serializing_if = "R2ActionDisposition::is_predecessor")]
    pub r2_action: R2ActionDisposition,
    pub delivered_correctness_probability: f64,
    pub expected_provider_cost: f64,
    /// Frozen learned correctness estimate or typed reason for using the prior.
    #[serde(
        default = "default_prediction_estimate",
        skip_serializing_if = "is_default_prediction_estimate"
    )]
    pub outcome_prediction: PredictionEstimate,
    /// Frozen learned provider-cost estimate or typed reason for using the price prior.
    #[serde(
        default = "default_prediction_estimate",
        skip_serializing_if = "is_default_prediction_estimate"
    )]
    pub cost_prediction: PredictionEstimate,
    /// Maximum provider liability for this action under the frozen token and fixed-cost bounds.
    pub provider_cost_upper_bound: f64,
    pub normalized_cost: f64,
    /// Lower is better.
    pub risk: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Provider-liability quote that must be reserved atomically before dispatch.
pub struct TrajectoryReservationQuote {
    /// Router ceiling plus every provider ceiling in the permitted ballot prefix.
    pub amount: f64,
    /// Number of ballot entries covered by the quote.
    pub provider_attempts: usize,
}

fn default_prediction_estimate() -> PredictionEstimate {
    PredictionEstimate::Abstained {
        reason: PredictionFallbackReason::NoActiveState,
        uncertainty: 1.0,
    }
}

fn is_default_prediction_estimate(estimate: &PredictionEstimate) -> bool {
    *estimate == default_prediction_estimate()
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
/// Fail-closed reasons that prevent construction of a hard-budget reservation quote.
pub enum TrajectoryReservationError {
    #[error("invalid trajectory reservation policy: {reason}")]
    InvalidPolicy { reason: &'static str },
    #[error("invalid routing ballot for reservation: {reason}")]
    InvalidBallot { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionFingerprints {
    /// Content fingerprints for trace correlation. They become replay versions only when the
    /// canonical snapshot payloads are durably stored by the server.
    pub policy: String,
    pub catalog: String,
    pub prices: String,
    pub evaluator: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RoutingBallot {
    /// Immutable by construction. Consumers advance by index and never reorder it.
    entries: Box<[BallotEntry]>,
    reservation_authority: ReservationAuthority,
    /// Desired and selected capability-band evidence.
    pub band_decision: BandDecision,
    /// Ordered quality-only relaxations applied by selection.
    pub relaxations: Vec<RelaxationReason>,
    /// Every route rejected by hard eligibility and its reasons.
    pub exclusions: Vec<ModelExclusion>,
    /// Versions of every frozen decision input.
    pub fingerprints: DecisionFingerprints,
    /// Advisory BELLA decision applied strictly within the hard-eligible route set.
    #[serde(skip_serializing_if = "BellaCapabilityDecision::is_authorization_neutral")]
    pub bella: BellaCapabilityDecision,
    /// Joint route-budget evidence. Exact predecessor fallback omits this field.
    #[serde(skip_serializing_if = "R2Decision::is_predecessor")]
    pub r2: R2Decision,
    /// Replayable P5 recommendation that cannot modify the executable ballot.
    #[serde(skip_serializing_if = "CompoundShadowDecision::is_inactive")]
    pub compound: CompoundShadowDecision,
}

/// Failure to compute the content identity of an immutable ballot.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum BallotIdentityError {
    /// Canonical serialization of ballot content failed.
    #[error("routing ballot serialization failed: {0}")]
    Serialization(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
struct ReservationAuthority {
    max_provider_attempts: u32,
    router_cost_upper_bound: f64,
}

impl RoutingBallot {
    /// Returns the immutable candidate order authorized for execution.
    pub fn entries(&self) -> &[BallotEntry] {
        &self.entries
    }

    /// Content identity of this exact immutable ballot and authorized order.
    pub fn content_id(&self) -> Result<String, BallotIdentityError> {
        let canonical = serde_json::to_vec(self)
            .map_err(|error| BallotIdentityError::Serialization(error.to_string()))?;
        Ok(digest_parts("routing-ballot/v1", [canonical.as_slice()]))
    }
}

fn digest_parts<'a, I>(domain: &str, parts: I) -> String
where
    I: IntoIterator<Item = &'a [u8]>,
{
    let mut digest = Sha256::new();
    digest.update((domain.len() as u64).to_be_bytes());
    digest.update(domain.as_bytes());
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    format!("{:x}", digest.finalize())
}

/// Derives Spec 11's hard-budget reservation from the frozen ballot prefix.
pub fn quote_trajectory_reservation(
    ballot: &RoutingBallot,
) -> Result<TrajectoryReservationQuote, TrajectoryReservationError> {
    let authority = ballot.reservation_authority;
    if authority.max_provider_attempts == 0 {
        return Err(TrajectoryReservationError::InvalidPolicy {
            reason: "max_provider_attempts must be positive",
        });
    }
    if !authority.router_cost_upper_bound.is_finite() || authority.router_cost_upper_bound < 0.0 {
        return Err(TrajectoryReservationError::InvalidPolicy {
            reason: "router_cost_upper_bound must be finite and non-negative",
        });
    }

    let max_provider_attempts = usize::try_from(authority.max_provider_attempts).map_err(|_| {
        TrajectoryReservationError::InvalidPolicy {
            reason: "max_provider_attempts is not representable on this platform",
        }
    })?;
    let provider_attempts = max_provider_attempts.min(ballot.entries.len());
    if provider_attempts == 0 {
        return Err(TrajectoryReservationError::InvalidBallot {
            reason: "ballot must contain at least one action",
        });
    }

    let mut amount = authority.router_cost_upper_bound;
    for entry in ballot.entries.iter().take(provider_attempts) {
        if !entry.provider_cost_upper_bound.is_finite() || entry.provider_cost_upper_bound < 0.0 {
            return Err(TrajectoryReservationError::InvalidBallot {
                reason: "provider cost upper bounds must be finite and non-negative",
            });
        }
        amount += entry.provider_cost_upper_bound;
        if !amount.is_finite() {
            return Err(TrajectoryReservationError::InvalidBallot {
                reason: "trajectory cost upper bound overflowed",
            });
        }
    }

    Ok(TrajectoryReservationQuote {
        amount,
        provider_attempts,
    })
}

#[derive(Clone, Debug, Error, PartialEq, Serialize, Deserialize)]
pub enum RoutingUnavailable {
    #[error("no model satisfies every hard routing constraint")]
    NoHardEligibleModels { exclusions: Vec<ModelExclusion> },
    #[error("invalid routing decision input: {reason}")]
    InvalidInput { reason: String },
}

impl RoutingUnavailable {
    pub fn exclusions(&self) -> &[ModelExclusion] {
        match self {
            Self::NoHardEligibleModels { exclusions } => exclusions,
            Self::InvalidInput { .. } => &[],
        }
    }
}

/// Select an ordered, reproducible ballot from immutable decision inputs.
pub fn select(
    context: &RoutingContext,
    policy: &ProjectPolicy,
    catalog: &CatalogSnapshot,
    evidence: &EvidenceSnapshot,
) -> Result<RoutingBallot, RoutingUnavailable> {
    validate_inputs(context, policy, catalog, evidence)?;

    let mut exclusions = Vec::new();
    let hard_eligible: Vec<&CatalogModel> = catalog
        .models()
        .iter()
        .filter(|model| {
            let reasons = hard_exclusion_reasons(model, context, policy);
            if reasons.is_empty() {
                true
            } else {
                exclusions.push(ModelExclusion {
                    model_id: model.model_id.clone(),
                    provider_id: model.provider_id.clone(),
                    reasons,
                });
                false
            }
        })
        .collect();

    exclusions.sort();
    if hard_eligible.is_empty() {
        return Err(RoutingUnavailable::NoHardEligibleModels { exclusions });
    }

    let mut hard_eligible_routes: Vec<_> = hard_eligible
        .iter()
        .map(|model| {
            predictors::RouteIdentity::new(&model.provider_id, &model.model_id).map_err(|error| {
                RoutingUnavailable::InvalidInput {
                    reason: format!("hard-eligible route identity is invalid: {error}"),
                }
            })
        })
        .collect::<Result<_, _>>()?;
    hard_eligible_routes.sort();
    let bella = evidence
        .bella
        .capability_decision(&hard_eligible_routes)
        .map_err(|error| RoutingUnavailable::InvalidInput {
            reason: format!("BELLA capability evidence is invalid: {error}"),
        })?;
    // Freeze predecessor band geometry before BELLA removes candidates. Membership may narrow,
    // but the surviving routes must retain the predecessor's relative order.
    let predecessor_quality_survivors: Vec<&CatalogModel> = hard_eligible
        .iter()
        .copied()
        .filter(|model| model.semantic_quality_prior >= policy.quality_floor)
        .collect();
    let predecessor_quality_survivors = if predecessor_quality_survivors.is_empty() {
        hard_eligible.clone()
    } else {
        predecessor_quality_survivors
    };
    let desired_band = desired_band(context.complexity, policy);
    let selected_band = nearest_available_band(
        desired_band,
        &predecessor_quality_survivors,
        context,
        evidence,
    )
    .ok_or_else(|| RoutingUnavailable::InvalidInput {
        reason: "quality-qualified candidates must contain an available capability band".into(),
    })?;
    let selected_routes: BTreeSet<(&str, &str)> = bella
        .selected_routes()
        .iter()
        .map(|route| (route.provider_id(), route.model_id()))
        .collect();
    let capability_eligible: Vec<&CatalogModel> = hard_eligible
        .iter()
        .copied()
        .filter(|model| {
            selected_routes.contains(&(model.provider_id.as_str(), model.model_id.as_str()))
        })
        .collect();

    let mut relaxations = Vec::new();
    let quality_survivors: Vec<&CatalogModel> = capability_eligible
        .iter()
        .copied()
        .filter(|model| model.semantic_quality_prior >= policy.quality_floor)
        .collect();
    let quality_survivors = if quality_survivors.is_empty() {
        relaxations.push(RelaxationReason::QualityFloorRelaxed);
        capability_eligible
    } else {
        quality_survivors
    };

    let band_decision = BandDecision {
        desired: desired_band,
        selected: selected_band,
        reason: if selected_band == desired_band {
            BandSelectionReason::DesiredBand
        } else {
            BandSelectionReason::NearestAvailableBand
        },
    };

    let usable_r2 = (evidence.r2.validate().is_ok()
        && evidence.r2.is_compatible_with(
            &evidence.catalog_version,
            &evidence.price_version,
            &evidence.evaluator_version,
        ))
    .then_some(&evidence.r2);
    let r2_selection = usable_r2.and_then(|r2| {
        let provenance = r2.provenance()?;
        r2_ballot_entries(&quality_survivors, context, policy, evidence, r2)
            .map(|(entries, evaluated_actions)| (entries, evaluated_actions, provenance))
    });
    let (mut entries, r2) = if let Some((entries, evaluated_actions, provenance)) = r2_selection {
        (
            entries,
            R2Decision::Applied {
                learner_state_id: provenance.learner_state_id.clone(),
                predictor_version: provenance.predictor_version.clone(),
                instruction_version: provenance.instruction_version.clone(),
                evaluated_actions,
            },
        )
    } else {
        (
            quality_survivors
                .iter()
                .map(|model| ballot_entry(model, context, policy, evidence))
                .collect(),
            R2Decision::default(),
        )
    };
    let r2_applied = !r2.is_predecessor();
    entries.sort_by(|left, right| {
        band_distance(left.capability_band, selected_band)
            .cmp(&band_distance(right.capability_band, selected_band))
            .then_with(|| left.capability_band.cmp(&right.capability_band))
            .then_with(|| {
                if r2_applied {
                    compare_r2_entries(left, right)
                } else {
                    compare_entries(left, right)
                }
            })
    });
    let compound = evidence.compound.shadow_decision(
        &entries,
        &evidence.policy_version,
        &evidence.catalog_version,
        &evidence.price_version,
        &evidence.evaluator_version,
    );

    Ok(RoutingBallot {
        entries: entries.into_boxed_slice(),
        reservation_authority: ReservationAuthority {
            max_provider_attempts: policy.max_provider_attempts,
            router_cost_upper_bound: policy.router_cost_upper_bound,
        },
        band_decision,
        relaxations,
        exclusions,
        fingerprints: DecisionFingerprints {
            policy: evidence.policy_version.clone(),
            catalog: evidence.catalog_version.clone(),
            prices: evidence.price_version.clone(),
            evaluator: evidence.evaluator_version.clone(),
        },
        bella,
        r2,
        compound,
    })
}

fn validate_inputs(
    context: &RoutingContext,
    policy: &ProjectPolicy,
    catalog: &CatalogSnapshot,
    evidence: &EvidenceSnapshot,
) -> Result<(), RoutingUnavailable> {
    let probabilities_valid = catalog.models().iter().all(|model| {
        valid_probability(model.semantic_quality_prior)
            && valid_probability(model.transport_success_probability)
            && model
                .skill_proficiency
                .iter()
                .all(|value| valid_probability(*value))
    });
    let prices_valid = catalog.models().iter().all(|model| {
        model.price.input_per_million.is_finite()
            && model.price.input_per_million >= 0.0
            && model.price.output_per_million.is_finite()
            && model.price.output_per_million >= 0.0
            && model.price.expected_fixed_cost.is_finite()
            && model.price.expected_fixed_cost >= 0.0
            && model.price.fixed_cost_upper_bound.is_finite()
            && model.price.fixed_cost_upper_bound >= 0.0
            && model.price.expected_fixed_cost <= model.price.fixed_cost_upper_bound
    });
    let thresholds_valid = context.complexity.is_finite()
        && (0.0..=1.0).contains(&context.complexity)
        && valid_probability(policy.cost_weight)
        && valid_probability(policy.quality_floor)
        && policy.max_provider_attempts > 0
        && policy.router_cost_upper_bound.is_finite()
        && policy.router_cost_upper_bound >= 0.0
        && policy.moderate_complexity_threshold.is_finite()
        && policy.high_complexity_threshold.is_finite()
        && (0.0..=1.0).contains(&policy.moderate_complexity_threshold)
        && (0.0..=1.0).contains(&policy.high_complexity_threshold)
        && policy.moderate_complexity_threshold < policy.high_complexity_threshold
        && context.hard_skills.iter().all(|requirement| {
            requirement.skill_index < SKILL_DIMENSIONS
                && valid_probability(requirement.minimum_proficiency)
        })
        && context.input_token_upper_bound >= context.estimated_input_tokens;
    let normalization_valid = evidence.normalized_cost_floor.is_finite()
        && evidence.normalized_cost_ceiling.is_finite()
        && evidence.normalized_cost_floor >= 0.0
        && evidence.normalized_cost_ceiling >= evidence.normalized_cost_floor;

    let predictions_valid = evidence.predictions.validate().is_ok();
    let bella_valid = evidence.bella.validate().is_ok()
        && evidence
            .bella
            .is_compatible_with(&evidence.catalog_version, &evidence.evaluator_version);

    if probabilities_valid
        && prices_valid
        && thresholds_valid
        && normalization_valid
        && predictions_valid
        && bella_valid
    {
        Ok(())
    } else {
        Err(RoutingUnavailable::InvalidInput {
            reason: "probabilities, prices, thresholds, skill indices, cost bounds, or frozen evidence are invalid".into(),
        })
    }
}

fn valid_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn hard_exclusion_reasons(
    model: &CatalogModel,
    context: &RoutingContext,
    policy: &ProjectPolicy,
) -> Vec<ExclusionReason> {
    let mut reasons = Vec::new();
    if !model.credential_available {
        reasons.push(ExclusionReason::CredentialUnavailable);
    }
    if !model.adapter_registered {
        reasons.push(ExclusionReason::AdapterNotRegistered);
    }
    if !model.adapter_supports_model {
        reasons.push(ExclusionReason::AdapterDoesNotSupportModel);
    }
    if !model.tenant_allowed {
        reasons.push(ExclusionReason::TenantDenied);
    }
    if !model.compliant {
        reasons.push(ExclusionReason::ComplianceDenied);
    }
    if model.capability_band > policy.max_band {
        reasons.push(ExclusionReason::AboveAbsoluteBandCeiling);
    }
    for requirement in &context.hard_skills {
        if model.skill_proficiency[requirement.skill_index] < requirement.minimum_proficiency {
            reasons.push(ExclusionReason::HardSkillBelowThreshold {
                skill_index: requirement.skill_index,
            });
        }
    }
    reasons
}

fn desired_band(complexity: f32, policy: &ProjectPolicy) -> CapabilityBand {
    let desired = if complexity >= policy.high_complexity_threshold {
        CapabilityBand::Frontier
    } else if complexity >= policy.moderate_complexity_threshold {
        CapabilityBand::Advanced
    } else {
        CapabilityBand::Baseline
    };
    desired.min(policy.max_band)
}

fn minimum_expected_cost_in_band(
    band: CapabilityBand,
    candidates: &[&CatalogModel],
    context: &RoutingContext,
    evidence: &EvidenceSnapshot,
) -> f64 {
    candidates
        .iter()
        .filter(|model| model.capability_band == band)
        .map(|model| effective_predictions(model, context, evidence).expected_provider_cost)
        .fold(f64::INFINITY, f64::min)
}

fn expected_provider_cost(model: &CatalogModel, context: &RoutingContext) -> f64 {
    (f64::from(context.estimated_input_tokens) * model.price.input_per_million
        + f64::from(context.output_token_budget) * model.price.output_per_million)
        / 1_000_000.0
        + model.price.expected_fixed_cost
}

fn nearest_available_band(
    desired: CapabilityBand,
    candidates: &[&CatalogModel],
    context: &RoutingContext,
    evidence: &EvidenceSnapshot,
) -> Option<CapabilityBand> {
    [
        CapabilityBand::Baseline,
        CapabilityBand::Advanced,
        CapabilityBand::Frontier,
    ]
    .into_iter()
    .filter(|band| {
        candidates
            .iter()
            .any(|model| model.capability_band == *band)
    })
    .min_by(|left, right| {
        band_distance(*left, desired)
            .cmp(&band_distance(*right, desired))
            .then_with(|| {
                minimum_expected_cost_in_band(*left, candidates, context, evidence).total_cmp(
                    &minimum_expected_cost_in_band(*right, candidates, context, evidence),
                )
            })
            .then_with(|| left.cmp(right))
    })
}

fn band_distance(left: CapabilityBand, right: CapabilityBand) -> u8 {
    let ordinal = |band| match band {
        CapabilityBand::Baseline => 0_i8,
        CapabilityBand::Advanced => 1,
        CapabilityBand::Frontier => 2,
    };
    ordinal(left).abs_diff(ordinal(right))
}

fn ballot_entry(
    model: &CatalogModel,
    context: &RoutingContext,
    policy: &ProjectPolicy,
    evidence: &EvidenceSnapshot,
) -> BallotEntry {
    let predictions = effective_predictions(model, context, evidence);
    let expected_provider_cost = predictions.expected_provider_cost;
    let provider_cost_upper_bound = provider_cost_upper_bound(model, context);
    let normalized_cost = normalize_cost(expected_provider_cost, evidence);
    let delivered_correctness_probability = predictions.delivered_correctness_probability;
    let risk = canonical_risk(
        delivered_correctness_probability,
        normalized_cost,
        policy.cost_weight,
    );

    BallotEntry {
        model_id: model.model_id.clone(),
        provider_id: model.provider_id.clone(),
        capability_band: model.capability_band,
        deployment_kind: model.deployment_kind,
        output_token_budget: context.output_token_budget,
        r2_action: R2ActionDisposition::default(),
        delivered_correctness_probability,
        expected_provider_cost,
        outcome_prediction: predictions.outcome,
        cost_prediction: predictions.cost,
        provider_cost_upper_bound,
        normalized_cost,
        risk,
    }
}

fn r2_ballot_entries(
    models: &[&CatalogModel],
    context: &RoutingContext,
    policy: &ProjectPolicy,
    evidence: &EvidenceSnapshot,
    r2: &FrozenR2Evidence,
) -> Option<(Vec<BallotEntry>, Vec<R2EvaluatedAction>)> {
    let provenance = r2.provenance()?;
    let predecessor_instruction = r2.predecessor_instruction()?;
    let mut entries = Vec::with_capacity(models.len());
    let mut evaluated_actions = Vec::new();
    let mut evaluated_estimate = false;

    for model in models {
        let route = predictors::RouteIdentity::new(&model.provider_id, &model.model_id).ok()?;
        let predecessor =
            r2_predecessor_ballot_entry(model, context, policy, evidence, predecessor_instruction)?;
        let predecessor_action =
            R2ActionIdentity::new(route.clone(), context.output_token_budget).ok()?;
        let mut candidates = BTreeMap::from([(context.output_token_budget, predecessor.clone())]);
        let predecessor_disposition = r2
            .route_fallback_reason(&route, context.output_token_budget)
            .map_or(R2EvaluationDisposition::Predecessor, |reason| {
                R2EvaluationDisposition::Abstained {
                    reason,
                    uncertainty: 1.0,
                    diagnostics: Vec::new(),
                }
            });
        let mut route_evidence = BTreeMap::from([(
            predecessor_action.clone(),
            evaluated_action(
                predecessor_action,
                predecessor_disposition,
                Some(&predecessor),
            ),
        )]);

        for prediction in r2.predictions_for(&route, context.output_token_budget) {
            match prediction {
                R2HeadPrediction::Estimated(prediction) => {
                    let entry =
                        r2_ballot_entry(model, context, policy, evidence, provenance, prediction)?;
                    evaluated_estimate = true;
                    candidates.insert(prediction.action.output_budget(), entry.clone());
                    route_evidence.insert(
                        prediction.action.clone(),
                        evaluated_action(
                            prediction.action.clone(),
                            R2EvaluationDisposition::Estimated,
                            Some(&entry),
                        ),
                    );
                }
                R2HeadPrediction::Abstained(abstention) => {
                    let predecessor_entry = (abstention.action.output_budget()
                        == context.output_token_budget)
                        .then_some(&predecessor);
                    route_evidence.insert(
                        abstention.action.clone(),
                        evaluated_action(
                            abstention.action.clone(),
                            R2EvaluationDisposition::Abstained {
                                reason: abstention.reason,
                                uncertainty: abstention.uncertainty,
                                diagnostics: abstention.diagnostics.clone(),
                            },
                            predecessor_entry,
                        ),
                    );
                }
            }
        }

        let selected = candidates.into_values().min_by(compare_r2_entries)?;
        for action in route_evidence.values_mut() {
            action.selected = action.action.output_budget() == selected.output_token_budget;
        }
        entries.push(selected);
        evaluated_actions.extend(route_evidence.into_values());
    }

    if !evaluated_estimate {
        return None;
    }
    evaluated_actions.sort_by(|left, right| left.action.cmp(&right.action));
    Some((entries, evaluated_actions))
}

fn r2_predecessor_ballot_entry(
    model: &CatalogModel,
    context: &RoutingContext,
    policy: &ProjectPolicy,
    evidence: &EvidenceSnapshot,
    instruction: R2InstructionInputEstimate,
) -> Option<BallotEntry> {
    let mut entry = ballot_entry(model, context, policy, evidence);
    entry.expected_provider_cost +=
        f64::from(instruction.expected_tokens()) * model.price.input_per_million / 1_000_000.0;
    entry.provider_cost_upper_bound +=
        f64::from(instruction.upper_bound()) * model.price.input_per_million / 1_000_000.0;
    if !entry.expected_provider_cost.is_finite()
        || !entry.provider_cost_upper_bound.is_finite()
        || entry.expected_provider_cost > entry.provider_cost_upper_bound
    {
        return None;
    }
    entry.normalized_cost = normalize_cost(entry.expected_provider_cost, evidence);
    entry.risk = canonical_risk(
        entry.delivered_correctness_probability,
        entry.normalized_cost,
        policy.cost_weight,
    );
    entry.risk.is_finite().then_some(entry)
}

fn r2_ballot_entry(
    model: &CatalogModel,
    context: &RoutingContext,
    policy: &ProjectPolicy,
    evidence: &EvidenceSnapshot,
    provenance: &r2::R2Provenance,
    prediction: &r2::R2ActionPrediction,
) -> Option<BallotEntry> {
    let semantic_correctness = estimated_prediction_value(&prediction.semantic_correctness)?;
    let expected_output_tokens = estimated_prediction_value(&prediction.expected_output_tokens)?;
    if semantic_correctness > 1.0
        || expected_output_tokens > f64::from(prediction.action.output_budget())
    {
        return None;
    }
    let delivered_correctness_probability =
        model.transport_success_probability * semantic_correctness;
    let expected_provider_cost = ((f64::from(context.estimated_input_tokens)
        + f64::from(prediction.instruction_input_tokens))
        * model.price.input_per_million
        + expected_output_tokens * model.price.output_per_million)
        / 1_000_000.0
        + model.price.expected_fixed_cost;
    let provider_cost_upper_bound = ((f64::from(context.input_token_upper_bound)
        + f64::from(prediction.instruction_input_upper_bound))
        * model.price.input_per_million
        + f64::from(prediction.action.output_budget()) * model.price.output_per_million)
        / 1_000_000.0
        + model.price.fixed_cost_upper_bound;
    if !expected_provider_cost.is_finite()
        || !provider_cost_upper_bound.is_finite()
        || expected_provider_cost < 0.0
        || expected_provider_cost > provider_cost_upper_bound
    {
        return None;
    }

    let normalized_cost = normalize_cost(expected_provider_cost, evidence);
    let risk = canonical_risk(
        delivered_correctness_probability,
        normalized_cost,
        policy.cost_weight,
    );
    if !risk.is_finite() {
        return None;
    }
    let predecessor_predictions = effective_predictions(model, context, evidence);

    Some(BallotEntry {
        model_id: model.model_id.clone(),
        provider_id: model.provider_id.clone(),
        capability_band: model.capability_band,
        deployment_kind: model.deployment_kind,
        output_token_budget: prediction.action.output_budget(),
        r2_action: R2ActionDisposition::estimated(provenance, prediction),
        delivered_correctness_probability,
        expected_provider_cost,
        outcome_prediction: predecessor_predictions.outcome,
        cost_prediction: predecessor_predictions.cost,
        provider_cost_upper_bound,
        normalized_cost,
        risk,
    })
}

fn evaluated_action(
    action: R2ActionIdentity,
    disposition: R2EvaluationDisposition,
    entry: Option<&BallotEntry>,
) -> R2EvaluatedAction {
    R2EvaluatedAction {
        action,
        disposition,
        delivered_correctness_probability: entry
            .map(|entry| entry.delivered_correctness_probability),
        expected_provider_cost: entry.map(|entry| entry.expected_provider_cost),
        provider_cost_upper_bound: entry.map(|entry| entry.provider_cost_upper_bound),
        normalized_cost: entry.map(|entry| entry.normalized_cost),
        risk: entry.map(|entry| entry.risk),
        selected: false,
    }
}

fn estimated_prediction_value(estimate: &PredictionEstimate) -> Option<f64> {
    match estimate {
        PredictionEstimate::Estimated { value, .. } if value.is_finite() && *value >= 0.0 => {
            Some(*value)
        }
        PredictionEstimate::Estimated { .. } | PredictionEstimate::Abstained { .. } => None,
    }
}

fn normalize_cost(expected_provider_cost: f64, evidence: &EvidenceSnapshot) -> f64 {
    if evidence.normalized_cost_ceiling == evidence.normalized_cost_floor {
        0.0
    } else {
        ((expected_provider_cost - evidence.normalized_cost_floor)
            / (evidence.normalized_cost_ceiling - evidence.normalized_cost_floor))
            .clamp(0.0, 1.0)
    }
}

fn canonical_risk(
    delivered_correctness_probability: f64,
    normalized_cost: f64,
    cost_weight: f64,
) -> f64 {
    (1.0 - cost_weight) * (1.0 - delivered_correctness_probability) + cost_weight * normalized_cost
}

fn provider_cost_upper_bound(model: &CatalogModel, context: &RoutingContext) -> f64 {
    (f64::from(context.input_token_upper_bound) * model.price.input_per_million
        + f64::from(context.output_token_budget) * model.price.output_per_million)
        / 1_000_000.0
        + model.price.fixed_cost_upper_bound
}

struct EffectivePredictions {
    delivered_correctness_probability: f64,
    expected_provider_cost: f64,
    outcome: PredictionEstimate,
    cost: PredictionEstimate,
}

fn effective_predictions(
    model: &CatalogModel,
    context: &RoutingContext,
    evidence: &EvidenceSnapshot,
) -> EffectivePredictions {
    let prior_outcome = model.transport_success_probability * model.semantic_quality_prior;
    let prior_cost = expected_provider_cost(model, context);
    let (outcome, mut cost) = evidence
        .predictions
        .estimates_for(&model.provider_id, &model.model_id);
    let delivered_correctness_probability = match outcome {
        PredictionEstimate::Estimated { value, .. } => value,
        PredictionEstimate::Abstained { .. } => prior_outcome,
    };
    let expected_provider_cost = match cost {
        PredictionEstimate::Estimated { value, .. }
            if value <= provider_cost_upper_bound(model, context) =>
        {
            value
        }
        PredictionEstimate::Estimated { .. } => {
            cost = PredictionEstimate::Abstained {
                reason: PredictionFallbackReason::InvalidState,
                uncertainty: 1.0,
            };
            prior_cost
        }
        PredictionEstimate::Abstained { .. } => prior_cost,
    };
    EffectivePredictions {
        delivered_correctness_probability,
        expected_provider_cost,
        outcome,
        cost,
    }
}

fn compare_entries(left: &BallotEntry, right: &BallotEntry) -> Ordering {
    left.risk
        .total_cmp(&right.risk)
        .then_with(|| {
            left.expected_provider_cost
                .total_cmp(&right.expected_provider_cost)
        })
        .then_with(|| left.model_id.cmp(&right.model_id))
        .then_with(|| left.provider_id.cmp(&right.provider_id))
}

fn compare_r2_entries(left: &BallotEntry, right: &BallotEntry) -> Ordering {
    left.risk
        .total_cmp(&right.risk)
        .then_with(|| {
            left.expected_provider_cost
                .total_cmp(&right.expected_provider_cost)
        })
        .then_with(|| {
            right
                .delivered_correctness_probability
                .total_cmp(&left.delivered_correctness_probability)
        })
        .then_with(|| left.output_token_budget.cmp(&right.output_token_budget))
        .then_with(|| left.model_id.cmp(&right.model_id))
        .then_with(|| left.provider_id.cmp(&right.provider_id))
}
