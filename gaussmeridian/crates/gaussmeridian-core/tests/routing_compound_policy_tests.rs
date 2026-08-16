use gaussmeridian_core::routing_policy::{
    compound::{
        realized_reward, CompoundActionCost, CompoundActionKind, CompoundFallbackReason,
        CompoundLineage, CompoundPolicy, CompoundRejectionReason, CompoundRouteAction,
        CompoundShadowDecision, CompoundStep, CompoundTrajectoryCandidate, FrozenCompoundEvidence,
    },
    quote_trajectory_reservation, select, CapabilityBand, CatalogModel, CatalogSnapshot,
    DeploymentKind, EvidenceSnapshot, Price, ProjectPolicy, RoutingContext,
};

fn model(provider: &str, id: &str) -> CatalogModel {
    CatalogModel {
        model_id: id.into(),
        provider_id: provider.into(),
        model_version: "model-v1".into(),
        capability_band: CapabilityBand::Baseline,
        deployment_kind: DeploymentKind::Managed,
        price: Price {
            input_per_million: 1.0,
            output_per_million: 2.0,
            expected_fixed_cost: 0.0,
            fixed_cost_upper_bound: 0.0,
        },
        semantic_quality_prior: 0.9,
        transport_success_probability: 1.0,
        credential_available: true,
        adapter_registered: true,
        adapter_supports_model: true,
        tenant_allowed: true,
        compliant: true,
        skill_proficiency: [1.0; 12],
    }
}

fn context() -> RoutingContext {
    RoutingContext {
        complexity: 0.1,
        estimated_input_tokens: 1_000,
        input_token_upper_bound: 1_000,
        output_token_budget: 500,
        hard_skills: Vec::new(),
    }
}

fn routing_policy() -> ProjectPolicy {
    ProjectPolicy {
        cost_weight: 0.5,
        quality_floor: 0.7,
        max_band: CapabilityBand::Frontier,
        moderate_complexity_threshold: 0.35,
        high_complexity_threshold: 0.70,
        max_provider_attempts: 3,
        router_cost_upper_bound: 0.001,
    }
}

fn compound_policy() -> CompoundPolicy {
    CompoundPolicy {
        version: "compound-policy-v1".into(),
        max_steps: 8,
        max_provider_calls: 4,
        max_synthesis_calls: 1,
        max_trajectory_liability: 2.0,
        reward_constant: 1.0,
        cost_weight: 1.0,
        penalty_cap: 0.9,
    }
}

fn lineage() -> CompoundLineage {
    CompoundLineage {
        learner_state_id: "a".repeat(64),
        learner_version: "xrouter-shadow-state/v1".into(),
        feature_version: "xrouter-shadow-features/v1".into(),
        evaluator_version: "evaluator-v1".into(),
        corpus_version: "p5-controlled-corpus/v1".into(),
        catalog_version: "catalog-v1".into(),
        price_version: "price-v1".into(),
        policy_version: "policy-v1".into(),
        training_content_hash: "b".repeat(64),
    }
}

fn evidence(compound: FrozenCompoundEvidence) -> EvidenceSnapshot {
    EvidenceSnapshot {
        policy_version: "policy-v1".into(),
        catalog_version: "catalog-v1".into(),
        price_version: "price-v1".into(),
        evaluator_version: "evaluator-v1".into(),
        normalized_cost_floor: 0.0,
        normalized_cost_ceiling: 1.0,
        complexity: None,
        predictions: Default::default(),
        bella: Default::default(),
        r2: Default::default(),
        compound,
    }
}

fn cost(provider: f64, tool: f64, selection: f64, synthesis: f64) -> CompoundActionCost {
    CompoundActionCost {
        provider,
        tool,
        selection,
        synthesis,
    }
}

fn route(provider: &str, model: &str, output_budget: u32) -> CompoundRouteAction {
    CompoundRouteAction {
        provider_id: provider.into(),
        model_id: model.into(),
        output_budget,
    }
}

fn step(
    id: &str,
    kind: CompoundActionKind,
    dependencies: &[&str],
    route: Option<CompoundRouteAction>,
    expected_cost: CompoundActionCost,
    cost_upper_bound: CompoundActionCost,
) -> CompoundStep {
    CompoundStep {
        step_id: id.into(),
        kind,
        dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
        route,
        expected_cost,
        cost_upper_bound,
    }
}

