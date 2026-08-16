use gaussmeridian_core::routing_policy::{
    bella::{
        BellaEstimatorPolicy, BellaLearnerState, FrozenBellaEvidence, MeridianSkillProfiler,
        ProfilerPolicy, ProfilerTrainingExample, SkillDefinition, SkillPosterior, SkillTaxonomy,
    },
    compound::{CompoundFallbackReason, FrozenCompoundEvidence},
    predictors::{
        FrozenPredictionEvidence, FrozenPredictionSet, PredictionEstimate, PredictionFeatureVector,
        PredictionProvenance, RouteIdentity, RoutePrediction,
    },
    r2::{
        FrozenR2Evidence, R2ActionIdentity, R2AnchorHead, R2EstimatorPolicy, R2FallbackReason,
        R2InstructionInputEstimate, R2LearnerState, R2Provenance, R2SharedEncoder,
        R2_INSTRUCTION_VERSION, R2_PREDICTOR_VERSION,
    },
    select,
    snapshot::{RoutingInputSnapshot, SnapshotDegradation, SnapshotError},
    CapabilityBand, CatalogModel, CatalogSnapshot, DeploymentKind, EvidenceSnapshot, Price,
    ProjectPolicy, RoutingContext,
};
use gaussmeridian_core::{
    ComplexityEvidence, ComplexitySignalEvidence, ComplexitySignalKind,
    MERIDIAN_COMPLEXITY_V2_VERSION,
};

fn sample_complexity_evidence() -> ComplexityEvidence {
    ComplexityEvidence {
        estimator_version: MERIDIAN_COMPLEXITY_V2_VERSION.to_string(),
        score: 0.8,
        estimated_input_tokens: 1_200,
        signals: vec![
            ComplexitySignalEvidence {
                kind: ComplexitySignalKind::InputLoad,
                normalized_value: 0.5,
                weight: 0.1,
                contribution: 0.05,
            },
            ComplexitySignalEvidence {
                kind: ComplexitySignalKind::TaskOperatorDemand,
                normalized_value: 1.0,
                weight: 0.2,
                contribution: 0.2,
            },
            ComplexitySignalEvidence {
                kind: ComplexitySignalKind::FormalObjectDemand,
                normalized_value: 1.0,
                weight: 0.2,
                contribution: 0.2,
            },
            ComplexitySignalEvidence {
                kind: ComplexitySignalKind::ConstraintCoupling,
                normalized_value: 0.5,
                weight: 0.15,
                contribution: 0.075,
            },
            ComplexitySignalEvidence {
                kind: ComplexitySignalKind::OutputVerificationContract,
                normalized_value: 0.5,
                weight: 0.15,
                contribution: 0.075,
            },
            ComplexitySignalEvidence {
                kind: ComplexitySignalKind::DemandInteraction,
                normalized_value: 1.0,
                weight: 0.2,
                contribution: 0.2,
            },
        ],
        active_instruction: None,
    }
}

fn sample_input() -> RoutingInputSnapshot {
    RoutingInputSnapshot {
        schema_version: "routing-input/v2".to_string(),
        project_id: "project-alpha".to_string(),
        request: RoutingContext {
            complexity: 0.8,
            estimated_input_tokens: 1_200,
            input_token_upper_bound: 1_200,
            output_token_budget: 600,
            hard_skills: Vec::new(),
        },
        policy: ProjectPolicy {
            cost_weight: 0.25,
            quality_floor: 0.8,
            max_band: CapabilityBand::Frontier,
            moderate_complexity_threshold: 0.35,
            high_complexity_threshold: 0.7,
            max_provider_attempts: 3,
            router_cost_upper_bound: 0.001,
        },
        catalog: CatalogSnapshot::new(vec![CatalogModel {
            model_id: "frontier-model".to_string(),
            provider_id: "provider-a".to_string(),
            model_version: "model-v1".to_string(),
            capability_band: CapabilityBand::Frontier,
            deployment_kind: DeploymentKind::Managed,
            price: Price {
                input_per_million: 3.0,
                output_per_million: 15.0,
                expected_fixed_cost: 0.0,
                fixed_cost_upper_bound: 0.0,
            },
            semantic_quality_prior: 0.93,
            transport_success_probability: 0.99,
            credential_available: true,
            adapter_registered: true,
            adapter_supports_model: true,
            tenant_allowed: true,
            compliant: true,
            skill_proficiency: [0.9; 12],
        }]),
        evidence: EvidenceSnapshot {
            policy_version: "policy-v3".to_string(),
            catalog_version: "catalog-v7".to_string(),
            price_version: "prices-v5".to_string(),
            evaluator_version: "evaluator-v2".to_string(),
            normalized_cost_floor: 0.0,
            normalized_cost_ceiling: 1.0,
            predictions: Default::default(),
            complexity: None,
            bella: Default::default(),
            r2: Default::default(),
            compound: Default::default(),
        },
        feature_version: "meridian-features-v1".to_string(),
        degradations: vec![SnapshotDegradation::AdvisoryEvidenceUnavailable {
            source: "skill-evidence".to_string(),
            fallback_version: "skill-prior-v1".to_string(),
        }],
    }
}

