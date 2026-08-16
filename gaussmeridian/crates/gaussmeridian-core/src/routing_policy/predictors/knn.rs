//! Deterministic K-nearest-neighbor predictors over controlled all-model evidence.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    require_nonblank, validate_nonnegative, validate_probability, CostPredictor, OutcomePredictor,
    PredictionError, PredictionEstimate, PredictionFallbackReason, PredictionFeatureVector,
    PredictionQuery, RouteIdentity,
};

/// Stable predictor identifier for the controlled CARROT KNN implementation.
pub const KNN_PREDICTOR_VERSION: &str = "carrot-knn/v1";

/// Whether a learner state is mechanism-only or carries self-described promotion evidence.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "disposition", content = "evidence")]
pub enum PredictorPromotion {
    /// The state may be qualified offline but cannot control production routing.
    #[default]
    MechanismOnly,
    /// The state contains promotion metadata that still requires external authority verification.
    ProductionEligible(ProductionPromotionEvidence),
}

impl PredictorPromotion {
    fn validate(&self) -> Result<(), PredictionError> {
        match self {
            Self::MechanismOnly => Ok(()),
            Self::ProductionEligible(evidence) => evidence.validate(),
        }
    }
}

/// Metadata claimed by a learner state for an externally governed promotion decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionPromotionEvidence {
    criteria_version: String,
    qualification_report_sha256: String,
    dataset_card_id: String,
    governance_authority_id: String,
}

impl ProductionPromotionEvidence {
    /// Construct and validate nonblank promotion metadata.
    pub fn new(
        criteria_version: impl Into<String>,
        qualification_report_sha256: impl Into<String>,
        dataset_card_id: impl Into<String>,
        governance_authority_id: impl Into<String>,
    ) -> Result<Self, PredictionError> {
        let evidence = Self {
            criteria_version: criteria_version.into(),
            qualification_report_sha256: qualification_report_sha256.into(),
            dataset_card_id: dataset_card_id.into(),
            governance_authority_id: governance_authority_id.into(),
        };
        evidence.validate()?;
        Ok(evidence)
    }

    fn validate(&self) -> Result<(), PredictionError> {
        for (field, value) in [
            ("promotion.criteria_version", self.criteria_version.as_str()),
            (
                "promotion.qualification_report_sha256",
                self.qualification_report_sha256.as_str(),
            ),
            ("promotion.dataset_card_id", self.dataset_card_id.as_str()),
            (
                "promotion.governance_authority_id",
                self.governance_authority_id.as_str(),
            ),
        ] {
            require_nonblank(field, value)?;
        }
        Ok(())
    }

    /// Return the promotion-criteria version claimed by this state.
    pub fn criteria_version(&self) -> &str {
        &self.criteria_version
    }

    /// Return the claimed qualification-report digest.
    pub fn qualification_report_sha256(&self) -> &str {
        &self.qualification_report_sha256
    }

    /// Return the claimed governed dataset-card identifier.
    pub fn dataset_card_id(&self) -> &str {
        &self.dataset_card_id
    }

