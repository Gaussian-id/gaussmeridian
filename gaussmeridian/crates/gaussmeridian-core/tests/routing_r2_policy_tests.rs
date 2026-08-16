use std::collections::BTreeSet;

use gaussmeridian_core::routing_policy::{
    predictors::{PredictionEstimate, PredictionFeatureVector, RouteIdentity},
    quote_trajectory_reservation,
    r2::{
        FrozenR2Evidence, R2ActionAbstention, R2ActionDisposition, R2ActionIdentity,
        R2ActionPrediction, R2AnchorHead, R2Decision, R2EstimatorPolicy, R2EvaluationDisposition,
        R2FallbackReason, R2HeadPrediction, R2InstructionInputEstimate, R2LearnerState,
        R2Provenance, R2SharedEncoder, R2SupportDiagnostic, R2_INSTRUCTION_VERSION,
        R2_PREDICTOR_VERSION,
    },
    select, CapabilityBand, CatalogModel, CatalogSnapshot, DeploymentKind, EvidenceSnapshot, Price,
    ProjectPolicy, RoutingContext,
};

fn route(provider_id: &str, model_id: &str) -> RouteIdentity {
    RouteIdentity::new(provider_id, model_id).expect("valid route")
}

fn model(provider_id: &str, model_id: &str, quality: f64) -> CatalogModel {
    CatalogModel {
        model_id: model_id.into(),
        provider_id: provider_id.into(),
        model_version: "model/v1".into(),
        capability_band: CapabilityBand::Baseline,
        deployment_kind: DeploymentKind::Managed,
        price: Price {
            input_per_million: 10.0,
            output_per_million: 20.0,
            expected_fixed_cost: 0.0,
            fixed_cost_upper_bound: 0.0,
        },
        semantic_quality_prior: quality,
        transport_success_probability: 1.0,
        credential_available: true,
        adapter_registered: true,
        adapter_supports_model: true,
        tenant_allowed: true,
        compliant: true,
        skill_proficiency: [1.0; 12],
    }
}

fn context(output_token_budget: u32) -> RoutingContext {
    RoutingContext {
        complexity: 0.1,
        estimated_input_tokens: 100,
        input_token_upper_bound: 200,
        output_token_budget,
        hard_skills: Vec::new(),
    }
}

fn policy() -> ProjectPolicy {
    ProjectPolicy {
        cost_weight: 0.5,
        quality_floor: 0.0,
        max_band: CapabilityBand::Frontier,
        moderate_complexity_threshold: 0.35,
        high_complexity_threshold: 0.70,
        max_provider_attempts: 3,
        router_cost_upper_bound: 0.0,
    }
}

fn evidence() -> EvidenceSnapshot {
    EvidenceSnapshot {
        policy_version: "policy/v1".into(),
        catalog_version: "catalog/v7".into(),
        price_version: "price/v3".into(),
        evaluator_version: "controlled-r2-evaluator/v1".into(),
        normalized_cost_floor: 0.0,
        normalized_cost_ceiling: 0.1,
        complexity: None,
        predictions: Default::default(),
        bella: Default::default(),
        r2: Default::default(),
        compound: Default::default(),
    }
}

fn provenance(catalog_version: &str) -> R2Provenance {
    R2Provenance::new(
        R2_PREDICTOR_VERSION,
        "meridian-r2-encoder/v1",
        "routing-features/v2",
        "controlled-r2-evaluator/v1",
        "routing-p4-corpus/v1",
        catalog_version,
        "price/v3",
        R2_INSTRUCTION_VERSION,
        "routing-r2-label/v1",
        "a".repeat(64),
        "b".repeat(64),
    )
    .expect("valid provenance")
}

fn head(
    provider_id: &str,
    model_id: &str,
    output_budget: u32,
    quality_bias: f64,
    output_bias: f64,
    support: u32,
) -> R2AnchorHead {
    R2AnchorHead::new(
        R2ActionIdentity::new(route(provider_id, model_id), output_budget).unwrap(),
        vec![0.0],
        quality_bias,
        vec![0.0],
        output_bias,
        12,
        128,
        support,
        0.0,
        0.0,
    )
    .unwrap()
}

