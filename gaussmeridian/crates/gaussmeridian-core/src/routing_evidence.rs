//! Immutable, typed evidence and learning-path eligibility.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::routing_policy::compound::{
    realized_reward, CompoundActionCost, CompoundActionKind, CompoundLineage, CompoundPolicy,
    CompoundRejectionReason, CompoundRouteAction,
};

pub const MAX_SKILL_RATIONALE_BYTES: usize = 2_048;
pub const ROUTING_EVIDENCE_SCHEMA_V1: &str = "routing-evidence/v1";
pub const ROUTING_EVIDENCE_SCHEMA_V2: &str = "routing-evidence/v2";
/// Historical complete single-action-label evidence schema.
pub const ROUTING_EVIDENCE_SCHEMA_V3: &str = "routing-evidence/v3";
/// Current complete single-action and compound-trajectory evidence schema.
pub const ROUTING_EVIDENCE_SCHEMA_V4: &str = "routing-evidence/v4";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "value")]
pub enum Observation<T> {
    Observed(T),
    Unobserved { reason: String },
    Abstained { reason: String, uncertainty: f64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluatorProvenance {
    pub name: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CollectionMode {
    SelectedModelOnline,
    ControlledAllModel {
        group_id: String,
    },
    /// Controlled counterfactual coverage over every eligible model and tested output budget.
    ControlledAllModelBudget {
        group_id: String,
    },
    ExternalObservation {
        source: String,
        source_version: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryEvidence {
    pub delivered: bool,
    pub terminal_outcome: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SemanticEvidence {
    pub correct: bool,
    pub confidence: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillJudgement {
    pub correct: bool,
    pub confidence: f64,
    pub criticality: f64,
    pub rationale: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkillEvidence {
    pub skill_index: u16,
    pub skill_id: String,
    pub skill_name: String,
    pub assessment: Observation<SkillJudgement>,
    pub critic: EvaluatorProvenance,
    pub taxonomy_version: String,
    pub task_fingerprint: String,
    pub reference_fingerprint: String,
    pub answer_fingerprint: String,
}

/// Complete observed outcome for one provider-model-output-budget action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2Evidence {
    /// Caller-supplied absolute output-token ceiling.
    pub requested_output_tokens: u32,
    /// Tested output-budget action selected for the provider request.
    pub selected_output_budget: u32,
    /// Output tokens reported by the provider.
    pub actual_output_tokens: u32,
    /// Whether the observed answer terminated because the output budget was exhausted.
    pub truncated: bool,
    /// Whether the evaluator judged the answer semantically incomplete.
    pub incomplete: bool,
    /// Whether the provider output honored the versioned length instruction and token ceiling.
    pub instruction_compliant: bool,
    /// Evaluated semantic quality in the closed interval `[0, 1]`.
    pub quality: f64,
    /// Content identity of the predictor state evaluated by this label.
    pub predictor_state_id: String,
    /// Version of the output-budget instruction applied to the request.
    pub instruction_version: String,
    /// Version of the complete R2 action-label contract.
    pub label_version: String,
    /// Complete observed learner lineage. Historical v3 evidence may omit this field, but omitted
    /// lineage is never eligible for R2 learning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<R2EvidenceLineage>,
}

/// Observed lineage of the R2 state and controlled corpus that produced one action label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R2EvidenceLineage {
    pub predictor_version: String,
    pub encoder_version: String,
    pub feature_version: String,
    pub corpus_version: String,
    pub catalog_version: String,
    pub price_version: String,
    pub training_content_hash: String,
}

/// Immutable authority required before current R2 evidence may enter the learning projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct R2LearningAuthority {
    /// Controlled collection group whose outcomes may update this learner.
    pub collection_group_id: String,
    /// Exact content identity of the learner state evaluated by the evidence.
    pub predictor_state_id: String,
    /// Predictor implementation version expected by the learner state.
    pub predictor_version: String,
    /// Encoder implementation version expected by the learner state.
    pub encoder_version: String,
    /// Feature contract version expected by the learner state.
    pub feature_version: String,
    /// Evaluator name and version authorized to produce labels.
    pub evaluator: EvaluatorProvenance,
    /// Controlled corpus version that trained the learner state.
    pub corpus_version: String,
    /// Catalog version used for the controlled action space.
    pub catalog_version: String,
    /// Price version used for the controlled action economics.
    pub price_version: String,
    /// Output-budget instruction version applied to the request.
    pub instruction_version: String,
    /// Complete R2 action-label version produced by the evaluator.
    pub label_version: String,
    /// Content hash of the exact controlled training input.
    pub training_content_hash: String,
}

/// One exact authority axis used by typed R2 learning rejection evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum R2LineageField {
    CollectionGroup,
    PredictorState,
    Predictor,
    Encoder,
    Feature,
    EvaluatorName,
    EvaluatorVersion,
    Corpus,
    Catalog,
    Price,
    Instruction,
    Label,
    TrainingContent,
}

/// Typed reason complete evidence is not authorized for the R2 learning path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason", content = "field")]
pub enum R2LearningIneligibility {
    CollectionModeNotControlledAllModelBudget,
    R2NotObserved,
    MissingObservedLineage,
    LineageMismatch(R2LineageField),
}

/// Explainable R2 learning decision. Ineligibility never suppresses independent learning paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "detail")]
pub enum R2LearningEligibility {
    Eligible,
    Ineligible(R2LearningIneligibility),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyR2EvidenceV2 {
    accepted: bool,
    label_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryEvidence {
    pub reward: f64,
    pub step_count: u32,
    pub trajectory_version: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompoundActionOutcome {
    Completed,
    PartialFailure,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundObservedStep {
    pub step_id: String,
    pub kind: CompoundActionKind,
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<CompoundRouteAction>,
    pub outcome: CompoundActionOutcome,
    pub realized_cost: CompoundActionCost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CompoundPolicyResult {
    Passed,
    Violated {
        violations: Vec<CompoundRejectionReason>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompoundTrajectoryEvidence {
    pub trajectory_id: String,
    pub steps: Vec<CompoundObservedStep>,
    pub router_inference_cost_usd: f64,
    pub total_provider_cost_usd: f64,
    pub total_trajectory_cost_usd: f64,
    pub terminal_correct: Observation<bool>,
    pub terminal_outcome: String,
    pub policy_result: CompoundPolicyResult,
    pub realized_reward: f64,
    pub reward_policy_version: String,
    pub lineage: CompoundLineage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XRouterLearningAuthority {
    pub collection_group_id: String,
    pub evaluator: EvaluatorProvenance,
    pub lineage: CompoundLineage,
    pub policy: CompoundPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XRouterLineageField {
    CollectionGroup,
    LearnerState,
    Learner,
    Feature,
    EvaluatorName,
    EvaluatorVersion,
    Corpus,
    Catalog,
    Price,
    Policy,
    TrainingContent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason", content = "field")]
pub enum XRouterLearningIneligibility {
    CollectionModeNotControlledAllModelBudget,
    CompoundTrajectoryNotObserved,
    TerminalCorrectnessNotObserved,
    PolicyViolation,
    ProviderCostMismatch,
    TrajectoryCostMismatch,
    EnvelopeProviderCostMismatch,
    RewardMismatch,
    LineageMismatch(XRouterLineageField),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "detail")]
pub enum XRouterLearningEligibility {
    Eligible,
    Ineligible(XRouterLearningIneligibility),
}

/// Canonical current evidence envelope.
///
/// The default `R2Evidence` parameter is the public v4 contract. Historical v2 parsing substitutes
/// a private legacy label internally so its boolean cannot be constructed as current evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceEnvelope<R = R2Evidence> {
    pub schema_version: String,
    pub evidence_id: Option<String>,
    pub finalization_id: String,
    pub project_id: String,
    pub snapshot_fingerprint: String,
    pub provider_id: String,
    pub model_id: String,
    pub evaluator: EvaluatorProvenance,
    pub collection_mode: CollectionMode,
    pub delivery: Observation<DeliveryEvidence>,
    pub contract_validity: Observation<bool>,
    pub semantic: Observation<SemanticEvidence>,
    pub skills: Vec<SkillEvidence>,
    pub r2: Observation<R>,
    pub trajectory: Observation<TrajectoryEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compound_trajectory: Option<CompoundTrajectoryEvidence>,
    pub provider_cost_usd: f64,
    pub customer_charge_usd: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySkillEvidenceV1 {
    skill_index: u16,
    skill_name: String,
    correct: bool,
    critic_version: String,
    taxonomy_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEvidenceEnvelopeV1 {
    schema_version: String,
    evidence_id: Option<String>,
    finalization_id: String,
    project_id: String,
    snapshot_fingerprint: String,
    model_id: String,
    evaluator: EvaluatorProvenance,
    collection_mode: CollectionMode,
    delivery: Observation<DeliveryEvidence>,
    contract_validity: Observation<bool>,
    semantic: Observation<SemanticEvidence>,
    skills: Vec<LegacySkillEvidenceV1>,
    r2: Observation<LegacyR2EvidenceV2>,
    trajectory: Observation<TrajectoryEvidence>,
    provider_cost_usd: f64,
    customer_charge_usd: f64,
}

type LegacyEvidenceEnvelopeV2 = EvidenceEnvelope<LegacyR2EvidenceV2>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalEvidence {
    pub evidence_id: String,
    pub content_hash: String,
    pub canonical_payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningPath {
    MeridianSemantic,
    Carrot,
    Bella,
    R2,
    XRouter,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LearningUpdate {
    pub evidence_id: String,
    pub path: LearningPath,
    pub subject: String,
    pub observed_value: f64,
    pub provider_cost_usd: f64,
    pub customer_charge_usd: f64,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum EvidenceError {
    #[error("evidence has blank authoritative identifier: {field}")]
    BlankIdentifier { field: &'static str },
    #[error("evidence contains a non-finite value at {field}")]
    NonFinite { field: &'static str },
    #[error("evidence contains a negative value at {field}")]
    Negative { field: &'static str },
    #[error("evidence contains an out-of-range value at {field}")]
    OutOfRange { field: &'static str },
    #[error("evidence contains duplicate skill index {index}")]
    DuplicateSkillIndex { index: u16 },
    #[error("evidence contains duplicate skill identity {taxonomy_version}/{skill_id}")]
    DuplicateSkillIdentity {
        taxonomy_version: String,
        skill_id: String,
    },
    #[error("evidence skill observations mix taxonomy versions")]
    MixedSkillTaxonomy,
    #[error("evidence contains an invalid SHA-256 fingerprint at {field}")]
    InvalidFingerprint { field: &'static str },
    #[error("evidence text exceeds its bounded size at {field}")]
    TextTooLong { field: &'static str },
    #[error("evidence route identity must contain both provider and model or neither")]
    IncompleteRouteIdentity,
    #[error("unsupported routing evidence schema: {schema_version}")]
    UnsupportedSchema { schema_version: String },
    #[error("persisted routing evidence cannot be deserialized: {reason}")]
    Deserialization { reason: String },
    #[error("evidence trajectory has no steps")]
    EmptyTrajectory,
    #[error("evidence contains invalid compound trajectory content: {reason:?}")]
    InvalidCompound { reason: CompoundRejectionReason },
    #[error("evidence contains invalid compound lineage")]
    InvalidCompoundLineage,
    #[error("xRouter learning authority is invalid at {field}")]
    InvalidXRouterAuthority { field: &'static str },
    #[error("schema {schema_version} cannot contain compound trajectory evidence")]
    UnexpectedCompoundEvidence { schema_version: String },
    #[error(
        "R2 selected output budget {selected_output_budget} exceeds requested output tokens {requested_output_tokens}"
    )]
    R2SelectedBudgetExceedsRequest {
        requested_output_tokens: u32,
        selected_output_budget: u32,
    },
    #[error(
        "R2 instruction-compliant actual output {actual_output_tokens} exceeds selected budget {selected_output_budget}"
    )]
    R2CompliantTokenOverrun {
        selected_output_budget: u32,
        actual_output_tokens: u32,
    },
    #[error("supplied evidence id does not match canonical content: supplied={supplied}, expected={expected}")]
    EvidenceIdMismatch { supplied: String, expected: String },
    #[error("evidence cannot be serialized: {reason}")]
    Serialization { reason: String },
}

pub fn canonicalize_evidence(
    evidence: &EvidenceEnvelope,
) -> Result<CanonicalEvidence, EvidenceError> {
    validate(evidence)?;
    canonicalize_versioned(evidence)
}

fn canonicalize_versioned<R>(
    evidence: &EvidenceEnvelope<R>,
) -> Result<CanonicalEvidence, EvidenceError>
where
    R: Clone + Serialize,
{
    let mut content = evidence.clone();
    content.evidence_id = None;
    content.skills.sort_unstable_by(|left, right| {
        (
            &left.taxonomy_version,
            &left.skill_id,
            left.skill_index,
            &left.skill_name,
        )
            .cmp(&(
                &right.taxonomy_version,
                &right.skill_id,
                right.skill_index,
                &right.skill_name,
            ))
    });
    if let Some(compound) = &mut content.compound_trajectory {
        for step in &mut compound.steps {
            step.dependencies.sort_unstable();
        }
    }
    canonicalize_content(&content, evidence.evidence_id.as_deref())
}

/// Replays historical v1-v3 evidence or current v4 evidence without changing its schema.
///
/// Legacy evidence is deliberately not upconverted. Replay preserves the original schema and
/// content identity while version-specific learning qualification prevents historical v2's
/// boolean R2 label from gaining P4 learning authority.
pub fn canonicalize_persisted_evidence(payload: &[u8]) -> Result<CanonicalEvidence, EvidenceError> {
    let value: Value =
        serde_json::from_slice(payload).map_err(|error| EvidenceError::Deserialization {
            reason: error.to_string(),
        })?;
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| EvidenceError::UnsupportedSchema {
            schema_version: "missing".to_string(),
        })?;

    match schema_version {
        ROUTING_EVIDENCE_SCHEMA_V1 => {
            let evidence: LegacyEvidenceEnvelopeV1 =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            canonicalize_legacy_v1(&evidence)
        }
        ROUTING_EVIDENCE_SCHEMA_V2 => {
            let evidence: LegacyEvidenceEnvelopeV2 =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            canonicalize_legacy_v2(&evidence)
        }
        ROUTING_EVIDENCE_SCHEMA_V3 => {
            let evidence: EvidenceEnvelope =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            canonicalize_legacy_v3(&evidence)
        }
        ROUTING_EVIDENCE_SCHEMA_V4 => {
            let evidence: EvidenceEnvelope =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            canonicalize_evidence(&evidence)
        }
        other => Err(EvidenceError::UnsupportedSchema {
            schema_version: other.to_string(),
        }),
    }
}

/// Qualifies learning from a persisted version without granting newer paths to legacy evidence.
pub fn qualify_persisted_learning_updates(
    payload: &[u8],
) -> Result<Vec<LearningUpdate>, EvidenceError> {
    let value: Value =
        serde_json::from_slice(payload).map_err(|error| EvidenceError::Deserialization {
            reason: error.to_string(),
        })?;
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| EvidenceError::UnsupportedSchema {
            schema_version: "missing".to_string(),
        })?;

    match schema_version {
        ROUTING_EVIDENCE_SCHEMA_V1 => {
            let evidence: LegacyEvidenceEnvelopeV1 =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            canonicalize_legacy_v1(&evidence)?;
            Ok(Vec::new())
        }
        ROUTING_EVIDENCE_SCHEMA_V2 => {
            let evidence: LegacyEvidenceEnvelopeV2 =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            let canonical = canonicalize_legacy_v2(&evidence)?;
            qualify_non_r2_updates(&evidence, &canonical.evidence_id)
        }
        ROUTING_EVIDENCE_SCHEMA_V3 => {
            let evidence: EvidenceEnvelope =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            let canonical = canonicalize_legacy_v3(&evidence)?;
            qualify_non_r2_updates(&evidence, &canonical.evidence_id)
        }
        ROUTING_EVIDENCE_SCHEMA_V4 => {
            let evidence: EvidenceEnvelope =
                serde_json::from_value(value).map_err(|error| EvidenceError::Deserialization {
                    reason: error.to_string(),
                })?;
            qualify_learning_updates(&evidence)
        }
        other => Err(EvidenceError::UnsupportedSchema {
            schema_version: other.to_string(),
        }),
    }
}

pub fn qualify_learning_updates(
    evidence: &EvidenceEnvelope,
) -> Result<Vec<LearningUpdate>, EvidenceError> {
    let canonical = canonicalize_evidence(evidence)?;
    qualify_non_r2_updates(evidence, &canonical.evidence_id)
}

/// Qualifies all independent paths and grants R2 only when observed lineage exactly matches the
/// caller's immutable controlled-learning authority.
pub fn qualify_learning_updates_with_r2_authority(
    evidence: &EvidenceEnvelope,
    authority: &R2LearningAuthority,
) -> Result<Vec<LearningUpdate>, EvidenceError> {
    let canonical = canonicalize_evidence(evidence)?;
    validate_r2_learning_authority(authority)?;
    let mut updates = qualify_non_r2_updates(evidence, &canonical.evidence_id)?;

    if r2_learning_eligibility_unchecked(evidence, authority) == R2LearningEligibility::Eligible {
        let Observation::Observed(r2) = &evidence.r2 else {
            unreachable!("eligible R2 evidence is observed");
        };
        let subject = serde_json::to_string(&(
            evidence.provider_id.as_str(),
            evidence.model_id.as_str(),
            r2.selected_output_budget,
        ))
        .map_err(|error| EvidenceError::Serialization {
            reason: error.to_string(),
        })?;
        updates.push(LearningUpdate {
            evidence_id: canonical.evidence_id,
            path: LearningPath::R2,
            subject,
            observed_value: r2.quality,
            provider_cost_usd: evidence.provider_cost_usd,
            customer_charge_usd: evidence.customer_charge_usd,
        });
        updates
            .sort_by(|left, right| (left.path, &left.subject).cmp(&(right.path, &right.subject)));
    }

    Ok(updates)
}

/// Returns the exact, typed reason an R2 label is eligible or rejected.
pub fn r2_learning_eligibility(
    evidence: &EvidenceEnvelope,
    authority: &R2LearningAuthority,
) -> Result<R2LearningEligibility, EvidenceError> {
    validate(evidence)?;
    validate_r2_learning_authority(authority)?;
    Ok(r2_learning_eligibility_unchecked(evidence, authority))
}

/// Qualifies independent paths and grants xRouter only under exact immutable controlled authority.
pub fn qualify_learning_updates_with_xrouter_authority(
    evidence: &EvidenceEnvelope,
    authority: &XRouterLearningAuthority,
) -> Result<Vec<LearningUpdate>, EvidenceError> {
    let canonical = canonicalize_evidence(evidence)?;
    validate_xrouter_learning_authority(authority)?;
    let mut updates = qualify_non_r2_updates(evidence, &canonical.evidence_id)?;

    if xrouter_learning_eligibility_unchecked(evidence, authority)
        == XRouterLearningEligibility::Eligible
    {
        let trajectory = evidence
            .compound_trajectory
            .as_ref()
            .expect("eligible xRouter evidence has a trajectory");
        let terminal_correct = match trajectory.terminal_correct {
            Observation::Observed(value) => value,
            Observation::Unobserved { .. } | Observation::Abstained { .. } => {
                unreachable!("eligible xRouter evidence has observed correctness")
            }
        };
        let economics = recompute_compound_economics(trajectory)?;
        let reward = realized_reward(
            &authority.policy,
            terminal_correct,
            economics.total_trajectory_cost_usd,
        )
        .map_err(|_| EvidenceError::InvalidXRouterAuthority {
            field: "policy.reward",
        })?;
        updates.push(LearningUpdate {
            evidence_id: canonical.evidence_id,
            path: LearningPath::XRouter,
            subject: trajectory.trajectory_id.clone(),
            observed_value: reward,
            provider_cost_usd: economics.total_provider_cost_usd,
            customer_charge_usd: evidence.customer_charge_usd,
        });
        updates
            .sort_by(|left, right| (left.path, &left.subject).cmp(&(right.path, &right.subject)));
    }
    Ok(updates)
}

/// Returns the exact typed reason compound evidence is eligible or rejected for xRouter learning.
pub fn xrouter_learning_eligibility(
    evidence: &EvidenceEnvelope,
    authority: &XRouterLearningAuthority,
) -> Result<XRouterLearningEligibility, EvidenceError> {
    validate(evidence)?;
    validate_xrouter_learning_authority(authority)?;
    Ok(xrouter_learning_eligibility_unchecked(evidence, authority))
}

fn qualify_non_r2_updates<R>(
    evidence: &EvidenceEnvelope<R>,
    evidence_id: &str,
) -> Result<Vec<LearningUpdate>, EvidenceError> {
    let mut updates = Vec::new();
    let mut add = |path, subject: String, observed_value| {
        updates.push(LearningUpdate {
            evidence_id: evidence_id.to_string(),
            path,
            subject,
            observed_value,
            provider_cost_usd: evidence.provider_cost_usd,
            customer_charge_usd: evidence.customer_charge_usd,
        });
    };

    if let Observation::Observed(semantic) = &evidence.semantic {
        let semantic_value = f64::from(semantic.correct);
        add(
            LearningPath::MeridianSemantic,
            evidence.model_id.clone(),
            semantic_value,
        );
        if matches!(
            evidence.collection_mode,
            CollectionMode::ControlledAllModel { .. }
                | CollectionMode::ControlledAllModelBudget { .. }
        ) {
            add(
                LearningPath::Carrot,
                evidence.model_id.clone(),
                semantic_value,
            );
        }
    }

    for skill in &evidence.skills {
        if let Observation::Observed(judgement) = &skill.assessment {
            let subject = serde_json::to_string(&[
                evidence.provider_id.as_str(),
                evidence.model_id.as_str(),
                skill.taxonomy_version.as_str(),
                skill.skill_id.as_str(),
            ])
            .map_err(|error| EvidenceError::Serialization {
                reason: error.to_string(),
            })?;
            add(LearningPath::Bella, subject, f64::from(judgement.correct));
        }
    }
    updates.sort_by(|left, right| (left.path, &left.subject).cmp(&(right.path, &right.subject)));
    Ok(updates)
}

fn validate(evidence: &EvidenceEnvelope) -> Result<(), EvidenceError> {
    if evidence.schema_version != ROUTING_EVIDENCE_SCHEMA_V4 {
        return Err(EvidenceError::UnsupportedSchema {
            schema_version: evidence.schema_version.clone(),
        });
    }
    validate_versioned(evidence)?;
    validate_r2_evidence(evidence)?;
    if let Some(compound) = &evidence.compound_trajectory {
        validate_compound_trajectory(compound)?;
    }
    Ok(())
}

fn validate_r2_evidence(evidence: &EvidenceEnvelope) -> Result<(), EvidenceError> {
    if let Observation::Observed(r2) = &evidence.r2 {
        for (field, value) in [
            ("r2.predictor_state_id", r2.predictor_state_id.as_str()),
            ("r2.instruction_version", r2.instruction_version.as_str()),
            ("r2.label_version", r2.label_version.as_str()),
        ] {
            require_nonblank(field, value)?;
        }
        for (field, value) in [
            ("r2.requested_output_tokens", r2.requested_output_tokens),
            ("r2.selected_output_budget", r2.selected_output_budget),
            ("r2.actual_output_tokens", r2.actual_output_tokens),
        ] {
            if value == 0 {
                return Err(EvidenceError::OutOfRange { field });
            }
        }
        if r2.selected_output_budget > r2.requested_output_tokens {
            return Err(EvidenceError::R2SelectedBudgetExceedsRequest {
                requested_output_tokens: r2.requested_output_tokens,
                selected_output_budget: r2.selected_output_budget,
            });
        }
        if r2.instruction_compliant && r2.actual_output_tokens > r2.selected_output_budget {
            return Err(EvidenceError::R2CompliantTokenOverrun {
                selected_output_budget: r2.selected_output_budget,
                actual_output_tokens: r2.actual_output_tokens,
            });
        }
        validate_probability("r2.quality", r2.quality)?;
        validate_sha256("r2.predictor_state_id", &r2.predictor_state_id)?;
        if let Some(lineage) = &r2.lineage {
            for (field, value) in [
                (
                    "r2.lineage.predictor_version",
                    lineage.predictor_version.as_str(),
                ),
                (
                    "r2.lineage.encoder_version",
                    lineage.encoder_version.as_str(),
                ),
                (
                    "r2.lineage.feature_version",
                    lineage.feature_version.as_str(),
                ),
                ("r2.lineage.corpus_version", lineage.corpus_version.as_str()),
                (
                    "r2.lineage.catalog_version",
                    lineage.catalog_version.as_str(),
                ),
                ("r2.lineage.price_version", lineage.price_version.as_str()),
                (
                    "r2.lineage.training_content_hash",
                    lineage.training_content_hash.as_str(),
                ),
            ] {
                require_nonblank(field, value)?;
            }
            validate_sha256(
                "r2.lineage.training_content_hash",
                &lineage.training_content_hash,
            )?;
        }
    }

    Ok(())
}

fn validate_compound_trajectory(
    trajectory: &CompoundTrajectoryEvidence,
) -> Result<(), EvidenceError> {
    require_nonblank(
        "compound_trajectory.trajectory_id",
        &trajectory.trajectory_id,
    )?;
    require_nonblank(
        "compound_trajectory.terminal_outcome",
        &trajectory.terminal_outcome,
    )?;
    require_nonblank(
        "compound_trajectory.reward_policy_version",
        &trajectory.reward_policy_version,
    )?;
    trajectory
        .lineage
        .validate()
        .map_err(|_| EvidenceError::InvalidCompoundLineage)?;
    validate_observation(
        "compound_trajectory.terminal_correct",
        &trajectory.terminal_correct,
    )?;
    for (field, value) in [
        (
            "compound_trajectory.router_inference_cost_usd",
            trajectory.router_inference_cost_usd,
        ),
        (
            "compound_trajectory.total_provider_cost_usd",
            trajectory.total_provider_cost_usd,
        ),
        (
            "compound_trajectory.total_trajectory_cost_usd",
            trajectory.total_trajectory_cost_usd,
        ),
        (
            "compound_trajectory.realized_reward",
            trajectory.realized_reward,
        ),
    ] {
        validate_economics(field, value)?;
    }
    if trajectory.steps.is_empty() {
        return invalid_compound(CompoundRejectionReason::EmptyTrajectory);
    }
    if matches!(
        &trajectory.policy_result,
        CompoundPolicyResult::Violated { violations } if violations.is_empty()
    ) {
        return invalid_compound(CompoundRejectionReason::InvalidActionShape);
    }

    let mut prior_steps = BTreeSet::new();
    for step in &trajectory.steps {
        if step.step_id.trim().is_empty() {
            return invalid_compound(CompoundRejectionReason::BlankStepIdentity);
        }
        if prior_steps.contains(step.step_id.as_str()) {
            return invalid_compound(CompoundRejectionReason::DuplicateStepIdentity);
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &step.dependencies {
            if !dependencies.insert(dependency.as_str()) {
                return invalid_compound(CompoundRejectionReason::DuplicateDependency);
            }
        }
        if dependencies
            .iter()
            .any(|dependency| !prior_steps.contains(dependency))
        {
            return invalid_compound(CompoundRejectionReason::UnknownOrForwardDependency);
        }

        let provider_action = matches!(
            step.kind,
            CompoundActionKind::Delegate
                | CompoundActionKind::ModelCall
                | CompoundActionKind::Synthesize
        );
        if provider_action {
            let Some(route) = &step.route else {
                return invalid_compound(CompoundRejectionReason::RouteRequired);
            };
            if route.provider_id.trim().is_empty()
                || route.model_id.trim().is_empty()
                || route.output_budget == 0
            {
                return invalid_compound(CompoundRejectionReason::InvalidRoute);
            }
        } else if step.route.is_some() {
            return invalid_compound(CompoundRejectionReason::RouteForbidden);
        } else if step.realized_cost.provider > 0.0 {
            return invalid_compound(CompoundRejectionReason::InvalidActionShape);
        }

        match step.kind {
            CompoundActionKind::DirectAnswer if !step.dependencies.is_empty() => {
                return invalid_compound(CompoundRejectionReason::InvalidActionShape);
            }
            CompoundActionKind::Select if step.dependencies.len() < 2 => {
                return invalid_compound(CompoundRejectionReason::InvalidActionShape);
            }
            CompoundActionKind::Synthesize if step.dependencies.is_empty() => {
                return invalid_compound(CompoundRejectionReason::InvalidActionShape);
            }
            _ => {}
        }
        validate_compound_cost(step.realized_cost)?;
        prior_steps.insert(step.step_id.as_str());
    }
    Ok(())
}

fn validate_compound_cost(cost: CompoundActionCost) -> Result<(), EvidenceError> {
    for (field, value) in [
        ("compound_trajectory.steps.cost.provider", cost.provider),
        ("compound_trajectory.steps.cost.tool", cost.tool),
        ("compound_trajectory.steps.cost.selection", cost.selection),
        ("compound_trajectory.steps.cost.synthesis", cost.synthesis),
    ] {
        validate_economics(field, value)?;
    }
    Ok(())
}

fn invalid_compound<T>(reason: CompoundRejectionReason) -> Result<T, EvidenceError> {
    Err(EvidenceError::InvalidCompound { reason })
}

#[derive(Clone, Copy)]
struct CompoundEconomics {
    total_provider_cost_usd: f64,
    total_trajectory_cost_usd: f64,
}

fn recompute_compound_economics(
    trajectory: &CompoundTrajectoryEvidence,
) -> Result<CompoundEconomics, EvidenceError> {
    let mut provider = 0.0;
    let mut total = trajectory.router_inference_cost_usd;
    for step in &trajectory.steps {
        provider += step.realized_cost.provider;
        total += step.realized_cost.total();
        if !provider.is_finite() || !total.is_finite() {
            return Err(EvidenceError::NonFinite {
                field: "compound_trajectory.recomputed_cost",
            });
        }
    }
    Ok(CompoundEconomics {
        total_provider_cost_usd: provider,
        total_trajectory_cost_usd: total,
    })
}

fn validate_xrouter_learning_authority(
    authority: &XRouterLearningAuthority,
) -> Result<(), EvidenceError> {
    for (field, value) in [
        (
            "xrouter_authority.collection_group_id",
            authority.collection_group_id.as_str(),
        ),
        (
            "xrouter_authority.evaluator.name",
            authority.evaluator.name.as_str(),
        ),
        (
            "xrouter_authority.evaluator.version",
            authority.evaluator.version.as_str(),
        ),
    ] {
        require_nonblank(field, value)?;
    }
    authority
        .lineage
        .validate()
        .map_err(|_| EvidenceError::InvalidXRouterAuthority { field: "lineage" })?;
    authority
        .policy
        .validate()
        .map_err(|_| EvidenceError::InvalidXRouterAuthority { field: "policy" })?;
    if authority.evaluator.version != authority.lineage.evaluator_version {
        return Err(EvidenceError::InvalidXRouterAuthority {
            field: "evaluator_version",
        });
    }
    if authority.policy.version != authority.lineage.policy_version {
        return Err(EvidenceError::InvalidXRouterAuthority {
            field: "policy_version",
        });
    }
    Ok(())
}

fn xrouter_learning_eligibility_unchecked(
    evidence: &EvidenceEnvelope,
    authority: &XRouterLearningAuthority,
) -> XRouterLearningEligibility {
    let CollectionMode::ControlledAllModelBudget { group_id } = &evidence.collection_mode else {
        return xrouter_ineligible(
            XRouterLearningIneligibility::CollectionModeNotControlledAllModelBudget,
        );
    };
    if group_id != &authority.collection_group_id {
        return xrouter_lineage_mismatch(XRouterLineageField::CollectionGroup);
    }
    let Some(trajectory) = &evidence.compound_trajectory else {
        return xrouter_ineligible(XRouterLearningIneligibility::CompoundTrajectoryNotObserved);
    };
    let Observation::Observed(terminal_correct) = trajectory.terminal_correct else {
        return xrouter_ineligible(XRouterLearningIneligibility::TerminalCorrectnessNotObserved);
    };
    if !matches!(trajectory.policy_result, CompoundPolicyResult::Passed) {
        return xrouter_ineligible(XRouterLearningIneligibility::PolicyViolation);
    }

    for (field, observed, expected) in [
        (
            XRouterLineageField::LearnerState,
            trajectory.lineage.learner_state_id.as_str(),
            authority.lineage.learner_state_id.as_str(),
        ),
        (
            XRouterLineageField::Learner,
            trajectory.lineage.learner_version.as_str(),
            authority.lineage.learner_version.as_str(),
        ),
        (
            XRouterLineageField::Feature,
            trajectory.lineage.feature_version.as_str(),
            authority.lineage.feature_version.as_str(),
        ),
        (
            XRouterLineageField::EvaluatorName,
            evidence.evaluator.name.as_str(),
            authority.evaluator.name.as_str(),
        ),
        (
            XRouterLineageField::EvaluatorVersion,
            evidence.evaluator.version.as_str(),
            authority.evaluator.version.as_str(),
        ),
        (
            XRouterLineageField::EvaluatorVersion,
            trajectory.lineage.evaluator_version.as_str(),
            authority.lineage.evaluator_version.as_str(),
        ),
        (
            XRouterLineageField::Corpus,
            trajectory.lineage.corpus_version.as_str(),
            authority.lineage.corpus_version.as_str(),
        ),
        (
            XRouterLineageField::Catalog,
            trajectory.lineage.catalog_version.as_str(),
            authority.lineage.catalog_version.as_str(),
        ),
        (
            XRouterLineageField::Price,
            trajectory.lineage.price_version.as_str(),
            authority.lineage.price_version.as_str(),
        ),
        (
            XRouterLineageField::Policy,
            trajectory.lineage.policy_version.as_str(),
            authority.lineage.policy_version.as_str(),
        ),
        (
            XRouterLineageField::Policy,
            trajectory.reward_policy_version.as_str(),
            authority.policy.version.as_str(),
        ),
        (
            XRouterLineageField::TrainingContent,
            trajectory.lineage.training_content_hash.as_str(),
            authority.lineage.training_content_hash.as_str(),
        ),
    ] {
        if observed != expected {
            return xrouter_lineage_mismatch(field);
        }
    }

    let provider_calls = trajectory
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.kind,
                CompoundActionKind::Delegate
                    | CompoundActionKind::ModelCall
                    | CompoundActionKind::Synthesize
            )
        })
        .count() as u32;
    let synthesis_calls = trajectory
        .steps
        .iter()
        .filter(|step| step.kind == CompoundActionKind::Synthesize)
        .count() as u32;
    let Ok(economics) = recompute_compound_economics(trajectory) else {
        return xrouter_ineligible(XRouterLearningIneligibility::TrajectoryCostMismatch);
    };
    if trajectory.steps.len() > authority.policy.max_steps as usize
        || provider_calls > authority.policy.max_provider_calls
        || synthesis_calls > authority.policy.max_synthesis_calls
        || economics.total_trajectory_cost_usd > authority.policy.max_trajectory_liability
    {
        return xrouter_ineligible(XRouterLearningIneligibility::PolicyViolation);
    }
    if !approximately_equal(
        trajectory.total_provider_cost_usd,
        economics.total_provider_cost_usd,
    ) {
        return xrouter_ineligible(XRouterLearningIneligibility::ProviderCostMismatch);
    }
    if !approximately_equal(
        trajectory.total_trajectory_cost_usd,
        economics.total_trajectory_cost_usd,
    ) {
        return xrouter_ineligible(XRouterLearningIneligibility::TrajectoryCostMismatch);
    }
    if !approximately_equal(
        evidence.provider_cost_usd,
        economics.total_provider_cost_usd,
    ) {
        return xrouter_ineligible(XRouterLearningIneligibility::EnvelopeProviderCostMismatch);
    }
    let Ok(recomputed_reward) = realized_reward(
        &authority.policy,
        terminal_correct,
        economics.total_trajectory_cost_usd,
    ) else {
        return xrouter_ineligible(XRouterLearningIneligibility::RewardMismatch);
    };
    if !approximately_equal(trajectory.realized_reward, recomputed_reward) {
        return xrouter_ineligible(XRouterLearningIneligibility::RewardMismatch);
    }
    XRouterLearningEligibility::Eligible
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-12 * left.abs().max(right.abs()).max(1.0)
}

const fn xrouter_ineligible(reason: XRouterLearningIneligibility) -> XRouterLearningEligibility {
    XRouterLearningEligibility::Ineligible(reason)
}

const fn xrouter_lineage_mismatch(field: XRouterLineageField) -> XRouterLearningEligibility {
    xrouter_ineligible(XRouterLearningIneligibility::LineageMismatch(field))
}

fn validate_r2_learning_authority(authority: &R2LearningAuthority) -> Result<(), EvidenceError> {
    for (field, value) in [
        (
            "r2_authority.collection_group_id",
            authority.collection_group_id.as_str(),
        ),
        (
            "r2_authority.predictor_state_id",
            authority.predictor_state_id.as_str(),
        ),
        (
            "r2_authority.predictor_version",
            authority.predictor_version.as_str(),
        ),
        (
            "r2_authority.encoder_version",
            authority.encoder_version.as_str(),
        ),
        (
            "r2_authority.feature_version",
            authority.feature_version.as_str(),
        ),
        (
            "r2_authority.evaluator.name",
            authority.evaluator.name.as_str(),
        ),
        (
            "r2_authority.evaluator.version",
            authority.evaluator.version.as_str(),
        ),
        (
            "r2_authority.corpus_version",
            authority.corpus_version.as_str(),
        ),
        (
            "r2_authority.catalog_version",
            authority.catalog_version.as_str(),
        ),
        (
            "r2_authority.price_version",
            authority.price_version.as_str(),
        ),
        (
            "r2_authority.instruction_version",
            authority.instruction_version.as_str(),
        ),
        (
            "r2_authority.label_version",
            authority.label_version.as_str(),
        ),
        (
            "r2_authority.training_content_hash",
            authority.training_content_hash.as_str(),
        ),
    ] {
        require_nonblank(field, value)?;
    }
    validate_sha256(
        "r2_authority.predictor_state_id",
        &authority.predictor_state_id,
    )?;
    validate_sha256(
        "r2_authority.training_content_hash",
        &authority.training_content_hash,
    )?;
    Ok(())
}

fn r2_learning_eligibility_unchecked(
    evidence: &EvidenceEnvelope,
    authority: &R2LearningAuthority,
) -> R2LearningEligibility {
    let CollectionMode::ControlledAllModelBudget { group_id } = &evidence.collection_mode else {
        return R2LearningEligibility::Ineligible(
            R2LearningIneligibility::CollectionModeNotControlledAllModelBudget,
        );
    };
    if group_id != &authority.collection_group_id {
        return lineage_mismatch(R2LineageField::CollectionGroup);
    }
    let Observation::Observed(r2) = &evidence.r2 else {
        return R2LearningEligibility::Ineligible(R2LearningIneligibility::R2NotObserved);
    };
    let Some(lineage) = &r2.lineage else {
        return R2LearningEligibility::Ineligible(R2LearningIneligibility::MissingObservedLineage);
    };

    for (field, observed, expected) in [
        (
            R2LineageField::PredictorState,
            r2.predictor_state_id.as_str(),
            authority.predictor_state_id.as_str(),
        ),
        (
            R2LineageField::Predictor,
            lineage.predictor_version.as_str(),
            authority.predictor_version.as_str(),
        ),
        (
            R2LineageField::Encoder,
            lineage.encoder_version.as_str(),
            authority.encoder_version.as_str(),
        ),
        (
            R2LineageField::Feature,
            lineage.feature_version.as_str(),
            authority.feature_version.as_str(),
        ),
        (
            R2LineageField::EvaluatorName,
            evidence.evaluator.name.as_str(),
            authority.evaluator.name.as_str(),
        ),
        (
            R2LineageField::EvaluatorVersion,
            evidence.evaluator.version.as_str(),
            authority.evaluator.version.as_str(),
        ),
        (
            R2LineageField::Corpus,
            lineage.corpus_version.as_str(),
            authority.corpus_version.as_str(),
        ),
        (
            R2LineageField::Catalog,
            lineage.catalog_version.as_str(),
            authority.catalog_version.as_str(),
        ),
        (
            R2LineageField::Price,
            lineage.price_version.as_str(),
            authority.price_version.as_str(),
        ),
        (
            R2LineageField::Instruction,
            r2.instruction_version.as_str(),
            authority.instruction_version.as_str(),
        ),
        (
            R2LineageField::Label,
            r2.label_version.as_str(),
            authority.label_version.as_str(),
        ),
        (
            R2LineageField::TrainingContent,
            lineage.training_content_hash.as_str(),
            authority.training_content_hash.as_str(),
        ),
    ] {
        if observed != expected {
            return lineage_mismatch(field);
        }
    }

    R2LearningEligibility::Eligible
}

const fn lineage_mismatch(field: R2LineageField) -> R2LearningEligibility {
    R2LearningEligibility::Ineligible(R2LearningIneligibility::LineageMismatch(field))
}

fn validate_versioned<R>(evidence: &EvidenceEnvelope<R>) -> Result<(), EvidenceError> {
    for (field, value) in [
        ("schema_version", evidence.schema_version.as_str()),
        ("finalization_id", evidence.finalization_id.as_str()),
        ("project_id", evidence.project_id.as_str()),
        (
            "snapshot_fingerprint",
            evidence.snapshot_fingerprint.as_str(),
        ),
        ("provider_id", evidence.provider_id.as_str()),
        ("model_id", evidence.model_id.as_str()),
        ("evaluator.name", evidence.evaluator.name.as_str()),
        ("evaluator.version", evidence.evaluator.version.as_str()),
    ] {
        require_nonblank(field, value)?;
    }

    validate_economics("provider_cost_usd", evidence.provider_cost_usd)?;
    validate_economics("customer_charge_usd", evidence.customer_charge_usd)?;
    validate_observation("delivery", &evidence.delivery)?;
    validate_observation("contract_validity", &evidence.contract_validity)?;
    validate_observation("semantic", &evidence.semantic)?;
    validate_observation("r2", &evidence.r2)?;
    validate_observation("trajectory", &evidence.trajectory)?;

    match &evidence.collection_mode {
        CollectionMode::SelectedModelOnline => {}
        CollectionMode::ControlledAllModel { group_id }
        | CollectionMode::ControlledAllModelBudget { group_id } => {
            require_nonblank("collection_mode.group_id", group_id)?;
        }
        CollectionMode::ExternalObservation {
            source,
            source_version,
        } => {
            require_nonblank("collection_mode.source", source)?;
            require_nonblank("collection_mode.source_version", source_version)?;
        }
    }

    if let Observation::Observed(delivery) = &evidence.delivery {
        require_nonblank("delivery.terminal_outcome", &delivery.terminal_outcome)?;
    }
    if let Observation::Observed(semantic) = &evidence.semantic {
        validate_probability("semantic.confidence", semantic.confidence)?;
    }

    let mut skill_indices = BTreeSet::new();
    let mut skill_identities = BTreeSet::new();
    let mut taxonomy_version = None;
    for skill in &evidence.skills {
        if !skill_indices.insert(skill.skill_index) {
            return Err(EvidenceError::DuplicateSkillIndex {
                index: skill.skill_index,
            });
        }
        if !skill_identities.insert((&skill.taxonomy_version, &skill.skill_id)) {
            return Err(EvidenceError::DuplicateSkillIdentity {
                taxonomy_version: skill.taxonomy_version.clone(),
                skill_id: skill.skill_id.clone(),
            });
        }
        match taxonomy_version {
            Some(version) if version != skill.taxonomy_version.as_str() => {
                return Err(EvidenceError::MixedSkillTaxonomy);
            }
            None => taxonomy_version = Some(skill.taxonomy_version.as_str()),
            _ => {}
        }
        require_nonblank("skills.skill_id", &skill.skill_id)?;
        require_nonblank("skills.skill_name", &skill.skill_name)?;
        require_nonblank("skills.critic.name", &skill.critic.name)?;
        require_nonblank("skills.critic.version", &skill.critic.version)?;
        require_nonblank("skills.taxonomy_version", &skill.taxonomy_version)?;
        validate_sha256("skills.task_fingerprint", &skill.task_fingerprint)?;
        validate_sha256("skills.reference_fingerprint", &skill.reference_fingerprint)?;
        validate_sha256("skills.answer_fingerprint", &skill.answer_fingerprint)?;
        validate_observation("skills.assessment", &skill.assessment)?;
        if let Observation::Observed(judgement) = &skill.assessment {
            validate_probability("skills.confidence", judgement.confidence)?;
            validate_probability("skills.criticality", judgement.criticality)?;
            require_nonblank("skills.rationale", &judgement.rationale)?;
            if judgement.rationale.len() > MAX_SKILL_RATIONALE_BYTES {
                return Err(EvidenceError::TextTooLong {
                    field: "skills.rationale",
                });
            }
        }
    }
    if let Observation::Observed(trajectory) = &evidence.trajectory {
        require_nonblank(
            "trajectory.trajectory_version",
            &trajectory.trajectory_version,
        )?;
        if !trajectory.reward.is_finite() {
            return Err(EvidenceError::NonFinite {
                field: "trajectory.reward",
            });
        }
        if trajectory.step_count == 0 {
            return Err(EvidenceError::EmptyTrajectory);
        }
    }

    Ok(())
}

fn canonicalize_legacy_v3(evidence: &EvidenceEnvelope) -> Result<CanonicalEvidence, EvidenceError> {
    validate_legacy_v3(evidence)?;
    canonicalize_versioned(evidence)
}

fn validate_legacy_v3(evidence: &EvidenceEnvelope) -> Result<(), EvidenceError> {
    if evidence.schema_version != ROUTING_EVIDENCE_SCHEMA_V3 {
        return Err(EvidenceError::UnsupportedSchema {
            schema_version: evidence.schema_version.clone(),
        });
    }
    if evidence.compound_trajectory.is_some() {
        return Err(EvidenceError::UnexpectedCompoundEvidence {
            schema_version: evidence.schema_version.clone(),
        });
    }
    validate_versioned(evidence)?;
    validate_r2_evidence(evidence)
}

fn canonicalize_legacy_v2(
    evidence: &LegacyEvidenceEnvelopeV2,
) -> Result<CanonicalEvidence, EvidenceError> {
    validate_legacy_v2(evidence)?;
    canonicalize_versioned(evidence)
}

fn validate_legacy_v2(evidence: &LegacyEvidenceEnvelopeV2) -> Result<(), EvidenceError> {
    if evidence.schema_version != ROUTING_EVIDENCE_SCHEMA_V2 {
        return Err(EvidenceError::UnsupportedSchema {
            schema_version: evidence.schema_version.clone(),
        });
    }
    if evidence.compound_trajectory.is_some() {
        return Err(EvidenceError::UnexpectedCompoundEvidence {
            schema_version: evidence.schema_version.clone(),
        });
    }
    validate_versioned(evidence)?;
    if let Observation::Observed(r2) = &evidence.r2 {
        require_nonblank("r2.label_version", &r2.label_version)?;
    }
    Ok(())
}

fn canonicalize_legacy_v1(
    evidence: &LegacyEvidenceEnvelopeV1,
) -> Result<CanonicalEvidence, EvidenceError> {
    validate_legacy_v1(evidence)?;
    let mut content = evidence.clone();
    content.evidence_id = None;
    content.skills.sort_unstable_by(|left, right| {
        (left.skill_index, &left.skill_name).cmp(&(right.skill_index, &right.skill_name))
    });
    canonicalize_content(&content, evidence.evidence_id.as_deref())
}

fn validate_legacy_v1(evidence: &LegacyEvidenceEnvelopeV1) -> Result<(), EvidenceError> {
    if evidence.schema_version != ROUTING_EVIDENCE_SCHEMA_V1 {
        return Err(EvidenceError::UnsupportedSchema {
            schema_version: evidence.schema_version.clone(),
        });
    }
    for (field, value) in [
        ("finalization_id", evidence.finalization_id.as_str()),
        ("project_id", evidence.project_id.as_str()),
        (
            "snapshot_fingerprint",
            evidence.snapshot_fingerprint.as_str(),
        ),
        ("model_id", evidence.model_id.as_str()),
        ("evaluator.name", evidence.evaluator.name.as_str()),
        ("evaluator.version", evidence.evaluator.version.as_str()),
    ] {
        require_nonblank(field, value)?;
    }
    validate_economics("provider_cost_usd", evidence.provider_cost_usd)?;
    validate_economics("customer_charge_usd", evidence.customer_charge_usd)?;
    validate_observation("delivery", &evidence.delivery)?;
    validate_observation("contract_validity", &evidence.contract_validity)?;
    validate_observation("semantic", &evidence.semantic)?;
    validate_observation("r2", &evidence.r2)?;
    validate_observation("trajectory", &evidence.trajectory)?;

    match &evidence.collection_mode {
        CollectionMode::SelectedModelOnline => {}
        CollectionMode::ControlledAllModel { group_id }
        | CollectionMode::ControlledAllModelBudget { group_id } => {
            require_nonblank("collection_mode.group_id", group_id)?;
        }
        CollectionMode::ExternalObservation {
            source,
            source_version,
        } => {
            require_nonblank("collection_mode.source", source)?;
            require_nonblank("collection_mode.source_version", source_version)?;
        }
    }
    if let Observation::Observed(delivery) = &evidence.delivery {
        require_nonblank("delivery.terminal_outcome", &delivery.terminal_outcome)?;
    }
    if let Observation::Observed(semantic) = &evidence.semantic {
        validate_probability("semantic.confidence", semantic.confidence)?;
    }
    let mut skill_indices = BTreeSet::new();
    for skill in &evidence.skills {
        if !skill_indices.insert(skill.skill_index) {
            return Err(EvidenceError::DuplicateSkillIndex {
                index: skill.skill_index,
            });
        }
        require_nonblank("skills.skill_name", &skill.skill_name)?;
        require_nonblank("skills.critic_version", &skill.critic_version)?;
        require_nonblank("skills.taxonomy_version", &skill.taxonomy_version)?;
    }
    if let Observation::Observed(r2) = &evidence.r2 {
        require_nonblank("r2.label_version", &r2.label_version)?;
    }
    if let Observation::Observed(trajectory) = &evidence.trajectory {
        require_nonblank(
            "trajectory.trajectory_version",
            &trajectory.trajectory_version,
        )?;
        if !trajectory.reward.is_finite() {
            return Err(EvidenceError::NonFinite {
                field: "trajectory.reward",
            });
        }
        if trajectory.step_count == 0 {
            return Err(EvidenceError::EmptyTrajectory);
        }
    }
    Ok(())
}

fn canonicalize_content<T: Serialize>(
    content: &T,
    supplied_id: Option<&str>,
) -> Result<CanonicalEvidence, EvidenceError> {
    let value = serde_json::to_value(content).map_err(|error| EvidenceError::Serialization {
        reason: error.to_string(),
    })?;
    let canonical_payload = serde_json::to_vec(&canonicalize_json(value)).map_err(|error| {
        EvidenceError::Serialization {
            reason: error.to_string(),
        }
    })?;
    let content_hash = format!("{:x}", Sha256::digest(&canonical_payload));
    if let Some(supplied) = supplied_id {
        if supplied != content_hash {
            return Err(EvidenceError::EvidenceIdMismatch {
                supplied: supplied.to_string(),
                expected: content_hash,
            });
        }
    }
    Ok(CanonicalEvidence {
        evidence_id: content_hash.clone(),
        content_hash,
        canonical_payload,
    })
}

fn validate_economics(field: &'static str, value: f64) -> Result<(), EvidenceError> {
    if !value.is_finite() {
        return Err(EvidenceError::NonFinite { field });
    }
    if value < 0.0 {
        return Err(EvidenceError::Negative { field });
    }
    Ok(())
}

fn validate_probability(field: &'static str, value: f64) -> Result<(), EvidenceError> {
    if !value.is_finite() {
        return Err(EvidenceError::NonFinite { field });
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(EvidenceError::OutOfRange { field });
    }
    Ok(())
}

fn validate_observation<T>(
    axis: &'static str,
    observation: &Observation<T>,
) -> Result<(), EvidenceError> {
    match observation {
        Observation::Observed(_) => Ok(()),
        Observation::Unobserved { reason } => require_nonblank(axis, reason),
        Observation::Abstained {
            reason,
            uncertainty,
        } => {
            require_nonblank(axis, reason)?;
            validate_probability(axis, *uncertainty)
        }
    }
}

fn require_nonblank(field: &'static str, value: &str) -> Result<(), EvidenceError> {
    if value.trim().is_empty() {
        Err(EvidenceError::BlankIdentifier { field })
    } else {
        Ok(())
    }
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), EvidenceError> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(EvidenceError::InvalidFingerprint { field })
    }
}

fn canonicalize_json(value: Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.into_iter().map(canonicalize_json).collect()),
        Value::Object(values) => {
            let mut entries: Vec<_> = values.into_iter().collect();
            entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonicalize_json(value)))
                    .collect(),
            )
        }
        scalar => scalar,
    }
}