fn bella_taxonomy(minimum_proficiency: f64) -> SkillTaxonomy {
    SkillTaxonomy::new(
        "skills/v1",
        vec![SkillDefinition::new(
            0,
            "code_synthesis",
            "Code synthesis",
            "Implement and compile code.",
            minimum_proficiency,
        )
        .expect("valid skill")],
    )
    .expect("valid taxonomy")
}

fn bella_profiler(minimum_proficiency: f64) -> MeridianSkillProfiler {
    MeridianSkillProfiler::train(
        &bella_taxonomy(minimum_proficiency),
        vec![ProfilerTrainingExample::new(
            "code",
            "Implement and compile Rust code",
            vec!["code_synthesis".into()],
        )
        .expect("valid training example")],
        ProfilerPolicy::new(0.2, 0.05, 4).expect("valid profiler policy"),
    )
    .expect("valid profiler")
}

fn bella_posterior(
    provider_id: &str,
    model_id: &str,
    positive: u64,
    negative: u64,
) -> SkillPosterior {
    SkillPosterior::new(
        RouteIdentity::new(provider_id, model_id).expect("valid route"),
        "skills/v1",
        "code_synthesis",
        1.0,
        1.0,
        positive,
        negative,
    )
    .expect("valid posterior")
}

fn bella_state(
    minimum_proficiency: f64,
    corpus_version: &str,
    posteriors: Vec<SkillPosterior>,
) -> BellaLearnerState {
    BellaLearnerState::new(
        bella_taxonomy(minimum_proficiency),
        bella_profiler(minimum_proficiency),
        "controlled-skill-critic/v1",
        "evaluator-v2",
        corpus_version,
        "catalog-v7",
        BellaEstimatorPolicy::new(3, 0.0).expect("valid estimator policy"),
        posteriors,
    )
    .expect("valid learner state")
}

fn v6_input(state: &BellaLearnerState, prompt: &str) -> RoutingInputSnapshot {
    let mut input = sample_input();
    input.schema_version = "routing-input/v6".into();
    input.evidence.complexity = Some(sample_complexity_evidence());
    input.evidence.bella = state.freeze(prompt, "bella-state/v1", "catalog-v7");
    assert!(matches!(
        input.evidence.bella,
        FrozenBellaEvidence::Active(_)
    ));
    input
}

fn r2_state(quality_bias: f64) -> R2LearnerState {
    R2LearnerState::new(
        R2Provenance::new(
            R2_PREDICTOR_VERSION,
            "meridian-r2-encoder/v1",
            "routing-features/v2",
            "evaluator-v2",
            "routing-p4-corpus/v1",
            "catalog-v7",
            "prices-v5",
            R2_INSTRUCTION_VERSION,
            "routing-r2-label/v1",
            "a".repeat(64),
            "b".repeat(64),
        )
        .expect("valid R2 provenance"),
        R2SharedEncoder::new(vec![0.0], vec![1.0], vec![vec![0.0]], vec![0.0])
            .expect("valid R2 encoder"),
        vec![R2AnchorHead::new(
            R2ActionIdentity::new(
                RouteIdentity::new("provider-a", "frontier-model").expect("valid route"),
                128,
            )
            .expect("valid action"),
            vec![0.0],
            quality_bias,
            vec![0.0],
            0.0,
            12,
            128,
            12,
            0.02,
            0.02,
        )
        .expect("valid R2 head")],
        R2EstimatorPolicy::new(4, 0.25, 0.10, 0.10).expect("valid R2 policy"),
        vec![0.0],
        vec![1.0],
    )
    .expect("valid R2 learner")
}

