use gaussmeridian_core::routing_policy::predictors::{
    FrozenPredictionEvidence, FrozenPredictionSet, PredictionEstimate, PredictionFallbackReason,
    PredictionFeatureVector, PredictionProvenance, PredictionUseStatus, RouteIdentity,
    RoutePrediction,
};

fn route(provider: &str, model: &str) -> RouteIdentity {
    RouteIdentity::new(provider, model).expect("valid route identity")
}

fn provenance() -> PredictionProvenance {
    PredictionProvenance::new(
        "carrot-knn/v1",
        "carrot-runtime-features/v1",
        "semantic-evaluator/v1",
        "controlled-corpus/v1",
        "catalog/v1",
        "prices/v1",
        "learner-state-1",
        "training-sha256",
    )
    .expect("valid provenance")
}

fn features(values: Vec<f64>) -> PredictionFeatureVector {
    PredictionFeatureVector::new("carrot-runtime-features/v1", values)
        .expect("valid feature vector")
}

fn estimated_route(provider: &str, model: &str, correctness: f64, cost: f64) -> RoutePrediction {
    RoutePrediction::new(
        route(provider, model),
        PredictionEstimate::estimated(correctness, 0.8).expect("valid outcome estimate"),
        PredictionEstimate::expected_cost(cost, 0.7).expect("valid cost estimate"),
    )
    .expect("valid route prediction")
}

#[test]
fn authoritative_prediction_identifiers_and_features_fail_closed() {
    assert!(RouteIdentity::new("", "model-a").is_err());
    assert!(RouteIdentity::new("provider-a", " ").is_err());
    assert!(PredictionFeatureVector::new("", vec![0.1]).is_err());
    assert!(PredictionFeatureVector::new("features/v1", vec![]).is_err());
    assert!(PredictionFeatureVector::new("features/v1", vec![f64::NAN]).is_err());
    assert!(PredictionProvenance::new(
        "predictor/v1",
        "features/v1",
        "",
        "corpus/v1",
        "catalog/v1",
        "prices/v1",
        "state-1",
        "hash-1",
    )
    .is_err());
}

#[test]
fn prediction_estimates_enforce_axis_ranges() {
    assert!(PredictionEstimate::estimated(-0.01, 0.5).is_err());
    assert!(PredictionEstimate::estimated(1.01, 0.5).is_err());
    assert!(PredictionEstimate::estimated(0.5, -0.01).is_err());
    assert!(PredictionEstimate::estimated(0.5, 1.01).is_err());
    assert!(PredictionEstimate::expected_cost(-0.01, 0.5).is_err());
    assert!(PredictionEstimate::expected_cost(f64::INFINITY, 0.5).is_err());
    assert!(
        PredictionEstimate::abstained(PredictionFallbackReason::OutOfDistribution, 1.01).is_err()
    );
}

#[test]
fn frozen_predictions_are_route_exact_and_order_independent() {
    let left = FrozenPredictionSet::new(
        provenance(),
        features(vec![0.2, 4.0, 5.0]),
        vec![
            estimated_route("provider-b", "model-b", 0.9, 0.03),
            estimated_route("provider-a", "model-a", 0.8, 0.01),
        ],
    )
    .expect("valid prediction set");
    let right = FrozenPredictionSet::new(
        provenance(),
        features(vec![0.2, 4.0, 5.0]),
        vec![
            estimated_route("provider-a", "model-a", 0.8, 0.01),
            estimated_route("provider-b", "model-b", 0.9, 0.03),
        ],
    )
    .expect("valid prediction set");

    assert_eq!(left, right);
    assert_eq!(
        left.predictions()
            .iter()
            .map(|prediction| (
                prediction.route().provider_id(),
                prediction.route().model_id()
            ))
            .collect::<Vec<_>>(),
        vec![("provider-a", "model-a"), ("provider-b", "model-b")]
    );
    assert_eq!(
        left.prediction_for("provider-a", "model-a")
            .expect("exact route prediction")
            .route(),
        &route("provider-a", "model-a")
    );
    assert!(left.prediction_for("provider-b", "model-a").is_none());
    assert_eq!(
        left.content_id().expect("content identity"),
        right.content_id().expect("content identity")
    );
}

#[test]
fn frozen_predictions_reject_duplicate_routes() {
    let duplicate = estimated_route("provider-a", "model-a", 0.8, 0.01);
    let result = FrozenPredictionSet::new(
        provenance(),
        features(vec![0.2, 4.0, 5.0]),
        vec![duplicate.clone(), duplicate],
    );

    assert!(result.is_err());
}

#[test]
fn frozen_predictions_reject_axis_invalid_estimates() {
    assert!(RoutePrediction::new(
        route("provider-a", "model-a"),
        PredictionEstimate::expected_cost(2.0, 0.8).expect("valid cost-shaped estimate"),
        PredictionEstimate::expected_cost(0.01, 0.7).expect("valid cost estimate"),
    )
    .is_err());

    let mut serialized = serde_json::to_value(estimated_route("provider-a", "model-a", 0.8, 0.01))
        .expect("serialize route prediction");
    serialized["cost"]["value"] = serde_json::json!(-0.01);
    let invalid_cost: RoutePrediction =
        serde_json::from_value(serialized).expect("serde accepts persisted representation");
    assert!(FrozenPredictionSet::new(
        provenance(),
        features(vec![0.2, 4.0, 5.0]),
        vec![invalid_cost],
    )
    .is_err());
}

