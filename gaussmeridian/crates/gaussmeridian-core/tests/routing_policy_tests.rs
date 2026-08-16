use gaussmeridian_core::routing_policy::{
    quote_trajectory_reservation, select, BandSelectionReason, CapabilityBand, CatalogModel,
    CatalogSnapshot, DeploymentKind, EvidenceSnapshot, ExclusionReason, Price, ProjectPolicy,
    RelaxationReason, RoutingContext, RoutingUnavailable, SkillRequirement,
};

fn model(id: &str, band: CapabilityBand, quality: f64, input: f64, output: f64) -> CatalogModel {
    CatalogModel {
        model_id: id.into(),
        provider_id: "provider".into(),
        model_version: "model-v1".into(),
        capability_band: band,
        deployment_kind: DeploymentKind::Managed,
        price: Price {
            input_per_million: input,
            output_per_million: output,
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

fn context(complexity: f32) -> RoutingContext {
    RoutingContext {
        complexity,
        estimated_input_tokens: 1_000,
        input_token_upper_bound: 1_000,
        output_token_budget: 500,
        hard_skills: Vec::new(),
    }
}

fn policy() -> ProjectPolicy {
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

fn evidence() -> EvidenceSnapshot {
    EvidenceSnapshot {
        policy_version: "policy-v1".into(),
        catalog_version: "catalog-v1".into(),
        price_version: "price-v1".into(),
        evaluator_version: "evaluator-v1".into(),
        normalized_cost_floor: 0.0,
        normalized_cost_ceiling: 0.02,
        complexity: None,
        predictions: Default::default(),
        bella: Default::default(),
        r2: Default::default(),
        compound: Default::default(),
    }
}

#[test]
fn hard_eligibility_is_never_relaxed_when_every_model_is_unsupported() {
    let mut unsupported = model("unsupported", CapabilityBand::Baseline, 0.99, 0.1, 0.1);
    unsupported.adapter_supports_model = false;

    let error = select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(vec![unsupported]),
        &evidence(),
    )
    .expect_err("an empty hard-eligible set must stop routing");

    assert!(matches!(
        error,
        RoutingUnavailable::NoHardEligibleModels { .. }
    ));
    assert_eq!(
        error.exclusions()[0].reasons,
        vec![ExclusionReason::AdapterDoesNotSupportModel]
    );
}

#[test]
fn exclusion_evidence_identifies_the_model_and_provider_route() {
    let mut first = model("shared-model", CapabilityBand::Baseline, 0.9, 1.0, 1.0);
    first.provider_id = "provider-a".into();
    first.tenant_allowed = false;
    let mut second = model("shared-model", CapabilityBand::Baseline, 0.9, 1.0, 1.0);
    second.provider_id = "provider-b".into();
    second.compliant = false;

    let error = select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(vec![first, second]),
        &evidence(),
    )
    .expect_err("both provider routes violate different hard constraints");

    let exclusions = error.exclusions();
    assert!(exclusions.iter().any(|entry| {
        entry.model_id == "shared-model"
            && entry.provider_id == "provider-a"
            && entry.reasons == vec![ExclusionReason::TenantDenied]
    }));
    assert!(exclusions.iter().any(|entry| {
        entry.model_id == "shared-model"
            && entry.provider_id == "provider-b"
            && entry.reasons == vec![ExclusionReason::ComplianceDenied]
    }));
}

#[test]
fn every_hard_skill_threshold_and_absolute_band_ceiling_are_enforced() {
    let mut missing_skill = model("missing-skill", CapabilityBand::Baseline, 0.9, 1.0, 1.0);
    missing_skill.skill_proficiency[3] = 0.79;
    let above_ceiling = model("above-ceiling", CapabilityBand::Frontier, 0.95, 1.0, 1.0);
    let mut ctx = context(0.9);
    ctx.hard_skills = vec![SkillRequirement {
        skill_index: 3,
        minimum_proficiency: 0.8,
    }];
    let mut project = policy();
    project.max_band = CapabilityBand::Advanced;

    let error = select(
        &ctx,
        &project,
        &CatalogSnapshot::new(vec![missing_skill, above_ceiling]),
        &evidence(),
    )
    .expect_err("hard constraints must not be downgraded");

    let exclusions = error.exclusions();
    assert!(exclusions.iter().any(|e| e.model_id == "missing-skill"
        && e.reasons
            .contains(&ExclusionReason::HardSkillBelowThreshold { skill_index: 3 })));
    assert!(exclusions.iter().any(|e| e.model_id == "above-ceiling"
        && e.reasons
            .contains(&ExclusionReason::AboveAbsoluteBandCeiling)));
}

#[test]
fn quality_floor_is_the_only_filter_that_relaxes_to_hard_eligible_models() {
    let low_quality = model("low-quality", CapabilityBand::Baseline, 0.6, 1.0, 1.0);
    let ballot = select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(vec![low_quality]),
        &evidence(),
    )
    .unwrap();

    assert_eq!(ballot.entries()[0].model_id, "low-quality");
    assert_eq!(
        ballot.relaxations,
        vec![RelaxationReason::QualityFloorRelaxed]
    );
}

#[test]
fn complexity_selects_a_preferred_band_without_crossing_the_ceiling() {
    let catalog = CatalogSnapshot::new(vec![
        model("baseline", CapabilityBand::Baseline, 0.8, 1.0, 1.0),
        model("advanced", CapabilityBand::Advanced, 0.8, 1.0, 1.0),
        model("frontier", CapabilityBand::Frontier, 0.8, 1.0, 1.0),
    ]);

    let easy = select(&context(0.1), &policy(), &catalog, &evidence()).unwrap();
    let moderate = select(&context(0.5), &policy(), &catalog, &evidence()).unwrap();
    let hard = select(&context(0.9), &policy(), &catalog, &evidence()).unwrap();

    assert_eq!(easy.entries()[0].capability_band, CapabilityBand::Baseline);
    assert_eq!(easy.band_decision.desired, CapabilityBand::Baseline);
    assert_eq!(easy.band_decision.selected, CapabilityBand::Baseline);
    assert_eq!(easy.band_decision.reason, BandSelectionReason::DesiredBand);
    assert_eq!(
        moderate.entries()[0].capability_band,
        CapabilityBand::Advanced
    );
    assert_eq!(moderate.band_decision.desired, CapabilityBand::Advanced);
    assert_eq!(moderate.band_decision.selected, CapabilityBand::Advanced);
    assert_eq!(hard.entries()[0].capability_band, CapabilityBand::Frontier);
    assert_eq!(hard.band_decision.desired, CapabilityBand::Frontier);
    assert_eq!(hard.band_decision.selected, CapabilityBand::Frontier);
    assert_eq!(
        easy.entries().len(),
        3,
        "the immutable ballot retains hard-eligible fallback bands"
    );
    assert!(easy
        .entries()
        .iter()
        .all(|entry| entry.capability_band <= CapabilityBand::Frontier));
}

#[test]
fn canonical_cost_weight_has_stable_endpoints_and_uses_both_token_rates() {
    let quality = model("quality", CapabilityBand::Baseline, 0.99, 10.0, 10.0);
    let cheap = model("cheap", CapabilityBand::Baseline, 0.80, 1.0, 2.0);
    let catalog = CatalogSnapshot::new(vec![quality, cheap]);
    let mut quality_only = policy();
    quality_only.cost_weight = 0.0;
    let mut cost_only = policy();
    cost_only.cost_weight = 1.0;

    let quality_ballot = select(&context(0.1), &quality_only, &catalog, &evidence()).unwrap();
    let cost_ballot = select(&context(0.1), &cost_only, &catalog, &evidence()).unwrap();

    assert_eq!(quality_ballot.entries()[0].model_id, "quality");
    assert_eq!(cost_ballot.entries()[0].model_id, "cheap");
    assert!((cost_ballot.entries()[0].expected_provider_cost - 0.002).abs() < f64::EPSILON);
}

#[test]
fn ballots_are_deterministic_and_capture_all_decision_fingerprints() {
    let catalog = CatalogSnapshot::new(vec![
        model("z-model", CapabilityBand::Baseline, 0.8, 1.0, 1.0),
        model("a-model", CapabilityBand::Baseline, 0.8, 1.0, 1.0),
    ]);

    let first = select(&context(0.1), &policy(), &catalog, &evidence()).unwrap();
    let second = select(&context(0.1), &policy(), &catalog, &evidence()).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.entries()[0].model_id, "a-model");
    assert_eq!(first.fingerprints.policy, "policy-v1");
    assert_eq!(first.fingerprints.catalog, "catalog-v1");
    assert_eq!(first.fingerprints.prices, "price-v1");
    assert_eq!(first.fingerprints.evaluator, "evaluator-v1");
}

