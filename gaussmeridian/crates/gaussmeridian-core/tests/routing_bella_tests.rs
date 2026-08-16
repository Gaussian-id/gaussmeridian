use gaussmeridian_core::routing_policy::{
    bella::{
        BellaCapabilityDecision, BellaEstimatorPolicy, BellaFallbackReason, BellaLearnerState,
        BellaRouteCapability, BellaSkillAssessmentOutcome, BellaUnavailableReason, BellaUseStatus,
        FrozenBellaEvidence, FrozenBellaUnavailable, MeridianSkillProfiler,
        ProfileAbstentionReason, ProfilerPolicy, ProfilerTrainingExample, SkillDefinition,
        SkillEstimate, SkillEstimateAbstentionReason, SkillPosterior, SkillTaxonomy,
        TaskProfileResult, MAX_PROFILE_TOKENS, MAX_RATIONALE_BYTES,
    },
    predictors::RouteIdentity,
    SKILL_DIMENSIONS,
};

fn skill(index: u16, skill_id: &str, minimum_proficiency: f64) -> SkillDefinition {
    SkillDefinition::new(
        index,
        skill_id,
        skill_id.replace('_', " "),
        format!("Controlled definition for {skill_id}."),
        minimum_proficiency,
    )
    .expect("valid skill definition")
}

fn taxonomy() -> SkillTaxonomy {
    SkillTaxonomy::new(
        "skills/v2",
        vec![
            skill(0, "code_synthesis", 0.6),
            skill(1, "mathematical_reasoning", 0.7),
        ],
    )
    .expect("valid taxonomy")
}

fn profiler_policy() -> ProfilerPolicy {
    ProfilerPolicy::new(0.3, 0.05, 5).expect("valid profiler policy")
}

fn training_rows() -> Vec<ProfilerTrainingExample> {
    vec![
        ProfilerTrainingExample::new(
            "code-1",
            "Implement a Rust function and compile the code",
            vec!["code_synthesis".into()],
        )
        .expect("valid row"),
        ProfilerTrainingExample::new(
            "math-1",
            "Prove the algebra theorem with a derivation",
            vec!["mathematical_reasoning".into()],
        )
        .expect("valid row"),
    ]
}

fn profiler() -> MeridianSkillProfiler {
    MeridianSkillProfiler::train(&taxonomy(), training_rows(), profiler_policy())
        .expect("valid profiler")
}

fn route(provider: &str, model: &str) -> RouteIdentity {
    RouteIdentity::new(provider, model).expect("valid route")
}

fn posterior(
    route: RouteIdentity,
    skill_id: &str,
    positive_count: u64,
    negative_count: u64,
) -> SkillPosterior {
    SkillPosterior::new(
        route,
        "skills/v2",
        skill_id,
        1.0,
        1.0,
        positive_count,
        negative_count,
    )
    .expect("valid posterior")
}

fn estimator_policy() -> BellaEstimatorPolicy {
    BellaEstimatorPolicy::new(3, 0.0).expect("valid estimator policy")
}

fn learner(posteriors: Vec<SkillPosterior>) -> BellaLearnerState {
    BellaLearnerState::new(
        taxonomy(),
        profiler(),
        "controlled-skill-critic/v2",
        "semantic-judge/v2",
        "bella-controlled-corpus/v1",
        "catalog/v7",
        estimator_policy(),
        posteriors,
    )
    .expect("valid learner")
}

#[test]
fn taxonomy_rejects_blank_duplicate_and_out_of_range_skill_identity() {
    assert!(SkillDefinition::new(0, "", "Code", "Description", 0.6).is_err());
    assert!(SkillDefinition::new(0, "code", "", "Description", 0.6).is_err());
    assert!(SkillDefinition::new(0, "code", "Code", "", 0.6).is_err());
    assert!(SkillDefinition::new(0, "code", "Code", "Description", 1.01).is_err());
    assert!(SkillDefinition::new(
        SKILL_DIMENSIONS as u16,
        "overflow",
        "Overflow",
        "Description",
        0.6,
    )
    .is_err());

    assert!(SkillTaxonomy::new("", vec![skill(0, "code", 0.6)]).is_err());
    assert!(SkillTaxonomy::new(
        "skills/v2",
        vec![skill(0, "duplicate", 0.6), skill(1, "duplicate", 0.7)],
    )
    .is_err());
    assert!(SkillTaxonomy::new(
        "skills/v2",
        vec![skill(0, "code", 0.6), skill(0, "math", 0.7)],
    )
    .is_err());
}

