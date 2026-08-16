use gaussmeridian_core::routing_policy::{
    predictors::{PredictionEstimate, PredictionFeatureVector, RouteIdentity},
    r2::{
        FrozenR2Evidence, R2ActionIdentity, R2AnchorHead, R2BudgetAnchors, R2Error,
        R2EstimatorPolicy, R2EvaluationDisposition, R2FallbackReason, R2HeadPrediction,
        R2InstructionInputEstimate, R2LearnerState, R2OutputBudgetConstraint, R2Provenance,
        R2SharedEncoder, R2SupportDiagnostic, R2_INSTRUCTION_VERSION, R2_PREDICTOR_VERSION,
    },
};

fn route(provider_id: &str, model_id: &str) -> RouteIdentity {
    RouteIdentity::new(provider_id, model_id).expect("valid route")
}

fn provenance(price_version: &str) -> R2Provenance {
    R2Provenance::new(
        R2_PREDICTOR_VERSION,
        "meridian-r2-encoder/v1",
        "routing-features/v2",
        "controlled-r2-evaluator/v1",
        "routing-p4-corpus/v1",
        "catalog/v7",
        price_version,
        R2_INSTRUCTION_VERSION,
        "routing-r2-label/v1",
        "a".repeat(64),
        "b".repeat(64),
    )
    .expect("valid provenance")
}

fn features(value: f64) -> PredictionFeatureVector {
    PredictionFeatureVector::new("routing-features/v2", vec![value]).expect("valid features")
}

fn encoder() -> R2SharedEncoder {
    R2SharedEncoder::new(vec![0.0], vec![1.0], vec![vec![0.0]], vec![0.0]).expect("valid encoder")
}

fn head(
    output_budget: u32,
    support: u32,
    quality_residual: f64,
    output_residual: f64,
) -> R2AnchorHead {
    R2AnchorHead::new(
        R2ActionIdentity::new(route("provider-a", "model-a"), output_budget).expect("valid action"),
        vec![0.0],
        0.0,
        vec![0.0],
        0.0,
        12,
        128,
        support,
        quality_residual,
        output_residual,
    )
    .expect("valid head")
}

fn estimator_policy() -> R2EstimatorPolicy {
    R2EstimatorPolicy::new(4, 0.25, 0.10, 0.10).expect("valid policy")
}

fn learner(heads: Vec<R2AnchorHead>) -> R2LearnerState {
    R2LearnerState::new(
        provenance("price/v3"),
        encoder(),
        heads,
        estimator_policy(),
        vec![0.0],
        vec![1.0],
    )
    .expect("valid learner")
}

fn estimated_value(estimate: &PredictionEstimate) -> (f64, f64) {
    match estimate {
        PredictionEstimate::Estimated { value, confidence } => (*value, *confidence),
        PredictionEstimate::Abstained { reason, .. } => {
            panic!("expected estimate, got abstention: {reason:?}")
        }
    }
}

#[test]
fn predecessor_instruction_estimate_rejects_invalid_bounds() {
    assert!(matches!(
        R2InstructionInputEstimate::new(0, 0),
        Err(R2Error::OutOfRange {
            field: "instruction_input_estimate"
        })
    ));
    assert!(matches!(
        R2InstructionInputEstimate::new(129, 128),
        Err(R2Error::OutOfRange {
            field: "instruction_input_estimate"
        })
    ));
}

#[test]
fn output_budget_contract_is_versioned_byte_exact_and_anchor_specific() {
    let short = R2OutputBudgetConstraint::new(64).expect("valid short anchor");
    assert_eq!(short.instruction_version, R2_INSTRUCTION_VERSION);
    assert_eq!(
        short.instruction,
        "Complete the requested task in no more than 64 output tokens.\n\
Prioritize correctness and a complete answer within that limit."
    );
    assert_eq!(short.estimated_input_tokens, 27);
    assert_eq!(short.input_token_upper_bound, 125);

    let long = R2OutputBudgetConstraint::new(128).expect("valid long anchor");
    assert_eq!(long.estimated_input_tokens, 27);
    assert_eq!(long.input_token_upper_bound, 126);

    assert_eq!(
        R2OutputBudgetConstraint::new(0).unwrap_err(),
        R2Error::ZeroOutputBudget
    );
}

