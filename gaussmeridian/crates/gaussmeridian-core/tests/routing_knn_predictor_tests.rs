use gaussmeridian_core::routing_policy::predictors::knn::{
    training_content_hash, ControlledRouteOutcome, ControlledTrainingExample, KnnHyperparameters,
    KnnLearnerState, KnnStateMetadata, PredictorPromotion, ProductionPromotionEvidence,
};
use gaussmeridian_core::routing_policy::predictors::{
    CostPredictor, OutcomePredictor, PredictionEstimate, PredictionFallbackReason,
    PredictionFeatureVector, PredictionQuery, RouteIdentity,
};

const FEATURE_VERSION: &str = "carrot-runtime-features/v1";

fn route(provider: &str, model: &str) -> RouteIdentity {
    RouteIdentity::new(provider, model).expect("valid route")
}

fn features(values: Vec<f64>) -> PredictionFeatureVector {
    PredictionFeatureVector::new(FEATURE_VERSION, values).expect("valid features")
}

fn outcome(
    provider: &str,
    model: &str,
    delivered_correctness: f64,
    provider_cost: f64,
) -> ControlledRouteOutcome {
    ControlledRouteOutcome::new(route(provider, model), delivered_correctness, provider_cost)
        .expect("valid controlled outcome")
}

fn example(
    prompt_id: &str,
    values: Vec<f64>,
    outcomes: Vec<ControlledRouteOutcome>,
) -> ControlledTrainingExample {
    ControlledTrainingExample::new(prompt_id, features(values), outcomes)
        .expect("valid controlled example")
}

fn metadata(training_hash: &str) -> KnnStateMetadata {
    KnnStateMetadata::new(
        "carrot-knn/v1",
        FEATURE_VERSION,
        "semantic-evaluator/v1",
        "controlled-corpus/v1",
        "catalog/v1",
        "prices/v1",
        training_hash,
    )
    .expect("valid state metadata")
}

#[test]
fn learner_metadata_pins_catalog_and_price_authorities() {
    let metadata = metadata("training-sha256");

    assert_eq!(metadata.catalog_version(), "catalog/v1");
    assert_eq!(metadata.price_version(), "prices/v1");
}

fn promotion_evidence() -> ProductionPromotionEvidence {
    ProductionPromotionEvidence::new(
        "routing-predictor-promotion/v1",
        "qualification-report-sha256",
        "dataset-card:controlled-corpus/v1",
        "governance-authority:routing-council/v1",
    )
    .expect("valid production promotion evidence")
}

fn state(
    neighbor_count: usize,
    minimum_neighbors: usize,
    ood_threshold: f64,
    examples: Vec<ControlledTrainingExample>,
) -> KnnLearnerState {
    let training_hash = training_content_hash(&examples).expect("canonical training hash");
    KnnLearnerState::new(
        metadata(&training_hash),
        KnnHyperparameters::new(neighbor_count, minimum_neighbors, ood_threshold)
            .expect("valid hyperparameters"),
        examples,
    )
    .expect("valid learner state")
}

fn assert_estimated(estimate: PredictionEstimate, expected: f64) {
    match estimate {
        PredictionEstimate::Estimated { value, confidence } => {
            assert!((value - expected).abs() < 1e-12, "{value} != {expected}");
            assert!((0.0..=1.0).contains(&confidence));
        }
        other => panic!("expected estimate, got {other:?}"),
    }
}

fn assert_abstained(estimate: PredictionEstimate, expected: PredictionFallbackReason) {
    match estimate {
        PredictionEstimate::Abstained {
            reason,
            uncertainty,
        } => {
            assert_eq!(reason, expected);
            assert!((0.0..=1.0).contains(&uncertainty));
        }
        other => panic!("expected abstention, got {other:?}"),
    }
}

#[test]
fn learner_state_is_canonical_and_order_independent() {
    let first = example(
        "prompt-a",
        vec![0.0, 0.0, 0.0],
        vec![
            outcome("provider-b", "model-b", 0.8, 0.02),
            outcome("provider-a", "model-a", 0.9, 0.01),
        ],
    );
    let second = example(
        "prompt-b",
        vec![2.0, 0.0, 0.0],
        vec![
            outcome("provider-a", "model-a", 0.5, 0.05),
            outcome("provider-b", "model-b", 0.6, 0.04),
        ],
    );

    let left = state(2, 1, 3.0, vec![first.clone(), second.clone()]);
    let right = state(
        2,
        1,
        3.0,
        vec![
            example(
                "prompt-b",
                vec![2.0, 0.0, 0.0],
                second.outcomes().iter().cloned().rev().collect(),
            ),
            example(
                "prompt-a",
                vec![0.0, 0.0, 0.0],
                first.outcomes().iter().cloned().rev().collect(),
            ),
        ],
    );

    assert_eq!(left, right);
    assert_eq!(
        left.content_id().expect("state content id"),
        right.content_id().expect("state content id")
    );
}