#[test]
fn profiler_policy_and_training_rows_are_bounded_and_validated() {
    assert!(ProfilerPolicy::new(f64::NAN, 0.05, 5).is_err());
    assert!(ProfilerPolicy::new(0.3, -0.01, 5).is_err());
    assert!(ProfilerPolicy::new(0.3, 0.05, 0).is_err());
    assert!(ProfilerTrainingExample::new("", "task", vec!["code".into()]).is_err());
    assert!(ProfilerTrainingExample::new("task", "", vec!["code".into()]).is_err());
    assert!(ProfilerTrainingExample::new("task", "task", Vec::new()).is_err());

    let duplicate_rows = vec![
        ProfilerTrainingExample::new("same", "one", vec!["code_synthesis".into()]).unwrap(),
        ProfilerTrainingExample::new("same", "two", vec!["code_synthesis".into()]).unwrap(),
    ];
    assert!(MeridianSkillProfiler::train(&taxonomy(), duplicate_rows, profiler_policy()).is_err());

    let unknown_skill =
        vec![
            ProfilerTrainingExample::new("unknown", "task", vec!["not_in_taxonomy".into()])
                .unwrap(),
        ];
    assert!(MeridianSkillProfiler::train(&taxonomy(), unknown_skill, profiler_policy()).is_err());
}

#[test]
fn deserialized_profiler_and_posterior_inputs_cannot_bypass_validation() {
    let malformed_row: ProfilerTrainingExample = serde_json::from_value(serde_json::json!({
        "example_id": "",
        "task": "implement code",
        "skill_ids": ["code_synthesis"]
    }))
    .unwrap();
    let valid_math_row = ProfilerTrainingExample::new(
        "math",
        "prove algebra",
        vec!["mathematical_reasoning".into()],
    )
    .unwrap();
    assert!(MeridianSkillProfiler::train(
        &taxonomy(),
        vec![malformed_row, valid_math_row],
        profiler_policy()
    )
    .is_err());

    let mut blank_route = serde_json::to_value(posterior(
        route("provider-a", "model-a"),
        "code_synthesis",
        5,
        0,
    ))
    .unwrap();
    blank_route["route"]["provider_id"] = serde_json::Value::String(String::new());
    let blank_route: SkillPosterior = serde_json::from_value(blank_route).unwrap();
    assert!(BellaLearnerState::new(
        taxonomy(),
        profiler(),
        "critic/v2",
        "evaluator/v2",
        "corpus/v1",
        "catalog/v7",
        estimator_policy(),
        vec![blank_route],
    )
    .is_err());

    let mut denormalized_profiler = serde_json::to_value(profiler()).unwrap();
    let weights = denormalized_profiler["centroids"][0]["weights"]
        .as_array_mut()
        .unwrap();
    for weight in weights {
        *weight = serde_json::json!(0.0);
    }
    let denormalized_profiler: MeridianSkillProfiler =
        serde_json::from_value(denormalized_profiler).unwrap();
    assert!(BellaLearnerState::new(
        taxonomy(),
        denormalized_profiler,
        "critic/v2",
        "evaluator/v2",
        "corpus/v1",
        "catalog/v7",
        estimator_policy(),
        Vec::new(),
    )
    .is_err());
}

#[test]
fn profiler_training_is_row_order_independent_and_serializes_canonically() {
    let mut reversed = training_rows();
    reversed.reverse();

    let first =
        MeridianSkillProfiler::train(&taxonomy(), training_rows(), profiler_policy()).unwrap();
    let second = MeridianSkillProfiler::train(&taxonomy(), reversed, profiler_policy()).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
}