fn v7_input(quality_bias: f64) -> RoutingInputSnapshot {
    let state = r2_state(quality_bias);
    let predictions = state
        .predict(
            &PredictionFeatureVector::new("routing-features/v2", vec![0.5])
                .expect("valid R2 features"),
            600,
        )
        .expect("valid R2 prediction");
    let mut input = sample_input();
    input.schema_version = "routing-input/v7".into();
    input.evidence.complexity = Some(sample_complexity_evidence());
    input.evidence.r2 = FrozenR2Evidence::active(
        state.provenance().clone(),
        R2InstructionInputEstimate::new(12, 128).expect("valid predecessor instruction"),
        predictions,
    )
    .expect("valid frozen R2 evidence");
    input
}

#[test]
fn v5_complexity_signal_evidence_is_part_of_the_snapshot_fingerprint() {
    let mut baseline = sample_input();
    baseline.schema_version = "routing-input/v5".to_string();
    baseline.evidence.complexity = Some(sample_complexity_evidence());
    let baseline = baseline.freeze().expect("v5 evidence freezes");

    let mut changed = sample_input();
    changed.schema_version = "routing-input/v5".to_string();
    let mut changed_evidence = sample_complexity_evidence();
    changed_evidence.signals[0].normalized_value = 0.6;
    changed_evidence.signals[0].contribution = 0.06;
    changed_evidence.score = 0.81;
    changed.request.complexity = 0.81;
    changed.evidence.complexity = Some(changed_evidence);
    let changed = changed.freeze().expect("changed v5 evidence freezes");

    assert_ne!(baseline.fingerprint, changed.fingerprint);
    assert_ne!(baseline.canonical_payload, changed.canonical_payload);
}

#[test]
fn v5_rejects_missing_or_request_mismatched_complexity_evidence() {
    let mut missing = sample_input();
    missing.schema_version = "routing-input/v5".to_string();
    assert!(missing.freeze().is_err(), "v5 requires frozen evidence");

    let mut score_mismatch = sample_input();
    score_mismatch.schema_version = "routing-input/v5".to_string();
    score_mismatch.request.complexity = 0.7;
    score_mismatch.evidence.complexity = Some(sample_complexity_evidence());
    assert!(
        score_mismatch.freeze().is_err(),
        "request score must equal generated evidence"
    );

    let mut token_mismatch = sample_input();
    token_mismatch.schema_version = "routing-input/v5".to_string();
    token_mismatch.request.estimated_input_tokens = 1_199;
    token_mismatch.evidence.complexity = Some(sample_complexity_evidence());
    assert!(
        token_mismatch.freeze().is_err(),
        "request tokens must equal generated evidence"
    );
}

#[test]
fn v5_rejects_malformed_complexity_math() {
    let mut malformed = Vec::new();

    let mut duplicate = sample_complexity_evidence();
    duplicate.signals[1].kind = ComplexitySignalKind::InputLoad;
    malformed.push(duplicate);

    let mut out_of_range = sample_complexity_evidence();
    out_of_range.signals[0].normalized_value = 1.1;
    malformed.push(out_of_range);

    let mut wrong_weight = sample_complexity_evidence();
    wrong_weight.signals[0].weight = 0.2;
    malformed.push(wrong_weight);

    let mut wrong_contribution = sample_complexity_evidence();
    wrong_contribution.signals[0].contribution = 0.06;
    malformed.push(wrong_contribution);

    let mut wrong_score = sample_complexity_evidence();
    wrong_score.score = 0.9;
    malformed.push(wrong_score);

    let mut wrong_version = sample_complexity_evidence();
    wrong_version.estimator_version = "unknown-estimator/v1".to_string();
    malformed.push(wrong_version);

    for evidence in malformed {
        let mut input = sample_input();
        input.schema_version = "routing-input/v5".to_string();
        input.evidence.complexity = Some(evidence);
        assert!(
            input.freeze().is_err(),
            "malformed complexity evidence must never be hashed"
        );
    }
}