fn candidate(
    id: &str,
    success: f64,
    router_cost: f64,
    router_upper_bound: f64,
    steps: Vec<CompoundStep>,
) -> CompoundTrajectoryCandidate {
    CompoundTrajectoryCandidate {
        trajectory_id: id.into(),
        terminal_success_probability: success,
        router_expected_cost: router_cost,
        router_cost_upper_bound: router_upper_bound,
        steps,
    }
}

fn direct(id: &str, success: f64, router_cost: f64) -> CompoundTrajectoryCandidate {
    candidate(
        id,
        success,
        router_cost,
        router_cost,
        vec![step(
            "direct",
            CompoundActionKind::DirectAnswer,
            &[],
            None,
            cost(0.0, 0.0, 0.0, 0.0),
            cost(0.0, 0.0, 0.0, 0.0),
        )],
    )
}

fn active(candidates: Vec<CompoundTrajectoryCandidate>) -> FrozenCompoundEvidence {
    FrozenCompoundEvidence::active(lineage(), compound_policy(), candidates)
        .expect("valid frozen compound evidence")
}

fn catalog() -> CatalogSnapshot {
    CatalogSnapshot::new(vec![
        model("provider-a", "model-a"),
        model("provider-b", "model-b"),
    ])
}

fn selected(compound: FrozenCompoundEvidence) -> CompoundShadowDecision {
    select(
        &context(),
        &routing_policy(),
        &catalog(),
        &evidence(compound),
    )
    .expect("P4 ballot remains available")
    .compound
}

fn rejection_reasons(decision: &CompoundShadowDecision) -> Vec<CompoundRejectionReason> {
    decision
        .rejections()
        .iter()
        .map(|rejection| rejection.reason.clone())
        .collect()
}

#[test]
fn all_compound_action_kinds_are_representable() {
    let kinds = [
        CompoundActionKind::DirectAnswer,
        CompoundActionKind::Delegate,
        CompoundActionKind::ModelCall,
        CompoundActionKind::Select,
        CompoundActionKind::Synthesize,
    ];
    assert_eq!(
        serde_json::to_value(kinds).unwrap(),
        serde_json::json!([
            "direct_answer",
            "delegate",
            "model_call",
            "select",
            "synthesize"
        ])
    );
}

#[test]
fn dependency_graph_rejects_forward_unknown_self_and_duplicate_edges() {
    let cases = [
        (
            "forward",
            vec![
                step(
                    "first",
                    CompoundActionKind::ModelCall,
                    &["second"],
                    Some(route("provider-a", "model-a", 500)),
                    cost(0.1, 0.0, 0.0, 0.0),
                    cost(0.2, 0.0, 0.0, 0.0),
                ),
                step(
                    "second",
                    CompoundActionKind::Select,
                    &["first"],
                    None,
                    cost(0.0, 0.0, 0.01, 0.0),
                    cost(0.0, 0.0, 0.02, 0.0),
                ),
            ],
            CompoundRejectionReason::UnknownOrForwardDependency,
        ),
        (
            "self",
            vec![step(
                "self",
                CompoundActionKind::Select,
                &["self", "self"],
                None,
                cost(0.0, 0.0, 0.01, 0.0),
                cost(0.0, 0.0, 0.02, 0.0),
            )],
            CompoundRejectionReason::DuplicateDependency,
        ),
        (
            "unknown",
            vec![step(
                "select",
                CompoundActionKind::Select,
                &["missing", "also-missing"],
                None,
                cost(0.0, 0.0, 0.01, 0.0),
                cost(0.0, 0.0, 0.02, 0.0),
            )],
            CompoundRejectionReason::UnknownOrForwardDependency,
        ),
    ];

    for (id, steps, expected) in cases {
        let decision = selected(active(vec![candidate(id, 0.9, 0.01, 0.02, steps)]));
        assert!(
            rejection_reasons(&decision).contains(&expected),
            "{id} should be rejected as {expected:?}: {decision:?}"
        );
    }
}