#[test]
fn ballot_content_identity_is_stable_and_changes_with_authorized_order() {
    let mut denied = model("denied", CapabilityBand::Baseline, 0.9, 1.0, 1.0);
    denied.tenant_allowed = false;
    let mut unavailable = model("unavailable", CapabilityBand::Advanced, 0.9, 1.0, 1.0);
    unavailable.credential_available = false;
    let first_catalog = CatalogSnapshot::new(vec![
        model("alpha", CapabilityBand::Baseline, 0.9, 1.0, 1.0),
        denied.clone(),
        model("beta", CapabilityBand::Baseline, 0.9, 2.0, 2.0),
        unavailable.clone(),
    ]);
    let permuted_catalog = CatalogSnapshot::new(vec![
        unavailable,
        model("beta", CapabilityBand::Baseline, 0.9, 2.0, 2.0),
        denied,
        model("alpha", CapabilityBand::Baseline, 0.9, 1.0, 1.0),
    ]);
    let reversed_economics = CatalogSnapshot::new(vec![
        model("alpha", CapabilityBand::Baseline, 0.9, 2.0, 2.0),
        model("beta", CapabilityBand::Baseline, 0.9, 1.0, 1.0),
    ]);

    let first = select(&context(0.1), &policy(), &first_catalog, &evidence()).unwrap();
    let permuted = select(&context(0.1), &policy(), &permuted_catalog, &evidence()).unwrap();
    let reversed = select(&context(0.1), &policy(), &reversed_economics, &evidence()).unwrap();

    assert_eq!(first.content_id().unwrap(), permuted.content_id().unwrap());
    assert_ne!(first.content_id().unwrap(), reversed.content_id().unwrap());
    assert_eq!(first.content_id().unwrap().len(), 64);
}