#[test]
fn historical_snapshots_without_complexity_evidence_keep_the_field_absent() {
    let input = sample_input();
    let serialized = serde_json::to_value(&input).expect("legacy snapshot serializes");

    assert!(
        serialized["evidence"].get("complexity").is_none(),
        "None must not alter historical canonical bytes"
    );

    let restored: RoutingInputSnapshot =
        serde_json::from_value(serialized).expect("legacy snapshot remains readable");
    assert!(restored.evidence.complexity.is_none());
}

#[test]
fn identical_inputs_freeze_to_identical_canonical_payload_and_fingerprint() {
    let first = sample_input().freeze().expect("valid input freezes");
    let second = sample_input().freeze().expect("same valid input freezes");

    assert_eq!(first.canonical_payload, second.canonical_payload);
    assert_eq!(first.fingerprint, second.fingerprint);
    assert_eq!(
        first.fingerprint.len(),
        64,
        "SHA-256 is rendered as lowercase hex"
    );
}

#[test]
fn non_finite_decision_values_are_rejected_before_hashing() {
    let mut input = sample_input();
    input.policy.cost_weight = f64::NAN;

    let error = input
        .freeze()
        .expect_err("NaN must never become canonical JSON");

    assert!(
        error.to_string().contains("non-finite"),
        "the failure must identify the invalid numeric boundary"
    );
}

#[test]
fn blank_canonical_project_is_rejected_before_hashing() {
    let mut input = sample_input();
    input.project_id = "   ".to_string();

    let error = input
        .freeze()
        .expect_err("a snapshot without canonical project identity is unusable");

    assert!(error.to_string().contains("project_id"));
}

#[test]
fn financial_authority_bounds_are_part_of_the_snapshot_fingerprint() {
    let baseline = sample_input().freeze().unwrap();

    let mut input_ceiling_changed = sample_input();
    input_ceiling_changed.request.input_token_upper_bound += 1;
    let input_ceiling_changed = input_ceiling_changed.freeze().unwrap();

    let mut fixed_cost_changed = sample_input();
    let mut repriced_model = fixed_cost_changed.catalog.models()[0].clone();
    repriced_model.price.fixed_cost_upper_bound = 0.001;
    fixed_cost_changed.catalog = CatalogSnapshot::new(vec![repriced_model]);
    let fixed_cost_changed = fixed_cost_changed.freeze().unwrap();

    assert_ne!(baseline.fingerprint, input_ceiling_changed.fingerprint);
    assert_ne!(baseline.fingerprint, fixed_cost_changed.fingerprint);

    let mut attempt_policy_changed = sample_input();
    attempt_policy_changed.policy.max_provider_attempts += 1;
    let attempt_policy_changed = attempt_policy_changed.freeze().unwrap();

    let mut router_ceiling_changed = sample_input();
    router_ceiling_changed.policy.router_cost_upper_bound += 0.001;
    let router_ceiling_changed = router_ceiling_changed.freeze().unwrap();

    assert_ne!(baseline.fingerprint, attempt_policy_changed.fingerprint);
    assert_ne!(baseline.fingerprint, router_ceiling_changed.fingerprint);
}

#[test]
fn frozen_predictor_state_and_estimates_are_part_of_the_snapshot_fingerprint() {
    let baseline = sample_input().freeze().expect("baseline freezes");
    let mut learned = sample_input();
    learned.evidence.predictions = FrozenPredictionEvidence::Active(
        FrozenPredictionSet::new(
            PredictionProvenance::new(
                "carrot-knn/v1",
                "carrot-runtime-features/v1",
                "evaluator-v2",
                "controlled-corpus/v1",
                "catalog-v7",
                "price-v11",
                "state-sha256",
                "training-sha256",
            )
            .expect("valid provenance"),
            PredictionFeatureVector::new("carrot-runtime-features/v1", vec![0.8, 7.1, 6.4])
                .expect("valid features"),
            vec![RoutePrediction::new(
                RouteIdentity::new("provider-a", "frontier-model").expect("valid route"),
                PredictionEstimate::estimated(0.97, 0.9).expect("valid outcome"),
                PredictionEstimate::expected_cost(0.02, 0.9).expect("valid cost"),
            )
            .expect("valid route prediction")],
        )
        .expect("valid prediction set"),
    );
    let learned = learned.freeze().expect("learned snapshot freezes");

    assert_ne!(baseline.fingerprint, learned.fingerprint);
    assert_ne!(baseline.canonical_payload, learned.canonical_payload);
}