#[test]
fn provider_steps_must_reference_exact_ballot_actions() {
    let absent_route = candidate(
        "absent",
        0.99,
        0.01,
        0.02,
        vec![step(
            "delegate",
            CompoundActionKind::Delegate,
            &[],
            Some(route("hard-excluded", "model-x", 500)),
            cost(0.1, 0.0, 0.0, 0.0),
            cost(0.2, 0.0, 0.0, 0.0),
        )],
    );
    let wrong_budget = candidate(
        "wrong-budget",
        0.99,
        0.01,
        0.02,
        vec![step(
            "call",
            CompoundActionKind::ModelCall,
            &[],
            Some(route("provider-a", "model-a", 499)),
            cost(0.1, 0.0, 0.0, 0.0),
            cost(0.2, 0.0, 0.0, 0.0),
        )],
    );
    let missing_route = candidate(
        "missing-route",
        0.99,
        0.01,
        0.02,
        vec![step(
            "synthesis",
            CompoundActionKind::Synthesize,
            &[],
            None,
            cost(0.0, 0.0, 0.0, 0.1),
            cost(0.0, 0.0, 0.0, 0.2),
        )],
    );

    let decision = selected(active(vec![absent_route, wrong_budget, missing_route]));
    let reasons = rejection_reasons(&decision);
    assert_eq!(
        reasons
            .iter()
            .filter(|reason| **reason == CompoundRejectionReason::ActionNotInBallot)
            .count(),
        2
    );
    assert!(reasons.contains(&CompoundRejectionReason::RouteRequired));
}

#[test]
fn non_provider_steps_reject_route_actions() {
    for kind in [CompoundActionKind::DirectAnswer, CompoundActionKind::Select] {
        let candidate = candidate(
            &format!("{kind:?}"),
            0.9,
            0.01,
            0.02,
            vec![step(
                "invalid",
                kind,
                &[],
                Some(route("provider-a", "model-a", 500)),
                cost(0.1, 0.0, 0.0, 0.0),
                cost(0.2, 0.0, 0.0, 0.0),
            )],
        );
        assert!(rejection_reasons(&selected(active(vec![candidate])))
            .contains(&CompoundRejectionReason::RouteForbidden));
    }
}

#[test]
fn non_provider_steps_reject_provider_cost_without_a_route() {
    let candidate = candidate(
        "hidden-provider-cost",
        0.9,
        0.01,
        0.02,
        vec![step(
            "direct",
            CompoundActionKind::DirectAnswer,
            &[],
            None,
            cost(0.1, 0.0, 0.0, 0.0),
            cost(0.2, 0.0, 0.0, 0.0),
        )],
    );

    assert!(rejection_reasons(&selected(active(vec![candidate])))
        .contains(&CompoundRejectionReason::InvalidActionShape));
}

#[test]
fn policy_bounds_reject_oversized_trajectories() {
    let call = || {
        step(
            "call",
            CompoundActionKind::ModelCall,
            &[],
            Some(route("provider-a", "model-a", 500)),
            cost(0.1, 0.0, 0.0, 0.0),
            cost(0.2, 0.0, 0.0, 0.0),
        )
    };
    let mut too_many_steps = Vec::new();
    for index in 0..9 {
        let mut value = call();
        value.step_id = format!("call-{index}");
        too_many_steps.push(value);
    }
    let mut policy = compound_policy();
    policy.max_provider_calls = 1;
    policy.max_synthesis_calls = 0;
    policy.max_trajectory_liability = 0.1;
    let evidence = FrozenCompoundEvidence::active(
        lineage(),
        policy,
        vec![
            candidate("steps", 0.9, 0.01, 0.02, too_many_steps),
            candidate(
                "calls",
                0.9,
                0.01,
                0.02,
                vec![call(), {
                    let mut second = call();
                    second.step_id = "call-2".into();
                    second
                }],
            ),
            candidate(
                "synthesis",
                0.9,
                0.01,
                0.02,
                vec![
                    step(
                        "source",
                        CompoundActionKind::DirectAnswer,
                        &[],
                        None,
                        cost(0.0, 0.0, 0.0, 0.0),
                        cost(0.0, 0.0, 0.0, 0.0),
                    ),
                    step(
                        "synthesis",
                        CompoundActionKind::Synthesize,
                        &["source"],
                        Some(route("provider-a", "model-a", 500)),
                        cost(0.1, 0.0, 0.0, 0.1),
                        cost(0.2, 0.0, 0.0, 0.2),
                    ),
                ],
            ),
            direct("liability", 0.9, 0.2),
        ],
    )
    .unwrap();
    let reasons = rejection_reasons(&selected(evidence));
    for expected in [
        CompoundRejectionReason::StepLimitExceeded,
        CompoundRejectionReason::ProviderCallLimitExceeded,
        CompoundRejectionReason::SynthesisCallLimitExceeded,
        CompoundRejectionReason::LiabilityLimitExceeded,
    ] {
        assert!(
            reasons.contains(&expected),
            "missing {expected:?}: {reasons:?}"
        );
    }
}