fn frozen_r2_with_catalog(
    heads: Vec<R2AnchorHead>,
    feature: f64,
    prediction_ceiling: u32,
    catalog_version: &str,
) -> FrozenR2Evidence {
    let state = R2LearnerState::new(
        provenance(catalog_version),
        R2SharedEncoder::new(vec![0.0], vec![1.0], vec![vec![0.0]], vec![0.0]).unwrap(),
        heads,
        R2EstimatorPolicy::new(4, 0.25, 0.1, 0.1).unwrap(),
        vec![0.0],
        vec![1.0],
    )
    .unwrap();
    let predictions = state
        .predict(
            &PredictionFeatureVector::new("routing-features/v2", vec![feature]).unwrap(),
            prediction_ceiling,
        )
        .unwrap();
    FrozenR2Evidence::active(
        state.provenance().clone(),
        R2InstructionInputEstimate::new(12, 128).unwrap(),
        predictions,
    )
    .unwrap()
}

fn frozen_r2(heads: Vec<R2AnchorHead>, feature: f64, prediction_ceiling: u32) -> FrozenR2Evidence {
    frozen_r2_with_catalog(heads, feature, prediction_ceiling, "catalog/v7")
}

#[test]
fn active_r2_selects_a_shorter_tested_budget_and_uses_action_specific_economics() {
    let mut evidence = evidence();
    evidence.r2 = frozen_r2(
        vec![head("provider-a", "model-a", 128, 4.0, 0.0, 8)],
        0.5,
        512,
    );

    let ballot = select(
        &context(512),
        &policy(),
        &CatalogSnapshot::new(vec![model("provider-a", "model-a", 0.7)]),
        &evidence,
    )
    .unwrap();
    let selected = &ballot.entries()[0];

    assert_eq!(ballot.entries().len(), 1);
    assert_eq!(selected.output_token_budget, 128);
    assert!(!selected.r2_action.is_predecessor());
    assert!(matches!(ballot.r2, R2Decision::Applied { .. }));
    assert!((selected.expected_provider_cost - 0.0024).abs() < 1e-12);
    assert!((selected.provider_cost_upper_bound - 0.00584).abs() < 1e-12);
}

#[test]
fn unavailable_incompatible_and_ood_r2_reproduce_exact_p3_ballots() {
    let catalog = CatalogSnapshot::new(vec![model("provider-a", "model-a", 0.8)]);
    let context = context(512);
    let baseline = select(&context, &policy(), &catalog, &evidence()).unwrap();
    let baseline_bytes = serde_json::to_vec(&baseline).unwrap();
    let baseline_id = baseline.content_id().unwrap();

    for reason in [
        R2FallbackReason::RepositoryUnavailable,
        R2FallbackReason::InvalidState,
        R2FallbackReason::FeatureVersionMismatch,
        R2FallbackReason::EvaluatorVersionMismatch,
        R2FallbackReason::CatalogVersionMismatch,
        R2FallbackReason::PriceVersionMismatch,
        R2FallbackReason::InstructionVersionMismatch,
        R2FallbackReason::LabelVersionMismatch,
        R2FallbackReason::ProductionPromotionBlocked,
        R2FallbackReason::ModelEvidenceMissing,
        R2FallbackReason::NoAllowedAnchor,
    ] {
        let mut unavailable = evidence();
        unavailable.r2 = FrozenR2Evidence::unavailable(reason);
        let ballot = select(&context, &policy(), &catalog, &unavailable).unwrap();
        assert_eq!(serde_json::to_vec(&ballot).unwrap(), baseline_bytes);
        assert_eq!(ballot.content_id().unwrap(), baseline_id);
    }

    let mut incompatible = evidence();
    incompatible.r2 = frozen_r2_with_catalog(
        vec![head("provider-a", "model-a", 128, 4.0, 0.0, 8)],
        0.5,
        512,
        "catalog/v8",
    );
    let incompatible = select(&context, &policy(), &catalog, &incompatible).unwrap();
    assert_eq!(serde_json::to_vec(&incompatible).unwrap(), baseline_bytes);
    assert_eq!(incompatible.content_id().unwrap(), baseline_id);

    let mut ood = evidence();
    ood.r2 = frozen_r2(
        vec![head("provider-a", "model-a", 128, 4.0, 0.0, 8)],
        2.0,
        512,
    );
    let ood = select(&context, &policy(), &catalog, &ood).unwrap();
    assert_eq!(serde_json::to_vec(&ood).unwrap(), baseline_bytes);
    assert_eq!(ood.content_id().unwrap(), baseline_id);

    let mut invalid_instruction = evidence();
    invalid_instruction.r2 = frozen_r2(
        vec![head("provider-a", "model-a", 128, 4.0, 0.0, 8)],
        0.5,
        512,
    );
    let mut serialized = serde_json::to_value(&invalid_instruction.r2).unwrap();
    serialized["predecessor_instruction"]["upper_bound"] = serde_json::json!(0);
    invalid_instruction.r2 = serde_json::from_value(serialized).unwrap();
    let invalid_instruction = select(&context, &policy(), &catalog, &invalid_instruction).unwrap();
    assert_eq!(
        serde_json::to_vec(&invalid_instruction).unwrap(),
        baseline_bytes
    );
    assert_eq!(invalid_instruction.content_id().unwrap(), baseline_id);
}

