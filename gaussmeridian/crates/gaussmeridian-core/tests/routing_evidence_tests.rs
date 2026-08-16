use gaussmeridian_core::routing_evidence::{
    canonicalize_evidence, canonicalize_persisted_evidence, qualify_learning_updates,
    qualify_learning_updates_with_r2_authority, qualify_persisted_learning_updates,
    r2_learning_eligibility, CollectionMode, DeliveryEvidence, EvaluatorProvenance,
    EvidenceEnvelope, EvidenceError, LearningPath, Observation, R2Evidence, R2EvidenceLineage,
    R2LearningAuthority, R2LearningEligibility, SemanticEvidence, SkillEvidence, SkillJudgement,
    TrajectoryEvidence, ROUTING_EVIDENCE_SCHEMA_V1, ROUTING_EVIDENCE_SCHEMA_V2,
    ROUTING_EVIDENCE_SCHEMA_V4,
};

fn fixture() -> EvidenceEnvelope {
    EvidenceEnvelope {
        schema_version: ROUTING_EVIDENCE_SCHEMA_V4.into(),
        evidence_id: None,
        finalization_id: "finalization-1".into(),
        project_id: "project-1".into(),
        snapshot_fingerprint: "snapshot-1".into(),
        provider_id: "provider-1".into(),
        model_id: "model-1".into(),
        evaluator: EvaluatorProvenance {
            name: "semantic-judge".into(),
            version: "judge/v1".into(),
        },
        collection_mode: CollectionMode::SelectedModelOnline,
        delivery: Observation::Observed(DeliveryEvidence {
            delivered: true,
            terminal_outcome: "delivered".into(),
        }),
        contract_validity: Observation::Unobserved {
            reason: "no contract evaluator".into(),
        },
        semantic: Observation::Observed(SemanticEvidence {
            correct: true,
            confidence: 0.9,
        }),
        skills: Vec::new(),
        r2: Observation::Unobserved {
            reason: "no R2 label".into(),
        },
        trajectory: Observation::Unobserved {
            reason: "no trajectory label".into(),
        },
        compound_trajectory: None,
        provider_cost_usd: 0.002,
        customer_charge_usd: 0.003,
    }
}

fn complete_r2() -> R2Evidence {
    R2Evidence {
        requested_output_tokens: 512,
        selected_output_budget: 128,
        actual_output_tokens: 96,
        truncated: false,
        incomplete: false,
        instruction_compliant: true,
        quality: 0.86,
        predictor_state_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .into(),
        instruction_version: "meridian-output-budget/v1".into(),
        label_version: "r2-label/v2".into(),
        lineage: Some(r2_lineage()),
    }
}

fn r2_lineage() -> R2EvidenceLineage {
    R2EvidenceLineage {
        predictor_version: "r2-anchor-head/v1".into(),
        encoder_version: "r2-encoder/v1".into(),
        feature_version: "r2-features/v1".into(),
        corpus_version: "r2-corpus/v1".into(),
        catalog_version: "catalog/v1".into(),
        price_version: "prices/v1".into(),
        training_content_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .into(),
    }
}

fn r2_authority() -> R2LearningAuthority {
    R2LearningAuthority {
        collection_group_id: "all-model-all-budget-1".into(),
        predictor_state_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .into(),
        predictor_version: "r2-anchor-head/v1".into(),
        encoder_version: "r2-encoder/v1".into(),
        feature_version: "r2-features/v1".into(),
        evaluator: EvaluatorProvenance {
            name: "semantic-judge".into(),
            version: "judge/v1".into(),
        },
        corpus_version: "r2-corpus/v1".into(),
        catalog_version: "catalog/v1".into(),
        price_version: "prices/v1".into(),
        instruction_version: "meridian-output-budget/v1".into(),
        label_version: "r2-label/v2".into(),
        training_content_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            .into(),
    }
}

fn controlled_r2_fixture() -> EvidenceEnvelope {
    let mut evidence = fixture();
    evidence.collection_mode = CollectionMode::ControlledAllModelBudget {
        group_id: "all-model-all-budget-1".into(),
    };
    evidence.r2 = Observation::Observed(complete_r2());
    evidence
}