#[test]
fn profiler_generates_cosine_requirements_and_a_stable_bounded_fingerprint() {
    let profiler = profiler();
    let result = profiler.profile("Please implement and compile this Rust function.");
    let profile = match result {
        TaskProfileResult::Profiled(profile) => profile,
        other => panic!("expected a profile, got {other:?}"),
    };

    assert_eq!(profile.profiler_version, "meridian-skill-profiler/v1");
    assert_eq!(profile.taxonomy_version, "skills/v2");
    assert_eq!(profile.task_fingerprint.len(), 64);
    assert_eq!(profile.requirements.len(), 1);
    assert_eq!(profile.requirements[0].skill_id, "code_synthesis");
    assert!(profile.requirements[0].similarity >= 0.3);
    assert!(profile.requirements[0].rationale.len() <= MAX_RATIONALE_BYTES);

    let bounded_prefix = "implement ".repeat(MAX_PROFILE_TOKENS);
    let first = profiler.profile(&(bounded_prefix.clone() + "ignored suffix one"));
    let second = profiler.profile(&(bounded_prefix + "different ignored suffix"));
    assert_eq!(first.task_fingerprint(), second.task_fingerprint());
}

#[test]
fn profiler_has_typed_no_overlap_and_ambiguous_abstention() {
    let no_overlap = profiler().profile("photosynthesis chlorophyll");
    assert_eq!(
        no_overlap.abstention_reason(),
        Some(ProfileAbstentionReason::NoVocabularyOverlap)
    );

    let ambiguous_taxonomy = SkillTaxonomy::new(
        "skills/v2",
        vec![skill(0, "first", 0.5), skill(1, "second", 0.5)],
    )
    .unwrap();
    let rows = vec![
        ProfilerTrainingExample::new("first", "shared alpha", vec!["first".into()]).unwrap(),
        ProfilerTrainingExample::new("second", "shared beta", vec!["second".into()]).unwrap(),
    ];
    let ambiguous_profiler = MeridianSkillProfiler::train(
        &ambiguous_taxonomy,
        rows,
        ProfilerPolicy::new(0.99, 0.05, 3).unwrap(),
    )
    .unwrap();

    assert_eq!(
        ambiguous_profiler.profile("shared").abstention_reason(),
        Some(ProfileAbstentionReason::AmbiguousSkillMatch)
    );
}

#[test]
fn beta_posterior_matches_the_exact_mean_variance_and_lcb_formula() {
    let posterior = posterior(route("provider-a", "model-a"), "code_synthesis", 3, 1);
    let estimate = posterior
        .estimate(&BellaEstimatorPolicy::new(1, 1.0).unwrap())
        .expect("valid estimate");

    let SkillEstimate::Estimated {
        support,
        mean,
        variance,
        uncertainty,
        conservative_proficiency,
        ..
    } = estimate
    else {
        panic!("expected an estimate");
    };
    let expected_variance = 8.0 / 252.0;
    let expected_uncertainty = f64::sqrt(expected_variance);

    assert_eq!(support, 4);
    assert!((mean - (2.0 / 3.0)).abs() < 1e-12);
    assert!((variance - expected_variance).abs() < 1e-12);
    assert!((uncertainty - expected_uncertainty).abs() < 1e-12);
    assert!((conservative_proficiency - ((2.0 / 3.0) - expected_uncertainty)).abs() < 1e-12);
}

#[test]
fn posterior_rejects_invalid_parameters_and_abstains_below_minimum_support() {
    assert!(SkillPosterior::new(
        route("provider-a", "model-a"),
        "skills/v2",
        "code_synthesis",
        0.0,
        1.0,
        1,
        0,
    )
    .is_err());
    assert!(SkillPosterior::new(
        route("provider-a", "model-a"),
        "skills/v2",
        "code_synthesis",
        1.0,
        1.0,
        u64::MAX,
        1,
    )
    .is_err());
    assert!(BellaEstimatorPolicy::new(0, 1.0).is_err());
    assert!(BellaEstimatorPolicy::new(3, -0.1).is_err());

    let estimate = posterior(route("provider-a", "model-a"), "code_synthesis", 1, 0)
        .estimate(&BellaEstimatorPolicy::new(3, 1.0).unwrap())
        .unwrap();
    assert!(matches!(
        estimate,
        SkillEstimate::Abstained {
            reason: SkillEstimateAbstentionReason::InsufficientSupport,
            support: 1,
            ..
        }
    ));
}

