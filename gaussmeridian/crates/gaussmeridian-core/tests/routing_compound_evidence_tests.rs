use gaussmeridian_core::{
    routing_evidence::{
        canonicalize_evidence, canonicalize_persisted_evidence,
        qualify_learning_updates_with_xrouter_authority, qualify_persisted_learning_updates,
        xrouter_learning_eligibility, CollectionMode, CompoundActionOutcome, CompoundObservedStep,
        CompoundPolicyResult, CompoundTrajectoryEvidence, DeliveryEvidence, EvaluatorProvenance,
        EvidenceEnvelope, EvidenceError, LearningPath, Observation, XRouterLearningAuthority,
        XRouterLearningEligibility, XRouterLearningIneligibility, XRouterLineageField,
        ROUTING_EVIDENCE_SCHEMA_V3, ROUTING_EVIDENCE_SCHEMA_V4,
    },
    routing_policy::compound::{
        realized_reward, CompoundActionCost, CompoundActionKind, CompoundLineage, CompoundPolicy,
        CompoundRejectionReason, CompoundRouteAction,
    },
};

fn policy() -> CompoundPolicy {
    CompoundPolicy {
        version: "compound-policy/v1".into(),
        max_steps: 8,
        max_provider_calls: 4,
        max_synthesis_calls: 1,
        max_trajectory_liability: 2.0,
        reward_constant: 1.0,
        cost_weight: 0.5,
        penalty_cap: 0.75,
    }
}

fn lineage() -> CompoundLineage {
    CompoundLineage {
        learner_state_id: "a".repeat(64),
        learner_version: "xrouter-shadow-state/v1".into(),
        feature_version: "xrouter-shadow-features/v1".into(),
        evaluator_version: "xrouter-controlled-evaluator/v1".into(),
        corpus_version: "xrouter-corpus/v1".into(),
        catalog_version: "catalog/v1".into(),
        price_version: "prices/v1".into(),
        policy_version: "compound-policy/v1".into(),
        training_content_hash: "b".repeat(64),
    }
}

