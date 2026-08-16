//! Pure, versioned contracts for R2-aligned route-budget actions.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::predictors::{PredictionEstimate, PredictionFeatureVector, RouteIdentity};

pub const R2_STATE_SCHEMA_VERSION: &str = "routing-r2-state/v1";
pub const R2_PREDICTOR_VERSION: &str = "meridian-r2-anchor-head/v1";
pub const R2_INSTRUCTION_VERSION: &str = "meridian-output-budget/v1";
/// Runtime feature contract: complexity, ln(1 + input tokens), ln(1 + output ceiling).
pub const R2_RUNTIME_FEATURE_VERSION: &str = "routing-features/v2";
const R2_RUNTIME_FEATURE_NAMES: [&str; 3] = [
    "complexity",
    "ln_1p_estimated_input_tokens",
    "ln_1p_output_token_ceiling",
];
/// Complete route-budget outcome label accepted by the P4 runtime.
pub const R2_LABEL_VERSION: &str = "routing-r2-label/v1";

/// Byte-exact output-budget instruction and its frozen input accounting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct R2OutputBudgetConstraint {
    /// Version defining the instruction bytes and accounting rules.
    pub instruction_version: &'static str,
    /// Byte-exact instruction sent to the provider.
    pub instruction: String,
    /// Deterministic expected input-token overhead used in action ranking.
    pub estimated_input_tokens: u32,
    /// Conservative input-token overhead used for provider-liability reservation.
    pub input_token_upper_bound: u32,
}