#[test]
fn learner_rejects_duplicate_route_skill_and_version_mismatches() {
    let duplicate = posterior(route("provider-a", "model-a"), "code_synthesis", 5, 0);
    assert!(BellaLearnerState::new(
        taxonomy(),
        profiler(),
        "critic/v2",
        "evaluator/v2",
        "corpus/v1",
        "catalog/v7",
        estimator_policy(),
        vec![duplicate.clone(), duplicate],
    )
    .is_err());

    let other_taxonomy =
        SkillTaxonomy::new("skills/v3", vec![skill(0, "code_synthesis", 0.6)]).unwrap();
    assert!(BellaLearnerState::new(
        other_taxonomy,
        profiler(),
        "critic/v2",
        "evaluator/v2",
        "corpus/v1",
        "catalog/v7",
        estimator_policy(),
        Vec::new(),
    )
    .is_err());

    let wrong_taxonomy = SkillPosterior::new(
        route("provider-a", "model-a"),
        "skills/v3",
        "code_synthesis",
        1.0,
        1.0,
        5,
        0,
    )
    .unwrap();
    assert!(BellaLearnerState::new(
        taxonomy(),
        profiler(),
        "critic/v2",
        "evaluator/v2",
        "corpus/v1",
        "catalog/v7",
        estimator_policy(),
        vec![wrong_taxonomy],
    )
    .is_err());
}

#[test]
fn learner_training_hash_is_canonical_and_tamper_evident() {
    let first = learner(vec![
        posterior(route("provider-b", "model-b"), "code_synthesis", 4, 1),
        posterior(
            route("provider-a", "model-a"),
            "mathematical_reasoning",
            5,
            0,
        ),
    ]);
    let second = learner(vec![
        posterior(
            route("provider-a", "model-a"),
            "mathematical_reasoning",
            5,
            0,
        ),
        posterior(route("provider-b", "model-b"), "code_synthesis", 4, 1),
    ]);
    assert_eq!(
        first.training_content_hash(),
        second.training_content_hash()
    );
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );

    let mut payload = serde_json::to_value(&first).unwrap();
    payload["training_content_hash"] = serde_json::Value::String("0".repeat(64));
    let tampered: BellaLearnerState = serde_json::from_value(payload).unwrap();
    assert!(tampered.validate().is_err());
}

#[test]
fn runtime_catalog_mismatch_is_typed_and_preserves_the_predecessor_set() {
    let state = learner(vec![posterior(
        route("provider-a", "model-a"),
        "code_synthesis",
        8,
        0,
    )]);
    let frozen = state.freeze(
        "implement rust code",
        "state-id-1",
        "different-catalog-version",
    );
    let hard_eligible = vec![
        route("provider-b", "model-b"),
        route("provider-a", "model-a"),
    ];
    let FrozenBellaEvidence::Unavailable(unavailable) = &frozen else {
        panic!("catalog mismatch must freeze unavailable evidence");
    };
    assert_eq!(
        unavailable.critic_version.as_deref(),
        Some("controlled-skill-critic/v2")
    );
    let decision = frozen
        .capability_decision(&hard_eligible)
        .expect("typed fallback decision");

    assert_eq!(decision.status(), BellaUseStatus::Unavailable);
    assert_eq!(decision.selected_routes(), hard_eligible.as_slice());
    assert_eq!(
        decision.summary().dominant_fallback_reason,
        Some(BellaFallbackReason::CatalogVersionMismatch)
    );
}

#[test]
fn profiler_abstention_retains_critic_provenance_and_typed_reason() {
    let frozen =
        learner(Vec::new()).freeze("photosynthesis chlorophyll", "state-id-1", "catalog/v7");
    let FrozenBellaEvidence::Abstained(abstained) = &frozen else {
        panic!("unrelated task must freeze a profiler abstention");
    };
    assert_eq!(
        abstained.critic_version.as_deref(),
        Some("controlled-skill-critic/v2")
    );

    let hard_eligible = vec![route("provider-a", "model-a")];
    let decision = frozen
        .capability_decision(&hard_eligible)
        .expect("abstention has a predecessor fallback");
    assert_eq!(
        decision.summary().dominant_fallback_reason,
        Some(BellaFallbackReason::NoVocabularyOverlap)
    );
}