fn authority() -> XRouterLearningAuthority {
    XRouterLearningAuthority {
        collection_group_id: "xrouter-controlled-1".into(),
        evaluator: EvaluatorProvenance {
            name: "xrouter-controlled-evaluator".into(),
            version: "xrouter-controlled-evaluator/v1".into(),
        },
        lineage: lineage(),
        policy: policy(),
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

fn observed_step(
    step_id: &str,
    kind: CompoundActionKind,
    dependencies: &[&str],
    route: Option<CompoundRouteAction>,
    outcome: CompoundActionOutcome,
    realized_cost: CompoundActionCost,
) -> CompoundObservedStep {
    CompoundObservedStep {
        step_id: step_id.into(),
        kind,
        dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
        route,
        outcome,
        realized_cost,
    }
}

fn compound_trajectory() -> CompoundTrajectoryEvidence {
    let total_provider_cost_usd = 0.5;
    let total_trajectory_cost_usd = 0.58;
    CompoundTrajectoryEvidence {
        trajectory_id: "trajectory-controlled-1".into(),
        steps: vec![
            observed_step(
                "draft-a",
                CompoundActionKind::ModelCall,
                &[],
                Some(route("provider-a", "model-a", 512)),
                CompoundActionOutcome::PartialFailure,
                cost(0.2, 0.01, 0.0, 0.0),
            ),
            observed_step(
                "draft-b",
                CompoundActionKind::ModelCall,
                &[],
                Some(route("provider-b", "model-b", 512)),
                CompoundActionOutcome::Completed,
                cost(0.3, 0.0, 0.0, 0.0),
            ),
            observed_step(
                "select",
                CompoundActionKind::Select,
                &["draft-a", "draft-b"],
                None,
                CompoundActionOutcome::Completed,
                cost(0.0, 0.0, 0.02, 0.0),
            ),
        ],
        router_inference_cost_usd: 0.05,
        total_provider_cost_usd,
        total_trajectory_cost_usd,
        terminal_correct: Observation::Observed(true),
        terminal_outcome: "delivered".into(),
        policy_result: CompoundPolicyResult::Passed,
        realized_reward: realized_reward(&policy(), true, total_trajectory_cost_usd)
            .expect("valid reward policy"),
        reward_policy_version: "compound-policy/v1".into(),
        lineage: lineage(),
    }
}

fn controlled_evidence() -> EvidenceEnvelope {
    EvidenceEnvelope {
        schema_version: ROUTING_EVIDENCE_SCHEMA_V4.into(),
        evidence_id: None,
        finalization_id: "finalization-xrouter-1".into(),
        project_id: "project-xrouter-1".into(),
        snapshot_fingerprint: "snapshot-xrouter-1".into(),
        provider_id: "provider-b".into(),
        model_id: "model-b".into(),
        evaluator: EvaluatorProvenance {
            name: "xrouter-controlled-evaluator".into(),
            version: "xrouter-controlled-evaluator/v1".into(),
        },
        collection_mode: CollectionMode::ControlledAllModelBudget {
            group_id: "xrouter-controlled-1".into(),
        },
        delivery: Observation::Observed(DeliveryEvidence {
            delivered: true,
            terminal_outcome: "delivered".into(),
        }),
        contract_validity: Observation::Unobserved {
            reason: "not evaluated".into(),
        },
        semantic: Observation::Unobserved {
            reason: "compound terminal correctness is authoritative".into(),
        },
        skills: Vec::new(),
        r2: Observation::Unobserved {
            reason: "not an R2 collection".into(),
        },
        trajectory: Observation::Unobserved {
            reason: "legacy trajectory label is not authoritative".into(),
        },
        compound_trajectory: Some(compound_trajectory()),
        provider_cost_usd: 0.5,
        customer_charge_usd: 0.73,
    }
}

fn xrouter_updates(
    evidence: &EvidenceEnvelope,
) -> Vec<gaussmeridian_core::routing_evidence::LearningUpdate> {
    qualify_learning_updates_with_xrouter_authority(evidence, &authority())
        .expect("compound evidence remains canonical")
        .into_iter()
        .filter(|update| update.path == LearningPath::XRouter)
        .collect()
}

#[test]
fn complete_compound_trajectory_is_canonical() {
    let evidence = controlled_evidence();
    let first = canonicalize_evidence(&evidence).expect("complete v4 evidence");
    let second = canonicalize_evidence(&evidence).expect("canonicalization is repeatable");
    assert_eq!(first, second);

    let payload: serde_json::Value =
        serde_json::from_slice(&first.canonical_payload).expect("canonical JSON");
    assert_eq!(payload["schema_version"], ROUTING_EVIDENCE_SCHEMA_V4);
    assert_eq!(
        payload["compound_trajectory"]["steps"][0]["outcome"],
        "partial_failure"
    );
    assert_eq!(
        payload["compound_trajectory"]["steps"][2]["dependencies"],
        serde_json::json!(["draft-a", "draft-b"])
    );
    assert_eq!(
        payload["compound_trajectory"]["lineage"]["learner_state_id"],
        "a".repeat(64)
    );
    assert_eq!(
        payload["compound_trajectory"]["total_trajectory_cost_usd"],
        0.58
    );
}

#[test]
fn compound_totals_and_reward_are_recomputed() {
    let valid = controlled_evidence();
    let update = xrouter_updates(&valid)
        .into_iter()
        .next()
        .expect("exact totals qualify");
    assert!((update.provider_cost_usd - 0.5).abs() < 1e-12);
    assert!((update.observed_value - 0.71).abs() < 1e-12);

    let mut false_provider_total = valid.clone();
    false_provider_total
        .compound_trajectory
        .as_mut()
        .unwrap()
        .total_provider_cost_usd = 0.01;
    assert!(matches!(
        xrouter_learning_eligibility(&false_provider_total, &authority()),
        Ok(XRouterLearningEligibility::Ineligible(
            XRouterLearningIneligibility::ProviderCostMismatch
        ))
    ));
    assert!(xrouter_updates(&false_provider_total).is_empty());

    let mut false_trajectory_total = valid.clone();
    false_trajectory_total
        .compound_trajectory
        .as_mut()
        .unwrap()
        .total_trajectory_cost_usd = 0.01;
    assert!(matches!(
        xrouter_learning_eligibility(&false_trajectory_total, &authority()),
        Ok(XRouterLearningEligibility::Ineligible(
            XRouterLearningIneligibility::TrajectoryCostMismatch
        ))
    ));

    let mut false_reward = valid;
    false_reward
        .compound_trajectory
        .as_mut()
        .unwrap()
        .realized_reward = 0.99;
    assert!(matches!(
        xrouter_learning_eligibility(&false_reward, &authority()),
        Ok(XRouterLearningEligibility::Ineligible(
            XRouterLearningIneligibility::RewardMismatch
        ))
    ));
}

#[test]
fn partial_failed_action_cost_is_not_dropped() {
    let evidence = controlled_evidence();
    let update = xrouter_updates(&evidence)
        .into_iter()
        .next()
        .expect("complete partial-failure trajectory qualifies");
    assert_eq!(
        evidence.compound_trajectory.as_ref().unwrap().steps[0].outcome,
        CompoundActionOutcome::PartialFailure
    );
    assert!((update.provider_cost_usd - 0.5).abs() < 1e-12);
    assert!((update.observed_value - 0.71).abs() < 1e-12);
}

#[test]
fn compound_projection_preserves_provider_cost_and_customer_charge() {
    let update = xrouter_updates(&controlled_evidence())
        .into_iter()
        .next()
        .expect("exact controlled evidence qualifies");
    assert!((update.provider_cost_usd - 0.5).abs() < 1e-12);
    assert!((update.customer_charge_usd - 0.73).abs() < 1e-12);
    assert_ne!(update.provider_cost_usd, update.customer_charge_usd);
}

#[test]
fn xrouter_learning_fail_closed_matrix() {
    let mut cases = Vec::new();

    let mut selected_online = controlled_evidence();
    selected_online.collection_mode = CollectionMode::SelectedModelOnline;
    cases.push((
        "selected-model online",
        selected_online,
        XRouterLearningIneligibility::CollectionModeNotControlledAllModelBudget,
    ));

    let mut absent = controlled_evidence();
    absent.compound_trajectory = None;
    cases.push((
        "absent compound label",
        absent,
        XRouterLearningIneligibility::CompoundTrajectoryNotObserved,
    ));

    let mut unobserved_correctness = controlled_evidence();
    unobserved_correctness
        .compound_trajectory
        .as_mut()
        .unwrap()
        .terminal_correct = Observation::Unobserved {
        reason: "judge unavailable".into(),
    };
    cases.push((
        "unobserved correctness",
        unobserved_correctness,
        XRouterLearningIneligibility::TerminalCorrectnessNotObserved,
    ));

    let mut policy_violation = controlled_evidence();
    policy_violation
        .compound_trajectory
        .as_mut()
        .unwrap()
        .policy_result = CompoundPolicyResult::Violated {
        violations: vec![CompoundRejectionReason::StepLimitExceeded],
    };
    cases.push((
        "policy violation",
        policy_violation,
        XRouterLearningIneligibility::PolicyViolation,
    ));

    let mut stale_lineage = controlled_evidence();
    stale_lineage
        .compound_trajectory
        .as_mut()
        .unwrap()
        .lineage
        .corpus_version = "stale-corpus/v0".into();
    cases.push((
        "stale lineage",
        stale_lineage,
        XRouterLearningIneligibility::LineageMismatch(XRouterLineageField::Corpus),
    ));

    let mut envelope_total = controlled_evidence();
    envelope_total.provider_cost_usd = 0.51;
    cases.push((
        "envelope provider total mismatch",
        envelope_total,
        XRouterLearningIneligibility::EnvelopeProviderCostMismatch,
    ));

    for (name, evidence, expected) in cases {
        assert_eq!(
            xrouter_learning_eligibility(&evidence, &authority()).expect("durable evidence"),
            XRouterLearningEligibility::Ineligible(expected),
            "{name}"
        );
        assert!(xrouter_updates(&evidence).is_empty(), "{name}");
    }
}

#[test]
fn exact_xrouter_authority_emits_one_trajectory_projection() {
    let evidence = controlled_evidence();
    assert_eq!(
        xrouter_learning_eligibility(&evidence, &authority()).unwrap(),
        XRouterLearningEligibility::Eligible
    );
    let updates = xrouter_updates(&evidence);
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].subject, "trajectory-controlled-1");
    assert_eq!(updates[0].evidence_id.len(), 64);
}

