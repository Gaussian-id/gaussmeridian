use gaussmeridian_core::routing_policy::{
    bella::{
        BellaEstimatorPolicy, BellaLearnerState, BellaUseStatus, MeridianSkillProfiler,
        ProfilerPolicy, ProfilerTrainingExample, SkillDefinition, SkillPosterior, SkillTaxonomy,
    },
    predictors::RouteIdentity,
    select, CapabilityBand, CatalogModel, CatalogSnapshot, DeploymentKind, EvidenceSnapshot, Price,
    ProjectPolicy, RoutingContext, SkillRequirement,
};

fn route(provider: &str, model: &str) -> RouteIdentity {
    RouteIdentity::new(provider, model).expect("valid route")
}

fn model(provider: &str, model: &str, quality: f64) -> CatalogModel {
    CatalogModel {
        model_id: model.into(),
        provider_id: provider.into(),
        model_version: "model/v1".into(),
        capability_band: CapabilityBand::Baseline,
        deployment_kind: DeploymentKind::Managed,
        price: Price {
            input_per_million: 10.0,
            output_per_million: 10.0,
            expected_fixed_cost: 0.0,
            fixed_cost_upper_bound: 1.0,
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

fn context() -> RoutingContext {
    RoutingContext {
        complexity: 0.1,
        estimated_input_tokens: 1_000,
        input_token_upper_bound: 1_000,
        output_token_budget: 500,
        hard_skills: vec![SkillRequirement {
            skill_index: 3,
            minimum_proficiency: 0.8,
        }],
    }
}

fn policy() -> ProjectPolicy {
    ProjectPolicy {
        cost_weight: 0.0,
        quality_floor: 0.0,
        max_band: CapabilityBand::Advanced,
        moderate_complexity_threshold: 0.35,
        high_complexity_threshold: 0.7,
        max_provider_attempts: 3,
        router_cost_upper_bound: 0.0,
    }
}

fn evidence() -> EvidenceSnapshot {
    EvidenceSnapshot {
        policy_version: "policy/v1".into(),
        catalog_version: "catalog/v1".into(),
        price_version: "price/v1".into(),
        evaluator_version: "semantic-evaluator/v1".into(),
        normalized_cost_floor: 0.0,
        normalized_cost_ceiling: 1.0,
        complexity: None,
        predictions: Default::default(),
        bella: Default::default(),
        r2: Default::default(),
        compound: Default::default(),
    }
}

fn taxonomy() -> SkillTaxonomy {
    SkillTaxonomy::new(
        "skills/v1",
        vec![
            SkillDefinition::new(
                0,
                "code_synthesis",
                "Code synthesis",
                "Implement and compile code.",
                0.6,
            )
            .unwrap(),
            SkillDefinition::new(
                1,
                "mathematical_reasoning",
                "Mathematical reasoning",
                "Prove mathematical claims.",
                0.7,
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn profiler() -> MeridianSkillProfiler {
    MeridianSkillProfiler::train(
        &taxonomy(),
        vec![
            ProfilerTrainingExample::new(
                "code",
                "Implement and compile Rust code",
                vec!["code_synthesis".into()],
            )
            .unwrap(),
            ProfilerTrainingExample::new(
                "math",
                "Prove an algebra theorem",
                vec!["mathematical_reasoning".into()],
            )
            .unwrap(),
        ],
        ProfilerPolicy::new(0.3, 0.05, 4).unwrap(),
    )
    .unwrap()
}

fn posterior(
    provider: &str,
    model: &str,
    skill_id: &str,
    positive: u64,
    negative: u64,
) -> SkillPosterior {
    SkillPosterior::new(
        route(provider, model),
        "skills/v1",
        skill_id,
        1.0,
        1.0,
        positive,
        negative,
    )
    .unwrap()
}

fn learner(posteriors: Vec<SkillPosterior>) -> BellaLearnerState {
    BellaLearnerState::new(
        taxonomy(),
        profiler(),
        "controlled-skill-critic/v1",
        "semantic-evaluator/v1",
        "bella-corpus/v1",
        "catalog/v1",
        BellaEstimatorPolicy::new(3, 0.0).unwrap(),
        posteriors,
    )
    .unwrap()
}

fn with_bella(state: &BellaLearnerState, prompt: &str) -> EvidenceSnapshot {
    let mut evidence = evidence();
    evidence.bella = state.freeze(prompt, "bella-state/v1", "catalog/v1");
    evidence
}

fn routes(ballot: &gaussmeridian_core::routing_policy::RoutingBallot) -> Vec<RouteIdentity> {
    ballot
        .entries()
        .iter()
        .map(|entry| route(&entry.provider_id, &entry.model_id))
        .collect()
}

#[test]
fn bella_cannot_resurrect_any_hard_exclusion_and_does_not_relabel_it() {
    let allowed = model("provider-ok", "allowed", 0.8);

    let mut no_credential = model("provider-credential", "no-credential", 1.0);
    no_credential.credential_available = false;
    let mut no_adapter = model("provider-adapter", "no-adapter", 1.0);
    no_adapter.adapter_registered = false;
    let mut unsupported = model("provider-unsupported", "unsupported", 1.0);
    unsupported.adapter_supports_model = false;
    let mut tenant_denied = model("provider-tenant", "tenant-denied", 1.0);
    tenant_denied.tenant_allowed = false;
    let mut compliance_denied = model("provider-compliance", "compliance-denied", 1.0);
    compliance_denied.compliant = false;
    let mut above_ceiling = model("provider-ceiling", "above-ceiling", 1.0);
    above_ceiling.capability_band = CapabilityBand::Frontier;
    let mut missing_hard_skill = model("provider-skill", "missing-hard-skill", 1.0);
    missing_hard_skill.skill_proficiency[3] = 0.1;

    let catalog_models = vec![
        no_credential,
        no_adapter,
        unsupported,
        tenant_denied,
        compliance_denied,
        above_ceiling,
        missing_hard_skill,
        allowed,
    ];
    let posteriors = catalog_models
        .iter()
        .map(|candidate| {
            posterior(
                &candidate.provider_id,
                &candidate.model_id,
                "code_synthesis",
                10,
                0,
            )
        })
        .collect();
    let state = learner(posteriors);
    let catalog = CatalogSnapshot::new(catalog_models);

    let inactive = select(&context(), &policy(), &catalog, &evidence()).unwrap();
    let active = select(
        &context(),
        &policy(),
        &catalog,
        &with_bella(&state, "Implement and compile Rust code"),
    )
    .unwrap();

    assert_eq!(active.exclusions, inactive.exclusions);
    assert_eq!(routes(&active), vec![route("provider-ok", "allowed")]);
    assert_eq!(active.bella.status(), BellaUseStatus::Applied);
    assert_eq!(
        active
            .bella
            .assessments()
            .iter()
            .map(|assessment| assessment.route().clone())
            .collect::<Vec<_>>(),
        vec![route("provider-ok", "allowed")]
    );
    assert!(active.entries().iter().all(|entry| {
        inactive.entries().iter().any(|predecessor| {
            predecessor.provider_id == entry.provider_id && predecessor.model_id == entry.model_id
        })
    }));
}

#[test]
fn bella_survivors_are_still_ordered_by_the_existing_risk_policy() {
    let catalog = CatalogSnapshot::new(vec![
        model("provider-c", "model-c", 0.70),
        model("provider-a", "model-a", 0.95),
        model("provider-b", "model-b", 0.80),
    ]);
    let state = learner(vec![
        posterior("provider-a", "model-a", "code_synthesis", 10, 0),
        posterior("provider-b", "model-b", "code_synthesis", 10, 0),
        posterior("provider-c", "model-c", "code_synthesis", 10, 0),
    ]);

    let inactive = select(&context(), &policy(), &catalog, &evidence()).unwrap();
    let active = select(
        &context(),
        &policy(),
        &catalog,
        &with_bella(&state, "Implement and compile Rust code"),
    )
    .unwrap();

    assert_eq!(routes(&active), routes(&inactive));
    assert_eq!(
        routes(&active),
        vec![
            route("provider-a", "model-a"),
            route("provider-b", "model-b"),
            route("provider-c", "model-c"),
        ]
    );
    assert_eq!(active.bella.status(), BellaUseStatus::Applied);
    assert_eq!(active.bella.summary().capable_route_count, 3);
}

#[test]
fn bella_narrowing_preserves_predecessor_order_across_band_ties() {
    let mut cheap_removed = model("provider-cheap", "cheap-baseline", 0.99);
    cheap_removed.capability_band = CapabilityBand::Baseline;
    cheap_removed.price.input_per_million = 1.0;
    cheap_removed.price.output_per_million = 1.0;

    let mut baseline_survivor = model("provider-baseline", "baseline-survivor", 0.80);
    baseline_survivor.capability_band = CapabilityBand::Baseline;
    baseline_survivor.price.input_per_million = 100.0;
    baseline_survivor.price.output_per_million = 100.0;

    let mut frontier_survivor = model("provider-frontier", "frontier-survivor", 0.80);
    frontier_survivor.capability_band = CapabilityBand::Frontier;
    frontier_survivor.price.input_per_million = 10.0;
    frontier_survivor.price.output_per_million = 10.0;

    let catalog = CatalogSnapshot::new(vec![frontier_survivor, baseline_survivor, cheap_removed]);
    let state = learner(vec![
        posterior("provider-cheap", "cheap-baseline", "code_synthesis", 0, 10),
        posterior(
            "provider-baseline",
            "baseline-survivor",
            "code_synthesis",
            10,
            0,
        ),
        posterior(
            "provider-frontier",
            "frontier-survivor",
            "code_synthesis",
            10,
            0,
        ),
    ]);
    let mut moderate_context = context();
    moderate_context.complexity = 0.5;
    let mut frontier_policy = policy();
    frontier_policy.max_band = CapabilityBand::Frontier;

    let inactive = select(&moderate_context, &frontier_policy, &catalog, &evidence()).unwrap();
    let active = select(
        &moderate_context,
        &frontier_policy,
        &catalog,
        &with_bella(&state, "Implement and compile Rust code"),
    )
    .unwrap();

    let active_route_set: std::collections::BTreeSet<_> = routes(&active).into_iter().collect();
    let predecessor_survivors: Vec<_> = routes(&inactive)
        .into_iter()
        .filter(|route| active_route_set.contains(route))
        .collect();

    assert_eq!(
        routes(&active),
        predecessor_survivors,
        "BELLA may remove a route but must not rerank the surviving predecessor ballot"
    );
    assert_eq!(
        active.band_decision, inactive.band_decision,
        "capability narrowing must not recompute the predecessor band decision"
    );
}

#[test]
fn unrelated_profile_preserves_the_predecessor_authorization_identity() {
    let catalog = CatalogSnapshot::new(vec![
        model("provider-a", "model-a", 0.95),
        model("provider-b", "model-b", 0.80),
    ]);
    let state = learner(vec![posterior(
        "provider-a",
        "model-a",
        "code_synthesis",
        10,
        0,
    )]);

    let inactive = select(&context(), &policy(), &catalog, &evidence()).unwrap();
    let unrelated = select(
        &context(),
        &policy(),
        &catalog,
        &with_bella(&state, "photosynthesis chlorophyll"),
    )
    .unwrap();

    assert_eq!(routes(&unrelated), routes(&inactive));
    assert_eq!(unrelated.exclusions, inactive.exclusions);
    assert_eq!(unrelated.bella.status(), BellaUseStatus::Abstained);
    assert_eq!(
        unrelated.content_id().unwrap(),
        inactive.content_id().unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&unrelated).unwrap(),
        serde_json::to_vec(&inactive).unwrap()
    );
    assert!(
        serde_json::to_value(&inactive)
            .unwrap()
            .get("bella")
            .is_none(),
        "authorization-neutral BELLA must preserve predecessor ballot bytes"
    );
}

#[test]
fn matching_profile_narrows_and_no_capable_route_falls_back_exactly() {
    let weak = model("provider-weak", "model-weak", 0.99);
    let strong = model("provider-strong", "model-strong", 0.80);
    let catalog = CatalogSnapshot::new(vec![weak, strong]);
    let inactive = select(&context(), &policy(), &catalog, &evidence()).unwrap();

    let one_capable = learner(vec![
        posterior("provider-weak", "model-weak", "code_synthesis", 0, 10),
        posterior("provider-strong", "model-strong", "code_synthesis", 10, 0),
    ]);
    let narrowed = select(
        &context(),
        &policy(),
        &catalog,
        &with_bella(&one_capable, "Implement and compile Rust code"),
    )
    .unwrap();
    assert_eq!(
        routes(&narrowed),
        vec![route("provider-strong", "model-strong")]
    );
    assert_eq!(narrowed.bella.status(), BellaUseStatus::Applied);

    let none_capable = learner(vec![
        posterior("provider-weak", "model-weak", "code_synthesis", 0, 10),
        posterior("provider-strong", "model-strong", "code_synthesis", 0, 10),
    ]);
    let fallback = select(
        &context(),
        &policy(),
        &catalog,
        &with_bella(&none_capable, "Implement and compile Rust code"),
    )
    .unwrap();
    assert_eq!(routes(&fallback), routes(&inactive));
    assert_eq!(fallback.bella.status(), BellaUseStatus::NoCapableFallback);
    assert_eq!(fallback.bella.summary().capable_route_count, 0);
}