fn legacy_v2_payload() -> &'static [u8] {
    br#"{
        "schema_version": "routing-evidence/v2",
        "evidence_id": null,
        "finalization_id": "legacy-v2-finalization",
        "project_id": "legacy-v2-project",
        "snapshot_fingerprint": "legacy-v2-snapshot",
        "provider_id": "legacy-v2-provider",
        "model_id": "legacy-v2-model",
        "evaluator": {
            "name": "legacy-v2-evaluator",
            "version": "legacy-v2-evaluator/v1"
        },
        "collection_mode": {
            "kind": "controlled_all_model",
            "group_id": "legacy-v2-group"
        },
        "delivery": {
            "state": "observed",
            "value": { "delivered": true, "terminal_outcome": "delivered" }
        },
        "contract_validity": {
            "state": "observed",
            "value": true
        },
        "semantic": {
            "state": "observed",
            "value": { "correct": true, "confidence": 0.9 }
        },
        "skills": [],
        "r2": {
            "state": "observed",
            "value": { "accepted": true, "label_version": "legacy-r2-label/v1" }
        },
        "trajectory": {
            "state": "unobserved",
            "value": { "reason": "legacy-no-trajectory" }
        },
        "provider_cost_usd": 0.002,
        "customer_charge_usd": 0.003
    }"#
}

#[test]
fn legacy_v1_payload_replays_with_its_historical_canonical_identity() {
    let payload = br#"{
        "schema_version": "routing-evidence/v1",
        "evidence_id": null,
        "finalization_id": "legacy-finalization",
        "project_id": "legacy-project",
        "snapshot_fingerprint": "legacy-snapshot",
        "model_id": "legacy-model",
        "evaluator": {
            "name": "legacy-evaluator",
            "version": "legacy-evaluator/v1"
        },
        "collection_mode": { "kind": "selected_model_online" },
        "delivery": {
            "state": "observed",
            "value": { "delivered": true, "terminal_outcome": "delivered" }
        },
        "contract_validity": {
            "state": "unobserved",
            "value": { "reason": "legacy-no-contract" }
        },
        "semantic": {
            "state": "observed",
            "value": { "correct": true, "confidence": 0.9 }
        },
        "skills": [{
            "skill_index": 3,
            "skill_name": "code synthesis",
            "correct": true,
            "critic_version": "legacy-critic/v1",
            "taxonomy_version": "legacy-taxonomy/v1"
        }],
        "r2": {
            "state": "unobserved",
            "value": { "reason": "legacy-no-r2" }
        },
        "trajectory": {
            "state": "unobserved",
            "value": { "reason": "legacy-no-trajectory" }
        },
        "provider_cost_usd": 0.002,
        "customer_charge_usd": 0.003
    }"#;

    let canonical = canonicalize_persisted_evidence(payload).expect("legacy v1 must replay");

    assert_eq!(
        canonical.evidence_id,
        "4065353623c219b28fa6171af5607bc82241eaa4268c8ee802bd33ec4c35a265"
    );
    assert_eq!(canonical.evidence_id, canonical.content_hash);
    let replayed: serde_json::Value =
        serde_json::from_slice(&canonical.canonical_payload).expect("canonical v1 remains JSON");
    assert_eq!(
        replayed["schema_version"],
        serde_json::Value::String(ROUTING_EVIDENCE_SCHEMA_V1.into())
    );
    assert!(
        replayed.get("provider_id").is_none(),
        "legacy replay must not invent provider provenance"
    );
    assert!(
        replayed["skills"][0].get("assessment").is_none(),
        "legacy replay must not invent P3-qualified critic evidence"
    );
}