#[test]
fn r2_never_evaluates_or_resurrects_a_hard_excluded_route() {
    let mut denied = model("provider-a", "denied", 0.99);
    denied.tenant_allowed = false;
    let allowed = model("provider-b", "allowed", 0.7);
    let mut evidence = evidence();
    evidence.r2 = frozen_r2(
        vec![
            head("provider-a", "denied", 64, 6.0, 0.0, 8),
            head("provider-b", "allowed", 128, 4.0, 0.0, 8),
        ],
        0.5,
        512,
    );

    let ballot = select(
        &context(512),
        &policy(),
        &CatalogSnapshot::new(vec![denied, allowed]),
        &evidence,
    )
    .unwrap();

    assert_eq!(ballot.entries().len(), 1);
    assert_eq!(ballot.entries()[0].model_id, "allowed");
    let R2Decision::Applied {
        evaluated_actions, ..
    } = &ballot.r2
    else {
        panic!("expected applied R2 evidence");
    };
    assert!(evaluated_actions
        .iter()
        .all(|action| action.action.route().model_id() != "denied"));
}

#[test]
fn tested_anchors_are_intersected_with_the_ceiling_without_clamping() {
    let mut evidence = evidence();
    evidence.r2 = frozen_r2(
        vec![
            head("provider-a", "model-a", 64, 6.0, 0.0, 8),
            head("provider-a", "model-a", 128, 6.0, 0.0, 8),
        ],
        0.5,
        512,
    );

    let ballot = select(
        &context(100),
        &policy(),
        &CatalogSnapshot::new(vec![model("provider-a", "model-a", 0.7)]),
        &evidence,
    )
    .unwrap();

    assert_eq!(ballot.entries().len(), 1);
    assert_eq!(ballot.entries()[0].output_token_budget, 64);
    let R2Decision::Applied {
        evaluated_actions, ..
    } = &ballot.r2
    else {
        panic!("expected applied R2 evidence");
    };
    assert!(evaluated_actions
        .iter()
        .all(|action| action.action.output_budget() <= 100));
    assert!(evaluated_actions
        .iter()
        .all(|action| action.action.output_budget() != 100 || action.disposition.is_predecessor()));
}

