//! Pure, bounded compound-policy contracts for P5 shadow evaluation.
//!
//! This module cannot dispatch. It validates frozen trajectory candidates only
//! against the immutable P4 ballot supplied by the caller and returns
//! replayable shadow evidence.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::BallotEntry;

pub const COMPOUND_STATE_SCHEMA_VERSION: &str = "compound-learner-state/v1";
pub const COMPOUND_LEARNER_VERSION: &str = "xrouter-shadow-state/v1";
pub const COMPOUND_RUNTIME_FEATURE_VERSION: &str = "xrouter-shadow-features/v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundActionKind {
    DirectAnswer,
    Delegate,
    ModelCall,
    Select,
    Synthesize,
}

impl CompoundActionKind {
    const fn requires_route(self) -> bool {
        matches!(self, Self::Delegate | Self::ModelCall | Self::Synthesize)
    }

    const fn is_provider_call(self) -> bool {
        self.requires_route()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundRouteAction {
    pub provider_id: String,
    pub model_id: String,
    pub output_budget: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundActionCost {
    pub provider: f64,
    pub tool: f64,
    pub selection: f64,
    pub synthesis: f64,
}

impl CompoundActionCost {
    pub fn total(self) -> f64 {
        self.provider + self.tool + self.selection + self.synthesis
    }

    fn values(self) -> [f64; 4] {
        [self.provider, self.tool, self.selection, self.synthesis]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundStep {
    pub step_id: String,
    pub kind: CompoundActionKind,
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<CompoundRouteAction>,
    pub expected_cost: CompoundActionCost,
    pub cost_upper_bound: CompoundActionCost,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundTrajectoryCandidate {
    pub trajectory_id: String,
    pub terminal_success_probability: f64,
    pub router_expected_cost: f64,
    pub router_cost_upper_bound: f64,
    pub steps: Vec<CompoundStep>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundLearnerState {
    schema_version: String,
    lineage: CompoundLineage,
    policy: CompoundPolicy,
    candidates: Vec<CompoundTrajectoryCandidate>,
}

impl CompoundLearnerState {
    pub fn new(
        mut lineage: CompoundLineage,
        policy: CompoundPolicy,
        mut candidates: Vec<CompoundTrajectoryCandidate>,
    ) -> Result<Self, CompoundError> {
        lineage.learner_state_id = "0".repeat(64);
        candidates.sort_by(|left, right| left.trajectory_id.cmp(&right.trajectory_id));
        let mut state = Self {
            schema_version: COMPOUND_STATE_SCHEMA_VERSION.into(),
            lineage,
            policy,
            candidates,
        };
        state.validate_structure()?;
        state.lineage.learner_state_id = state.compute_content_id()?;
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), CompoundError> {
        self.validate_structure()?;
        if self.compute_content_id()? != self.lineage.learner_state_id {
            return Err(CompoundError::LearnerStateIdMismatch);
        }
        Ok(())
    }

    pub fn lineage(&self) -> &CompoundLineage {
        &self.lineage
    }

    pub fn policy(&self) -> &CompoundPolicy {
        &self.policy
    }

    pub fn candidates(&self) -> &[CompoundTrajectoryCandidate] {
        &self.candidates
    }

    pub fn content_id(&self) -> Result<String, CompoundError> {
        self.validate_structure()?;
        self.compute_content_id()
    }

    pub fn freeze(&self) -> Result<FrozenCompoundEvidence, CompoundError> {
        self.validate()?;
        FrozenCompoundEvidence::active(
            self.lineage.clone(),
            self.policy.clone(),
            self.candidates.clone(),
        )
    }

    fn validate_structure(&self) -> Result<(), CompoundError> {
        if self.schema_version != COMPOUND_STATE_SCHEMA_VERSION {
            return Err(CompoundError::VersionMismatch {
                field: "state.schema_version",
            });
        }
        self.lineage.validate()?;
        self.policy.validate()?;
        if self.lineage.learner_version != COMPOUND_LEARNER_VERSION {
            return Err(CompoundError::VersionMismatch {
                field: "state.learner_version",
            });
        }
        if self.lineage.feature_version != COMPOUND_RUNTIME_FEATURE_VERSION {
            return Err(CompoundError::VersionMismatch {
                field: "state.feature_version",
            });
        }
        if self.policy.version != self.lineage.policy_version {
            return Err(CompoundError::VersionMismatch {
                field: "state.policy_version",
            });
        }
        let mut canonical = self.candidates.clone();
        canonical.sort_by(|left, right| left.trajectory_id.cmp(&right.trajectory_id));
        if canonical != self.candidates {
            return Err(CompoundError::NoncanonicalCandidates);
        }
        Ok(())
    }

    fn compute_content_id(&self) -> Result<String, CompoundError> {
        let mut content = self.clone();
        content.lineage.learner_state_id = "0".repeat(64);
        let payload =
            serde_json::to_vec(&content).map_err(|error| CompoundError::Serialization {
                reason: error.to_string(),
            })?;
        Ok(format!("{:x}", Sha256::digest(payload)))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundPolicy {
    pub version: String,
    pub max_steps: u32,
    pub max_provider_calls: u32,
    pub max_synthesis_calls: u32,
    pub max_trajectory_liability: f64,
    pub reward_constant: f64,
    pub cost_weight: f64,
    pub penalty_cap: f64,
}

impl CompoundPolicy {
    pub fn validate(&self) -> Result<(), CompoundError> {
        require_nonblank("policy.version", &self.version)?;
        if self.max_steps == 0 {
            return Err(CompoundError::InvalidPolicy {
                field: "policy.max_steps",
            });
        }
        for (field, value) in [
            (
                "policy.max_trajectory_liability",
                self.max_trajectory_liability,
            ),
            ("policy.reward_constant", self.reward_constant),
            ("policy.cost_weight", self.cost_weight),
            ("policy.penalty_cap", self.penalty_cap),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(CompoundError::InvalidPolicy { field });
            }
        }
        if self.reward_constant == 0.0 || self.penalty_cap > self.reward_constant {
            return Err(CompoundError::InvalidPolicy {
                field: "policy.reward_bounds",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundLineage {
    pub learner_state_id: String,
    pub learner_version: String,
    pub feature_version: String,
    pub evaluator_version: String,
    pub corpus_version: String,
    pub catalog_version: String,
    pub price_version: String,
    pub policy_version: String,
    pub training_content_hash: String,
}

impl CompoundLineage {
    pub fn validate(&self) -> Result<(), CompoundError> {
        for (field, value) in [
            ("lineage.learner_state_id", self.learner_state_id.as_str()),
            ("lineage.learner_version", self.learner_version.as_str()),
            ("lineage.feature_version", self.feature_version.as_str()),
            ("lineage.evaluator_version", self.evaluator_version.as_str()),
            ("lineage.corpus_version", self.corpus_version.as_str()),
            ("lineage.catalog_version", self.catalog_version.as_str()),
            ("lineage.price_version", self.price_version.as_str()),
            ("lineage.policy_version", self.policy_version.as_str()),
            (
                "lineage.training_content_hash",
                self.training_content_hash.as_str(),
            ),
        ] {
            require_nonblank(field, value)?;
        }
        for (field, value) in [
            ("lineage.learner_state_id", self.learner_state_id.as_str()),
            (
                "lineage.training_content_hash",
                self.training_content_hash.as_str(),
            ),
        ] {
            if !is_canonical_sha256(value) {
                return Err(CompoundError::InvalidSha256 { field });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundLineageField {
    Evaluator,
    Catalog,
    Price,
    Policy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason", content = "field")]
pub enum CompoundFallbackReason {
    RepositoryUnavailable,
    IdentityMismatch,
    InvalidState,
    ProductionPromotionBlocked,
    LineageMismatch(CompoundLineageField),
    NoValidCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundRejectionReason {
    BlankTrajectoryIdentity,
    DuplicateTrajectoryIdentity,
    EmptyTrajectory,
    StepLimitExceeded,
    ProviderCallLimitExceeded,
    SynthesisCallLimitExceeded,
    LiabilityLimitExceeded,
    BlankStepIdentity,
    DuplicateStepIdentity,
    DuplicateDependency,
    UnknownOrForwardDependency,
    RouteRequired,
    RouteForbidden,
    InvalidRoute,
    ActionNotInBallot,
    InvalidActionShape,
    InvalidProbability,
    InvalidCost,
    ExpectedCostExceedsUpperBound,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundCandidateRejection {
    pub trajectory_id: String,
    pub reason: CompoundRejectionReason,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundRecommendation {
    pub learner_state_id: String,
    pub policy_version: String,
    pub trajectory_id: String,
    pub action_count: u32,
    pub provider_call_count: u32,
    pub expected_total_cost: f64,
    pub liability_upper_bound: f64,
    pub expected_reward: f64,
    pub steps: Vec<CompoundStep>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CompoundShadowDecision {
    #[default]
    Inactive,
    Fallback {
        reason: CompoundFallbackReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state_id: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        rejections: Vec<CompoundCandidateRejection>,
    },
    Recommended {
        recommendation: CompoundRecommendation,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        rejections: Vec<CompoundCandidateRejection>,
    },
}

impl CompoundShadowDecision {
    pub const fn is_inactive(&self) -> bool {
        matches!(self, Self::Inactive)
    }

    pub fn recommendation(&self) -> Option<&CompoundRecommendation> {
        match self {
            Self::Recommended { recommendation, .. } => Some(recommendation),
            Self::Inactive | Self::Fallback { .. } => None,
        }
    }

    pub fn rejections(&self) -> &[CompoundCandidateRejection] {
        match self {
            Self::Recommended { rejections, .. } | Self::Fallback { rejections, .. } => rejections,
            Self::Inactive => &[],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FrozenCompoundEvidence {
    #[default]
    Inactive,
    Unavailable {
        reason: CompoundFallbackReason,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state_id: Option<String>,
    },
    Active {
        /// Durable repository-record identity authorized by the runtime boundary, when retained.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        record_state_id: Option<String>,
        lineage: CompoundLineage,
        policy: CompoundPolicy,
        candidates: Vec<CompoundTrajectoryCandidate>,
    },
}

impl FrozenCompoundEvidence {
    pub fn active(
        lineage: CompoundLineage,
        policy: CompoundPolicy,
        candidates: Vec<CompoundTrajectoryCandidate>,
    ) -> Result<Self, CompoundError> {
        Self::build_active(None, lineage, policy, candidates)
    }

    /// Builds active evidence with a separately verified durable repository-record identity.
    ///
    /// This preserves audit provenance only. The caller remains responsible for proving that the
    /// canonical record bytes hash to this identity and that an opaque runtime grant authorizes it.
    pub fn active_with_record_state_id(
        record_state_id: impl Into<String>,
        lineage: CompoundLineage,
        policy: CompoundPolicy,
        candidates: Vec<CompoundTrajectoryCandidate>,
    ) -> Result<Self, CompoundError> {
        let record_state_id = record_state_id.into();
        if !is_canonical_sha256(&record_state_id) {
            return Err(CompoundError::InvalidSha256 {
                field: "active.record_state_id",
            });
        }
        Self::build_active(Some(record_state_id), lineage, policy, candidates)
    }

    fn build_active(
        record_state_id: Option<String>,
        lineage: CompoundLineage,
        policy: CompoundPolicy,
        mut candidates: Vec<CompoundTrajectoryCandidate>,
    ) -> Result<Self, CompoundError> {
        lineage.validate()?;
        policy.validate()?;
        candidates.sort_by(|left, right| left.trajectory_id.cmp(&right.trajectory_id));
        Ok(Self::Active {
            record_state_id,
            lineage,
            policy,
            candidates,
        })
    }

    pub const fn unavailable(reason: CompoundFallbackReason) -> Self {
        Self::Unavailable {
            reason,
            state_id: None,
        }
    }

    pub fn unavailable_with_state(
        reason: CompoundFallbackReason,
        state_id: Option<String>,
    ) -> Self {
        Self::Unavailable { reason, state_id }
    }

    pub const fn is_inactive(&self) -> bool {
        matches!(self, Self::Inactive)
    }

    pub fn validate(&self) -> Result<(), CompoundError> {
        match self {
            Self::Inactive => Ok(()),
            Self::Unavailable {
                state_id: Some(state_id),
                ..
            } if !is_canonical_sha256(state_id) => Err(CompoundError::InvalidSha256 {
                field: "unavailable.state_id",
            }),
            Self::Unavailable { .. } => Ok(()),
            Self::Active {
                record_state_id,
                lineage,
                policy,
                candidates,
            } => {
                if record_state_id
                    .as_deref()
                    .is_some_and(|state_id| !is_canonical_sha256(state_id))
                {
                    return Err(CompoundError::InvalidSha256 {
                        field: "active.record_state_id",
                    });
                }
                lineage.validate()?;
                policy.validate()?;
                let mut canonical = candidates.clone();
                canonical.sort_by(|left, right| left.trajectory_id.cmp(&right.trajectory_id));
                if canonical != *candidates {
                    return Err(CompoundError::NoncanonicalCandidates);
                }
                Ok(())
            }
        }
    }

    pub fn shadow_decision(
        &self,
        entries: &[BallotEntry],
        policy_version: &str,
        catalog_version: &str,
        price_version: &str,
        evaluator_version: &str,
    ) -> CompoundShadowDecision {
        let Self::Active {
            lineage,
            policy,
            candidates,
            ..
        } = self
        else {
            return match self {
                Self::Inactive => CompoundShadowDecision::Inactive,
                Self::Unavailable { reason, state_id } => CompoundShadowDecision::Fallback {
                    reason: *reason,
                    state_id: state_id.clone(),
                    rejections: Vec::new(),
                },
                Self::Active { .. } => unreachable!("matched active evidence above"),
            };
        };

        if lineage.validate().is_err() || policy.validate().is_err() {
            return fallback(
                CompoundFallbackReason::InvalidState,
                Some(lineage.learner_state_id.clone()),
                Vec::new(),
            );
        }
        for (field, observed, expected) in [
            (
                CompoundLineageField::Policy,
                lineage.policy_version.as_str(),
                policy_version,
            ),
            (
                CompoundLineageField::Catalog,
                lineage.catalog_version.as_str(),
                catalog_version,
            ),
            (
                CompoundLineageField::Price,
                lineage.price_version.as_str(),
                price_version,
            ),
            (
                CompoundLineageField::Evaluator,
                lineage.evaluator_version.as_str(),
                evaluator_version,
            ),
        ] {
            if observed != expected {
                return fallback(
                    CompoundFallbackReason::LineageMismatch(field),
                    Some(lineage.learner_state_id.clone()),
                    Vec::new(),
                );
            }
        }

        evaluate_candidates(lineage, policy, candidates, entries)
    }

    /// Returns the separately verified durable repository-record identity, when retained.
    pub fn record_state_id(&self) -> Option<&str> {
        match self {
            Self::Active {
                record_state_id: Some(record_state_id),
                ..
            } => Some(record_state_id),
            Self::Inactive | Self::Unavailable { .. } | Self::Active { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum CompoundError {
    #[error("compound value is blank: {field}")]
    Blank { field: &'static str },
    #[error("compound value is not a canonical SHA-256 digest: {field}")]
    InvalidSha256 { field: &'static str },
    #[error("compound policy is invalid: {field}")]
    InvalidPolicy { field: &'static str },
    #[error("compound realized cost must be finite and non-negative")]
    InvalidRealizedCost,
    #[error("compound contract version mismatch: {field}")]
    VersionMismatch { field: &'static str },
    #[error("compound learner candidates are not canonically ordered")]
    NoncanonicalCandidates,
    #[error("compound learner-state identity does not match canonical content")]
    LearnerStateIdMismatch,
    #[error("compound canonical serialization failed: {reason}")]
    Serialization { reason: String },
}

pub fn realized_reward(
    policy: &CompoundPolicy,
    terminal_correct: bool,
    total_cost: f64,
) -> Result<f64, CompoundError> {
    policy.validate()?;
    if !total_cost.is_finite() || total_cost < 0.0 {
        return Err(CompoundError::InvalidRealizedCost);
    }
    if !terminal_correct {
        return Ok(0.0);
    }
    Ok(policy.reward_constant - (policy.cost_weight * total_cost).min(policy.penalty_cap))
}

fn evaluate_candidates(
    lineage: &CompoundLineage,
    policy: &CompoundPolicy,
    candidates: &[CompoundTrajectoryCandidate],
    entries: &[BallotEntry],
) -> CompoundShadowDecision {
    let ballot_actions: BTreeSet<_> = entries
        .iter()
        .map(|entry| CompoundRouteAction {
            provider_id: entry.provider_id.clone(),
            model_id: entry.model_id.clone(),
            output_budget: entry.output_token_budget,
        })
        .collect();
    let identity_counts =
        candidates
            .iter()
            .fold(BTreeMap::<&str, usize>::new(), |mut counts, candidate| {
                *counts.entry(candidate.trajectory_id.as_str()).or_default() += 1;
                counts
            });

    let mut recommendations = Vec::new();
    let mut rejections = Vec::new();
    for candidate in candidates {
        let result = if identity_counts
            .get(candidate.trajectory_id.as_str())
            .copied()
            .unwrap_or_default()
            > 1
        {
            Err(CompoundRejectionReason::DuplicateTrajectoryIdentity)
        } else {
            validate_candidate(candidate, policy, &ballot_actions)
        };
        match result {
            Ok(metrics) => recommendations.push(CompoundRecommendation {
                learner_state_id: lineage.learner_state_id.clone(),
                policy_version: policy.version.clone(),
                trajectory_id: candidate.trajectory_id.clone(),
                action_count: candidate.steps.len() as u32,
                provider_call_count: metrics.provider_calls,
                expected_total_cost: metrics.expected_total_cost,
                liability_upper_bound: metrics.liability_upper_bound,
                expected_reward: candidate.terminal_success_probability
                    * realized_reward(policy, true, metrics.expected_total_cost)
                        .expect("candidate and policy costs were validated"),
                steps: candidate.steps.clone(),
            }),
            Err(reason) => rejections.push(CompoundCandidateRejection {
                trajectory_id: candidate.trajectory_id.clone(),
                reason,
            }),
        }
    }

    rejections.sort();
    recommendations.sort_by(compare_recommendations);
    let Some(recommendation) = recommendations.into_iter().next() else {
        return fallback(
            CompoundFallbackReason::NoValidCandidate,
            Some(lineage.learner_state_id.clone()),
            rejections,
        );
    };
    CompoundShadowDecision::Recommended {
        recommendation,
        rejections,
    }
}

#[derive(Clone, Copy)]
struct CandidateMetrics {
    provider_calls: u32,
    expected_total_cost: f64,
    liability_upper_bound: f64,
}

fn validate_candidate(
    candidate: &CompoundTrajectoryCandidate,
    policy: &CompoundPolicy,
    ballot_actions: &BTreeSet<CompoundRouteAction>,
) -> Result<CandidateMetrics, CompoundRejectionReason> {
    if candidate.trajectory_id.trim().is_empty() {
        return Err(CompoundRejectionReason::BlankTrajectoryIdentity);
    }
    if !valid_probability(candidate.terminal_success_probability) {
        return Err(CompoundRejectionReason::InvalidProbability);
    }
    validate_cost_pair(
        candidate.router_expected_cost,
        candidate.router_cost_upper_bound,
    )?;
    if candidate.steps.is_empty() {
        return Err(CompoundRejectionReason::EmptyTrajectory);
    }
    if candidate.steps.len() > policy.max_steps as usize {
        return Err(CompoundRejectionReason::StepLimitExceeded);
    }

    let mut prior_steps = BTreeSet::new();
    let mut provider_calls = 0_u32;
    let mut synthesis_calls = 0_u32;
    let mut expected_total_cost = candidate.router_expected_cost;
    let mut liability_upper_bound = candidate.router_cost_upper_bound;

    for step in &candidate.steps {
        if step.step_id.trim().is_empty() {
            return Err(CompoundRejectionReason::BlankStepIdentity);
        }
        if prior_steps.contains(step.step_id.as_str()) {
            return Err(CompoundRejectionReason::DuplicateStepIdentity);
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &step.dependencies {
            if !dependencies.insert(dependency.as_str()) {
                return Err(CompoundRejectionReason::DuplicateDependency);
            }
        }
        if dependencies
            .iter()
            .any(|dependency| !prior_steps.contains(dependency))
        {
            return Err(CompoundRejectionReason::UnknownOrForwardDependency);
        }

        if step.kind.requires_route() {
            let route = step
                .route
                .as_ref()
                .ok_or(CompoundRejectionReason::RouteRequired)?;
            if route.provider_id.trim().is_empty()
                || route.model_id.trim().is_empty()
                || route.output_budget == 0
            {
                return Err(CompoundRejectionReason::InvalidRoute);
            }
            if !ballot_actions.contains(route) {
                return Err(CompoundRejectionReason::ActionNotInBallot);
            }
        } else if step.route.is_some() {
            return Err(CompoundRejectionReason::RouteForbidden);
        } else if step.expected_cost.provider > 0.0 || step.cost_upper_bound.provider > 0.0 {
            return Err(CompoundRejectionReason::InvalidActionShape);
        }

        match step.kind {
            CompoundActionKind::DirectAnswer if !step.dependencies.is_empty() => {
                return Err(CompoundRejectionReason::InvalidActionShape);
            }
            CompoundActionKind::Select if step.dependencies.len() < 2 => {
                return Err(CompoundRejectionReason::InvalidActionShape);
            }
            CompoundActionKind::Synthesize if step.dependencies.is_empty() => {
                return Err(CompoundRejectionReason::InvalidActionShape);
            }
            _ => {}
        }

        validate_action_costs(step.expected_cost, step.cost_upper_bound)?;
        if step.kind.is_provider_call() {
            provider_calls = provider_calls.saturating_add(1);
        }
        if step.kind == CompoundActionKind::Synthesize {
            synthesis_calls = synthesis_calls.saturating_add(1);
        }
        expected_total_cost += step.expected_cost.total();
        liability_upper_bound += step.cost_upper_bound.total();
        if !expected_total_cost.is_finite() || !liability_upper_bound.is_finite() {
            return Err(CompoundRejectionReason::InvalidCost);
        }
        prior_steps.insert(step.step_id.as_str());
    }

    if provider_calls > policy.max_provider_calls {
        return Err(CompoundRejectionReason::ProviderCallLimitExceeded);
    }
    if synthesis_calls > policy.max_synthesis_calls {
        return Err(CompoundRejectionReason::SynthesisCallLimitExceeded);
    }
    if liability_upper_bound > policy.max_trajectory_liability {
        return Err(CompoundRejectionReason::LiabilityLimitExceeded);
    }
    Ok(CandidateMetrics {
        provider_calls,
        expected_total_cost,
        liability_upper_bound,
    })
}

fn validate_action_costs(
    expected: CompoundActionCost,
    upper: CompoundActionCost,
) -> Result<(), CompoundRejectionReason> {
    for (expected, upper) in expected.values().into_iter().zip(upper.values()) {
        validate_cost_pair(expected, upper)?;
    }
    Ok(())
}

fn validate_cost_pair(expected: f64, upper: f64) -> Result<(), CompoundRejectionReason> {
    if !expected.is_finite() || expected < 0.0 || !upper.is_finite() || upper < 0.0 {
        return Err(CompoundRejectionReason::InvalidCost);
    }
    if expected > upper {
        return Err(CompoundRejectionReason::ExpectedCostExceedsUpperBound);
    }
    Ok(())
}

fn compare_recommendations(
    left: &CompoundRecommendation,
    right: &CompoundRecommendation,
) -> Ordering {
    right
        .expected_reward
        .total_cmp(&left.expected_reward)
        .then_with(|| {
            left.expected_total_cost
                .total_cmp(&right.expected_total_cost)
        })
        .then_with(|| left.action_count.cmp(&right.action_count))
        .then_with(|| left.trajectory_id.cmp(&right.trajectory_id))
}

fn fallback(
    reason: CompoundFallbackReason,
    state_id: Option<String>,
    rejections: Vec<CompoundCandidateRejection>,
) -> CompoundShadowDecision {
    CompoundShadowDecision::Fallback {
        reason,
        state_id,
        rejections,
    }
}

fn valid_probability(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn require_nonblank(field: &'static str, value: &str) -> Result<(), CompoundError> {
    if value.trim().is_empty() {
        Err(CompoundError::Blank { field })
    } else {
        Ok(())
    }
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