#[test]
fn invalid_compound_values_are_typed_rejections() {
    let invalid_probability = direct("invalid-probability", f64::NAN, 0.01);
    let negative_cost = direct("negative-cost", 0.9, -0.01);
    let zero_budget = candidate(
        "zero-budget",
        0.9,
        0.01,
        0.02,
        vec![step(
            "call",
            CompoundActionKind::ModelCall,
            &[],
            Some(route("provider-a", "model-a", 0)),
            cost(0.1, 0.0, 0.0, 0.0),
            cost(0.2, 0.0, 0.0, 0.0),
        )],
    );
    let expected_exceeds_upper = candidate(
        "bad-upper",
        0.9,
        0.01,
        0.02,
        vec![step(
            "call",
            CompoundActionKind::ModelCall,
            &[],
            Some(route("provider-a", "model-a", 500)),
            cost(0.3, 0.0, 0.0, 0.0),
            cost(0.2, 0.0, 0.0, 0.0),
        )],
    );
    let reasons = rejection_reasons(&selected(active(vec![
        invalid_probability,
        negative_cost,
        zero_budget,
        expected_exceeds_upper,
    ])));
    for expected in [
        CompoundRejectionReason::InvalidProbability,
        CompoundRejectionReason::InvalidCost,
        CompoundRejectionReason::InvalidRoute,
        CompoundRejectionReason::ExpectedCostExceedsUpperBound,
    ] {
        assert!(
            reasons.contains(&expected),
            "missing {expected:?}: {reasons:?}"
        );
    }
}

#[test]
fn expected_cost_accounts_for_every_component() {
    let trajectory = candidate(
        "complete-cost",
        0.9,
        0.01,
        0.02,
        vec![
            step(
                "call",
                CompoundActionKind::ModelCall,
                &[],
                Some(route("provider-a", "model-a", 500)),
                cost(0.10, 0.02, 0.0, 0.0),
                cost(0.20, 0.04, 0.0, 0.0),
            ),
            step(
                "synthesize",
                CompoundActionKind::Synthesize,
                &["call"],
                Some(route("provider-b", "model-b", 500)),
                cost(0.20, 0.03, 0.04, 0.05),
                cost(0.30, 0.06, 0.08, 0.10),
            ),
        ],
    );
    let decision = selected(active(vec![trajectory]));
    let recommendation = decision.recommendation().expect("one valid recommendation");
    assert!((recommendation.expected_total_cost - 0.45).abs() < f64::EPSILON);
    assert!((recommendation.liability_upper_bound - 0.80).abs() < f64::EPSILON);
}

#[test]
fn realized_reward_is_success_gated_and_capped() {
    let policy = compound_policy();
    assert_eq!(realized_reward(&policy, false, 0.01).unwrap(), 0.0);
    assert_eq!(realized_reward(&policy, true, 0.2).unwrap(), 0.8);
    assert!((realized_reward(&policy, true, 10.0).unwrap() - 0.1).abs() < f64::EPSILON);
    assert!(realized_reward(&policy, true, f64::NAN).is_err());
}

#[test]
fn candidate_order_and_catalog_order_do_not_change_recommendation() {
    let candidates = vec![
        direct("z-costlier", 0.9, 0.2),
        direct("b-tied", 0.9, 0.1),
        direct("a-tied", 0.9, 0.1),
    ];
    let first = select(
        &context(),
        &routing_policy(),
        &catalog(),
        &evidence(active(candidates.clone())),
    )
    .unwrap();
    let mut reversed_candidates = candidates;
    reversed_candidates.reverse();
    let reversed_catalog = CatalogSnapshot::new(vec![
        model("provider-b", "model-b"),
        model("provider-a", "model-a"),
    ]);
    let second = select(
        &context(),
        &routing_policy(),
        &reversed_catalog,
        &evidence(active(reversed_candidates)),
    )
    .unwrap();
    assert_eq!(
        first.compound.recommendation().unwrap().trajectory_id,
        "a-tied"
    );
    assert_eq!(first.compound, second.compound);
}