#[test]
fn nearest_band_ties_prefer_the_band_with_lower_expected_cost_independent_of_catalog_order() {
    let advanced = context(0.5);
    let first = CatalogSnapshot::new(vec![
        model("frontier", CapabilityBand::Frontier, 0.8, 1.0, 1.0),
        model("baseline", CapabilityBand::Baseline, 0.8, 10.0, 10.0),
    ]);
    let second = CatalogSnapshot::new(vec![
        model("baseline", CapabilityBand::Baseline, 0.8, 10.0, 10.0),
        model("frontier", CapabilityBand::Frontier, 0.8, 1.0, 1.0),
    ]);

    let first_ballot = select(&advanced, &policy(), &first, &evidence()).unwrap();
    let second_ballot = select(&advanced, &policy(), &second, &evidence()).unwrap();

    assert_eq!(
        first_ballot.entries()[0].capability_band,
        CapabilityBand::Frontier
    );
    assert_eq!(
        second_ballot.entries()[0].capability_band,
        CapabilityBand::Frontier
    );
    assert_eq!(first_ballot.band_decision.desired, CapabilityBand::Advanced);
    assert_eq!(
        first_ballot.band_decision.selected,
        CapabilityBand::Frontier
    );
    assert_eq!(
        first_ballot.band_decision.reason,
        BandSelectionReason::NearestAvailableBand
    );
    assert_eq!(first_ballot.band_decision, second_ballot.band_decision);
}