#[test]
fn xrouter_projection_is_idempotent_and_canonical() {
    let evidence = controlled_evidence();
    assert_eq!(xrouter_updates(&evidence), xrouter_updates(&evidence));

    let mut permuted_dependencies = evidence.clone();
    permuted_dependencies
        .compound_trajectory
        .as_mut()
        .unwrap()
        .steps[2]
        .dependencies
        .reverse();
    assert_eq!(
        canonicalize_evidence(&evidence).unwrap(),
        canonicalize_evidence(&permuted_dependencies).unwrap(),
        "dependency-set input order must not change evidence identity"
    );
}

#[test]
fn historical_v1_v2_v3_evidence_cannot_gain_p5_learning_authority() {
    let current = controlled_evidence();
    let mut legacy = serde_json::to_value(current).expect("serialize evidence");
    legacy["schema_version"] = serde_json::Value::String(ROUTING_EVIDENCE_SCHEMA_V3.into());
    legacy
        .as_object_mut()
        .unwrap()
        .remove("compound_trajectory");
    legacy["trajectory"] = serde_json::json!({
        "state": "observed",
        "value": {
            "reward": 0.71,
            "step_count": 3,
            "trajectory_version": "legacy-trajectory/v1"
        }
    });
    let payload = serde_json::to_vec(&legacy).expect("serialize legacy v3");

    canonicalize_persisted_evidence(&payload).expect("historical v3 remains replayable");
    assert!(qualify_persisted_learning_updates(&payload)
        .expect("historical learning qualification remains replayable")
        .iter()
        .all(|update| update.path != LearningPath::XRouter));
}

#[test]
fn missing_action_cost_and_duplicate_steps_fail_canonicalization() {
    let evidence = controlled_evidence();
    let mut missing_cost = serde_json::to_value(&evidence).expect("serialize evidence");
    missing_cost["compound_trajectory"]["steps"][0]
        .as_object_mut()
        .unwrap()
        .remove("realized_cost");
    let missing_payload = serde_json::to_vec(&missing_cost).unwrap();
    assert!(matches!(
        canonicalize_persisted_evidence(&missing_payload),
        Err(EvidenceError::Deserialization { .. })
    ));

    let mut duplicate = evidence;
    duplicate.compound_trajectory.as_mut().unwrap().steps[1].step_id = "draft-a".into();
    assert!(matches!(
        canonicalize_evidence(&duplicate),
        Err(EvidenceError::InvalidCompound {
            reason: CompoundRejectionReason::DuplicateStepIdentity
        })
    ));
}