#[test]
fn active_bella_requires_v6() {
    let state = bella_state(
        0.6,
        "bella-corpus/v1",
        vec![bella_posterior("provider-a", "frontier-model", 10, 0)],
    );
    let input = v6_input(&state, "Implement and compile Rust code");
    input.freeze().expect("valid v6 BELLA evidence freezes");

    let mut v5 = input.clone();
    v5.schema_version = "routing-input/v5".into();
    assert!(
        v5.freeze().is_err(),
        "active BELLA must not leak into a historical schema"
    );
}

#[test]
fn v6_revalidates_frozen_bella_evidence() {
    let state = bella_state(
        0.6,
        "bella-corpus/v1",
        vec![bella_posterior("provider-a", "frontier-model", 10, 0)],
    );
    let input = v6_input(&state, "Implement and compile Rust code");
    let mut tampered = input;
    let mut bella = serde_json::to_value(&tampered.evidence.bella).expect("BELLA serializes");
    bella["Active"]["estimates"][0]["Estimated"]["conservative_proficiency"] =
        serde_json::json!(1.0);
    tampered.evidence.bella = serde_json::from_value(bella).expect("BELLA shape remains readable");
    assert!(
        tampered.freeze().is_err(),
        "v6 must reject forged estimator math before hashing"
    );

    let mut stale_catalog = v6_input(&state, "Implement and compile Rust code");
    stale_catalog.evidence.catalog_version = "catalog-v8".into();
    assert!(
        stale_catalog.freeze().is_err(),
        "v6 must reject catalog-incompatible BELLA evidence"
    );

    let mut stale_evaluator = v6_input(&state, "Implement and compile Rust code");
    stale_evaluator.evidence.evaluator_version = "evaluator-v3".into();
    assert!(
        stale_evaluator.freeze().is_err(),
        "v6 must reject evaluator-incompatible BELLA evidence"
    );
}

#[test]
fn v6_profiles_estimates_rationale_and_provenance_are_fingerprint_material() {
    let baseline_state = bella_state(
        0.6,
        "bella-corpus/v1",
        vec![bella_posterior("provider-a", "frontier-model", 10, 0)],
    );
    let baseline = v6_input(&baseline_state, "Implement and compile Rust code");
    let baseline_frozen = baseline.freeze().expect("baseline freezes");

    let threshold_state = bella_state(
        0.7,
        "bella-corpus/v1",
        vec![bella_posterior("provider-a", "frontier-model", 10, 0)],
    );
    let threshold_changed = v6_input(&threshold_state, "Implement and compile Rust code")
        .freeze()
        .expect("changed requirement freezes");

    let estimate_state = bella_state(
        0.6,
        "bella-corpus/v1",
        vec![bella_posterior("provider-a", "frontier-model", 7, 1)],
    );
    let estimate_changed = v6_input(&estimate_state, "Implement and compile Rust code")
        .freeze()
        .expect("changed estimate freezes");

    let mut rationale_changed = baseline.clone();
    let mut rationale =
        serde_json::to_value(&rationale_changed.evidence.bella).expect("BELLA serializes");
    rationale["Active"]["profile"]["requirements"][0]["rationale"] =
        serde_json::json!("implement + compile");
    rationale_changed.evidence.bella =
        serde_json::from_value(rationale).expect("valid rationale shape");
    let rationale_changed = rationale_changed
        .freeze()
        .expect("changed bounded rationale freezes");

    let provenance_state = bella_state(
        0.6,
        "bella-corpus/v2",
        vec![bella_posterior("provider-a", "frontier-model", 10, 0)],
    );
    let provenance_changed = v6_input(&provenance_state, "Implement and compile Rust code")
        .freeze()
        .expect("changed provenance freezes");

    for changed in [
        threshold_changed,
        estimate_changed,
        rationale_changed,
        provenance_changed,
    ] {
        assert_ne!(baseline_frozen.fingerprint, changed.fingerprint);
        assert_ne!(baseline_frozen.canonical_payload, changed.canonical_payload);
    }
}