#[test]
fn action_identity_rejects_zero_output_budget() {
    assert_eq!(
        R2ActionIdentity::new(route("provider-a", "model-a"), 0).unwrap_err(),
        R2Error::ZeroOutputBudget
    );
}

#[test]
fn action_identity_orders_by_route_then_output_budget() {
    let first = R2ActionIdentity::new(route("provider-a", "model-a"), 128).expect("valid action");
    let second = R2ActionIdentity::new(route("provider-a", "model-a"), 256).expect("valid action");
    let third = R2ActionIdentity::new(route("provider-b", "model-a"), 64).expect("valid action");

    assert!(first < second);
    assert!(second < third);
    assert_eq!(first.route().provider_id(), "provider-a");
    assert_eq!(first.route().model_id(), "model-a");
    assert_eq!(first.output_budget(), 128);
}

#[test]
fn deserialized_action_identity_cannot_bypass_budget_validation() {
    let action: R2ActionIdentity = serde_json::from_value(serde_json::json!({
        "route": {
            "provider_id": "provider-a",
            "model_id": "model-a"
        },
        "output_budget": 0
    }))
    .expect("structurally valid JSON");

    assert_eq!(action.validate().unwrap_err(), R2Error::ZeroOutputBudget);
}

#[test]
fn anchor_set_rejects_zero_duplicate_and_noncanonical_budgets() {
    assert_eq!(
        R2BudgetAnchors::new(vec![0, 64]).unwrap_err(),
        R2Error::ZeroOutputBudget
    );
    assert_eq!(
        R2BudgetAnchors::new(vec![64, 128, 128]).unwrap_err(),
        R2Error::DuplicateOutputBudget { output_budget: 128 }
    );
    assert_eq!(
        R2BudgetAnchors::new(vec![128, 64]).unwrap_err(),
        R2Error::NoncanonicalOutputBudgets
    );
}

#[test]
fn anchor_set_retains_the_exact_tested_budgets() {
    let anchors = R2BudgetAnchors::new(vec![64, 128, 256]).expect("canonical anchors");

    assert_eq!(anchors.values(), &[64, 128, 256]);
}

#[test]
fn provenance_rejects_blank_versions_and_malformed_hashes() {
    assert_eq!(
        R2Provenance::new(
            R2_PREDICTOR_VERSION,
            "meridian-r2-encoder/v1",
            "routing-features/v2",
            "controlled-r2-evaluator/v1",
            "routing-p4-corpus/v1",
            "catalog/v7",
            "price/v3",
            "",
            "routing-r2-label/v1",
            "a".repeat(64),
            "b".repeat(64),
        )
        .unwrap_err(),
        R2Error::BlankProvenanceField {
            field: "instruction_version"
        }
    );

    assert_eq!(
        R2Provenance::new(
            R2_PREDICTOR_VERSION,
            "meridian-r2-encoder/v1",
            "routing-features/v2",
            "controlled-r2-evaluator/v1",
            "routing-p4-corpus/v1",
            "catalog/v7",
            "price/v3",
            R2_INSTRUCTION_VERSION,
            "routing-r2-label/v1",
            "not-a-sha256",
            "b".repeat(64),
        )
        .unwrap_err(),
        R2Error::InvalidLearnerStateId
    );

    assert_eq!(
        R2Provenance::new(
            R2_PREDICTOR_VERSION,
            "meridian-r2-encoder/v1",
            "routing-features/v2",
            "controlled-r2-evaluator/v1",
            "routing-p4-corpus/v1",
            "catalog/v7",
            "price/v3",
            R2_INSTRUCTION_VERSION,
            "routing-r2-label/v1",
            "a".repeat(64),
            "not-a-sha256",
        )
        .unwrap_err(),
        R2Error::InvalidTrainingContentHash
    );
}