#[test]
fn multiple_anchors_reduce_to_one_entry_per_route_and_preserve_global_risk_order() {
    let mut evidence = evidence();
    evidence.r2 = frozen_r2(
        vec![
            head("provider-a", "model-a", 64, 5.0, 0.0, 8),
            head("provider-a", "model-a", 128, 4.0, 0.0, 8),
            head("provider-b", "model-b", 64, 2.0, 0.0, 8),
            head("provider-b", "model-b", 128, 1.0, 0.0, 8),
        ],
        0.5,
        512,
    );

    let ballot = select(
        &context(512),
        &policy(),
        &CatalogSnapshot::new(vec![
            model("provider-b", "model-b", 0.7),
            model("provider-a", "model-a", 0.7),
        ]),
        &evidence,
    )
    .unwrap();

    assert_eq!(ballot.entries().len(), 2);
    assert_eq!(ballot.entries()[0].model_id, "model-a");
    let unique_routes = ballot
        .entries()
        .iter()
        .map(|entry| (&entry.provider_id, &entry.model_id))
        .collect::<BTreeSet<_>>();
    assert_eq!(unique_routes.len(), ballot.entries().len());
}

#[test]
fn an_anchor_equal_to_the_caller_ceiling_is_one_executable_action_not_a_duplicate() {
    let mut evidence = evidence();
    evidence.r2 = frozen_r2(
        vec![head("provider-a", "model-a", 128, 4.0, 0.0, 8)],
        0.5,
        128,
    );

    let ballot = select(
        &context(128),
        &policy(),
        &CatalogSnapshot::new(vec![model("provider-a", "model-a", 0.7)]),
        &evidence,
    )
    .unwrap();

    assert_eq!(ballot.entries().len(), 1);
    assert_eq!(ballot.entries()[0].output_token_budget, 128);
    assert!(matches!(
        ballot.entries()[0].r2_action,
        R2ActionDisposition::Estimated { .. }
    ));
}

#[test]
fn applied_r2_predecessor_includes_constraint_overhead_in_risk_and_reservation() {
    let mut evidence = evidence();
    evidence.r2 = frozen_r2(
        vec![head("provider-a", "model-a", 128, -10.0, 0.0, 8)],
        0.5,
        512,
    );

    let ballot = select(
        &context(512),
        &policy(),
        &CatalogSnapshot::new(vec![model("provider-a", "model-a", 0.99)]),
        &evidence,
    )
    .unwrap();
    let selected = &ballot.entries()[0];
    let reservation = quote_trajectory_reservation(&ballot).unwrap();

    assert_eq!(selected.output_token_budget, 512);
    assert!(selected.r2_action.is_predecessor());
    assert!((selected.expected_provider_cost - 0.01136).abs() < 1e-12);
    assert!((selected.provider_cost_upper_bound - 0.01352).abs() < 1e-12);
    assert!((selected.risk - 0.0618).abs() < 1e-12);
    assert!((reservation.amount - 0.01352).abs() < 1e-12);
}

#[test]
fn reservation_quote_sums_each_selected_action_budget_and_constraint_overhead() {
    let mut evidence = evidence();
    evidence.r2 = frozen_r2(
        vec![
            head("provider-a", "model-a", 64, 6.0, 0.0, 8),
            head("provider-b", "model-b", 128, 5.0, 0.0, 8),
        ],
        0.5,
        512,
    );
    let mut policy = policy();
    policy.max_provider_attempts = 2;
    policy.router_cost_upper_bound = 0.0007;

    let ballot = select(
        &context(512),
        &policy,
        &CatalogSnapshot::new(vec![
            model("provider-a", "model-a", 0.7),
            model("provider-b", "model-b", 0.7),
        ]),
        &evidence,
    )
    .unwrap();
    let reservation = quote_trajectory_reservation(&ballot).unwrap();

    let mut action_bounds = ballot
        .entries()
        .iter()
        .map(|entry| (entry.output_token_budget, entry.provider_cost_upper_bound))
        .collect::<Vec<_>>();
    action_bounds.sort_by_key(|(output_budget, _)| *output_budget);
    assert_eq!(
        action_bounds
            .iter()
            .map(|(output_budget, _)| *output_budget)
            .collect::<Vec<_>>(),
        [64, 128]
    );
    assert!((action_bounds[0].1 - 0.00456).abs() < 1e-12);
    assert!((action_bounds[1].1 - 0.00584).abs() < 1e-12);
    assert_eq!(reservation.provider_attempts, 2);
    assert!((reservation.amount - (0.0007 + 0.00456 + 0.00584)).abs() < 1e-12);
}