#[test]
fn unavailable_version_and_promotion_reasons_require_state_provenance() {
    for reason in [
        BellaUnavailableReason::EvaluatorVersionMismatch,
        BellaUnavailableReason::CatalogVersionMismatch,
        BellaUnavailableReason::PriceVersionMismatch,
        BellaUnavailableReason::ProductionPromotionBlocked,
    ] {
        let unavailable = FrozenBellaEvidence::Unavailable(FrozenBellaUnavailable {
            state_id: None,
            taxonomy_version: None,
            critic_version: None,
            reason,
        });
        assert!(unavailable.validate().is_err());
    }

    let repository_outage = FrozenBellaEvidence::Unavailable(FrozenBellaUnavailable {
        state_id: None,
        taxonomy_version: None,
        critic_version: None,
        reason: BellaUnavailableReason::RepositoryUnavailable,
    });
    assert!(repository_outage.validate().is_ok());
}

#[test]
fn frozen_estimator_math_is_revalidated_after_deserialization() {
    let frozen = learner(vec![posterior(
        route("provider-a", "model-a"),
        "code_synthesis",
        8,
        0,
    )])
    .freeze("implement rust code", "state-id-1", "catalog/v7");
    let mut payload = serde_json::to_value(frozen).unwrap();
    payload["Active"]["estimates"][0]["Estimated"]["conservative_proficiency"] =
        serde_json::json!(1.0);
    let tampered: FrozenBellaEvidence = serde_json::from_value(payload).unwrap();

    assert!(tampered.validate().is_err());
}

#[test]
fn capability_requires_every_skill_and_preserves_survivor_order() {
    let weak = route("provider-weak", "model-weak");
    let strong_second = route("provider-strong-b", "model-strong-b");
    let strong_first = route("provider-strong-a", "model-strong-a");
    let state = learner(vec![
        posterior(weak.clone(), "code_synthesis", 10, 0),
        posterior(weak.clone(), "mathematical_reasoning", 0, 10),
        posterior(strong_second.clone(), "code_synthesis", 10, 0),
        posterior(strong_second.clone(), "mathematical_reasoning", 10, 0),
        posterior(strong_first.clone(), "code_synthesis", 10, 0),
        posterior(strong_first.clone(), "mathematical_reasoning", 10, 0),
    ]);
    let frozen = state.freeze(
        "Implement Rust code and prove the algebra theorem",
        "state-id-1",
        "catalog/v7",
    );
    assert!(matches!(frozen, FrozenBellaEvidence::Active(_)));

    let hard_eligible = vec![weak, strong_second.clone(), strong_first.clone()];
    let decision = frozen
        .capability_decision(&hard_eligible)
        .expect("valid capability decision");

    assert_eq!(decision.status(), BellaUseStatus::Applied);
    assert_eq!(decision.selected_routes(), &[strong_second, strong_first]);
    assert_eq!(decision.summary().required_skill_count, 2);
    assert_eq!(decision.summary().capable_route_count, 2);
    assert_eq!(decision.assessments().len(), hard_eligible.len());

    let weak_assessment = &decision.assessments()[0];
    assert_eq!(weak_assessment.route(), &hard_eligible[0]);
    assert_eq!(
        weak_assessment.capability(),
        BellaRouteCapability::NotCapable
    );
    assert!(weak_assessment.skills().iter().any(|skill| {
        skill.skill_id() == "mathematical_reasoning"
            && matches!(
                skill.outcome(),
                BellaSkillAssessmentOutcome::BelowThreshold { margin, .. } if *margin < 0.0
            )
    }));

    for assessment in &decision.assessments()[1..] {
        assert_eq!(assessment.capability(), BellaRouteCapability::Capable);
        assert!(assessment.skills().iter().all(|skill| {
            matches!(
                skill.outcome(),
                BellaSkillAssessmentOutcome::MeetsThreshold { margin, .. } if *margin >= 0.0
            )
        }));
    }
}

#[test]
fn no_capable_route_falls_back_to_the_exact_hard_eligible_order() {
    let first = route("provider-a", "model-a");
    let second = route("provider-b", "model-b");
    let state = learner(vec![
        posterior(first.clone(), "code_synthesis", 0, 10),
        posterior(first.clone(), "mathematical_reasoning", 0, 10),
        posterior(second.clone(), "code_synthesis", 0, 10),
        posterior(second.clone(), "mathematical_reasoning", 0, 10),
    ]);
    let frozen = state.freeze(
        "Implement Rust code and prove the algebra theorem",
        "state-id-1",
        "catalog/v7",
    );
    let hard_eligible = vec![second, first];
    let decision = frozen.capability_decision(&hard_eligible).unwrap();

    assert_eq!(decision.status(), BellaUseStatus::NoCapableFallback);
    assert_eq!(decision.selected_routes(), hard_eligible.as_slice());
    assert_eq!(decision.summary().capable_route_count, 0);
    assert_eq!(
        decision.summary().dominant_fallback_reason,
        Some(BellaFallbackReason::BelowProficiencyThreshold)
    );
}