#[test]
fn provenance_content_identity_is_stable_and_content_addressed() {
    let original = provenance("price/v3");
    let round_trip: R2Provenance =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();

    assert_eq!(original, round_trip);
    assert_eq!(
        original.content_id().unwrap(),
        round_trip.content_id().unwrap()
    );
    assert_ne!(
        original.content_id().unwrap(),
        provenance("price/v4").content_id().unwrap()
    );
    assert_eq!(original.content_id().unwrap().len(), 64);
}

#[test]
fn shared_encoder_and_anchor_head_match_hand_calculated_predictions() {
    let predictions = learner(vec![head(128, 8, 0.0, 0.0)])
        .predict(&features(0.5), 128)
        .expect("valid prediction");

    let R2HeadPrediction::Estimated(prediction) = &predictions[0] else {
        panic!("expected estimated action");
    };
    let (quality, quality_confidence) = estimated_value(&prediction.semantic_correctness);
    let (output_tokens, output_confidence) = estimated_value(&prediction.expected_output_tokens);

    assert_eq!(prediction.action.output_budget(), 128);
    assert!((quality - 0.5).abs() < 1e-12);
    assert!((output_tokens - 64.0).abs() < 1e-12);
    assert!((quality_confidence - (2.0 / 3.0)).abs() < 1e-12);
    assert!((output_confidence - (2.0 / 3.0)).abs() < 1e-12);
    assert_eq!(prediction.instruction_input_tokens, 12);
    assert_eq!(prediction.instruction_input_upper_bound, 128);
}

#[test]
fn insufficient_support_abstains_for_only_the_affected_anchor() {
    let predictions = learner(vec![head(64, 3, 0.0, 0.0), head(128, 8, 0.0, 0.0)])
        .predict(&features(0.5), 128)
        .expect("valid prediction");

    assert!(matches!(
        &predictions[0],
        R2HeadPrediction::Abstained(abstention)
            if abstention.reason == R2FallbackReason::InsufficientSupport
    ));
    assert!(matches!(
        &predictions[1],
        R2HeadPrediction::Estimated(prediction)
            if prediction.action.output_budget() == 128
    ));
}

#[test]
fn excessive_residual_and_out_of_distribution_requests_abstain_explicitly() {
    let residual_predictions = learner(vec![head(128, 8, 0.11, 0.0)])
        .predict(&features(0.5), 128)
        .expect("valid prediction");
    assert!(matches!(
        &residual_predictions[0],
        R2HeadPrediction::Abstained(abstention)
            if abstention.reason == R2FallbackReason::UncalibratedHead
    ));

    let ood_predictions = learner(vec![head(128, 8, 0.0, 0.0)])
        .predict(&features(1.5), 128)
        .expect("valid prediction");
    assert!(matches!(
        &ood_predictions[0],
        R2HeadPrediction::Abstained(abstention)
            if abstention.reason == R2FallbackReason::OutOfDistribution
    ));
}

#[test]
fn request_envelope_abstention_identifies_each_violated_feature_bound() {
    let below = learner(vec![head(128, 8, 0.0, 0.0)])
        .predict(&features(-0.5), 128)
        .expect("valid below-envelope prediction");
    let above = learner(vec![head(128, 8, 0.0, 0.0)])
        .predict(&features(1.5), 128)
        .expect("valid above-envelope prediction");

    let R2HeadPrediction::Abstained(below) = &below[0] else {
        panic!("below-envelope request must abstain");
    };
    assert_eq!(
        below.diagnostics,
        vec![R2SupportDiagnostic::RequestFeatureBelowMin {
            feature_index: 0,
            feature: "complexity".to_string(),
            observed: -0.5,
            bound: 0.0,
            scaled_distance: 0.5,
            limit: 0.25,
        }]
    );

    let R2HeadPrediction::Abstained(above) = &above[0] else {
        panic!("above-envelope request must abstain");
    };
    assert_eq!(
        above.diagnostics,
        vec![R2SupportDiagnostic::RequestFeatureAboveMax {
            feature_index: 0,
            feature: "complexity".to_string(),
            observed: 1.5,
            bound: 1.0,
            scaled_distance: 0.5,
            limit: 0.25,
        }]
    );
}