#[test]
fn mixed_active_evidence_records_missing_no_allowed_anchor_and_ood_fallbacks() {
    let estimated = R2HeadPrediction::Estimated(R2ActionPrediction {
        action: R2ActionIdentity::new(route("provider-a", "estimated"), 128).unwrap(),
        semantic_correctness: PredictionEstimate::estimated(0.9, 0.8).unwrap(),
        expected_output_tokens: PredictionEstimate::expected_cost(64.0, 0.8).unwrap(),
        instruction_input_tokens: 12,
        instruction_input_upper_bound: 128,
    });
    let above_ceiling = R2HeadPrediction::Estimated(R2ActionPrediction {
        action: R2ActionIdentity::new(route("provider-c", "above-ceiling"), 1024).unwrap(),
        semantic_correctness: PredictionEstimate::estimated(0.9, 0.8).unwrap(),
        expected_output_tokens: PredictionEstimate::expected_cost(64.0, 0.8).unwrap(),
        instruction_input_tokens: 12,
        instruction_input_upper_bound: 128,
    });
    let ood = R2HeadPrediction::Abstained(R2ActionAbstention {
        action: R2ActionIdentity::new(route("provider-d", "ood"), 128).unwrap(),
        reason: R2FallbackReason::OutOfDistribution,
        uncertainty: 1.0,
        diagnostics: vec![R2SupportDiagnostic::RequestFeatureAboveMax {
            feature_index: 0,
            feature: "complexity".to_string(),
            observed: 1.5,
            bound: 1.0,
            scaled_distance: 0.5,
            limit: 0.25,
        }],
    });
    let mut evidence = evidence();
    evidence.r2 = FrozenR2Evidence::active(
        provenance("catalog/v7"),
        R2InstructionInputEstimate::new(12, 128).unwrap(),
        vec![estimated, above_ceiling, ood],
    )
    .unwrap();

    let ballot = select(
        &context(512),
        &policy(),
        &CatalogSnapshot::new(vec![
            model("provider-a", "estimated", 0.7),
            model("provider-b", "missing", 0.7),
            model("provider-c", "above-ceiling", 0.7),
            model("provider-d", "ood", 0.7),
        ]),
        &evidence,
    )
    .unwrap();
    let R2Decision::Applied {
        evaluated_actions, ..
    } = &ballot.r2
    else {
        panic!("expected mixed active R2 evidence");
    };

    for (provider_id, model_id, reason) in [
        (
            "provider-b",
            "missing",
            R2FallbackReason::ModelEvidenceMissing,
        ),
        (
            "provider-c",
            "above-ceiling",
            R2FallbackReason::NoAllowedAnchor,
        ),
        ("provider-d", "ood", R2FallbackReason::OutOfDistribution),
    ] {
        assert!(evaluated_actions.iter().any(|action| {
            action.action.route() == &route(provider_id, model_id)
                && matches!(
                    action.disposition,
                    R2EvaluationDisposition::Abstained {
                        reason: actual,
                        ..
                    } if actual == reason
                )
        }));
    }

    let ood_action = evaluated_actions
        .iter()
        .find(|action| action.action.route() == &route("provider-d", "ood"))
        .expect("OOD action is retained as evaluated evidence");
    assert!(matches!(
        &ood_action.disposition,
        R2EvaluationDisposition::Abstained { diagnostics, .. }
            if diagnostics == &vec![R2SupportDiagnostic::RequestFeatureAboveMax {
                feature_index: 0,
                feature: "complexity".to_string(),
                observed: 1.5,
                bound: 1.0,
                scaled_distance: 0.5,
                limit: 0.25,
            }]
    ));
}