#[test]
fn persisted_schema_dispatch_replays_v4_and_rejects_mislabeled_or_unknown_content() {
    let evidence = fixture();
    let payload = serde_json::to_vec(&evidence).expect("serialize current evidence");
    let direct = canonicalize_evidence(&evidence).expect("current v4 envelope");
    let replayed = canonicalize_persisted_evidence(&payload).expect("persisted v4 replay");
    assert_eq!(replayed, direct);

    let mut mislabeled = controlled_r2_fixture();
    mislabeled.schema_version = ROUTING_EVIDENCE_SCHEMA_V2.into();
    assert!(matches!(
        canonicalize_evidence(&mislabeled),
        Err(EvidenceError::UnsupportedSchema { schema_version })
            if schema_version == ROUTING_EVIDENCE_SCHEMA_V2
    ));
    let mislabeled_payload =
        serde_json::to_vec(&mislabeled).expect("serialize mislabeled evidence");
    assert!(matches!(
        canonicalize_persisted_evidence(&mislabeled_payload),
        Err(EvidenceError::Deserialization { .. })
    ));

    assert!(matches!(
        canonicalize_persisted_evidence(br#"{"schema_version":"routing-evidence/v999"}"#),
        Err(EvidenceError::UnsupportedSchema { schema_version })
            if schema_version == "routing-evidence/v999"
    ));
}

#[test]
fn legacy_v2_payload_replays_without_upconversion_or_r2_learning_authority() {
    let first =
        canonicalize_persisted_evidence(legacy_v2_payload()).expect("legacy v2 must replay");
    let second =
        canonicalize_persisted_evidence(legacy_v2_payload()).expect("legacy v2 replay is stable");

    assert_eq!(first, second);
    assert_eq!(
        first.evidence_id,
        "7b090f521549243e2be00fa8221f41da1ddac7e2e5ef7a453bce3baca7018236"
    );
    assert_eq!(first.evidence_id, first.content_hash);
    let replayed: serde_json::Value =
        serde_json::from_slice(&first.canonical_payload).expect("canonical v2 remains JSON");
    assert_eq!(replayed["schema_version"], ROUTING_EVIDENCE_SCHEMA_V2);
    assert_eq!(replayed["r2"]["value"]["accepted"], true);

    assert!(qualify_persisted_learning_updates(legacy_v2_payload())
        .expect("legacy v2 non-R2 learning remains replayable")
        .iter()
        .all(|update| update.path != LearningPath::R2));
}

#[test]
fn v4_field_reordering_canonicalizes_to_one_identity() {
    let ordered = serde_json::to_vec(&controlled_r2_fixture()).expect("serialize v4 evidence");
    let reordered = br#"{
        "customer_charge_usd": 0.003,
        "provider_cost_usd": 0.002,
        "trajectory": {"value":{"reason":"no trajectory label"},"state":"unobserved"},
        "r2": {"value":{
            "label_version":"r2-label/v2",
            "instruction_version":"meridian-output-budget/v1",
            "predictor_state_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "lineage":{
                "predictor_version":"r2-anchor-head/v1",
                "encoder_version":"r2-encoder/v1",
                "feature_version":"r2-features/v1",
                "corpus_version":"r2-corpus/v1",
                "catalog_version":"catalog/v1",
                "price_version":"prices/v1",
                "training_content_hash":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            },
            "quality":0.86,
            "instruction_compliant":true,
            "incomplete":false,
            "truncated":false,
            "actual_output_tokens":96,
            "selected_output_budget":128,
            "requested_output_tokens":512
        },"state":"observed"},
        "skills": [],
        "semantic": {"value":{"confidence":0.9,"correct":true},"state":"observed"},
        "contract_validity": {"value":{"reason":"no contract evaluator"},"state":"unobserved"},
        "delivery": {"value":{"terminal_outcome":"delivered","delivered":true},"state":"observed"},
        "collection_mode": {"group_id":"all-model-all-budget-1","kind":"controlled_all_model_budget"},
        "evaluator": {"version":"judge/v1","name":"semantic-judge"},
        "model_id": "model-1",
        "provider_id": "provider-1",
        "snapshot_fingerprint": "snapshot-1",
        "project_id": "project-1",
        "finalization_id": "finalization-1",
        "evidence_id": null,
        "schema_version": "routing-evidence/v4"
    }"#;

    assert_eq!(
        canonicalize_persisted_evidence(&ordered).expect("ordered v4 evidence"),
        canonicalize_persisted_evidence(reordered).expect("reordered v4 evidence")
    );
}

fn observed_skill(skill_id: &str, skill_index: u16, correct: bool) -> SkillEvidence {
    SkillEvidence {
        skill_index,
        skill_id: skill_id.into(),
        skill_name: skill_id.replace('_', " "),
        assessment: Observation::Observed(SkillJudgement {
            correct,
            confidence: 0.92,
            criticality: 0.8,
            rationale: format!("The answer demonstrated {skill_id}."),
        }),
        critic: EvaluatorProvenance {
            name: "controlled-skill-critic".into(),
            version: "critic/v2".into(),
        },
        taxonomy_version: "skills/v2".into(),
        task_fingerprint: "a".repeat(64),
        reference_fingerprint: "b".repeat(64),
        answer_fingerprint: "c".repeat(64),
    }
}