#[test]
fn request_envelope_diagnostics_are_complete_and_ordered_by_runtime_feature() {
    let state = R2LearnerState::new(
        provenance("price/v3"),
        R2SharedEncoder::new(
            vec![0.0, 0.0, 0.0],
            vec![1.0, 2.0, 4.0],
            vec![vec![0.0, 0.0, 0.0]],
            vec![0.0],
        )
        .expect("valid three-feature encoder"),
        vec![head(128, 8, 0.0, 0.0)],
        estimator_policy(),
        vec![0.0, 0.0, 0.0],
        vec![1.0, 1.0, 1.0],
    )
    .expect("valid three-feature learner");
    let predictions = state
        .predict(
            &PredictionFeatureVector::new("routing-features/v2", vec![-0.5, 2.0, 3.0])
                .expect("valid features"),
            128,
        )
        .expect("valid prediction");

    let R2HeadPrediction::Abstained(abstention) = &predictions[0] else {
        panic!("multi-feature OOD request must abstain");
    };
    assert_eq!(abstention.diagnostics.len(), 3);
    assert!(matches!(
        &abstention.diagnostics[0],
        R2SupportDiagnostic::RequestFeatureBelowMin {
            feature_index: 0,
            feature,
            ..
        } if feature == "complexity"
    ));
    assert!(matches!(
        &abstention.diagnostics[1],
        R2SupportDiagnostic::RequestFeatureAboveMax {
            feature_index: 1,
            feature,
            ..
        } if feature == "ln_1p_estimated_input_tokens"
    ));
    assert!(matches!(
        &abstention.diagnostics[2],
        R2SupportDiagnostic::RequestFeatureAboveMax {
            feature_index: 2,
            feature,
            ..
        } if feature == "ln_1p_output_token_ceiling"
    ));
}

#[test]
fn action_abstention_identifies_support_and_each_residual_limit() {
    let support = learner(vec![head(64, 3, 0.0, 0.0)])
        .predict(&features(0.5), 64)
        .expect("valid support prediction");
    let residual = learner(vec![head(128, 8, 0.11, 0.12)])
        .predict(&features(0.5), 128)
        .expect("valid residual prediction");

    let R2HeadPrediction::Abstained(support) = &support[0] else {
        panic!("unsupported action must abstain");
    };
    assert_eq!(
        support.diagnostics,
        vec![R2SupportDiagnostic::ActionSupportBelowMin {
            observed: 3,
            required: 4,
        }]
    );

    let R2HeadPrediction::Abstained(residual) = &residual[0] else {
        panic!("uncalibrated action must abstain");
    };
    assert_eq!(
        residual.diagnostics,
        vec![
            R2SupportDiagnostic::QualityResidualAboveMax {
                observed: 0.11,
                maximum: 0.10,
            },
            R2SupportDiagnostic::OutputResidualAboveMax {
                observed: 0.12,
                maximum: 0.10,
            },
        ]
    );
}

#[test]
fn historical_abstention_without_diagnostics_round_trips_without_new_bytes() {
    let historical = serde_json::json!({
        "action": {
            "route": {
                "provider_id": "provider-a",
                "model_id": "model-a"
            },
            "output_budget": 128
        },
        "reason": "out_of_distribution",
        "uncertainty": 1.0
    });

    let abstention: gaussmeridian_core::routing_policy::r2::R2ActionAbstention =
        serde_json::from_value(historical.clone()).expect("historical abstention deserializes");

    assert!(abstention.diagnostics.is_empty());
    assert_eq!(
        serde_json::to_value(abstention).expect("historical abstention serializes"),
        historical
    );

    let historical_disposition = serde_json::json!({
        "disposition": "abstained",
        "reason": "out_of_distribution",
        "uncertainty": 1.0
    });
    let disposition: R2EvaluationDisposition =
        serde_json::from_value(historical_disposition.clone())
            .expect("historical evaluated disposition deserializes");
    assert_eq!(
        serde_json::to_value(disposition).expect("historical disposition serializes"),
        historical_disposition
    );
}