    /// Return the claimed governance-authority identifier.
    pub fn governance_authority_id(&self) -> &str {
        &self.governance_authority_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnnStateMetadata {
    predictor_version: String,
    feature_version: String,
    evaluator_version: String,
    corpus_version: String,
    catalog_version: String,
    price_version: String,
    training_content_hash: String,
}

impl KnnStateMetadata {
    pub fn new(
        predictor_version: impl Into<String>,
        feature_version: impl Into<String>,
        evaluator_version: impl Into<String>,
        corpus_version: impl Into<String>,
        catalog_version: impl Into<String>,
        price_version: impl Into<String>,
        training_content_hash: impl Into<String>,
    ) -> Result<Self, PredictionError> {
        let metadata = Self {
            predictor_version: predictor_version.into(),
            feature_version: feature_version.into(),
            evaluator_version: evaluator_version.into(),
            corpus_version: corpus_version.into(),
            catalog_version: catalog_version.into(),
            price_version: price_version.into(),
            training_content_hash: training_content_hash.into(),
        };
        for (field, value) in [
            (
                "state.predictor_version",
                metadata.predictor_version.as_str(),
            ),
            ("state.feature_version", metadata.feature_version.as_str()),
            (
                "state.evaluator_version",
                metadata.evaluator_version.as_str(),
            ),
            ("state.corpus_version", metadata.corpus_version.as_str()),
            ("state.catalog_version", metadata.catalog_version.as_str()),
            ("state.price_version", metadata.price_version.as_str()),
            (
                "state.training_content_hash",
                metadata.training_content_hash.as_str(),
            ),
        ] {
            require_nonblank(field, value)?;
        }
        Ok(metadata)
    }

    pub fn predictor_version(&self) -> &str {
        &self.predictor_version
    }

    pub fn feature_version(&self) -> &str {
        &self.feature_version
    }

    pub fn evaluator_version(&self) -> &str {
        &self.evaluator_version
    }

    pub fn corpus_version(&self) -> &str {
        &self.corpus_version
    }

    pub fn catalog_version(&self) -> &str {
        &self.catalog_version
    }

    pub fn price_version(&self) -> &str {
        &self.price_version
    }

    pub fn training_content_hash(&self) -> &str {
        &self.training_content_hash
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnnHyperparameters {
    neighbor_count: usize,
    minimum_neighbors: usize,
    ood_threshold: f64,
}

impl KnnHyperparameters {
    pub fn new(
        neighbor_count: usize,
        minimum_neighbors: usize,
        ood_threshold: f64,
    ) -> Result<Self, PredictionError> {
        if neighbor_count == 0 {
            return Err(PredictionError::OutOfRange {
                field: "hyperparameters.neighbor_count",
            });
        }
        if minimum_neighbors == 0 || minimum_neighbors > neighbor_count {
            return Err(PredictionError::OutOfRange {
                field: "hyperparameters.minimum_neighbors",
            });
        }
        validate_nonnegative("hyperparameters.ood_threshold", ood_threshold)?;
        Ok(Self {
            neighbor_count,
            minimum_neighbors,
            ood_threshold,
        })
    }

    pub fn neighbor_count(&self) -> usize {
        self.neighbor_count
    }

    pub fn minimum_neighbors(&self) -> usize {
        self.minimum_neighbors
    }

    pub fn ood_threshold(&self) -> f64 {
        self.ood_threshold
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlledRouteOutcome {
    route: RouteIdentity,
    delivered_correctness: f64,
    provider_cost: f64,
}

impl ControlledRouteOutcome {
    pub fn new(
        route: RouteIdentity,
        delivered_correctness: f64,
        provider_cost: f64,
    ) -> Result<Self, PredictionError> {
        validate_probability("outcome.delivered_correctness", delivered_correctness)?;
        validate_nonnegative("outcome.provider_cost", provider_cost)?;
        Ok(Self {
            route,
            delivered_correctness,
            provider_cost,
        })
    }

    pub fn route(&self) -> &RouteIdentity {
        &self.route
    }

    pub fn delivered_correctness(&self) -> f64 {
        self.delivered_correctness
    }

    pub fn provider_cost(&self) -> f64 {
        self.provider_cost
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlledTrainingExample {
    prompt_id: String,
    features: PredictionFeatureVector,
    outcomes: Vec<ControlledRouteOutcome>,
}

impl ControlledTrainingExample {
    pub fn new(
        prompt_id: impl Into<String>,
        features: PredictionFeatureVector,
        mut outcomes: Vec<ControlledRouteOutcome>,
    ) -> Result<Self, PredictionError> {
        let prompt_id = prompt_id.into();
        require_nonblank("example.prompt_id", &prompt_id)?;
        if outcomes.is_empty() {
            return Err(PredictionError::EmptyOutcomes);
        }
        outcomes.sort_unstable_by(|left, right| left.route.cmp(&right.route));
        let mut routes = BTreeSet::new();
        for outcome in &outcomes {
            if !routes.insert(outcome.route.clone()) {
                return Err(PredictionError::DuplicateRoute {
                    provider_id: outcome.route.provider_id().to_owned(),
                    model_id: outcome.route.model_id().to_owned(),
                });
            }
        }
        Ok(Self {
            prompt_id,
            features,
            outcomes,
        })
    }

    pub fn prompt_id(&self) -> &str {
        &self.prompt_id
    }

    pub fn features(&self) -> &PredictionFeatureVector {
        &self.features
    }

    pub fn outcomes(&self) -> &[ControlledRouteOutcome] {
        &self.outcomes
    }

    fn outcome_for(&self, route: &RouteIdentity) -> Option<&ControlledRouteOutcome> {
        self.outcomes
            .binary_search_by(|outcome| outcome.route.cmp(route))
            .ok()
            .map(|index| &self.outcomes[index])
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnnLearnerState {
    metadata: KnnStateMetadata,
    hyperparameters: KnnHyperparameters,
    feature_dimensions: usize,
    examples: Vec<ControlledTrainingExample>,
    #[serde(default)]
    promotion: PredictorPromotion,
}

impl KnnLearnerState {
    pub fn new(
        metadata: KnnStateMetadata,
        hyperparameters: KnnHyperparameters,
        mut examples: Vec<ControlledTrainingExample>,
    ) -> Result<Self, PredictionError> {
        let Some(first) = examples.first() else {
            return Err(PredictionError::EmptyTrainingSet);
        };
        let feature_dimensions = first.features.values().len();
        for example in &examples {
            if example.features.version() != metadata.feature_version
                || example.features.values().len() != feature_dimensions
            {
                return Err(PredictionError::OutOfRange {
                    field: "example.features",
                });
            }
        }
        examples.sort_unstable_by(|left, right| left.prompt_id.cmp(&right.prompt_id));
        for duplicate in examples.windows(2) {
            if duplicate[0].prompt_id == duplicate[1].prompt_id {
                return Err(PredictionError::DuplicatePrompt {
                    prompt_id: duplicate[0].prompt_id.clone(),
                });
            }
        }
        let state = Self {
            metadata,
            hyperparameters,
            feature_dimensions,
            examples,
            promotion: PredictorPromotion::MechanismOnly,
        };
        state.validate()?;
        Ok(state)
    }

    /// Revalidates every persisted field after deserialization bypasses constructors.
    pub fn validate(&self) -> Result<(), PredictionError> {
        KnnStateMetadata::new(
            self.metadata.predictor_version.clone(),
            self.metadata.feature_version.clone(),
            self.metadata.evaluator_version.clone(),
            self.metadata.corpus_version.clone(),
            self.metadata.catalog_version.clone(),
            self.metadata.price_version.clone(),
            self.metadata.training_content_hash.clone(),
        )?;
        KnnHyperparameters::new(
            self.hyperparameters.neighbor_count,
            self.hyperparameters.minimum_neighbors,
            self.hyperparameters.ood_threshold,
        )?;
        self.promotion.validate()?;
        if self.metadata.predictor_version != KNN_PREDICTOR_VERSION {
            return Err(PredictionError::OutOfRange {
                field: "state.predictor_version",
            });
        }

        let Some(first) = self.examples.first() else {
            return Err(PredictionError::EmptyTrainingSet);
        };
        let feature_dimensions = first.features.values.len();
        if self.feature_dimensions != feature_dimensions {
            return Err(PredictionError::OutOfRange {
                field: "state.feature_dimensions",
            });
        }

        let mut prompt_ids = BTreeSet::new();
        for example in &self.examples {
            require_nonblank("example.prompt_id", &example.prompt_id)?;
            if !prompt_ids.insert(example.prompt_id.clone()) {
                return Err(PredictionError::DuplicatePrompt {
                    prompt_id: example.prompt_id.clone(),
                });
            }

            PredictionFeatureVector::new(
                example.features.version.clone(),
                example.features.values.clone(),
            )?;
            if example.features.version != self.metadata.feature_version
                || example.features.values.len() != feature_dimensions
            {
                return Err(PredictionError::OutOfRange {
                    field: "example.features",
                });
            }
            if example.outcomes.is_empty() {
                return Err(PredictionError::EmptyOutcomes);
            }

            let mut routes = BTreeSet::new();
            for outcome in &example.outcomes {
                let route = RouteIdentity::new(
                    outcome.route.provider_id.clone(),
                    outcome.route.model_id.clone(),
                )?;
                ControlledRouteOutcome::new(
                    route.clone(),
                    outcome.delivered_correctness,
                    outcome.provider_cost,
                )?;
                if !routes.insert(route.clone()) {
                    return Err(PredictionError::DuplicateRoute {
                        provider_id: route.provider_id().to_owned(),
                        model_id: route.model_id().to_owned(),
                    });
                }
            }
        }
        if self.metadata.training_content_hash != training_content_hash(&self.examples)? {
            return Err(PredictionError::TrainingContentHashMismatch);
        }
        let canonical_examples = canonical_examples(&self.examples);
        if self.examples != canonical_examples {
            return Err(PredictionError::OutOfRange {
                field: "state.canonical_order",
            });
        }
        Ok(())
    }

    pub fn metadata(&self) -> &KnnStateMetadata {
        &self.metadata
    }

    pub fn hyperparameters(&self) -> &KnnHyperparameters {
        &self.hyperparameters
    }

    pub fn feature_dimensions(&self) -> usize {
        self.feature_dimensions
    }

    pub fn examples(&self) -> &[ControlledTrainingExample] {
        &self.examples
    }

    /// Return this state's serialized promotion disposition.
    pub fn promotion(&self) -> &PredictorPromotion {
        &self.promotion
    }

    /// Return a validated copy carrying promotion metadata.
    ///
    /// Runtime consumption must still verify this metadata against an external authority.
    pub fn promote_for_production(
        mut self,
        evidence: ProductionPromotionEvidence,
    ) -> Result<Self, PredictionError> {
        self.promotion = PredictorPromotion::ProductionEligible(evidence);
        self.validate()?;
        Ok(self)
    }

    pub fn content_id(&self) -> Result<String, PredictionError> {
        let payload = serde_json::to_vec(self).map_err(|error| PredictionError::Serialization {
            reason: error.to_string(),
        })?;
        Ok(format!("{:x}", Sha256::digest(payload)))
    }

    fn predict(
        &self,
        query: &PredictionQuery,
        route: &RouteIdentity,
        axis: PredictionAxis,
    ) -> PredictionEstimate {
        if query.features.version() != self.metadata.feature_version
            || query.features.values().len() != self.feature_dimensions
        {
            return abstain(PredictionFallbackReason::FeatureVersionMismatch);
        }

        let mut neighbors = self
            .examples
            .iter()
            .filter_map(|example| {
                example.outcome_for(route).map(|outcome| Neighbor {
                    distance: distance(query.features.values(), example.features.values()),
                    prompt_id: example.prompt_id(),
                    value: axis.value(outcome),
                })
            })
            .collect::<Vec<_>>();

        if neighbors.is_empty() {
            return abstain(PredictionFallbackReason::ModelEvidenceMissing);
        }
        if neighbors.len() < self.hyperparameters.minimum_neighbors {
            return abstain(PredictionFallbackReason::InsufficientNeighbors);
        }
        neighbors.sort_unstable_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.prompt_id.cmp(right.prompt_id))
        });
        if neighbors[0].distance > self.hyperparameters.ood_threshold {
            return abstain(PredictionFallbackReason::OutOfDistribution);
        }
        neighbors.truncate(self.hyperparameters.neighbor_count);

        let exact_match_count = neighbors
            .iter()
            .take_while(|neighbor| neighbor.distance == 0.0)
            .count();
        let selected = if exact_match_count > 0 {
            &neighbors[..exact_match_count]
        } else {
            neighbors.as_slice()
        };
        let weighted_value = if exact_match_count > 0 {
            selected.iter().map(|neighbor| neighbor.value).sum::<f64>() / selected.len() as f64
        } else {
            let weight_sum = selected
                .iter()
                .map(|neighbor| 1.0 / neighbor.distance)
                .sum::<f64>();
            selected
                .iter()
                .map(|neighbor| neighbor.value / neighbor.distance)
                .sum::<f64>()
                / weight_sum
        };
        let confidence = if self.hyperparameters.ood_threshold == 0.0 {
            1.0
        } else {
            (1.0 - neighbors[0].distance / self.hyperparameters.ood_threshold).clamp(0.0, 1.0)
        };
        axis.estimate(weighted_value, confidence)
            .unwrap_or_else(|_| abstain(PredictionFallbackReason::InvalidState))
    }
}

pub fn training_content_hash(
    examples: &[ControlledTrainingExample],
) -> Result<String, PredictionError> {
    let canonical_examples = canonical_examples(examples);
    let payload = serde_json::to_vec(&canonical_examples).map_err(|error| {
        PredictionError::Serialization {
            reason: error.to_string(),
        }
    })?;
    Ok(format!("{:x}", Sha256::digest(payload)))
}

fn canonical_examples(examples: &[ControlledTrainingExample]) -> Vec<ControlledTrainingExample> {
    let mut canonical_examples = examples.to_vec();
    for example in &mut canonical_examples {
        example
            .outcomes
            .sort_unstable_by(|left, right| left.route.cmp(&right.route));
    }
    canonical_examples.sort_unstable_by(|left, right| left.prompt_id.cmp(&right.prompt_id));
    canonical_examples
}

impl OutcomePredictor for KnnLearnerState {
    fn predict_outcome(
        &self,
        query: &PredictionQuery,
        route: &RouteIdentity,
    ) -> PredictionEstimate {
        self.predict(query, route, PredictionAxis::Outcome)
    }
}

impl CostPredictor for KnnLearnerState {
    fn predict_cost(&self, query: &PredictionQuery, route: &RouteIdentity) -> PredictionEstimate {
        self.predict(query, route, PredictionAxis::Cost)
    }
}

#[derive(Clone, Copy)]
enum PredictionAxis {
    Outcome,
    Cost,
}

impl PredictionAxis {
    fn value(self, outcome: &ControlledRouteOutcome) -> f64 {
        match self {
            Self::Outcome => outcome.delivered_correctness,
            Self::Cost => outcome.provider_cost,
        }
    }

    fn estimate(self, value: f64, confidence: f64) -> Result<PredictionEstimate, PredictionError> {
        match self {
            Self::Outcome => PredictionEstimate::estimated(value, confidence),
            Self::Cost => PredictionEstimate::expected_cost(value, confidence),
        }
    }
}

struct Neighbor<'a> {
    distance: f64,
    prompt_id: &'a str,
    value: f64,
}

fn distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .fold(0.0, |norm, (left, right)| norm.hypot(left - right))
}

fn abstain(reason: PredictionFallbackReason) -> PredictionEstimate {
    PredictionEstimate::Abstained {
        reason,
        uncertainty: 1.0,
    }
}