#[test]
fn validation_rejects_blank_versions_invalid_economics_and_duplicate_skills() {
    let mut blank = fixture();
    blank.evaluator.version = "  ".into();
    assert!(canonicalize_evidence(&blank).is_err());

    let mut negative = fixture();
    negative.provider_cost_usd = -0.001;
    assert!(canonicalize_evidence(&negative).is_err());

    let mut non_finite = fixture();
    non_finite.customer_charge_usd = f64::NAN;
    assert!(canonicalize_evidence(&non_finite).is_err());

    let skill = observed_skill("code_synthesis", 3, true);
    let mut duplicate = fixture();
    duplicate.skills = vec![skill.clone(), skill];
    assert!(canonicalize_evidence(&duplicate).is_err());
}

#[test]
fn observed_skill_requires_complete_critic_and_content_provenance() {
    let mut cases = Vec::new();

    let mut blank_skill_id = observed_skill("code_synthesis", 3, true);
    blank_skill_id.skill_id.clear();
    cases.push(blank_skill_id);

    let mut blank_critic = observed_skill("code_synthesis", 3, true);
    blank_critic.critic.version.clear();
    cases.push(blank_critic);

    let mut blank_task = observed_skill("code_synthesis", 3, true);
    blank_task.task_fingerprint.clear();
    cases.push(blank_task);

    let mut blank_reference = observed_skill("code_synthesis", 3, true);
    blank_reference.reference_fingerprint.clear();
    cases.push(blank_reference);

    let mut blank_answer = observed_skill("code_synthesis", 3, true);
    blank_answer.answer_fingerprint.clear();
    cases.push(blank_answer);

    let mut blank_rationale = observed_skill("code_synthesis", 3, true);
    blank_rationale.assessment = Observation::Observed(SkillJudgement {
        correct: true,
        confidence: 0.9,
        criticality: 0.8,
        rationale: " ".into(),
    });
    cases.push(blank_rationale);

    let mut invalid_confidence = observed_skill("code_synthesis", 3, true);
    invalid_confidence.assessment = Observation::Observed(SkillJudgement {
        correct: true,
        confidence: 1.01,
        criticality: 0.8,
        rationale: "Observed in the controlled answer.".into(),
    });
    cases.push(invalid_confidence);

    let mut invalid_criticality = observed_skill("code_synthesis", 3, true);
    invalid_criticality.assessment = Observation::Observed(SkillJudgement {
        correct: true,
        confidence: 0.9,
        criticality: f64::NAN,
        rationale: "Observed in the controlled answer.".into(),
    });
    cases.push(invalid_criticality);

    let mut invalid_fingerprint = observed_skill("code_synthesis", 3, true);
    invalid_fingerprint.answer_fingerprint = "not-a-sha256".into();
    cases.push(invalid_fingerprint);

    let mut excessive_rationale = observed_skill("code_synthesis", 3, true);
    excessive_rationale.assessment = Observation::Observed(SkillJudgement {
        correct: true,
        confidence: 0.9,
        criticality: 0.8,
        rationale: "x".repeat(2_049),
    });
    cases.push(excessive_rationale);

    for skill in cases {
        let mut evidence = fixture();
        evidence.skills = vec![skill];
        assert!(canonicalize_evidence(&evidence).is_err());
    }
}

#[test]
fn duplicate_stable_skill_identity_is_rejected_even_when_indices_differ() {
    let first = observed_skill("code_synthesis", 3, true);
    let second = observed_skill("code_synthesis", 4, false);
    let mut evidence = fixture();
    evidence.skills = vec![first, second];

    assert!(canonicalize_evidence(&evidence).is_err());
}

#[test]
fn one_evidence_envelope_cannot_mix_skill_taxonomies() {
    let first = observed_skill("code_synthesis", 3, true);
    let mut second = observed_skill("mathematical_reasoning", 4, true);
    second.taxonomy_version = "skills/v3".into();
    let mut evidence = fixture();
    evidence.skills = vec![first, second];

    assert!(canonicalize_evidence(&evidence).is_err());
}

#[test]
fn observed_semantics_require_evaluator_provenance() {
    let mut evidence = fixture();
    evidence.evaluator.name.clear();
    assert!(canonicalize_evidence(&evidence).is_err());
}