#[test]
fn frozen_evidence_rejects_noncanonical_diagnostic_order() {
    let state = learner(vec![head(128, 8, 0.11, 0.12)]);
    let mut predictions = state
        .predict(&features(0.5), 128)
        .expect("valid prediction");
    let R2HeadPrediction::Abstained(abstention) = &mut predictions[0] else {
        panic!("uncalibrated head must abstain");
    };
    abstention.diagnostics.reverse();

    assert!(matches!(
        FrozenR2Evidence::active(
            state.provenance().clone(),
            R2InstructionInputEstimate::new(12, 128).expect("valid instruction estimate"),
            predictions,
        ),
        Err(R2Error::OutOfRange {
            field: "frozen.abstention_diagnostics"
        })
    ));
}

#[test]
fn caller_ceiling_filters_untested_actions_without_clamping() {
    let predictions = learner(vec![head(64, 8, 0.0, 0.0), head(128, 8, 0.0, 0.0)])
        .predict(&features(0.5), 100)
        .expect("valid prediction");

    assert_eq!(predictions.len(), 1);
    assert_eq!(predictions[0].action().output_budget(), 64);
}

#[test]
fn learner_rejects_invalid_dimensions_and_noncanonical_heads() {
    assert!(matches!(
        R2SharedEncoder::new(vec![0.0], vec![0.0], vec![vec![0.0]], vec![0.0]),
        Err(R2Error::OutOfRange {
            field: "encoder.feature_scales"
        })
    ));

    let invalid_head = R2AnchorHead::new(
        R2ActionIdentity::new(route("provider-a", "model-a"), 128).unwrap(),
        vec![0.0, 0.0],
        0.0,
        vec![0.0],
        0.0,
        12,
        128,
        8,
        0.0,
        0.0,
    )
    .unwrap();
    assert!(matches!(
        R2LearnerState::new(
            provenance("price/v3"),
            encoder(),
            vec![invalid_head],
            estimator_policy(),
            vec![0.0],
            vec![1.0]
        ),
        Err(R2Error::DimensionMismatch {
            field: "head.quality_weights"
        })
    ));

    assert!(matches!(
        R2LearnerState::new(
            provenance("price/v3"),
            encoder(),
            vec![head(128, 8, 0.0, 0.0), head(64, 8, 0.0, 0.0)],
            estimator_policy(),
            vec![0.0],
            vec![1.0]
        ),
        Err(R2Error::NoncanonicalActions)
    ));
    assert!(matches!(
        R2LearnerState::new(
            provenance("price/v3"),
            encoder(),
            vec![head(128, 8, 0.0, 0.0), head(128, 8, 0.0, 0.0)],
            estimator_policy(),
            vec![0.0],
            vec![1.0]
        ),
        Err(R2Error::DuplicateAction { output_budget: 128 })
    ));
}

#[test]
fn learner_identity_is_derived_stable_and_tamper_evident() {
    let original = learner(vec![head(128, 8, 0.0, 0.0)]);
    let round_trip: R2LearnerState =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();

    assert_eq!(original, round_trip);
    assert_eq!(
        original.provenance().learner_state_id,
        original.content_id().unwrap()
    );
    assert_eq!(
        original.content_id().unwrap(),
        round_trip.content_id().unwrap()
    );
    round_trip.validate().expect("valid round trip");

    let mut tampered = serde_json::to_value(&original).unwrap();
    tampered["heads"][0]["quality_bias"] = serde_json::json!(0.25);
    let tampered: R2LearnerState = serde_json::from_value(tampered).unwrap();
    assert_eq!(
        tampered.validate().unwrap_err(),
        R2Error::LearnerStateIdMismatch
    );
}
