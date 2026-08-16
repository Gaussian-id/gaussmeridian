use gaussmeridian_core::routing_policy::{
    predictors::{
        FrozenPredictionEvidence, FrozenPredictionSet, PredictionEstimate,
        PredictionFallbackReason, PredictionFeatureVector, PredictionProvenance, RouteIdentity,
        RoutePrediction,
    },
    select, CapabilityBand, CatalogModel, CatalogSnapshot, DeploymentKind, EvidenceSnapshot, Price,
    ProjectPolicy, RoutingContext,
};

const FEATURE_VERSION: &str = "carrot-runtime-features/v1";

fn model(provider: &str, model: &str, prior: f64) -> CatalogModel {
    CatalogModel {
        model_id: model.to_string(),
        provider_id: provider.to_string(),
        model_version: "model/v1".to_string(),
        capability_band: CapabilityBand::Baseline,
        deployment_kind: DeploymentKind::Managed,
        price: Price {
            input_per_million: 10.0,
            output_per_million: 10.0,
            expected_fixed_cost: 0.0,
            fixed_cost_upper_bound: 1.0,
        },
        semantic_quality_prior: prior,
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

fn policy() -> ProjectPolicy {
    ProjectPolicy {
        cost_weight: 0.0,
        quality_floor: 0.0,
        max_band: CapabilityBand::Frontier,
        moderate_complexity_threshold: 0.35,
        high_complexity_threshold: 0.7,
        max_provider_attempts: 3,
        router_cost_upper_bound: 0.0,
    }
}

fn provenance() -> PredictionProvenance {
    PredictionProvenance::new(
        "carrot-knn/v1",
        FEATURE_VERSION,
        "semantic-evaluator/v1",
        "controlled-corpus/v1",
        "catalog/v1",
        "prices/v1",
        "state-sha256",
        "training-sha256",
    )
    .expect("valid provenance")
}

fn route_prediction(
    provider: &str,
    model: &str,
    outcome: PredictionEstimate,
    cost: PredictionEstimate,
) -> RoutePrediction {
    RoutePrediction::new(
        RouteIdentity::new(provider, model).expect("valid route"),
        outcome,
        cost,
    )
    .expect("valid route prediction")
}

fn active_predictions(predictions: Vec<RoutePrediction>) -> FrozenPredictionEvidence {
    FrozenPredictionEvidence::Active(
        FrozenPredictionSet::new(
            provenance(),
            PredictionFeatureVector::new(FEATURE_VERSION, vec![0.1, 6.9, 6.2])
                .expect("valid features"),
            predictions,
        )
        .expect("valid prediction set"),
    )
}

fn evidence(predictions: FrozenPredictionEvidence) -> EvidenceSnapshot {
    EvidenceSnapshot {
        policy_version: "policy/v1".to_string(),
        catalog_version: "catalog/v1".to_string(),
        price_version: "price/v1".to_string(),
        evaluator_version: "semantic-evaluator/v1".to_string(),
        normalized_cost_floor: 0.0,
        normalized_cost_ceiling: 1.0,
        complexity: None,
        predictions,
        bella: Default::default(),
        r2: Default::default(),
        compound: Default::default(),
    }
}

#[test]
fn learned_correctness_and_cost_rerank_hard_eligible_routes() {
    let catalog = CatalogSnapshot::new(vec![
        model("provider-a", "model-a", 0.95),
        model("provider-b", "model-b", 0.80),
    ]);
    let predictions = active_predictions(vec![
        route_prediction(
            "provider-a",
            "model-a",
            PredictionEstimate::estimated(0.40, 0.9).expect("valid outcome"),
            PredictionEstimate::expected_cost(0.02, 0.9).expect("valid cost"),
        ),
        route_prediction(
            "provider-b",
            "model-b",
            PredictionEstimate::estimated(0.90, 0.9).expect("valid outcome"),
            PredictionEstimate::expected_cost(0.01, 0.9).expect("valid cost"),
        ),
    ]);

    let ballot = select(&context(), &policy(), &catalog, &evidence(predictions))
        .expect("learned evidence selects");

    assert_eq!(ballot.entries()[0].provider_id, "provider-b");
    assert_eq!(ballot.entries()[0].model_id, "model-b");
    assert_eq!(ballot.entries()[0].delivered_correctness_probability, 0.90);
    assert_eq!(ballot.entries()[0].expected_provider_cost, 0.01);
    assert!(matches!(
        ballot.entries()[0].outcome_prediction,
        PredictionEstimate::Estimated { value: 0.90, .. }
    ));
}

#[test]
fn learned_predictions_cannot_resurrect_a_hard_excluded_route() {
    let allowed = model("provider-a", "model-a", 0.80);
    let mut denied = model("provider-b", "model-b", 0.99);
    denied.tenant_allowed = false;
    let predictions = active_predictions(vec![
        route_prediction(
            "provider-a",
            "model-a",
            PredictionEstimate::estimated(0.50, 0.9).expect("valid outcome"),
            PredictionEstimate::expected_cost(0.02, 0.9).expect("valid cost"),
        ),
        route_prediction(
            "provider-b",
            "model-b",
            PredictionEstimate::estimated(1.0, 1.0).expect("valid outcome"),
            PredictionEstimate::expected_cost(0.0, 1.0).expect("valid cost"),
        ),
    ]);

    let ballot = select(
        &context(),
        &policy(),
        &CatalogSnapshot::new(vec![denied, allowed]),
        &evidence(predictions),
    )
    .expect("one hard-eligible route remains");

    assert_eq!(ballot.entries().len(), 1);
    assert_eq!(ballot.entries()[0].provider_id, "provider-a");
    assert_eq!(ballot.exclusions.len(), 1);
    assert_eq!(ballot.exclusions[0].provider_id, "provider-b");
}

#[test]
fn typed_abstention_reproduces_p1_values_and_order() {
    let catalog = CatalogSnapshot::new(vec![
        model("provider-a", "model-a", 0.95),
        model("provider-b", "model-b", 0.80),
    ]);
    let prior = select(
        &context(),
        &policy(),
        &catalog,
        &evidence(FrozenPredictionEvidence::unavailable(
            PredictionFallbackReason::NoActiveState,
        )),
    )
    .expect("prior fallback selects");
    let abstained = PredictionEstimate::abstained(PredictionFallbackReason::OutOfDistribution, 1.0)
        .expect("valid abstention");
    let active = select(
        &context(),
        &policy(),
        &catalog,
        &evidence(active_predictions(vec![
            route_prediction(
                "provider-a",
                "model-a",
                abstained.clone(),
                abstained.clone(),
            ),
            route_prediction("provider-b", "model-b", abstained.clone(), abstained),
        ])),
    )
    .expect("typed route abstention selects");

    let prior_values = prior
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.provider_id.as_str(),
                entry.model_id.as_str(),
                entry.delivered_correctness_probability,
                entry.expected_provider_cost,
                entry.risk,
            )
        })
        .collect::<Vec<_>>();
    let active_values = active
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.provider_id.as_str(),
                entry.model_id.as_str(),
                entry.delivered_correctness_probability,
                entry.expected_provider_cost,
                entry.risk,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(prior_values, active_values);
    assert!(matches!(
        active.entries()[0].cost_prediction,
        PredictionEstimate::Abstained {
            reason: PredictionFallbackReason::OutOfDistribution,
            ..
        }
    ));
}

#[test]
fn production_selection_revalidates_deserialized_prediction_evidence() {
    let predictions = active_predictions(vec![route_prediction(
        "provider-a",
        "model-a",
        PredictionEstimate::estimated(0.8, 0.9).expect("valid outcome"),
        PredictionEstimate::expected_cost(0.02, 0.9).expect("valid cost"),
    )]);
    let mut serialized = serde_json::to_value(predictions).expect("serialize evidence");
    serialized["predictions"][0]["outcome"]["value"] = serde_json::json!(2.0);
    let tampered: FrozenPredictionEvidence =
        serde_json::from_value(serialized).expect("serde accepts persisted representation");

    assert!(select(
        &context(),
        &policy(),
        &CatalogSnapshot::new(vec![model("provider-a", "model-a", 0.8)]),
        &evidence(tampered),
    )
    .is_err());
}