#[test]
fn learner_state_requires_explicit_validated_production_promotion() {
    let mechanism_only = state(
        1,
        1,
        1.0,
        vec![example(
            "prompt-a",
            vec![0.0, 0.0, 0.0],
            vec![outcome("provider-a", "model-a", 0.9, 0.01)],
        )],
    );
    assert_eq!(
        mechanism_only.promotion(),
        &PredictorPromotion::MechanismOnly
    );

    let mut historical_json = serde_json::to_value(&mechanism_only).expect("serialize state");
    historical_json
        .as_object_mut()
        .expect("state object")
        .remove("promotion");
    let historical: KnnLearnerState =
        serde_json::from_value(historical_json).expect("deserialize historical state");
    assert_eq!(historical.promotion(), &PredictorPromotion::MechanismOnly);

    let mechanism_id = mechanism_only.content_id().expect("mechanism state id");
    let promoted = mechanism_only
        .promote_for_production(promotion_evidence())
        .expect("promote learner state");
    assert!(matches!(
        promoted.promotion(),
        PredictorPromotion::ProductionEligible(_)
    ));
    assert_ne!(
        mechanism_id,
        promoted.content_id().expect("promoted state id"),
        "promotion authority must be part of immutable learner-state identity"
    );
    promoted.validate().expect("promoted state remains valid");

    for invalid in [
        ("", "report", "dataset", "authority"),
        ("criteria", "", "dataset", "authority"),
        ("criteria", "report", "", "authority"),
        ("criteria", "report", "dataset", ""),
    ] {
        assert!(
            ProductionPromotionEvidence::new(invalid.0, invalid.1, invalid.2, invalid.3).is_err(),
            "blank promotion authority must be rejected"
        );
    }
}

#[test]
fn conditional_predictions_use_route_specific_nearest_neighbors() {
    let learner = state(
        2,
        1,
        3.0,
        vec![
            example(
                "prompt-a",
                vec![0.0, 0.0, 0.0],
                vec![
                    outcome("provider-a", "model-a", 0.9, 0.01),
                    outcome("provider-b", "model-b", 0.4, 0.02),
                ],
            ),
            example(
                "prompt-b",
                vec![2.0, 0.0, 0.0],
                vec![
                    outcome("provider-a", "model-a", 0.5, 0.05),
                    outcome("provider-b", "model-b", 0.8, 0.08),
                ],
            ),
        ],
    );
    let midpoint = PredictionQuery {
        features: features(vec![1.0, 0.0, 0.0]),
    };

    assert_estimated(
        learner.predict_outcome(&midpoint, &route("provider-a", "model-a")),
        0.7,
    );
    assert_estimated(
        learner.predict_cost(&midpoint, &route("provider-a", "model-a")),
        0.03,
    );
    assert_estimated(
        learner.predict_outcome(&midpoint, &route("provider-b", "model-b")),
        0.6,
    );
    assert_estimated(
        learner.predict_cost(&midpoint, &route("provider-b", "model-b")),
        0.05,
    );
}

#[test]
fn exact_feature_matches_do_not_mix_more_distant_examples() {
    let learner = state(
        2,
        1,
        3.0,
        vec![
            example(
                "prompt-a",
                vec![0.0, 0.0, 0.0],
                vec![outcome("provider-a", "model-a", 0.9, 0.01)],
            ),
            example(
                "prompt-b",
                vec![2.0, 0.0, 0.0],
                vec![outcome("provider-a", "model-a", 0.1, 0.09)],
            ),
        ],
    );
    let exact = PredictionQuery {
        features: features(vec![0.0, 0.0, 0.0]),
    };

    assert_estimated(
        learner.predict_outcome(&exact, &route("provider-a", "model-a")),
        0.9,
    );
    assert_estimated(
        learner.predict_cost(&exact, &route("provider-a", "model-a")),
        0.01,
    );
}

