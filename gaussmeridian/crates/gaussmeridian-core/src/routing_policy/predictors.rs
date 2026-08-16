//! Pure, versioned contracts for request-conditional routing predictions.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub mod knn;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PredictionError {
    #[error("prediction has blank authoritative identifier: {field}")]
    BlankIdentifier { field: &'static str },
    #[error("prediction feature vector is empty")]
    EmptyFeatures,
    #[error("prediction learner state contains no training examples")]
    EmptyTrainingSet,
    #[error("controlled training example contains no route outcomes")]
    EmptyOutcomes,
    #[error("prediction contains a non-finite value at {field}")]
    NonFinite { field: &'static str },
    #[error("prediction contains an out-of-range value at {field}")]
    OutOfRange { field: &'static str },
    #[error("prediction contains a negative value at {field}")]
    Negative { field: &'static str },
    #[error("prediction set contains duplicate route {provider_id}/{model_id}")]
    DuplicateRoute {
        provider_id: String,
        model_id: String,
    },
    #[error("prediction learner state contains duplicate prompt {prompt_id}")]
    DuplicatePrompt { prompt_id: String },
    #[error("prediction training content hash does not match canonical examples")]
    TrainingContentHashMismatch,
    #[error("prediction content serialization failed: {reason}")]
    Serialization { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RouteIdentity {
    provider_id: String,
    model_id: String,
}

impl RouteIdentity {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Self, PredictionError> {
        let route = Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
        };
        require_nonblank("route.provider_id", &route.provider_id)?;
        require_nonblank("route.model_id", &route.model_id)?;
        Ok(route)
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PredictionFeatureVector {
    version: String,
    values: Vec<f64>,
}

impl PredictionFeatureVector {
    pub fn new(version: impl Into<String>, values: Vec<f64>) -> Result<Self, PredictionError> {
        let features = Self {
            version: version.into(),
            values,
        };
        require_nonblank("features.version", &features.version)?;
        if features.values.is_empty() {
            return Err(PredictionError::EmptyFeatures);
        }
        if features.values.iter().any(|value| !value.is_finite()) {
            return Err(PredictionError::NonFinite {
                field: "features.values",
            });
        }
        Ok(features)
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictionProvenance {
    pub predictor_version: String,
    pub feature_version: String,
    pub evaluator_version: String,
    pub corpus_version: String,
    pub catalog_version: String,
    pub price_version: String,
    pub learner_state_id: String,
    pub training_content_hash: String,
}

impl PredictionProvenance {
    pub fn new(
        predictor_version: impl Into<String>,
        feature_version: impl Into<String>,
        evaluator_version: impl Into<String>,
        corpus_version: impl Into<String>,
        catalog_version: impl Into<String>,
        price_version: impl Into<String>,
        learner_state_id: impl Into<String>,
        training_content_hash: impl Into<String>,
    ) -> Result<Self, PredictionError> {
        let provenance = Self {
            predictor_version: predictor_version.into(),
            feature_version: feature_version.into(),
            evaluator_version: evaluator_version.into(),
            corpus_version: corpus_version.into(),
            catalog_version: catalog_version.into(),
            price_version: price_version.into(),
            learner_state_id: learner_state_id.into(),
            training_content_hash: training_content_hash.into(),
        };
        for (field, value) in [
            (
                "provenance.predictor_version",
                provenance.predictor_version.as_str(),
            ),
            (
                "provenance.feature_version",
                provenance.feature_version.as_str(),
            ),
            (
                "provenance.evaluator_version",
                provenance.evaluator_version.as_str(),
            ),
            (
                "provenance.corpus_version",
                provenance.corpus_version.as_str(),
            ),
            (
                "provenance.catalog_version",
                provenance.catalog_version.as_str(),
            ),
            (
                "provenance.price_version",
                provenance.price_version.as_str(),
            ),
            (
                "provenance.learner_state_id",
                provenance.learner_state_id.as_str(),
            ),
            (
                "provenance.training_content_hash",
                provenance.training_content_hash.as_str(),
            ),
        ] {
            require_nonblank(field, value)?;
        }
        Ok(provenance)
    }

    fn validate(&self) -> Result<(), PredictionError> {
        Self::new(
            self.predictor_version.clone(),
            self.feature_version.clone(),
            self.evaluator_version.clone(),
            self.corpus_version.clone(),
            self.catalog_version.clone(),
            self.price_version.clone(),
            self.learner_state_id.clone(),
            self.training_content_hash.clone(),
        )
        .map(|_| ())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictionFallbackReason {
    NoActiveState,
    RepositoryUnavailable,
    InvalidState,
    FeatureVersionMismatch,
    EvaluatorVersionMismatch,
    ProductionPromotionBlocked,
    ModelEvidenceMissing,
    InsufficientNeighbors,
    OutOfDistribution,
}

impl PredictionFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoActiveState => "no_active_state",
            Self::RepositoryUnavailable => "repository_unavailable",
            Self::InvalidState => "invalid_state",
            Self::FeatureVersionMismatch => "feature_version_mismatch",
            Self::EvaluatorVersionMismatch => "evaluator_version_mismatch",
            Self::ProductionPromotionBlocked => "production_promotion_blocked",
            Self::ModelEvidenceMissing => "model_evidence_missing",
            Self::InsufficientNeighbors => "insufficient_neighbors",
            Self::OutOfDistribution => "out_of_distribution",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "disposition")]
pub enum PredictionEstimate {
    Estimated {
        value: f64,
        confidence: f64,
    },
    Abstained {
        reason: PredictionFallbackReason,
        uncertainty: f64,
    },
}

impl PredictionEstimate {
    pub fn estimated(value: f64, confidence: f64) -> Result<Self, PredictionError> {
        validate_probability("estimate.value", value)?;
        validate_probability("estimate.confidence", confidence)?;
        Ok(Self::Estimated { value, confidence })
    }

    pub fn expected_cost(value: f64, confidence: f64) -> Result<Self, PredictionError> {
        validate_nonnegative("estimate.value", value)?;
        validate_probability("estimate.confidence", confidence)?;
        Ok(Self::Estimated { value, confidence })
    }

    pub fn abstained(
        reason: PredictionFallbackReason,
        uncertainty: f64,
    ) -> Result<Self, PredictionError> {
        validate_probability("estimate.uncertainty", uncertainty)?;
        Ok(Self::Abstained {
            reason,
            uncertainty,
        })
    }

    fn validate_outcome(&self) -> Result<(), PredictionError> {
        match self {
            Self::Estimated { value, confidence } => {
                validate_probability("outcome.value", *value)?;
                validate_probability("outcome.confidence", *confidence)
            }
            Self::Abstained { uncertainty, .. } => {
                validate_probability("outcome.uncertainty", *uncertainty)
            }
        }
    }

    fn validate_cost(&self) -> Result<(), PredictionError> {
        match self {
            Self::Estimated { value, confidence } => {
                validate_nonnegative("cost.value", *value)?;
                validate_probability("cost.confidence", *confidence)
            }
            Self::Abstained { uncertainty, .. } => {
                validate_probability("cost.uncertainty", *uncertainty)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoutePrediction {
    route: RouteIdentity,
    outcome: PredictionEstimate,
    cost: PredictionEstimate,
}

impl RoutePrediction {
    pub fn new(
        route: RouteIdentity,
        outcome: PredictionEstimate,
        cost: PredictionEstimate,
    ) -> Result<Self, PredictionError> {
        RouteIdentity::new(route.provider_id.clone(), route.model_id.clone())?;
        outcome.validate_outcome()?;
        cost.validate_cost()?;
        Ok(Self {
            route,
            outcome,
            cost,
        })
    }

    fn validate(&self) -> Result<(), PredictionError> {
        Self::new(self.route.clone(), self.outcome.clone(), self.cost.clone()).map(|_| ())
    }

    pub fn route(&self) -> &RouteIdentity {
        &self.route
    }

    pub fn outcome(&self) -> &PredictionEstimate {
        &self.outcome
    }

    pub fn cost(&self) -> &PredictionEstimate {
        &self.cost
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PredictionQuery {
    pub features: PredictionFeatureVector,
}

pub trait OutcomePredictor {
    fn predict_outcome(&self, query: &PredictionQuery, route: &RouteIdentity)
        -> PredictionEstimate;
}

pub trait CostPredictor {
    fn predict_cost(&self, query: &PredictionQuery, route: &RouteIdentity) -> PredictionEstimate;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// How learned predictions contributed to one routing decision.
pub enum PredictionUseStatus {
    Unavailable,
    PromotionBlocked,
    Estimated,
    Partial,
    Abstained,
}

impl PredictionUseStatus {
    /// Return the stable wire representation used in routing evidence and headers.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::PromotionBlocked => "promotion_blocked",
            Self::Estimated => "estimated",
            Self::Partial => "partial",
            Self::Abstained => "abstained",
        }
    }
}

/// Counts and fallback disposition for predictions considered by one decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictionUseSummary {
    /// Overall prediction-use disposition.
    pub status: PredictionUseStatus,
    /// Routes with an estimated conditional outcome.
    pub outcome_estimate_count: usize,
    /// Routes with an estimated provider cost.
    pub cost_estimate_count: usize,
    /// Routes for which either predictor abstained.
    pub abstained_route_count: usize,
    /// Stable fallback reason that best explains unavailable prediction evidence.
    pub dominant_fallback_reason: Option<PredictionFallbackReason>,
}

/// Frozen learned-prediction evidence or its typed unavailability reason.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "availability")]
pub enum FrozenPredictionEvidence {
    Active(FrozenPredictionSet),
    Unavailable {
        predictor_version: String,
        feature_version: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        learner_state_id: Option<String>,
        reason: PredictionFallbackReason,
    },
}

impl FrozenPredictionEvidence {
    pub fn unavailable(reason: PredictionFallbackReason) -> Self {
        Self::Unavailable {
            predictor_version: "carrot-knn/v1".to_string(),
            feature_version: "carrot-runtime-features/v1".to_string(),
            learner_state_id: None,
            reason,
        }
    }

    pub fn estimates_for(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> (PredictionEstimate, PredictionEstimate) {
        match self {
            Self::Active(predictions) => predictions
                .prediction_for(provider_id, model_id)
                .map(|prediction| (prediction.outcome.clone(), prediction.cost.clone()))
                .unwrap_or_else(|| {
                    let missing = fallback_estimate(PredictionFallbackReason::ModelEvidenceMissing);
                    (missing.clone(), missing)
                }),
            Self::Unavailable { reason, .. } => {
                let unavailable = fallback_estimate(*reason);
                (unavailable.clone(), unavailable)
            }
        }
    }

    /// Summarize whether and how the frozen predictions influenced selection.
    pub fn use_summary(&self) -> PredictionUseSummary {
        match self {
            Self::Unavailable { reason, .. } => PredictionUseSummary {
                status: if *reason == PredictionFallbackReason::ProductionPromotionBlocked {
                    PredictionUseStatus::PromotionBlocked
                } else {
                    PredictionUseStatus::Unavailable
                },
                outcome_estimate_count: 0,
                cost_estimate_count: 0,
                abstained_route_count: 0,
                dominant_fallback_reason: Some(*reason),
            },
            Self::Active(predictions) => summarize_active_predictions(predictions),
        }
    }

    pub fn validate(&self) -> Result<(), PredictionError> {
        match self {
            Self::Active(predictions) => predictions.validate(),
            Self::Unavailable {
                predictor_version,
                feature_version,
                learner_state_id,
                ..
            } => {
                require_nonblank("predictions.predictor_version", predictor_version)?;
                require_nonblank("predictions.feature_version", feature_version)?;
                if let Some(learner_state_id) = learner_state_id {
                    require_nonblank("predictions.learner_state_id", learner_state_id)?;
                }
                Ok(())
            }
        }
    }
}

fn summarize_active_predictions(predictions: &FrozenPredictionSet) -> PredictionUseSummary {
    let mut outcome_estimate_count = 0;
    let mut cost_estimate_count = 0;
    let mut abstained_route_count = 0;
    let mut fallback_counts = BTreeMap::new();
    for prediction in predictions.predictions() {
        let mut route_abstained = false;
        for (estimate, estimate_count) in [
            (&prediction.outcome, &mut outcome_estimate_count),
            (&prediction.cost, &mut cost_estimate_count),
        ] {
            match estimate {
                PredictionEstimate::Estimated { .. } => *estimate_count += 1,
                PredictionEstimate::Abstained { reason, .. } => {
                    route_abstained = true;
                    *fallback_counts.entry(*reason).or_insert(0usize) += 1;
                }
            }
        }
        if route_abstained {
            abstained_route_count += 1;
        }
    }
    let route_count = predictions.predictions().len();
    let status = if route_count > 0
        && outcome_estimate_count == route_count
        && cost_estimate_count == route_count
    {
        PredictionUseStatus::Estimated
    } else if outcome_estimate_count == 0 && cost_estimate_count == 0 {
        PredictionUseStatus::Abstained
    } else {
        PredictionUseStatus::Partial
    };
    let dominant_fallback_reason = fallback_counts
        .into_iter()
        .fold(None, |dominant, (reason, count)| match dominant {
            Some((_, dominant_count)) if dominant_count >= count => dominant,
            _ => Some((reason, count)),
        })
        .map(|(reason, _)| reason);
    PredictionUseSummary {
        status,
        outcome_estimate_count,
        cost_estimate_count,
        abstained_route_count,
        dominant_fallback_reason,
    }
}

impl Default for FrozenPredictionEvidence {
    fn default() -> Self {
        Self::unavailable(PredictionFallbackReason::NoActiveState)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrozenPredictionSet {
    provenance: PredictionProvenance,
    features: PredictionFeatureVector,
    predictions: Vec<RoutePrediction>,
}

impl FrozenPredictionSet {
    pub fn new(
        provenance: PredictionProvenance,
        features: PredictionFeatureVector,
        mut predictions: Vec<RoutePrediction>,
    ) -> Result<Self, PredictionError> {
        provenance.validate()?;
        PredictionFeatureVector::new(features.version.clone(), features.values.clone())?;
        if provenance.feature_version != features.version {
            return Err(PredictionError::OutOfRange {
                field: "features.version",
            });
        }
        predictions.sort_unstable_by(|left, right| left.route.cmp(&right.route));
        let mut routes = BTreeSet::new();
        for prediction in &predictions {
            prediction.validate()?;
            if !routes.insert(prediction.route.clone()) {
                return Err(PredictionError::DuplicateRoute {
                    provider_id: prediction.route.provider_id.clone(),
                    model_id: prediction.route.model_id.clone(),
                });
            }
        }
        Ok(Self {
            provenance,
            features,
            predictions,
        })
    }

    pub fn validate(&self) -> Result<(), PredictionError> {
        let canonical = Self::new(
            self.provenance.clone(),
            self.features.clone(),
            self.predictions.clone(),
        )?;
        if canonical != *self {
            return Err(PredictionError::OutOfRange {
                field: "predictions.canonical_order",
            });
        }
        Ok(())
    }

    pub fn provenance(&self) -> &PredictionProvenance {
        &self.provenance
    }

    pub fn features(&self) -> &PredictionFeatureVector {
        &self.features
    }

    pub fn predictions(&self) -> &[RoutePrediction] {
        &self.predictions
    }

    pub fn prediction_for(&self, provider_id: &str, model_id: &str) -> Option<&RoutePrediction> {
        self.predictions
            .binary_search_by(|prediction| {
                (
                    prediction.route.provider_id.as_str(),
                    prediction.route.model_id.as_str(),
                )
                    .cmp(&(provider_id, model_id))
            })
            .ok()
            .map(|index| &self.predictions[index])
    }

    pub fn content_id(&self) -> Result<String, PredictionError> {
        let payload = serde_json::to_vec(self).map_err(|error| PredictionError::Serialization {
            reason: error.to_string(),
        })?;
        Ok(format!("{:x}", Sha256::digest(payload)))
    }
}

pub(super) fn require_nonblank(field: &'static str, value: &str) -> Result<(), PredictionError> {
    if value.trim().is_empty() {
        Err(PredictionError::BlankIdentifier { field })
    } else {
        Ok(())
    }
}

pub(super) fn validate_probability(field: &'static str, value: f64) -> Result<(), PredictionError> {
    if !value.is_finite() {
        return Err(PredictionError::NonFinite { field });
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(PredictionError::OutOfRange { field });
    }
    Ok(())
}

pub(super) fn validate_nonnegative(field: &'static str, value: f64) -> Result<(), PredictionError> {
    if !value.is_finite() {
        return Err(PredictionError::NonFinite { field });
    }
    if value < 0.0 {
        return Err(PredictionError::Negative { field });
    }
    Ok(())
}

fn fallback_estimate(reason: PredictionFallbackReason) -> PredictionEstimate {
    PredictionEstimate::Abstained {
        reason,
        uncertainty: 1.0,
    }
}