#[test]
fn abstention_is_typed_and_canonical() {
    let prediction = RoutePrediction::new(
        route("provider-a", "model-a"),
        PredictionEstimate::abstained(PredictionFallbackReason::InsufficientNeighbors, 0.9)
            .expect("valid abstention"),
        PredictionEstimate::abstained(PredictionFallbackReason::OutOfDistribution, 1.0)
            .expect("valid abstention"),
    )
    .expect("valid route prediction");
    let frozen = FrozenPredictionSet::new(
        provenance(),
        features(vec![0.9, 8.0, 7.0]),
        vec![prediction],
    )
    .expect("valid prediction set");

    assert!(matches!(
        frozen
            .prediction_for("provider-a", "model-a")
            .expect("prediction")
            .outcome(),
        PredictionEstimate::Abstained {
            reason: PredictionFallbackReason::InsufficientNeighbors,
            uncertainty
        } if *uncertainty == 0.9
    ));
}

#[test]
fn predictor_use_summary_distinguishes_runtime_dispositions() {
    let unavailable =
        FrozenPredictionEvidence::unavailable(PredictionFallbackReason::NoActiveState)
            .use_summary();
    assert_eq!(unavailable.status, PredictionUseStatus::Unavailable);
    assert_eq!(unavailable.outcome_estimate_count, 0);
    assert_eq!(unavailable.cost_estimate_count, 0);
    assert_eq!(unavailable.abstained_route_count, 0);
    assert_eq!(
        unavailable.dominant_fallback_reason,
        Some(PredictionFallbackReason::NoActiveState)
    );

    let promotion_blocked =
        FrozenPredictionEvidence::unavailable(PredictionFallbackReason::ProductionPromotionBlocked)
            .use_summary();
    assert_eq!(
        promotion_blocked.status,
        PredictionUseStatus::PromotionBlocked
    );
    assert_eq!(
        promotion_blocked.dominant_fallback_reason,
        Some(PredictionFallbackReason::ProductionPromotionBlocked)
    );

    let estimated = FrozenPredictionEvidence::Active(
        FrozenPredictionSet::new(
            provenance(),
            features(vec![0.2, 4.0, 5.0]),
            vec![
                estimated_route("provider-a", "model-a", 0.8, 0.01),
                estimated_route("provider-b", "model-b", 0.9, 0.02),
            ],
        )
        .expect("estimated set"),
    )
    .use_summary();
    assert_eq!(estimated.status, PredictionUseStatus::Estimated);
    assert_eq!(estimated.outcome_estimate_count, 2);
    assert_eq!(estimated.cost_estimate_count, 2);
    assert_eq!(estimated.abstained_route_count, 0);
    assert_eq!(estimated.dominant_fallback_reason, None);

    let partial = FrozenPredictionEvidence::Active(
        FrozenPredictionSet::new(
            provenance(),
            features(vec![0.2, 4.0, 5.0]),
            vec![
                estimated_route("provider-a", "model-a", 0.8, 0.01),
                RoutePrediction::new(
                    route("provider-b", "model-b"),
                    PredictionEstimate::abstained(PredictionFallbackReason::OutOfDistribution, 1.0)
                        .expect("outcome abstention"),
                    PredictionEstimate::expected_cost(0.02, 0.8).expect("cost estimate"),
                )
                .expect("partial route"),
            ],
        )
        .expect("partial set"),
    )
    .use_summary();
    assert_eq!(partial.status, PredictionUseStatus::Partial);
    assert_eq!(partial.outcome_estimate_count, 1);
    assert_eq!(partial.cost_estimate_count, 2);
    assert_eq!(partial.abstained_route_count, 1);
    assert_eq!(
        partial.dominant_fallback_reason,
        Some(PredictionFallbackReason::OutOfDistribution)
    );

    let abstained = FrozenPredictionEvidence::Active(
        FrozenPredictionSet::new(
            provenance(),
            features(vec![0.2, 4.0, 5.0]),
            vec![RoutePrediction::new(
                route("provider-a", "model-a"),
                PredictionEstimate::abstained(PredictionFallbackReason::InsufficientNeighbors, 1.0)
                    .expect("outcome abstention"),
                PredictionEstimate::abstained(PredictionFallbackReason::InsufficientNeighbors, 1.0)
                    .expect("cost abstention"),
            )
            .expect("abstained route")],
        )
        .expect("abstained set"),
    )
    .use_summary();
    assert_eq!(abstained.status, PredictionUseStatus::Abstained);
    assert_eq!(abstained.outcome_estimate_count, 0);
    assert_eq!(abstained.cost_estimate_count, 0);
    assert_eq!(abstained.abstained_route_count, 1);
    assert_eq!(
        abstained.dominant_fallback_reason,
        Some(PredictionFallbackReason::InsufficientNeighbors)
    );
}