#[test]
fn expected_fixed_cost_affects_band_ties_and_cost_first_ranking() {
    let mut baseline = model("baseline", CapabilityBand::Baseline, 0.9, 1.0, 1.0);
    baseline.price.expected_fixed_cost = 0.5;
    baseline.price.fixed_cost_upper_bound = 0.5;
    let mut frontier = model("frontier", CapabilityBand::Frontier, 0.9, 10.0, 10.0);
    frontier.price.fixed_cost_upper_bound = 0.5;

    let tied_band_ballot = select(
        &context(0.5),
        &policy(),
        &CatalogSnapshot::new(vec![baseline, frontier]),
        &evidence(),
    )
    .expect("valid fixed-cost catalog selects");
    assert_eq!(
        tied_band_ballot.band_decision.selected,
        CapabilityBand::Frontier
    );

    let mut costly = model("costly", CapabilityBand::Advanced, 0.9, 1.0, 1.0);
    costly.price.expected_fixed_cost = 0.5;
    costly.price.fixed_cost_upper_bound = 0.5;
    let mut cheap = model("cheap", CapabilityBand::Advanced, 0.9, 10.0, 10.0);
    cheap.price.fixed_cost_upper_bound = 0.5;
    let mut cost_first = policy();
    cost_first.cost_weight = 1.0;

    let ranked = select(
        &context(0.5),
        &cost_first,
        &CatalogSnapshot::new(vec![costly, cheap]),
        &evidence(),
    )
    .expect("valid fixed-cost catalog ranks");
    assert_eq!(ranked.entries()[0].model_id, "cheap");
}

#[test]
fn degenerate_frozen_cost_bounds_are_neutral_instead_of_invalid() {
    let mut degenerate = evidence();
    degenerate.normalized_cost_floor = 0.01;
    degenerate.normalized_cost_ceiling = 0.01;

    let ballot = select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(vec![model(
            "baseline",
            CapabilityBand::Baseline,
            0.8,
            1.0,
            1.0,
        )]),
        &degenerate,
    )
    .expect("equal frozen bounds have neutral normalized cost");

    assert_eq!(ballot.entries()[0].normalized_cost, 0.0);

    degenerate.normalized_cost_ceiling = 0.009;
    assert!(matches!(
        select(
            &context(0.1),
            &policy(),
            &CatalogSnapshot::new(vec![model(
                "baseline",
                CapabilityBand::Baseline,
                0.8,
                1.0,
                1.0,
            )]),
            &degenerate,
        ),
        Err(RoutingUnavailable::InvalidInput { .. })
    ));
}

#[test]
fn equal_risk_prefers_lower_expected_cost_before_stable_identifiers() {
    let mut correctness_first = policy();
    correctness_first.cost_weight = 0.0;
    let catalog = CatalogSnapshot::new(vec![
        model("a-expensive", CapabilityBand::Baseline, 0.9, 10.0, 10.0),
        model("z-cheap", CapabilityBand::Baseline, 0.9, 1.0, 1.0),
    ]);

    let ballot = select(&context(0.1), &correctness_first, &catalog, &evidence()).unwrap();

    assert_eq!(ballot.entries()[0].model_id, "z-cheap");
}

#[test]
fn irrelevant_catalog_entries_do_not_rescale_versioned_cost_normalization() {
    let eligible = vec![
        model("quality", CapabilityBand::Baseline, 0.9, 5.0, 5.0),
        model("cheap", CapabilityBand::Baseline, 0.8, 1.0, 1.0),
    ];
    let baseline = select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(eligible.clone()),
        &evidence(),
    )
    .unwrap();

    let mut irrelevant = model(
        "ineligible-expensive",
        CapabilityBand::Baseline,
        0.99,
        1_000_000.0,
        1_000_000.0,
    );
    irrelevant.tenant_allowed = false;
    let mut expanded = eligible;
    expanded.push(irrelevant);
    let with_irrelevant = select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(expanded),
        &evidence(),
    )
    .unwrap();

    assert_eq!(baseline.entries(), with_irrelevant.entries());
}