#[test]
fn dominant_fallback_reason_is_frequency_based_and_order_independent() {
    let insufficient_a = route("provider-insufficient-a", "model-insufficient-a");
    let insufficient_b = route("provider-insufficient-b", "model-insufficient-b");
    let missing = route("provider-missing", "model-missing");
    let frozen = learner(vec![
        posterior(insufficient_a.clone(), "code_synthesis", 0, 0),
        posterior(insufficient_b.clone(), "code_synthesis", 0, 0),
    ])
    .freeze("Implement Rust code", "state-id-1", "catalog/v7");

    for hard_eligible in [
        vec![
            missing.clone(),
            insufficient_b.clone(),
            insufficient_a.clone(),
        ],
        vec![
            insufficient_a.clone(),
            insufficient_b.clone(),
            missing.clone(),
        ],
    ] {
        let decision = frozen.capability_decision(&hard_eligible).unwrap();
        assert_eq!(
            decision.summary().dominant_fallback_reason,
            Some(BellaFallbackReason::InsufficientSupport)
        );
    }

    let tied = frozen
        .capability_decision(&[missing, insufficient_a])
        .unwrap();
    assert_eq!(
        tied.summary().dominant_fallback_reason,
        Some(BellaFallbackReason::MissingEvidence),
        "missing evidence wins the documented severity tie-break"
    );
}

#[test]
fn optional_audit_provenance_fields_preserve_legacy_v6_replay() {
    let frozen =
        learner(Vec::new()).freeze("photosynthesis chlorophyll", "state-id-1", "catalog/v7");
    let mut evidence_payload = serde_json::to_value(&frozen).unwrap();
    evidence_payload["Abstained"]
        .as_object_mut()
        .unwrap()
        .remove("critic_version");
    let replayed_evidence: FrozenBellaEvidence =
        serde_json::from_value(evidence_payload).expect("legacy v6 evidence replays");
    assert!(replayed_evidence.validate().is_ok());
    assert_eq!(replayed_evidence.critic_version(), None);

    let decision = frozen
        .capability_decision(&[route("provider-a", "model-a")])
        .unwrap();
    let mut decision_payload = serde_json::to_value(&decision).unwrap();
    decision_payload["summary"]
        .as_object_mut()
        .unwrap()
        .remove("dominant_fallback_reason");
    let replayed_decision: BellaCapabilityDecision =
        serde_json::from_value(decision_payload).expect("legacy v6 decision replays");
    assert_eq!(replayed_decision.summary().dominant_fallback_reason, None);
}

#[test]
fn capability_audit_distinguishes_insufficient_support_from_missing_evidence() {
    let capable = route("provider-capable", "model-capable");
    let insufficient = route("provider-insufficient", "model-insufficient");
    let missing = route("provider-missing", "model-missing");
    let state = learner(vec![
        posterior(capable.clone(), "code_synthesis", 10, 0),
        posterior(insufficient.clone(), "code_synthesis", 0, 0),
    ]);
    let frozen = state.freeze("Implement Rust code", "state-id-1", "catalog/v7");
    let decision = frozen
        .capability_decision(&[capable, insufficient.clone(), missing.clone()])
        .expect("valid capability decision");

    let insufficient_assessment = decision
        .assessments()
        .iter()
        .find(|assessment| assessment.route() == &insufficient)
        .expect("insufficient route is audited");
    assert!(matches!(
        insufficient_assessment.skills()[0].outcome(),
        BellaSkillAssessmentOutcome::Abstained {
            reason: SkillEstimateAbstentionReason::InsufficientSupport,
            ..
        }
    ));

    let missing_assessment = decision
        .assessments()
        .iter()
        .find(|assessment| assessment.route() == &missing)
        .expect("missing route is audited");
    assert!(matches!(
        missing_assessment.skills()[0].outcome(),
        BellaSkillAssessmentOutcome::MissingEvidence
    ));
}