#[test]
fn v6_canonical_learner_order_has_one_payload_and_fingerprint() {
    let first_route = bella_posterior("provider-a", "frontier-model", 10, 0);
    let second_route = bella_posterior("provider-b", "other-model", 8, 1);
    let first = bella_state(
        0.6,
        "bella-corpus/v1",
        vec![first_route.clone(), second_route.clone()],
    );
    let reordered = bella_state(0.6, "bella-corpus/v1", vec![second_route, first_route]);

    let first = v6_input(&first, "Implement and compile Rust code")
        .freeze()
        .expect("first freezes");
    let reordered = v6_input(&reordered, "Implement and compile Rust code")
        .freeze()
        .expect("reordered freezes");

    assert_eq!(first.fingerprint, reordered.fingerprint);
    assert_eq!(first.canonical_payload, reordered.canonical_payload);
}

#[test]
fn v7_requires_noninactive_valid_and_authority_compatible_r2_evidence() {
    let input = v7_input(0.0);
    input.freeze().expect("valid v7 R2 evidence freezes");

    let mut inactive = input.clone();
    inactive.evidence.r2 = FrozenR2Evidence::default();
    assert!(matches!(
        inactive.freeze().unwrap_err(),
        SnapshotError::InvalidR2 { .. }
    ));

    let mut historical_schema = input.clone();
    historical_schema.schema_version = "routing-input/v6".into();
    let bella = bella_state(
        0.6,
        "bella-corpus/v1",
        vec![bella_posterior("provider-a", "frontier-model", 10, 0)],
    );
    historical_schema.evidence.bella = v6_input(&bella, "Implement and compile Rust code")
        .evidence
        .bella;
    assert!(matches!(
        historical_schema.freeze().unwrap_err(),
        SnapshotError::InvalidR2 { .. }
    ));

    for (field, value) in [
        ("catalog", "catalog-v8"),
        ("price", "prices-v6"),
        ("evaluator", "evaluator-v3"),
    ] {
        let mut stale = input.clone();
        match field {
            "catalog" => stale.evidence.catalog_version = value.into(),
            "price" => stale.evidence.price_version = value.into(),
            "evaluator" => stale.evidence.evaluator_version = value.into(),
            _ => unreachable!(),
        }
        assert!(matches!(
            stale.freeze().unwrap_err(),
            SnapshotError::InvalidR2 { .. }
        ));
    }

    for (field, value) in [
        ("feature_version", "routing-features/v3"),
        ("label_version", "routing-r2-label/v2"),
    ] {
        let mut stale = input.clone();
        let mut r2 = serde_json::to_value(&stale.evidence.r2).expect("R2 serializes");
        r2["provenance"][field] = serde_json::json!(value);
        stale.evidence.r2 = serde_json::from_value(r2).expect("R2 shape remains readable");
        assert!(matches!(
            stale.freeze().unwrap_err(),
            SnapshotError::InvalidR2 { .. }
        ));
    }
}

#[test]
fn v7_accepts_typed_unavailable_r2_but_rejects_forged_active_evidence() {
    let mut unavailable = sample_input();
    unavailable.schema_version = "routing-input/v7".into();
    unavailable.evidence.complexity = Some(sample_complexity_evidence());
    unavailable.evidence.r2 =
        FrozenR2Evidence::unavailable(R2FallbackReason::RepositoryUnavailable);
    unavailable
        .freeze()
        .expect("typed fail-closed R2 evidence freezes");

    let mut forged = v7_input(0.0);
    let mut r2 = serde_json::to_value(&forged.evidence.r2).expect("R2 serializes");
    r2["predictions"][0]["evidence"]["instruction_input_upper_bound"] = serde_json::json!(0);
    forged.evidence.r2 = serde_json::from_value(r2).expect("R2 shape remains readable");
    assert!(matches!(
        forged.freeze().unwrap_err(),
        SnapshotError::InvalidR2 { .. }
    ));
}

#[test]
fn v7_r2_predictions_and_provenance_are_fingerprint_material() {
    let baseline = v7_input(0.0).freeze().expect("baseline freezes");
    let changed = v7_input(1.0).freeze().expect("changed evidence freezes");

    assert_ne!(baseline.fingerprint, changed.fingerprint);
    assert_ne!(baseline.canonical_payload, changed.canonical_payload);
}