#[test]
fn controlled_collection_requires_a_group_id() {
    let mut evidence = fixture();
    evidence.collection_mode = CollectionMode::ControlledAllModel {
        group_id: " ".into(),
    };
    assert!(canonicalize_evidence(&evidence).is_err());
}

#[test]
fn supplied_evidence_id_must_equal_the_canonical_sha256() {
    let evidence = fixture();
    let canonical = canonicalize_evidence(&evidence).expect("fixture is valid");

    let mut matching = evidence.clone();
    matching.evidence_id = Some(canonical.evidence_id.clone());
    assert_eq!(
        canonicalize_evidence(&matching)
            .expect("matching id is valid")
            .evidence_id,
        canonical.evidence_id
    );

    let mut mismatch = evidence;
    mismatch.evidence_id = Some("0".repeat(64));
    assert!(canonicalize_evidence(&mismatch).is_err());
}

#[test]
fn delivery_contract_and_unknown_semantics_never_create_semantic_updates() {
    for semantic in [
        Observation::Unobserved {
            reason: "no semantic evaluator".into(),
        },
        Observation::Abstained {
            reason: "judge uncertainty".into(),
            uncertainty: 0.7,
        },
    ] {
        let mut evidence = fixture();
        evidence.semantic = semantic;
        evidence.contract_validity = Observation::Observed(true);
        assert!(qualify_learning_updates(&evidence)
            .expect("valid envelope")
            .iter()
            .all(|update| update.path != LearningPath::MeridianSemantic));
    }
}

#[test]
fn learning_paths_require_their_exact_typed_evidence() {
    let online = fixture();
    let online_updates = qualify_learning_updates(&online).expect("valid online evidence");
    assert_eq!(
        online_updates
            .iter()
            .map(|update| update.path)
            .collect::<Vec<_>>(),
        vec![LearningPath::MeridianSemantic]
    );

    let mut controlled = fixture();
    controlled.collection_mode = CollectionMode::ControlledAllModel {
        group_id: "experiment-1".into(),
    };
    let controlled_updates =
        qualify_learning_updates(&controlled).expect("valid controlled evidence");
    assert!(controlled_updates
        .iter()
        .any(|update| update.path == LearningPath::Carrot));

    let mut complete = controlled_r2_fixture();
    complete.skills = vec![observed_skill("summarisation", 8, false)];
    complete.trajectory = Observation::Observed(TrajectoryEvidence {
        reward: 0.75,
        step_count: 4,
        trajectory_version: "trajectory/v1".into(),
    });
    let complete_updates = qualify_learning_updates_with_r2_authority(&complete, &r2_authority())
        .expect("valid complete evidence");
    assert!(complete_updates
        .iter()
        .any(|update| update.path == LearningPath::Bella));
    assert!(complete_updates
        .iter()
        .any(|update| update.path == LearningPath::R2));
    assert!(complete_updates
        .iter()
        .all(|update| update.path != LearningPath::XRouter));
}

#[test]
fn generic_learning_qualification_never_grants_r2_authority() {
    let evidence = controlled_r2_fixture();
    let updates = qualify_learning_updates(&evidence).expect("controlled evidence remains valid");

    assert!(updates
        .iter()
        .any(|update| update.path == LearningPath::MeridianSemantic));
    assert!(updates
        .iter()
        .any(|update| update.path == LearningPath::Carrot));
    assert!(
        updates.iter().all(|update| update.path != LearningPath::R2),
        "collection mode alone must not grant R2 learning authority"
    );
}