#[test]
fn reservation_uses_the_bounded_ballot_prefix_not_expected_cost() {
    let mut cheap = model("cheap", CapabilityBand::Baseline, 0.9, 1.0, 2.0);
    cheap.price.fixed_cost_upper_bound = 0.01;
    let mut second = model("second", CapabilityBand::Baseline, 0.9, 2.0, 4.0);
    second.price.fixed_cost_upper_bound = 0.02;
    let mut excluded_tail = model("tail", CapabilityBand::Baseline, 0.9, 3.0, 6.0);
    excluded_tail.price.fixed_cost_upper_bound = 0.50;

    let mut request = context(0.1);
    request.input_token_upper_bound = 2_000;
    let mut reservation_policy = policy();
    reservation_policy.max_provider_attempts = 2;
    let ballot = select(
        &request,
        &reservation_policy,
        &CatalogSnapshot::new(vec![excluded_tail, second, cheap]),
        &evidence(),
    )
    .unwrap();

    reservation_policy.max_provider_attempts = 1;
    reservation_policy.router_cost_upper_bound = 100.0;
    assert_eq!(reservation_policy.max_provider_attempts, 1);
    assert_eq!(reservation_policy.router_cost_upper_bound, 100.0);
    let quote = quote_trajectory_reservation(&ballot).unwrap();

    // 0.001 router + (2_000*1 + 500*2)/1e6 + 0.01
    //              + (2_000*2 + 500*4)/1e6 + 0.02
    assert!((quote.amount - 0.040).abs() < 1e-12);
    assert_eq!(quote.provider_attempts, 2);
}

#[test]
fn reservation_policy_rejects_zero_attempts_and_non_finite_router_cost() {
    let catalog = CatalogSnapshot::new(vec![model(
        "candidate",
        CapabilityBand::Baseline,
        0.9,
        1.0,
        2.0,
    )]);

    let mut zero_attempts = policy();
    zero_attempts.max_provider_attempts = 0;
    assert!(select(&context(0.1), &zero_attempts, &catalog, &evidence()).is_err());

    let mut non_finite_router_cost = policy();
    non_finite_router_cost.router_cost_upper_bound = f64::NAN;
    assert!(select(
        &context(0.1),
        &non_finite_router_cost,
        &catalog,
        &evidence()
    )
    .is_err());

    let mut equal_thresholds = policy();
    equal_thresholds.moderate_complexity_threshold = 0.7;
    equal_thresholds.high_complexity_threshold = 0.7;
    assert!(select(&context(0.7), &equal_thresholds, &catalog, &evidence()).is_err());
}

#[test]
fn selection_rejects_invalid_financial_authority_bounds() {
    let catalog = CatalogSnapshot::new(vec![model(
        "candidate",
        CapabilityBand::Baseline,
        0.9,
        1.0,
        2.0,
    )]);
    let mut invalid_request = context(0.1);
    invalid_request.input_token_upper_bound = invalid_request.estimated_input_tokens - 1;
    assert!(select(&invalid_request, &policy(), &catalog, &evidence()).is_err());

    let mut invalid_price = model("candidate", CapabilityBand::Baseline, 0.9, 1.0, 2.0);
    invalid_price.price.fixed_cost_upper_bound = -0.01;
    assert!(select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(vec![invalid_price]),
        &evidence(),
    )
    .is_err());

    let mut invalid_expected = model("candidate", CapabilityBand::Baseline, 0.9, 1.0, 2.0);
    invalid_expected.price.expected_fixed_cost = 0.02;
    invalid_expected.price.fixed_cost_upper_bound = 0.01;
    assert!(select(
        &context(0.1),
        &policy(),
        &CatalogSnapshot::new(vec![invalid_expected]),
        &evidence(),
    )
    .is_err());
}

#[test]
fn production_selector_avoids_panic_shortcuts() {
    let source = include_str!("../src/routing_policy.rs");
    assert!(!source.contains(".unwrap("));
    assert!(!source.contains(".expect("));
}