#[test]
fn v8_requires_noninactive_compound_evidence_and_older_schemas_reject_it() {
    let mut input = sample_input();
    input.schema_version = "routing-input/v8".into();
    input.evidence.complexity = Some(sample_complexity_evidence());
    input.evidence.compound =
        FrozenCompoundEvidence::unavailable(CompoundFallbackReason::RepositoryUnavailable);
    input
        .freeze()
        .expect("typed fail-closed compound evidence freezes under v8");

    input.schema_version = "routing-input/v5".into();
    assert!(matches!(
        input.freeze().unwrap_err(),
        SnapshotError::InvalidCompound { .. }
    ));
}

#[test]
fn v7_snapshot_and_joint_ballot_replay_byte_identically_with_selected_budget() {
    let input = v7_input(6.0);
    let original_snapshot = input.freeze().expect("v7 input freezes");
    let original_ballot = select(
        &input.request,
        &input.policy,
        &input.catalog,
        &input.evidence,
    )
    .expect("v7 ballot selects");

    let restored: RoutingInputSnapshot =
        serde_json::from_slice(&original_snapshot.canonical_payload).expect("v7 input reads");
    let restored_snapshot = restored.freeze().expect("v7 input refreezes");
    let restored_ballot = select(
        &restored.request,
        &restored.policy,
        &restored.catalog,
        &restored.evidence,
    )
    .expect("restored v7 ballot selects");

    assert_eq!(original_snapshot.fingerprint, restored_snapshot.fingerprint);
    assert_eq!(
        original_snapshot.canonical_payload,
        restored_snapshot.canonical_payload
    );
    assert_eq!(
        original_ballot.content_id().expect("original ballot id"),
        restored_ballot.content_id().expect("restored ballot id")
    );
    assert_eq!(
        serde_json::to_vec(&original_ballot).expect("original ballot serializes"),
        serde_json::to_vec(&restored_ballot).expect("restored ballot serializes")
    );

    let selected = &restored_ballot.entries()[0];
    assert_eq!(selected.output_token_budget, 128);
    assert!(!selected.r2_action.is_predecessor());

    let ballot = serde_json::to_value(restored_ballot).expect("restored ballot serializes");
    assert_eq!(ballot["r2"]["status"], "applied");
    assert_eq!(
        ballot["r2"]["evaluated_actions"][0]["action"]["output_budget"],
        128
    );
    assert_eq!(ballot["r2"]["evaluated_actions"][0]["selected"], true);
}

#[test]
fn historical_v1_through_v6_omit_r2_and_refreeze_byte_identically() {
    for version in 1..=5 {
        let mut historical = sample_input();
        historical.schema_version = format!("routing-input/v{version}");
        if version == 5 {
            historical.evidence.complexity = Some(sample_complexity_evidence());
        }
        let original = historical.freeze().expect("historical input freezes");
        let value = serde_json::to_value(&historical).expect("historical input serializes");
        assert!(
            value["evidence"].get("bella").is_none(),
            "historical v{version} must not gain a BELLA field"
        );
        assert!(
            value["evidence"].get("r2").is_none(),
            "historical v{version} must not gain an R2 field"
        );

        let restored: RoutingInputSnapshot =
            serde_json::from_slice(&original.canonical_payload).expect("historical input reads");
        let restored = restored.freeze().expect("historical input refreezes");
        assert_eq!(original.fingerprint, restored.fingerprint);
        assert_eq!(original.canonical_payload, restored.canonical_payload);
    }

    let state = bella_state(
        0.6,
        "bella-corpus/v1",
        vec![bella_posterior("provider-a", "frontier-model", 10, 0)],
    );
    let historical = v6_input(&state, "Implement and compile Rust code");
    let original = historical.freeze().expect("historical v6 input freezes");
    let value = serde_json::to_value(&historical).expect("historical v6 input serializes");
    assert!(value["evidence"].get("r2").is_none());

    let restored: RoutingInputSnapshot =
        serde_json::from_slice(&original.canonical_payload).expect("historical v6 input reads");
    let restored = restored.freeze().expect("historical v6 input refreezes");
    assert_eq!(original.fingerprint, restored.fingerprint);
    assert_eq!(original.canonical_payload, restored.canonical_payload);
}