impl R2OutputBudgetConstraint {
    /// Constructs the single versioned instruction contract for one positive budget.
    pub fn new(output_budget: u32) -> Result<Self, R2Error> {
        if output_budget == 0 {
            return Err(R2Error::ZeroOutputBudget);
        }

        let instruction = format!(
            "Complete the requested task in no more than {output_budget} output tokens.\n\
             Prioritize correctness and a complete answer within that limit."
        );
        let input_token_upper_bound =
            u32::try_from(instruction.len()).map_err(|_| R2Error::OutOfRange {
                field: "output_budget_instruction",
            })?;

        Ok(Self {
            instruction_version: R2_INSTRUCTION_VERSION,
            estimated_input_tokens: crate::estimate_tokens(&instruction),
            input_token_upper_bound,
            instruction,
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum R2Error {
    #[error("R2 output budget must be positive")]
    ZeroOutputBudget,
    #[error("R2 output-budget anchor set is empty")]
    EmptyOutputBudgets,
    #[error("R2 output-budget anchor is duplicated: {output_budget}")]
    DuplicateOutputBudget { output_budget: u32 },
    #[error("R2 output-budget anchors are not in canonical ascending order")]
    NoncanonicalOutputBudgets,
    #[error("R2 provenance field is blank: {field}")]
    BlankProvenanceField { field: &'static str },
    #[error("R2 learner-state identity is not a canonical SHA-256 value")]
    InvalidLearnerStateId,
    #[error("R2 durable record-state identity is not a canonical SHA-256 value")]
    InvalidRecordStateId,
    #[error("R2 training-content hash is not a canonical SHA-256 value")]
    InvalidTrainingContentHash,
    #[error("R2 collection is empty: {field}")]
    Empty { field: &'static str },
    #[error("R2 value is non-finite: {field}")]
    NonFinite { field: &'static str },
    #[error("R2 value is out of range: {field}")]
    OutOfRange { field: &'static str },
    #[error("R2 dimensions do not match: {field}")]
    DimensionMismatch { field: &'static str },
    #[error("R2 state contains a duplicate action at budget {output_budget}")]
    DuplicateAction { output_budget: u32 },
    #[error("R2 actions are not in canonical order")]
    NoncanonicalActions,
    #[error("R2 versions are incompatible: {field}")]
    VersionMismatch { field: &'static str },
    #[error("R2 learner-state identity does not match canonical state content")]
    LearnerStateIdMismatch,
    #[error("R2 prediction construction failed: {reason}")]
    Prediction { reason: String },
    #[error("R2 content serialization failed: {reason}")]
    Serialization { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct R2ActionIdentity {
    route: RouteIdentity,
    output_budget: u32,
}

impl R2ActionIdentity {
    pub fn new(route: RouteIdentity, output_budget: u32) -> Result<Self, R2Error> {
        if output_budget == 0 {
            return Err(R2Error::ZeroOutputBudget);
        }
        Ok(Self {
            route,
            output_budget,
        })
    }

    pub fn route(&self) -> &RouteIdentity {
        &self.route
    }

    pub const fn output_budget(&self) -> u32 {
        self.output_budget
    }

    pub fn validate(&self) -> Result<(), R2Error> {
        Self::new(self.route.clone(), self.output_budget).map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct R2BudgetAnchors {
    values: Vec<u32>,
}

impl R2BudgetAnchors {
    pub fn new(values: Vec<u32>) -> Result<Self, R2Error> {
        if values.is_empty() {
            return Err(R2Error::EmptyOutputBudgets);
        }
        if values.contains(&0) {
            return Err(R2Error::ZeroOutputBudget);
        }

        let mut unique = BTreeSet::new();
        for output_budget in &values {
            if !unique.insert(*output_budget) {
                return Err(R2Error::DuplicateOutputBudget {
                    output_budget: *output_budget,
                });
            }
        }
        if values.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(R2Error::NoncanonicalOutputBudgets);
        }

        Ok(Self { values })
    }

    pub fn values(&self) -> &[u32] {
        &self.values
    }

    pub fn validate(&self) -> Result<(), R2Error> {
        Self::new(self.values.clone()).map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct R2Provenance {
    pub predictor_version: String,
    pub encoder_version: String,
    pub feature_version: String,
    pub evaluator_version: String,
    pub corpus_version: String,
    pub catalog_version: String,
    pub price_version: String,
    pub instruction_version: String,
    pub label_version: String,
    pub learner_state_id: String,
    pub training_content_hash: String,
}

impl R2Provenance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        predictor_version: impl Into<String>,
        encoder_version: impl Into<String>,
        feature_version: impl Into<String>,
        evaluator_version: impl Into<String>,
        corpus_version: impl Into<String>,
        catalog_version: impl Into<String>,
        price_version: impl Into<String>,
        instruction_version: impl Into<String>,
        label_version: impl Into<String>,
        learner_state_id: impl Into<String>,
        training_content_hash: impl Into<String>,
    ) -> Result<Self, R2Error> {
        let provenance = Self {
            predictor_version: predictor_version.into(),
            encoder_version: encoder_version.into(),
            feature_version: feature_version.into(),
            evaluator_version: evaluator_version.into(),
            corpus_version: corpus_version.into(),
            catalog_version: catalog_version.into(),
            price_version: price_version.into(),
            instruction_version: instruction_version.into(),
            label_version: label_version.into(),
            learner_state_id: learner_state_id.into(),
            training_content_hash: training_content_hash.into(),
        };
        provenance.validate()?;
        Ok(provenance)
    }

    pub fn validate(&self) -> Result<(), R2Error> {
        for (field, value) in [
            ("predictor_version", self.predictor_version.as_str()),
            ("encoder_version", self.encoder_version.as_str()),
            ("feature_version", self.feature_version.as_str()),
            ("evaluator_version", self.evaluator_version.as_str()),
            ("corpus_version", self.corpus_version.as_str()),
            ("catalog_version", self.catalog_version.as_str()),
            ("price_version", self.price_version.as_str()),
            ("instruction_version", self.instruction_version.as_str()),
            ("label_version", self.label_version.as_str()),
            ("learner_state_id", self.learner_state_id.as_str()),
            ("training_content_hash", self.training_content_hash.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(R2Error::BlankProvenanceField { field });
            }
        }
        if !is_canonical_sha256(&self.learner_state_id) {
            return Err(R2Error::InvalidLearnerStateId);
        }
        if !is_canonical_sha256(&self.training_content_hash) {
            return Err(R2Error::InvalidTrainingContentHash);
        }
        Ok(())
    }

    pub fn content_id(&self) -> Result<String, R2Error> {
        self.validate()?;
        let payload = serde_json::to_vec(self).map_err(|error| R2Error::Serialization {
            reason: error.to_string(),
        })?;
        Ok(format!("{:x}", Sha256::digest(payload)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum R2FallbackReason {
    NoActiveState,
    RepositoryUnavailable,
    InvalidState,
    FeatureVersionMismatch,
    EvaluatorVersionMismatch,
    CatalogVersionMismatch,
    PriceVersionMismatch,
    InstructionVersionMismatch,
    LabelVersionMismatch,
    ProductionPromotionBlocked,
    ModelEvidenceMissing,
    NoAllowedAnchor,
    InsufficientSupport,
    UncalibratedHead,
    OutOfDistribution,
}

impl R2FallbackReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoActiveState => "no_active_state",
            Self::RepositoryUnavailable => "repository_unavailable",
            Self::InvalidState => "invalid_state",
            Self::FeatureVersionMismatch => "feature_version_mismatch",
            Self::EvaluatorVersionMismatch => "evaluator_version_mismatch",
            Self::CatalogVersionMismatch => "catalog_version_mismatch",
            Self::PriceVersionMismatch => "price_version_mismatch",
            Self::InstructionVersionMismatch => "instruction_version_mismatch",
            Self::LabelVersionMismatch => "label_version_mismatch",
            Self::ProductionPromotionBlocked => "production_promotion_blocked",
            Self::ModelEvidenceMissing => "model_evidence_missing",
            Self::NoAllowedAnchor => "no_allowed_anchor",
            Self::InsufficientSupport => "insufficient_support",
            Self::UncalibratedHead => "uncalibrated_head",
            Self::OutOfDistribution => "out_of_distribution",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2SharedEncoder {
    feature_means: Vec<f64>,
    feature_scales: Vec<f64>,
    weights: Vec<Vec<f64>>,
    biases: Vec<f64>,
}

impl R2SharedEncoder {
    pub fn new(
        feature_means: Vec<f64>,
        feature_scales: Vec<f64>,
        weights: Vec<Vec<f64>>,
        biases: Vec<f64>,
    ) -> Result<Self, R2Error> {
        let encoder = Self {
            feature_means,
            feature_scales,
            weights,
            biases,
        };
        encoder.validate()?;
        Ok(encoder)
    }

    pub fn validate(&self) -> Result<(), R2Error> {
        if self.feature_means.is_empty() {
            return Err(R2Error::Empty {
                field: "encoder.feature_means",
            });
        }
        if self.weights.is_empty() {
            return Err(R2Error::Empty {
                field: "encoder.weights",
            });
        }
        if self.feature_scales.len() != self.feature_means.len() {
            return Err(R2Error::DimensionMismatch {
                field: "encoder.feature_scales",
            });
        }
        require_finite("encoder.feature_means", &self.feature_means)?;
        require_finite("encoder.feature_scales", &self.feature_scales)?;
        if self.feature_scales.iter().any(|scale| *scale <= 0.0) {
            return Err(R2Error::OutOfRange {
                field: "encoder.feature_scales",
            });
        }
        if self.biases.len() != self.weights.len() {
            return Err(R2Error::DimensionMismatch {
                field: "encoder.biases",
            });
        }
        require_finite("encoder.biases", &self.biases)?;
        for row in &self.weights {
            if row.len() != self.feature_means.len() {
                return Err(R2Error::DimensionMismatch {
                    field: "encoder.weights",
                });
            }
            require_finite("encoder.weights", row)?;
        }
        Ok(())
    }

    pub fn feature_dimensions(&self) -> usize {
        self.feature_means.len()
    }

    pub fn hidden_dimensions(&self) -> usize {
        self.biases.len()
    }

    fn encode(&self, features: &[f64]) -> Result<Vec<f64>, R2Error> {
        if features.len() != self.feature_dimensions() {
            return Err(R2Error::DimensionMismatch {
                field: "prediction.features",
            });
        }
        require_finite("prediction.features", features)?;
        let standardized = features
            .iter()
            .zip(&self.feature_means)
            .zip(&self.feature_scales)
            .map(|((value, mean), scale)| (value - mean) / scale)
            .collect::<Vec<_>>();
        Ok(self
            .weights
            .iter()
            .zip(&self.biases)
            .map(|(weights, bias)| (bias + dot(weights, &standardized)).tanh())
            .collect())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2AnchorHead {
    pub action: R2ActionIdentity,
    pub quality_weights: Vec<f64>,
    pub quality_bias: f64,
    pub output_weights: Vec<f64>,
    pub output_bias: f64,
    pub instruction_input_tokens: u32,
    pub instruction_input_upper_bound: u32,
    pub support: u32,
    pub quality_residual: f64,
    pub output_residual: f64,
}

impl R2AnchorHead {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action: R2ActionIdentity,
        quality_weights: Vec<f64>,
        quality_bias: f64,
        output_weights: Vec<f64>,
        output_bias: f64,
        instruction_input_tokens: u32,
        instruction_input_upper_bound: u32,
        support: u32,
        quality_residual: f64,
        output_residual: f64,
    ) -> Result<Self, R2Error> {
        let head = Self {
            action,
            quality_weights,
            quality_bias,
            output_weights,
            output_bias,
            instruction_input_tokens,
            instruction_input_upper_bound,
            support,
            quality_residual,
            output_residual,
        };
        head.validate()?;
        Ok(head)
    }

    pub fn validate(&self) -> Result<(), R2Error> {
        self.action.validate()?;
        if self.quality_weights.is_empty() {
            return Err(R2Error::Empty {
                field: "head.quality_weights",
            });
        }
        if self.output_weights.is_empty() {
            return Err(R2Error::Empty {
                field: "head.output_weights",
            });
        }
        require_finite("head.quality_weights", &self.quality_weights)?;
        require_finite("head.output_weights", &self.output_weights)?;
        require_finite("head.quality_bias", &[self.quality_bias])?;
        require_finite("head.output_bias", &[self.output_bias])?;
        require_nonnegative("head.quality_residual", self.quality_residual)?;
        require_nonnegative("head.output_residual", self.output_residual)?;
        if self.instruction_input_upper_bound == 0
            || self.instruction_input_tokens > self.instruction_input_upper_bound
        {
            return Err(R2Error::OutOfRange {
                field: "head.instruction_input_tokens",
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2EstimatorPolicy {
    pub minimum_support: u32,
    pub maximum_feature_distance: f64,
    pub maximum_quality_residual: f64,
    pub maximum_output_residual: f64,
}

impl R2EstimatorPolicy {
    pub fn new(
        minimum_support: u32,
        maximum_feature_distance: f64,
        maximum_quality_residual: f64,
        maximum_output_residual: f64,
    ) -> Result<Self, R2Error> {
        let policy = Self {
            minimum_support,
            maximum_feature_distance,
            maximum_quality_residual,
            maximum_output_residual,
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> Result<(), R2Error> {
        if self.minimum_support == 0 {
            return Err(R2Error::OutOfRange {
                field: "policy.minimum_support",
            });
        }
        require_nonnegative(
            "policy.maximum_feature_distance",
            self.maximum_feature_distance,
        )?;
        require_nonnegative(
            "policy.maximum_quality_residual",
            self.maximum_quality_residual,
        )?;
        require_nonnegative(
            "policy.maximum_output_residual",
            self.maximum_output_residual,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2ActionPrediction {
    pub action: R2ActionIdentity,
    pub semantic_correctness: PredictionEstimate,
    pub expected_output_tokens: PredictionEstimate,
    pub instruction_input_tokens: u32,
    pub instruction_input_upper_bound: u32,
}

/// Expected and conservative input-token overhead for the caller-ceiling instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct R2InstructionInputEstimate {
    expected_tokens: u32,
    upper_bound: u32,
}

impl R2InstructionInputEstimate {
    /// Creates a validated instruction estimate whose upper bound is positive.
    pub fn new(expected_tokens: u32, upper_bound: u32) -> Result<Self, R2Error> {
        let estimate = Self {
            expected_tokens,
            upper_bound,
        };
        estimate.validate()?;
        Ok(estimate)
    }

    /// Returns the expected input-token overhead used in action ranking.
    pub const fn expected_tokens(self) -> u32 {
        self.expected_tokens
    }

    /// Returns the conservative input-token overhead used in reservation.
    pub const fn upper_bound(self) -> u32 {
        self.upper_bound
    }

    fn validate(self) -> Result<(), R2Error> {
        if self.upper_bound == 0 || self.expected_tokens > self.upper_bound {
            return Err(R2Error::OutOfRange {
                field: "instruction_input_estimate",
            });
        }
        Ok(())
    }
}

/// Typed explanation for an existing R2 request- or action-support abstention.
///
/// Diagnostics are frozen evidence only. They do not participate in candidate
/// eligibility, prediction, risk comparison, or fallback selection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "diagnostic", rename_all = "snake_case")]
pub enum R2SupportDiagnostic {
    /// One request feature exceeded the permitted distance below its training envelope.
    RequestFeatureBelowMin {
        /// Stable zero-based position in the frozen feature vector.
        feature_index: u32,
        /// Stable name defined by the frozen runtime feature contract.
        feature: String,
        /// Request feature value presented to the learner.
        observed: f64,
        /// Minimum value in the learner state's frozen envelope.
        bound: f64,
        /// Distance below the bound divided by the frozen encoder scale.
        scaled_distance: f64,
        /// Maximum scaled distance permitted by the learner policy.
        limit: f64,
    },
    /// One request feature exceeded the permitted distance above its training envelope.
    RequestFeatureAboveMax {
        /// Stable zero-based position in the frozen feature vector.
        feature_index: u32,
        /// Stable name defined by the frozen runtime feature contract.
        feature: String,
        /// Request feature value presented to the learner.
        observed: f64,
        /// Maximum value in the learner state's frozen envelope.
        bound: f64,
        /// Distance above the bound divided by the frozen encoder scale.
        scaled_distance: f64,
        /// Maximum scaled distance permitted by the learner policy.
        limit: f64,
    },
    /// The route-budget head has fewer observations than the policy requires.
    ActionSupportBelowMin {
        /// Frozen support count for the route-budget head.
        observed: u32,
        /// Minimum support count required by the learner policy.
        required: u32,
    },
    /// The route-budget head's semantic-quality residual exceeds policy.
    QualityResidualAboveMax {
        /// Frozen semantic-quality residual for the head.
        observed: f64,
        /// Maximum semantic-quality residual permitted by policy.
        maximum: f64,
    },
    /// The route-budget head's output-token residual exceeds policy.
    OutputResidualAboveMax {
        /// Frozen output-token residual for the head.
        observed: f64,
        /// Maximum output-token residual permitted by policy.
        maximum: f64,
    },
}

impl R2SupportDiagnostic {
    fn validate(&self, reason: R2FallbackReason) -> bool {
        match self {
            Self::RequestFeatureBelowMin {
                feature_index,
                feature,
                observed,
                bound,
                scaled_distance,
                limit,
            } => {
                reason == R2FallbackReason::OutOfDistribution
                    && feature == &runtime_feature_name(*feature_index)
                    && finite_nonnegative(*scaled_distance)
                    && finite_nonnegative(*limit)
                    && observed.is_finite()
                    && bound.is_finite()
                    && observed < bound
                    && scaled_distance > limit
            }
            Self::RequestFeatureAboveMax {
                feature_index,
                feature,
                observed,
                bound,
                scaled_distance,
                limit,
            } => {
                reason == R2FallbackReason::OutOfDistribution
                    && feature == &runtime_feature_name(*feature_index)
                    && finite_nonnegative(*scaled_distance)
                    && finite_nonnegative(*limit)
                    && observed.is_finite()
                    && bound.is_finite()
                    && observed > bound
                    && scaled_distance > limit
            }
            Self::ActionSupportBelowMin { observed, required } => {
                reason == R2FallbackReason::InsufficientSupport && observed < required
            }
            Self::QualityResidualAboveMax { observed, maximum }
            | Self::OutputResidualAboveMax { observed, maximum } => {
                reason == R2FallbackReason::UncalibratedHead
                    && finite_nonnegative(*observed)
                    && finite_nonnegative(*maximum)
                    && observed > maximum
            }
        }
    }

    fn order_key(&self) -> (u8, u32) {
        match self {
            Self::RequestFeatureBelowMin { feature_index, .. }
            | Self::RequestFeatureAboveMax { feature_index, .. } => (0, *feature_index),
            Self::ActionSupportBelowMin { .. } => (1, 0),
            Self::QualityResidualAboveMax { .. } => (2, 0),
            Self::OutputResidualAboveMax { .. } => (3, 0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2ActionAbstention {
    /// Provider-model-budget action whose learner head abstained.
    pub action: R2ActionIdentity,
    /// Backward-compatible broad fallback category.
    pub reason: R2FallbackReason,
    /// Frozen uncertainty associated with the abstention.
    pub uncertainty: f64,
    /// Ordered explanation of the existing support decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<R2SupportDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "disposition", content = "evidence", rename_all = "snake_case")]
pub enum R2HeadPrediction {
    Estimated(R2ActionPrediction),
    Abstained(R2ActionAbstention),
}

impl R2HeadPrediction {
    pub fn action(&self) -> &R2ActionIdentity {
        match self {
            Self::Estimated(prediction) => &prediction.action,
            Self::Abstained(abstention) => &abstention.action,
        }
    }
}

/// Request-scoped R2 evidence frozen before deterministic ballot selection.
///
/// Inactive, unavailable, invalid, or incompatible evidence leaves the accepted P3 ballot
/// byte-identical. Only validated [`FrozenR2Evidence::Active`] evidence may expand a surviving
/// provider-model route into tested output-budget actions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FrozenR2Evidence {
    /// No R2 state was authorized for this request.
    Inactive,
    /// A typed runtime failure prevented R2 evaluation, so selection must use P3 behavior.
    Unavailable {
        /// Stable reason that R2 evidence could not be frozen.
        reason: R2FallbackReason,
    },
    /// Canonical, validated predictions bound to one learner-state provenance record.
    Active {
        /// Durable repository-record identity authorized by the runtime boundary, when retained.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        record_state_id: Option<String>,
        /// Versions and content identities that authorize these predictions.
        provenance: R2Provenance,
        /// Instruction overhead for the always-retained caller-ceiling predecessor action.
        predecessor_instruction: R2InstructionInputEstimate,
        /// Canonically ordered route-budget estimates or abstentions.
        predictions: Vec<R2HeadPrediction>,
    },
}

impl FrozenR2Evidence {
    /// Builds validated active evidence and canonicalizes predictions by action identity.
    ///
    /// Returns an error when provenance, estimates, abstentions, ordering, or action uniqueness
    /// violates the frozen-evidence contract.
    pub fn active(
        provenance: R2Provenance,
        predecessor_instruction: R2InstructionInputEstimate,
        predictions: Vec<R2HeadPrediction>,
    ) -> Result<Self, R2Error> {
        Self::build_active(None, provenance, predecessor_instruction, predictions)
    }

    /// Builds active evidence with a separately verified durable repository-record identity.
    ///
    /// This preserves audit provenance only. The caller remains responsible for proving that the
    /// canonical record bytes hash to this identity and that an opaque runtime grant authorizes it.
    pub fn active_with_record_state_id(
        record_state_id: impl Into<String>,
        provenance: R2Provenance,
        predecessor_instruction: R2InstructionInputEstimate,
        predictions: Vec<R2HeadPrediction>,
    ) -> Result<Self, R2Error> {
        let record_state_id = record_state_id.into();
        if !is_canonical_sha256(&record_state_id) {
            return Err(R2Error::InvalidRecordStateId);
        }
        Self::build_active(
            Some(record_state_id),
            provenance,
            predecessor_instruction,
            predictions,
        )
    }

    fn build_active(
        record_state_id: Option<String>,
        provenance: R2Provenance,
        predecessor_instruction: R2InstructionInputEstimate,
        mut predictions: Vec<R2HeadPrediction>,
    ) -> Result<Self, R2Error> {
        predictions.sort_by(|left, right| left.action().cmp(right.action()));
        let evidence = Self::Active {
            record_state_id,
            provenance,
            predecessor_instruction,
            predictions,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    /// Records a typed runtime failure that must preserve predecessor P3 selection.
    pub const fn unavailable(reason: R2FallbackReason) -> Self {
        Self::Unavailable { reason }
    }

    /// Returns whether the evidence is absent and should be omitted from historical snapshots.
    pub const fn is_inactive(&self) -> bool {
        matches!(self, Self::Inactive)
    }

    /// Revalidates frozen evidence at the production selection boundary.
    ///
    /// This prevents deserialization from bypassing constructor validation.
    pub fn validate(&self) -> Result<(), R2Error> {
        match self {
            Self::Inactive => Ok(()),
            Self::Unavailable { reason } => {
                if *reason == R2FallbackReason::NoActiveState {
                    Err(R2Error::OutOfRange {
                        field: "frozen.unavailable_reason",
                    })
                } else {
                    Ok(())
                }
            }
            Self::Active {
                record_state_id,
                provenance,
                predecessor_instruction,
                predictions,
            } => {
                if record_state_id
                    .as_deref()
                    .is_some_and(|state_id| !is_canonical_sha256(state_id))
                {
                    return Err(R2Error::InvalidRecordStateId);
                }
                provenance.validate()?;
                predecessor_instruction.validate()?;
                if provenance.predictor_version != R2_PREDICTOR_VERSION {
                    return Err(R2Error::VersionMismatch {
                        field: "frozen.predictor_version",
                    });
                }
                if provenance.instruction_version != R2_INSTRUCTION_VERSION {
                    return Err(R2Error::VersionMismatch {
                        field: "frozen.instruction_version",
                    });
                }
                if predictions.is_empty() {
                    return Err(R2Error::Empty {
                        field: "frozen.predictions",
                    });
                }
                for prediction in predictions {
                    validate_head_prediction(prediction)?;
                }
                for pair in predictions.windows(2) {
                    if pair[0].action() == pair[1].action() {
                        return Err(R2Error::DuplicateAction {
                            output_budget: pair[0].action().output_budget(),
                        });
                    }
                    if pair[0].action() > pair[1].action() {
                        return Err(R2Error::NoncanonicalActions);
                    }
                }
                Ok(())
            }
        }
    }

    /// Returns whether active evidence matches every runtime and frozen decision authority.
    pub fn is_compatible_with(
        &self,
        catalog_version: &str,
        price_version: &str,
        evaluator_version: &str,
    ) -> bool {
        match self {
            Self::Active { provenance, .. } => {
                provenance.catalog_version == catalog_version
                    && provenance.price_version == price_version
                    && provenance.evaluator_version == evaluator_version
                    && provenance.instruction_version == R2_INSTRUCTION_VERSION
                    && provenance.feature_version == R2_RUNTIME_FEATURE_VERSION
                    && provenance.label_version == R2_LABEL_VERSION
            }
            Self::Inactive | Self::Unavailable { .. } => false,
        }
    }

    /// Returns the caller-ceiling instruction overhead only for active evidence.
    pub const fn predecessor_instruction(&self) -> Option<R2InstructionInputEstimate> {
        match self {
            Self::Active {
                predecessor_instruction,
                ..
            } => Some(*predecessor_instruction),
            Self::Inactive | Self::Unavailable { .. } => None,
        }
    }

    /// Returns provenance only when the evidence contains an active learner state.
    pub fn provenance(&self) -> Option<&R2Provenance> {
        match self {
            Self::Active { provenance, .. } => Some(provenance),
            Self::Inactive | Self::Unavailable { .. } => None,
        }
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

    /// Returns predictions for one P3-surviving route at tested budgets within the caller ceiling.
    ///
    /// This method never clamps an anchor or synthesizes the predecessor caller-ceiling action;
    /// the selector retains that action independently.
    pub fn predictions_for<'a>(
        &'a self,
        route: &RouteIdentity,
        caller_ceiling: u32,
    ) -> Vec<&'a R2HeadPrediction> {
        match self {
            Self::Active { predictions, .. } => predictions
                .iter()
                .filter(|prediction| {
                    prediction.action().route() == route
                        && prediction.action().output_budget() <= caller_ceiling
                })
                .collect(),
            Self::Inactive | Self::Unavailable { .. } => Vec::new(),
        }
    }

    pub(super) fn route_fallback_reason(
        &self,
        route: &RouteIdentity,
        caller_ceiling: u32,
    ) -> Option<R2FallbackReason> {
        let Self::Active { predictions, .. } = self else {
            return None;
        };
        let mut route_predictions = predictions
            .iter()
            .filter(|prediction| prediction.action().route() == route)
            .peekable();
        if route_predictions.peek().is_none() {
            return Some(R2FallbackReason::ModelEvidenceMissing);
        }
        if route_predictions.all(|prediction| prediction.action().output_budget() > caller_ceiling)
        {
            return Some(R2FallbackReason::NoAllowedAnchor);
        }
        None
    }
}

impl Default for FrozenR2Evidence {
    fn default() -> Self {
        Self::Inactive
    }
}

/// Evidence attached to the one executable action retained for a ballot route.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum R2ActionDisposition {
    /// The route kept its accepted P3 caller-ceiling action.
    #[default]
    Predecessor,
    /// R2 selected a tested output-budget action using frozen predictions.
    Estimated {
        /// Content identity of the learner state that produced the prediction.
        learner_state_id: String,
        /// Version of the budget-conditioned predictor.
        predictor_version: String,
        /// Version of the output-budget instruction whose input overhead was predicted.
        instruction_version: String,
        /// Frozen semantic-correctness estimate for this action.
        semantic_correctness: PredictionEstimate,
        /// Frozen expected output-token estimate for this action.
        expected_output_tokens: PredictionEstimate,
        /// Expected input-token overhead of the versioned output instruction.
        instruction_input_tokens: u32,
        /// Conservative input-token overhead used for provider-liability reservation.
        instruction_input_upper_bound: u32,
    },
}

impl R2ActionDisposition {
    /// Returns whether this entry is the historical P3 action and may omit R2 serialization.
    pub const fn is_predecessor(&self) -> bool {
        matches!(self, Self::Predecessor)
    }

    /// Freezes the provenance and estimates that justify one selected R2 action.
    pub fn estimated(provenance: &R2Provenance, prediction: &R2ActionPrediction) -> Self {
        Self::Estimated {
            learner_state_id: provenance.learner_state_id.clone(),
            predictor_version: provenance.predictor_version.clone(),
            instruction_version: provenance.instruction_version.clone(),
            semantic_correctness: prediction.semantic_correctness.clone(),
            expected_output_tokens: prediction.expected_output_tokens.clone(),
            instruction_input_tokens: prediction.instruction_input_tokens,
            instruction_input_upper_bound: prediction.instruction_input_upper_bound,
        }
    }
}

/// Outcome of evaluating one allowed route-budget action.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum R2EvaluationDisposition {
    /// The action is the always-retained caller-ceiling P3 predecessor.
    #[default]
    Predecessor,
    /// The active learner state produced usable predictions for the action.
    Estimated,
    /// The learner state did not provide a usable estimate for the action.
    Abstained {
        /// Stable reason that the action could not be estimated.
        reason: R2FallbackReason,
        /// Frozen uncertainty value that triggered or accompanies abstention.
        uncertainty: f64,
        /// Ordered explanation of the request or action support boundary.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        diagnostics: Vec<R2SupportDiagnostic>,
    },
}

impl R2EvaluationDisposition {
    /// Returns whether the evaluated action is the P3 predecessor.
    pub const fn is_predecessor(&self) -> bool {
        matches!(self, Self::Predecessor)
    }
}

/// Replayable evidence for an allowed action considered during per-route reduction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2EvaluatedAction {
    /// Stable provider-model-output-budget identity.
    pub action: R2ActionIdentity,
    /// Whether the action used predecessor values, predictions, or abstained.
    pub disposition: R2EvaluationDisposition,
    /// Transport-adjusted semantic correctness when the action was scored.
    pub delivered_correctness_probability: Option<f64>,
    /// Predicted provider cost when the action was scored.
    pub expected_provider_cost: Option<f64>,
    /// Conservative provider-liability ceiling when the action was scored.
    pub provider_cost_upper_bound: Option<f64>,
    /// Expected cost normalized with the frozen predecessor bounds.
    pub normalized_cost: Option<f64>,
    /// Canonical delivered-correctness-and-cost risk when the action was scored.
    pub risk: Option<f64>,
    /// Whether this action won the per-route reduction and entered the executable ballot.
    pub selected: bool,
}

/// Ballot-level record of whether joint model-output-budget selection changed P3 behavior.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum R2Decision {
    /// R2 was not applied; serialization and content identity remain at the P3 contract.
    #[default]
    Predecessor,
    /// Valid compatible evidence was applied to at least one tested action.
    Applied {
        /// Content identity of the learner state used for the ballot.
        learner_state_id: String,
        /// Version of the budget-conditioned predictor used for the ballot.
        predictor_version: String,
        /// Version of the output-budget instruction assumed by action economics.
        instruction_version: String,
        /// Canonically ordered evidence for every allowed action considered.
        evaluated_actions: Vec<R2EvaluatedAction>,
    },
}

impl R2Decision {
    /// Returns whether R2 was not applied and the field should be omitted from serialization.
    pub const fn is_predecessor(&self) -> bool {
        matches!(self, Self::Predecessor)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct R2LearnerState {
    schema_version: String,
    provenance: R2Provenance,
    encoder: R2SharedEncoder,
    heads: Vec<R2AnchorHead>,
    policy: R2EstimatorPolicy,
    feature_minima: Vec<f64>,
    feature_maxima: Vec<f64>,
}

impl R2LearnerState {
    pub fn new(
        mut provenance: R2Provenance,
        encoder: R2SharedEncoder,
        heads: Vec<R2AnchorHead>,
        policy: R2EstimatorPolicy,
        feature_minima: Vec<f64>,
        feature_maxima: Vec<f64>,
    ) -> Result<Self, R2Error> {
        provenance.learner_state_id = "0".repeat(64);
        let mut state = Self {
            schema_version: R2_STATE_SCHEMA_VERSION.to_owned(),
            provenance,
            encoder,
            heads,
            policy,
            feature_minima,
            feature_maxima,
        };
        state.validate_structure()?;
        state.provenance.learner_state_id = state.compute_content_id()?;
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), R2Error> {
        self.validate_structure()?;
        if self.compute_content_id()? != self.provenance.learner_state_id {
            return Err(R2Error::LearnerStateIdMismatch);
        }
        Ok(())
    }

    pub fn provenance(&self) -> &R2Provenance {
        &self.provenance
    }

    pub fn encoder(&self) -> &R2SharedEncoder {
        &self.encoder
    }

    pub fn heads(&self) -> &[R2AnchorHead] {
        &self.heads
    }

    pub fn policy(&self) -> &R2EstimatorPolicy {
        &self.policy
    }

    pub fn content_id(&self) -> Result<String, R2Error> {
        self.validate_structure()?;
        self.compute_content_id()
    }

    pub fn predict(
        &self,
        features: &PredictionFeatureVector,
        caller_ceiling: u32,
    ) -> Result<Vec<R2HeadPrediction>, R2Error> {
        self.validate()?;
        if caller_ceiling == 0 {
            return Err(R2Error::ZeroOutputBudget);
        }
        if features.version() != self.provenance.feature_version {
            return Err(R2Error::VersionMismatch {
                field: "prediction.feature_version",
            });
        }
        let encoded = self.encoder.encode(features.values())?;
        let request_diagnostics = self.request_support_diagnostics(features.values())?;
        self.heads
            .iter()
            .filter(|head| head.action.output_budget() <= caller_ceiling)
            .map(|head| self.predict_head(head, &encoded, &request_diagnostics))
            .collect()
    }

    fn validate_structure(&self) -> Result<(), R2Error> {
        if self.schema_version != R2_STATE_SCHEMA_VERSION {
            return Err(R2Error::VersionMismatch {
                field: "state.schema_version",
            });
        }
        self.provenance.validate()?;
        if self.provenance.predictor_version != R2_PREDICTOR_VERSION {
            return Err(R2Error::VersionMismatch {
                field: "state.predictor_version",
            });
        }
        if self.provenance.instruction_version != R2_INSTRUCTION_VERSION {
            return Err(R2Error::VersionMismatch {
                field: "state.instruction_version",
            });
        }
        self.encoder.validate()?;
        self.policy.validate()?;
        if self.heads.is_empty() {
            return Err(R2Error::Empty {
                field: "state.heads",
            });
        }
        if self.feature_minima.len() != self.encoder.feature_dimensions()
            || self.feature_maxima.len() != self.encoder.feature_dimensions()
        {
            return Err(R2Error::DimensionMismatch {
                field: "state.feature_envelope",
            });
        }
        require_finite("state.feature_minima", &self.feature_minima)?;
        require_finite("state.feature_maxima", &self.feature_maxima)?;
        if self
            .feature_minima
            .iter()
            .zip(&self.feature_maxima)
            .any(|(minimum, maximum)| minimum > maximum)
        {
            return Err(R2Error::OutOfRange {
                field: "state.feature_envelope",
            });
        }

        for head in &self.heads {
            head.validate()?;
            if head.quality_weights.len() != self.encoder.hidden_dimensions() {
                return Err(R2Error::DimensionMismatch {
                    field: "head.quality_weights",
                });
            }
            if head.output_weights.len() != self.encoder.hidden_dimensions() {
                return Err(R2Error::DimensionMismatch {
                    field: "head.output_weights",
                });
            }
        }
        for pair in self.heads.windows(2) {
            if pair[0].action == pair[1].action {
                return Err(R2Error::DuplicateAction {
                    output_budget: pair[0].action.output_budget(),
                });
            }
            if pair[0].action > pair[1].action {
                return Err(R2Error::NoncanonicalActions);
            }
        }
        Ok(())
    }

    fn predict_head(
        &self,
        head: &R2AnchorHead,
        encoded: &[f64],
        request_diagnostics: &[R2SupportDiagnostic],
    ) -> Result<R2HeadPrediction, R2Error> {
        if !request_diagnostics.is_empty() {
            return Ok(abstention(
                head,
                R2FallbackReason::OutOfDistribution,
                1.0,
                request_diagnostics.to_vec(),
            ));
        }
        if head.support < self.policy.minimum_support {
            let uncertainty =
                1.0 - f64::from(head.support) / f64::from(self.policy.minimum_support);
            return Ok(abstention(
                head,
                R2FallbackReason::InsufficientSupport,
                uncertainty,
                vec![R2SupportDiagnostic::ActionSupportBelowMin {
                    observed: head.support,
                    required: self.policy.minimum_support,
                }],
            ));
        }

        let quality_ratio =
            residual_ratio(head.quality_residual, self.policy.maximum_quality_residual);
        let output_ratio =
            residual_ratio(head.output_residual, self.policy.maximum_output_residual);
        let maximum_residual_ratio = quality_ratio.max(output_ratio);
        if maximum_residual_ratio > 1.0 {
            let mut diagnostics = Vec::with_capacity(2);
            if quality_ratio > 1.0 {
                diagnostics.push(R2SupportDiagnostic::QualityResidualAboveMax {
                    observed: head.quality_residual,
                    maximum: self.policy.maximum_quality_residual,
                });
            }
            if output_ratio > 1.0 {
                diagnostics.push(R2SupportDiagnostic::OutputResidualAboveMax {
                    observed: head.output_residual,
                    maximum: self.policy.maximum_output_residual,
                });
            }
            return Ok(abstention(
                head,
                R2FallbackReason::UncalibratedHead,
                1.0,
                diagnostics,
            ));
        }

        let support = f64::from(head.support);
        let support_confidence = support / (support + f64::from(self.policy.minimum_support));
        let residual_confidence = 1.0 - maximum_residual_ratio;
        let confidence = (support_confidence * residual_confidence).clamp(0.0, 1.0);
        let quality = sigmoid(head.quality_bias + dot(&head.quality_weights, encoded));
        let output_fraction = sigmoid(head.output_bias + dot(&head.output_weights, encoded));
        let expected_output_tokens = f64::from(head.action.output_budget()) * output_fraction;

        Ok(R2HeadPrediction::Estimated(R2ActionPrediction {
            action: head.action.clone(),
            semantic_correctness: PredictionEstimate::estimated(quality, confidence)
                .map_err(prediction_error)?,
            expected_output_tokens: PredictionEstimate::expected_cost(
                expected_output_tokens,
                confidence,
            )
            .map_err(prediction_error)?,
            instruction_input_tokens: head.instruction_input_tokens,
            instruction_input_upper_bound: head.instruction_input_upper_bound,
        }))
    }

    fn request_support_diagnostics(
        &self,
        features: &[f64],
    ) -> Result<Vec<R2SupportDiagnostic>, R2Error> {
        if features.len() != self.encoder.feature_dimensions() {
            return Err(R2Error::DimensionMismatch {
                field: "prediction.features",
            });
        }
        Ok(features
            .iter()
            .zip(&self.feature_minima)
            .zip(&self.feature_maxima)
            .zip(&self.encoder.feature_scales)
            .enumerate()
            .filter_map(|(index, (((value, minimum), maximum), scale))| {
                let feature_index = u32::try_from(index).unwrap_or(u32::MAX);
                if value < minimum {
                    let scaled_distance = (minimum - value) / scale;
                    (scaled_distance > self.policy.maximum_feature_distance).then(|| {
                        R2SupportDiagnostic::RequestFeatureBelowMin {
                            feature_index,
                            feature: runtime_feature_name(feature_index),
                            observed: *value,
                            bound: *minimum,
                            scaled_distance,
                            limit: self.policy.maximum_feature_distance,
                        }
                    })
                } else if value > maximum {
                    let scaled_distance = (value - maximum) / scale;
                    (scaled_distance > self.policy.maximum_feature_distance).then(|| {
                        R2SupportDiagnostic::RequestFeatureAboveMax {
                            feature_index,
                            feature: runtime_feature_name(feature_index),
                            observed: *value,
                            bound: *maximum,
                            scaled_distance,
                            limit: self.policy.maximum_feature_distance,
                        }
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    fn compute_content_id(&self) -> Result<String, R2Error> {
        let payload = R2StateIdentityPayload {
            schema_version: &self.schema_version,
            provenance: R2ProvenanceIdentity {
                predictor_version: &self.provenance.predictor_version,
                encoder_version: &self.provenance.encoder_version,
                feature_version: &self.provenance.feature_version,
                evaluator_version: &self.provenance.evaluator_version,
                corpus_version: &self.provenance.corpus_version,
                catalog_version: &self.provenance.catalog_version,
                price_version: &self.provenance.price_version,
                instruction_version: &self.provenance.instruction_version,
                label_version: &self.provenance.label_version,
                training_content_hash: &self.provenance.training_content_hash,
            },
            encoder: &self.encoder,
            heads: &self.heads,
            policy: &self.policy,
            feature_minima: &self.feature_minima,
            feature_maxima: &self.feature_maxima,
        };
        let bytes = serde_json::to_vec(&payload).map_err(|error| R2Error::Serialization {
            reason: error.to_string(),
        })?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Serialize)]
struct R2StateIdentityPayload<'a> {
    schema_version: &'a str,
    provenance: R2ProvenanceIdentity<'a>,
    encoder: &'a R2SharedEncoder,
    heads: &'a [R2AnchorHead],
    policy: &'a R2EstimatorPolicy,
    feature_minima: &'a [f64],
    feature_maxima: &'a [f64],
}

#[derive(Serialize)]
struct R2ProvenanceIdentity<'a> {
    predictor_version: &'a str,
    encoder_version: &'a str,
    feature_version: &'a str,
    evaluator_version: &'a str,
    corpus_version: &'a str,
    catalog_version: &'a str,
    price_version: &'a str,
    instruction_version: &'a str,
    label_version: &'a str,
    training_content_hash: &'a str,
}

fn abstention(
    head: &R2AnchorHead,
    reason: R2FallbackReason,
    uncertainty: f64,
    diagnostics: Vec<R2SupportDiagnostic>,
) -> R2HeadPrediction {
    R2HeadPrediction::Abstained(R2ActionAbstention {
        action: head.action.clone(),
        reason,
        uncertainty: uncertainty.clamp(0.0, 1.0),
        diagnostics,
    })
}

fn runtime_feature_name(feature_index: u32) -> String {
    usize::try_from(feature_index)
        .ok()
        .and_then(|index| R2_RUNTIME_FEATURE_NAMES.get(index))
        .map_or_else(
            || format!("{R2_RUNTIME_FEATURE_VERSION}[{feature_index}]"),
            |feature| (*feature).to_string(),
        )
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn sigmoid(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exponent = value.exp();
        exponent / (1.0 + exponent)
    }
}

fn residual_ratio(residual: f64, maximum: f64) -> f64 {
    if maximum == 0.0 {
        if residual == 0.0 {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        residual / maximum
    }
}

fn require_finite(field: &'static str, values: &[f64]) -> Result<(), R2Error> {
    if values.iter().any(|value| !value.is_finite()) {
        Err(R2Error::NonFinite { field })
    } else {
        Ok(())
    }
}

fn require_nonnegative(field: &'static str, value: f64) -> Result<(), R2Error> {
    if !value.is_finite() {
        return Err(R2Error::NonFinite { field });
    }
    if value < 0.0 {
        return Err(R2Error::OutOfRange { field });
    }
    Ok(())
}

fn prediction_error(error: impl std::fmt::Display) -> R2Error {
    R2Error::Prediction {
        reason: error.to_string(),
    }
}

fn validate_head_prediction(prediction: &R2HeadPrediction) -> Result<(), R2Error> {
    prediction.action().validate()?;
    match prediction {
        R2HeadPrediction::Estimated(prediction) => {
            let semantic_correctness = estimated_value(
                "frozen.semantic_correctness",
                &prediction.semantic_correctness,
            )?;
            if !(0.0..=1.0).contains(&semantic_correctness) {
                return Err(R2Error::OutOfRange {
                    field: "frozen.semantic_correctness",
                });
            }
            let expected_output_tokens = estimated_value(
                "frozen.expected_output_tokens",
                &prediction.expected_output_tokens,
            )?;
            if expected_output_tokens > f64::from(prediction.action.output_budget()) {
                return Err(R2Error::OutOfRange {
                    field: "frozen.expected_output_tokens",
                });
            }
            if prediction.instruction_input_upper_bound == 0
                || prediction.instruction_input_tokens > prediction.instruction_input_upper_bound
            {
                return Err(R2Error::OutOfRange {
                    field: "frozen.instruction_input_tokens",
                });
            }
            Ok(())
        }
        R2HeadPrediction::Abstained(abstention) => {
            if !abstention.uncertainty.is_finite() || !(0.0..=1.0).contains(&abstention.uncertainty)
            {
                Err(R2Error::OutOfRange {
                    field: "frozen.abstention_uncertainty",
                })
            } else if !abstention
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.validate(abstention.reason))
                || !abstention
                    .diagnostics
                    .windows(2)
                    .all(|pair| pair[0].order_key() < pair[1].order_key())
            {
                Err(R2Error::OutOfRange {
                    field: "frozen.abstention_diagnostics",
                })
            } else {
                Ok(())
            }
        }
    }
}

fn estimated_value(field: &'static str, estimate: &PredictionEstimate) -> Result<f64, R2Error> {
    match estimate {
        PredictionEstimate::Estimated { value, confidence } => {
            if !value.is_finite()
                || !confidence.is_finite()
                || !(0.0..=1.0).contains(confidence)
                || *value < 0.0
            {
                Err(R2Error::OutOfRange { field })
            } else {
                Ok(*value)
            }
        }
        PredictionEstimate::Abstained { .. } => Err(R2Error::OutOfRange { field }),
    }
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