#[test]
fn mixed_valid_and_invalid_candidates_are_explainable() {
    let invalid = candidate(
        "excluded",
        1.0,
        0.0,
        0.0,
        vec![step(
            "call",
            CompoundActionKind::Delegate,
            &[],
            Some(route("excluded", "excluded", 500)),
            cost(0.0, 0.0, 0.0, 0.0),
            cost(0.0, 0.0, 0.0, 0.0),
        )],
    );
    let decision = selected(active(vec![invalid, direct("valid", 0.8, 0.1)]));
    assert_eq!(decision.recommendation().unwrap().trajectory_id, "valid");
    assert_eq!(decision.rejections().len(), 1);
    assert_eq!(
        decision.rejections()[0].reason,
        CompoundRejectionReason::ActionNotInBallot
    );
}

#[test]
fn inactive_and_fallback_compound_evidence_preserve_p4() {
    let baseline = select(
        &context(),
        &routing_policy(),
        &catalog(),
        &evidence(FrozenCompoundEvidence::default()),
    )
    .unwrap();
    let unavailable = select(
        &context(),
        &routing_policy(),
        &catalog(),
        &evidence(FrozenCompoundEvidence::unavailable(
            CompoundFallbackReason::RepositoryUnavailable,
        )),
    )
    .unwrap();
    let stale = {
        let mut stale = lineage();
        stale.catalog_version = "stale-catalog".into();
        let frozen = FrozenCompoundEvidence::active(
            stale,
            compound_policy(),
            vec![direct("valid", 0.9, 0.1)],
        )
        .unwrap();
        select(&context(), &routing_policy(), &catalog(), &evidence(frozen)).unwrap()
    };

    assert_eq!(baseline.entries(), unavailable.entries());
    assert_eq!(baseline.entries(), stale.entries());
    assert_eq!(
        quote_trajectory_reservation(&baseline).unwrap(),
        quote_trajectory_reservation(&unavailable).unwrap()
    );
    assert_eq!(
        quote_trajectory_reservation(&baseline).unwrap(),
        quote_trajectory_reservation(&stale).unwrap()
    );
    assert!(matches!(
        unavailable.compound,
        CompoundShadowDecision::Fallback {
            reason: CompoundFallbackReason::RepositoryUnavailable,
            ..
        }
    ));
    assert!(matches!(
        stale.compound,
        CompoundShadowDecision::Fallback {
            reason: CompoundFallbackReason::LineageMismatch(_),
            ..
        }
    ));
}

#[test]
fn hard_excluded_and_non_surviving_actions_never_enter_compound_policy() {
    let mut excluded = model("excluded-provider", "excluded-model");
    excluded.compliant = false;
    let catalog = CatalogSnapshot::new(vec![model("provider-a", "model-a"), excluded]);
    let candidate = candidate(
        "resurrection",
        1.0,
        0.0,
        0.0,
        vec![step(
            "call",
            CompoundActionKind::ModelCall,
            &[],
            Some(route("excluded-provider", "excluded-model", 500)),
            cost(0.0, 0.0, 0.0, 0.0),
            cost(0.0, 0.0, 0.0, 0.0),
        )],
    );
    let ballot = select(
        &context(),
        &routing_policy(),
        &catalog,
        &evidence(active(vec![candidate])),
    )
    .unwrap();
    assert_eq!(ballot.entries().len(), 1);
    assert_eq!(ballot.entries()[0].provider_id, "provider-a");
    assert!(
        rejection_reasons(&ballot.compound).contains(&CompoundRejectionReason::ActionNotInBallot)
    );
}

#[test]
fn shadow_decision_does_not_mutate_executable_authority() {
    let baseline = select(
        &context(),
        &routing_policy(),
        &catalog(),
        &evidence(FrozenCompoundEvidence::default()),
    )
    .unwrap();
    let shadow = select(
        &context(),
        &routing_policy(),
        &catalog(),
        &evidence(active(vec![direct("shadow", 0.9, 0.1)])),
    )
    .unwrap();

    assert_eq!(baseline.entries(), shadow.entries());
    assert_eq!(
        quote_trajectory_reservation(&baseline).unwrap(),
        quote_trajectory_reservation(&shadow).unwrap()
    );
    assert_eq!(baseline.entries().first(), shadow.entries().first());
}

#[test]
fn inactive_compound_serialization_is_predecessor_identical() {
    let snapshot = evidence(FrozenCompoundEvidence::default());
    let ballot = select(&context(), &routing_policy(), &catalog(), &snapshot).unwrap();
    let snapshot_json = serde_json::to_value(snapshot).unwrap();
    let ballot_json = serde_json::to_value(ballot).unwrap();

    assert!(snapshot_json.get("compound").is_none());
    assert!(ballot_json.get("compound").is_none());
}