#[test]
fn only_complete_counterfactual_action_evidence_emits_an_r2_projection() {
    let evidence = controlled_r2_fixture();
    let authority = r2_authority();
    let updates = qualify_learning_updates_with_r2_authority(&evidence, &authority)
        .expect("complete v3 evidence is valid");
    let r2: Vec<_> = updates
        .iter()
        .filter(|update| update.path == LearningPath::R2)
        .collect();

    assert_eq!(r2.len(), 1);
    assert_eq!(r2[0].subject, r#"["provider-1","model-1",128]"#);
    assert_eq!(r2[0].observed_value, 0.86);

    let mut selected_online = evidence.clone();
    selected_online.collection_mode = CollectionMode::SelectedModelOnline;
    assert!(
        qualify_learning_updates_with_r2_authority(&selected_online, &authority)
            .expect("selected-online transport facts remain valid evidence")
            .iter()
            .all(|update| update.path != LearningPath::R2)
    );

    let mut model_only = evidence.clone();
    model_only.collection_mode = CollectionMode::ControlledAllModel {
        group_id: "model-only-counterfactual".into(),
    };
    assert!(
        qualify_learning_updates_with_r2_authority(&model_only, &authority)
            .expect("model-only counterfactual evidence remains valid")
            .iter()
            .all(|update| update.path != LearningPath::R2)
    );

    for observation in [
        Observation::Unobserved {
            reason: "budget action not independently evaluated".into(),
        },
        Observation::Abstained {
            reason: "evaluator uncertainty".into(),
            uncertainty: 0.7,
        },
    ] {
        let mut unavailable = evidence.clone();
        unavailable.r2 = observation;
        assert!(
            qualify_learning_updates_with_r2_authority(&unavailable, &authority)
                .expect("typed unavailable evidence remains valid")
                .iter()
                .all(|update| update.path != LearningPath::R2)
        );
    }
}

#[test]
fn r2_projection_requires_exact_observed_and_authorized_lineage() {
    let evidence = controlled_r2_fixture();
    let authority = r2_authority();
    assert_eq!(
        r2_learning_eligibility(&evidence, &authority).expect("valid authority"),
        R2LearningEligibility::Eligible
    );

    let mut missing_lineage = evidence.clone();
    if let Observation::Observed(r2) = &mut missing_lineage.r2 {
        r2.lineage = None;
    }
    assert!(matches!(
        r2_learning_eligibility(&missing_lineage, &authority).expect("valid evidence"),
        R2LearningEligibility::Ineligible(_)
    ));
    assert!(
        qualify_learning_updates_with_r2_authority(&missing_lineage, &authority)
            .expect("missing lineage remains durable evidence")
            .iter()
            .all(|update| update.path != LearningPath::R2)
    );

    let mut mismatches = Vec::new();
    let mut changed = authority.clone();
    changed.collection_group_id = "other-group".into();
    mismatches.push(("collection_group_id", changed));
    let mut changed = authority.clone();
    changed.predictor_state_id =
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into();
    mismatches.push(("predictor_state_id", changed));
    let mut changed = authority.clone();
    changed.predictor_version = "other-predictor".into();
    mismatches.push(("predictor_version", changed));
    let mut changed = authority.clone();
    changed.encoder_version = "other-encoder".into();
    mismatches.push(("encoder_version", changed));
    let mut changed = authority.clone();
    changed.feature_version = "other-features".into();
    mismatches.push(("feature_version", changed));
    let mut changed = authority.clone();
    changed.evaluator.name = "other-evaluator".into();
    mismatches.push(("evaluator_name", changed));
    let mut changed = authority.clone();
    changed.evaluator.version = "other-evaluator/v1".into();
    mismatches.push(("evaluator_version", changed));
    let mut changed = authority.clone();
    changed.corpus_version = "other-corpus".into();
    mismatches.push(("corpus_version", changed));
    let mut changed = authority.clone();
    changed.catalog_version = "other-catalog".into();
    mismatches.push(("catalog_version", changed));
    let mut changed = authority.clone();
    changed.price_version = "other-prices".into();
    mismatches.push(("price_version", changed));
    let mut changed = authority.clone();
    changed.instruction_version = "other-instruction".into();
    mismatches.push(("instruction_version", changed));
    let mut changed = authority.clone();
    changed.label_version = "other-label".into();
    mismatches.push(("label_version", changed));
    let mut changed = authority;
    changed.training_content_hash =
        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into();
    mismatches.push(("training_content_hash", changed));

    for (field, mismatch) in mismatches {
        assert!(
            matches!(
                r2_learning_eligibility(&evidence, &mismatch).expect("valid mismatch authority"),
                R2LearningEligibility::Ineligible(_)
            ),
            "{field} mismatch must be typed ineligible"
        );
        let updates = qualify_learning_updates_with_r2_authority(&evidence, &mismatch)
            .expect("mismatched evidence remains available to other paths");
        assert!(
            updates.iter().all(|update| update.path != LearningPath::R2),
            "{field} mismatch must not emit R2"
        );
        assert!(
            updates
                .iter()
                .any(|update| update.path == LearningPath::MeridianSemantic),
            "{field} mismatch must preserve independent semantic learning"
        );
        assert!(
            updates
                .iter()
                .any(|update| update.path == LearningPath::Carrot),
            "{field} mismatch must preserve independent CARROT learning"
        );
    }
}

#[test]
fn invalid_or_inconsistent_complete_r2_labels_are_rejected() {
    let base = controlled_r2_fixture();
    let mut cases = Vec::new();

    let mut zero_requested = base.clone();
    if let Observation::Observed(r2) = &mut zero_requested.r2 {
        r2.requested_output_tokens = 0;
    }
    cases.push(("requested_output_tokens", zero_requested));

    let mut zero_selected = base.clone();
    if let Observation::Observed(r2) = &mut zero_selected.r2 {
        r2.selected_output_budget = 0;
    }
    cases.push(("selected_output_budget", zero_selected));

    let mut zero_actual = base.clone();
    if let Observation::Observed(r2) = &mut zero_actual.r2 {
        r2.actual_output_tokens = 0;
    }
    cases.push(("actual_output_tokens", zero_actual));

    let mut above_caller_ceiling = base.clone();
    if let Observation::Observed(r2) = &mut above_caller_ceiling.r2 {
        r2.selected_output_budget = r2.requested_output_tokens + 1;
    }
    cases.push(("selected budget above caller ceiling", above_caller_ceiling));

    let mut compliant_overrun = base.clone();
    if let Observation::Observed(r2) = &mut compliant_overrun.r2 {
        r2.actual_output_tokens = r2.selected_output_budget + 1;
    }
    cases.push((
        "compliant actual tokens above selected budget",
        compliant_overrun,
    ));

    let mut non_finite_quality = base.clone();
    if let Observation::Observed(r2) = &mut non_finite_quality.r2 {
        r2.quality = f64::NAN;
    }
    cases.push(("non-finite quality", non_finite_quality));

    let mut out_of_range_quality = base.clone();
    if let Observation::Observed(r2) = &mut out_of_range_quality.r2 {
        r2.quality = 1.01;
    }
    cases.push(("out-of-range quality", out_of_range_quality));

    for field in ["predictor_state_id", "instruction_version", "label_version"] {
        let mut blank_version = base.clone();
        if let Observation::Observed(r2) = &mut blank_version.r2 {
            match field {
                "predictor_state_id" => r2.predictor_state_id = " ".into(),
                "instruction_version" => r2.instruction_version = " ".into(),
                "label_version" => r2.label_version = " ".into(),
                _ => unreachable!(),
            }
        }
        cases.push((field, blank_version));
    }

    for (case, evidence) in cases {
        assert!(
            canonicalize_evidence(&evidence).is_err(),
            "{case} must reject the complete v3 label"
        );
    }
}

#[test]
fn complete_r2_labels_require_every_token_and_observation_field() {
    for field in [
        "requested_output_tokens",
        "selected_output_budget",
        "actual_output_tokens",
        "truncated",
        "incomplete",
        "instruction_compliant",
        "quality",
        "predictor_state_id",
        "instruction_version",
        "label_version",
    ] {
        let mut value =
            serde_json::to_value(controlled_r2_fixture()).expect("serialize complete v3 label");
        value["r2"]["value"]
            .as_object_mut()
            .expect("R2 observation is an object")
            .remove(field);
        let payload = serde_json::to_vec(&value).expect("serialize incomplete v3 label");

        assert!(
            matches!(
                canonicalize_persisted_evidence(&payload),
                Err(EvidenceError::Deserialization { .. })
            ),
            "missing {field} must reject the persisted v3 label"
        );
    }
}

#[test]
fn r2_learning_authority_rejects_equal_malformed_content_identities() {
    let mut evidence = controlled_r2_fixture();
    let mut authority = r2_authority();
    if let Observation::Observed(r2) = &mut evidence.r2 {
        r2.predictor_state_id = "malformed-state".into();
    }
    authority.predictor_state_id = "malformed-state".into();

    assert!(matches!(
        r2_learning_eligibility(&evidence, &authority),
        Err(EvidenceError::InvalidFingerprint { .. })
    ));

    let mut evidence = controlled_r2_fixture();
    let mut authority = r2_authority();
    if let Observation::Observed(r2) = &mut evidence.r2 {
        r2.lineage
            .as_mut()
            .expect("controlled lineage")
            .training_content_hash = "malformed-training".into();
    }
    authority.training_content_hash = "malformed-training".into();
    assert!(matches!(
        r2_learning_eligibility(&evidence, &authority),
        Err(EvidenceError::InvalidFingerprint { .. })
    ));
}

#[test]
fn explicit_instruction_noncompliance_retains_provider_token_overrun_evidence() {
    let mut evidence = controlled_r2_fixture();
    if let Observation::Observed(r2) = &mut evidence.r2 {
        r2.instruction_compliant = false;
        r2.actual_output_tokens = r2.selected_output_budget + 1;
    }

    let update = qualify_learning_updates_with_r2_authority(&evidence, &r2_authority())
        .expect("an observed provider violation remains durable evidence")
        .into_iter()
        .find(|update| update.path == LearningPath::R2)
        .expect("complete counterfactual evidence remains action-qualified");
    assert_eq!(update.subject, r#"["provider-1","model-1",128]"#);
}

#[test]
fn aggregate_semantic_success_cannot_substitute_for_unobserved_r2() {
    let evidence = fixture();
    assert!(matches!(evidence.semantic, Observation::Observed(_)));
    assert!(matches!(evidence.r2, Observation::Unobserved { .. }));
    assert!(qualify_learning_updates(&evidence)
        .expect("aggregate semantic evidence remains valid")
        .iter()
        .all(|update| update.path != LearningPath::R2));
}

#[test]
fn aggregate_semantics_do_not_create_bella_updates() {
    let evidence = fixture();
    assert!(qualify_learning_updates(&evidence)
        .expect("valid evidence")
        .iter()
        .all(|update| update.path != LearningPath::Bella));
}

#[test]
fn abstained_skill_does_not_create_a_bella_update() {
    let mut skill = observed_skill("code_synthesis", 3, true);
    skill.assessment = Observation::Abstained {
        reason: "critic disagreement".into(),
        uncertainty: 0.74,
    };
    let mut evidence = fixture();
    evidence.skills = vec![skill];

    assert!(qualify_learning_updates(&evidence)
        .expect("valid abstention")
        .iter()
        .all(|update| update.path != LearningPath::Bella));
}

#[test]
fn observed_skill_projection_is_route_taxonomy_and_skill_scoped() {
    let mut evidence = fixture();
    evidence.skills = vec![observed_skill("code_synthesis", 3, false)];

    let update = qualify_learning_updates(&evidence)
        .expect("valid skill evidence")
        .into_iter()
        .find(|update| update.path == LearningPath::Bella)
        .expect("observed skill creates one BELLA projection");

    assert_eq!(
        update.subject,
        r#"["provider-1","model-1","skills/v2","code_synthesis"]"#
    );
    assert_eq!(update.observed_value, 0.0);
}

#[test]
fn aggregate_semantic_changes_do_not_change_skill_projection() {
    let mut correct = fixture();
    correct.skills = vec![observed_skill("code_synthesis", 3, true)];
    let mut incorrect = correct.clone();
    incorrect.semantic = Observation::Observed(SemanticEvidence {
        correct: false,
        confidence: 0.95,
    });

    let bella_updates = |evidence: &EvidenceEnvelope| {
        qualify_learning_updates(evidence)
            .expect("valid evidence")
            .into_iter()
            .filter(|update| update.path == LearningPath::Bella)
            .map(|update| (update.subject, update.observed_value))
            .collect::<Vec<_>>()
    };

    assert_eq!(bella_updates(&correct), bella_updates(&incorrect));
}

#[test]
fn canonicalization_and_updates_are_byte_stable() {
    let evidence = fixture();
    let first = canonicalize_evidence(&evidence).expect("valid evidence");
    let second = canonicalize_evidence(&evidence).expect("valid evidence");
    assert_eq!(first, second);
    assert_eq!(first.evidence_id, first.content_hash);

    let first_updates = qualify_learning_updates(&evidence).expect("valid evidence");
    let second_updates = qualify_learning_updates(&evidence).expect("valid evidence");
    assert_eq!(first_updates, second_updates);
    assert!(first_updates
        .windows(2)
        .all(|pair| (pair[0].path, &pair[0].subject) <= (pair[1].path, &pair[1].subject)));
}