#[test]
fn missing_and_sparse_route_evidence_abstain_with_distinct_reasons() {
    let learner = state(
        3,
        2,
        3.0,
        vec![
            example(
                "prompt-a",
                vec![0.0, 0.0, 0.0],
                vec![outcome("provider-a", "model-a", 0.9, 0.01)],
            ),
            example(
                "prompt-b",
                vec![1.0, 0.0, 0.0],
                vec![outcome("provider-b", "model-b", 0.8, 0.02)],
            ),
        ],
    );
    let query = PredictionQuery {
        features: features(vec![0.5, 0.0, 0.0]),
    };

    assert_abstained(
        learner.predict_outcome(&query, &route("provider-a", "model-a")),
        PredictionFallbackReason::InsufficientNeighbors,
    );
    assert_abstained(
        learner.predict_cost(&query, &route("provider-c", "model-c")),
        PredictionFallbackReason::ModelEvidenceMissing,
    );
}

#[test]
fn feature_version_dimension_and_ood_fail_closed() {
    let learner = state(
        1,
        1,
        0.5,
        vec![example(
            "prompt-a",
            vec![0.0, 0.0, 0.0],
            vec![outcome("provider-a", "model-a", 0.9, 0.01)],
        )],
    );
    let wrong_version = PredictionQuery {
        features: PredictionFeatureVector::new("features/v2", vec![0.0, 0.0, 0.0])
            .expect("valid alternate features"),
    };
    let wrong_dimension = PredictionQuery {
        features: features(vec![0.0, 0.0]),
    };
    let distant = PredictionQuery {
        features: features(vec![1.0, 0.0, 0.0]),
    };

    for query in [&wrong_version, &wrong_dimension] {
        assert_abstained(
            learner.predict_outcome(query, &route("provider-a", "model-a")),
            PredictionFallbackReason::FeatureVersionMismatch,
        );
    }
    assert_abstained(
        learner.predict_cost(&distant, &route("provider-a", "model-a")),
        PredictionFallbackReason::OutOfDistribution,
    );
}

#[test]
fn invalid_training_state_is_rejected_before_activation() {
    assert!(KnnHyperparameters::new(0, 1, 1.0).is_err());
    assert!(KnnHyperparameters::new(1, 2, 1.0).is_err());
    assert!(KnnHyperparameters::new(1, 1, -1.0).is_err());

    let duplicate = example(
        "prompt-a",
        vec![0.0, 0.0, 0.0],
        vec![outcome("provider-a", "model-a", 0.9, 0.01)],
    );
    assert!(KnnLearnerState::new(
        metadata(
            &training_content_hash(&vec![duplicate.clone(), duplicate.clone()])
                .expect("canonical training hash")
        ),
        KnnHyperparameters::new(1, 1, 1.0).expect("valid hyperparameters"),
        vec![duplicate.clone(), duplicate],
    )
    .is_err());
    assert!(ControlledTrainingExample::new(
        "prompt-a",
        features(vec![0.0, 0.0, 0.0]),
        vec![
            outcome("provider-a", "model-a", 0.9, 0.01),
            outcome("provider-a", "model-a", 0.8, 0.02),
        ],
    )
    .is_err());
}

#[test]
fn deserialized_learner_state_must_revalidate_before_use() {
    let learner = state(
        1,
        1,
        1.0,
        vec![example(
            "prompt-a",
            vec![0.0, 0.0, 0.0],
            vec![outcome("provider-a", "model-a", 0.9, 0.01)],
        )],
    );
    let mut serialized = serde_json::to_value(learner).expect("serialize learner");
    serialized["hyperparameters"]["neighbor_count"] = serde_json::json!(0);
    let tampered: KnnLearnerState =
        serde_json::from_value(serialized).expect("serde alone accepts field representation");

    assert!(tampered.validate().is_err());
}

#[test]
fn learner_state_rejects_a_training_hash_that_does_not_match_canonical_examples() {
    let examples = vec![example(
        "prompt-a",
        vec![0.0, 0.0, 0.0],
        vec![outcome("provider-a", "model-a", 0.9, 0.01)],
    )];
    let mismatched = KnnLearnerState::new(
        metadata("training-sha256"),
        KnnHyperparameters::new(1, 1, 1.0).expect("valid hyperparameters"),
        examples,
    );

    assert!(mismatched.is_err());
}
