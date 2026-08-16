use std::collections::BTreeSet;

use gaussmeridian_core::routing_policy::{
    requirements::{AdapterFeature, AdvisoryFeatures, Modality, RequirementError},
    CapabilityBand, DeploymentKind, SkillRequirement,
};
use gaussmeridian_server::routing::generation::{GenerationRequest, GenerationTransport};
use gaussmeridian_server::routing::requirements::{
    normalize_transport_requirements, strip_internal_chat_extensions,
    strip_internal_completion_extensions, ExplicitTransportRequirements, ResponseFormat,
    TransportEndpoint,
};

fn explicit_request() -> ExplicitTransportRequirements {
    ExplicitTransportRequirements {
        requires_tools: true,
        response_format: Some(ResponseFormat::JsonSchema),
        required_modalities: BTreeSet::from([Modality::Text, Modality::ImageInput]),
        required_skills: vec![SkillRequirement {
            skill_index: 2,
            minimum_proficiency: 0.75,
        }],
        allowed_deployments: BTreeSet::from([DeploymentKind::Managed]),
        compliance: BTreeSet::from(["au-residency".to_string()]),
        allowed_model_ids: BTreeSet::from(["frontier-a".to_string()]),
        denied_model_ids: BTreeSet::from(["legacy-a".to_string()]),
        absolute_capability_ceiling: CapabilityBand::Frontier,
        advisory: AdvisoryFeatures::default(),
    }
}

#[test]
fn equivalent_chat_stream_and_text_inputs_normalize_identically() {
    let chat =
        normalize_transport_requirements(TransportEndpoint::Chat, explicit_request()).unwrap();
    let stream =
        normalize_transport_requirements(TransportEndpoint::ChatStream, explicit_request())
            .unwrap();
    let text =
        normalize_transport_requirements(TransportEndpoint::Text, explicit_request()).unwrap();

    assert_eq!(chat, stream);
    assert_eq!(stream, text);
    assert!(chat
        .hard()
        .required_adapter_features
        .contains(&AdapterFeature::ToolUse));
    assert!(chat
        .hard()
        .required_adapter_features
        .contains(&AdapterFeature::JsonSchemaResponse));
}

#[test]
fn malformed_explicit_transport_requirements_fail_before_routing() {
    let mut request = explicit_request();
    request.denied_model_ids.insert("frontier-a".to_string());

    let error = normalize_transport_requirements(TransportEndpoint::Chat, request).unwrap_err();

    assert_eq!(
        error,
        RequirementError::ModelAllowDenyConflict {
            model_id: "frontier-a".to_string(),
        }
    );
}

#[test]
fn canonical_generation_endpoints_derive_streaming_from_the_wire_flag() {
    let chat = GenerationRequest::from_http(
        "/v1/chat/completions",
        br#"{"model":"auto","messages":[{"role":"user","content":"hello"}],"stream":true}"#,
    )
    .expect("chat request parses");
    let text = GenerationRequest::from_http(
        "/v1/completions",
        br#"{"model":"auto","prompt":"hello","stream":true}"#,
    )
    .expect("text request parses");

    assert_eq!(chat.transport(), GenerationTransport::Streaming);
    assert_eq!(text.transport(), GenerationTransport::Streaming);
}

#[test]
fn legacy_chat_stream_route_forces_streaming_without_a_wire_flag() {
    let request = GenerationRequest::from_http(
        "/v1/chat/completions/stream",
        br#"{"model":"auto","messages":[{"role":"user","content":"hello"}]}"#,
    )
    .expect("legacy stream request parses");

    assert_eq!(request.transport(), GenerationTransport::Streaming);
}

#[test]
fn equivalent_native_chat_and_text_requests_normalize_identically() {
    let chat = GenerationRequest::from_http(
        "/v1/chat/completions",
        br#"{"model":"auto","messages":[{"role":"user","content":"hello"}]}"#,
    )
    .expect("chat request parses");
    let text =
        GenerationRequest::from_http("/v1/completions", br#"{"model":"auto","prompt":"hello"}"#)
            .expect("text request parses");

    assert_eq!(
        chat.requirements(&[false; 12])
            .expect("chat requirements normalize"),
        text.requirements(&[false; 12])
            .expect("text requirements normalize")
    );
}

#[test]
fn explicit_wire_requirements_are_hard_and_transport_invariant() {
    let chat = GenerationRequest::from_http(
        "/v1/chat/completions",
        br#"{
            "model": "auto",
            "messages": [{"role": "user", "content": "implement rust checkout parser code"}],
            "routing_requirements": {
                "required_skills": [
                    {"skill_index": 2, "minimum_proficiency": 0.8}
                ],
                "compliance": ["au-residency"],
                "allowed_deployments": ["managed"],
                "denied_model_ids": ["legacy-a"],
                "absolute_capability_ceiling": "advanced"
            }
        }"#,
    )
    .expect("chat request parses");
    let text = GenerationRequest::from_http(
        "/v1/completions",
        br#"{
            "model": "auto",
            "prompt": "implement rust checkout parser code",
            "routing_requirements": {
                "required_skills": [
                    {"skill_index": 2, "minimum_proficiency": 0.8}
                ],
                "compliance": ["au-residency"],
                "allowed_deployments": ["managed"],
                "denied_model_ids": ["legacy-a"],
                "absolute_capability_ceiling": "advanced"
            }
        }"#,
    )
    .expect("text request parses");
    let mut inferred = [false; 12];
    inferred[7] = true;

    let chat = chat
        .requirements(&inferred)
        .expect("chat requirements normalize");
    let text = text
        .requirements(&inferred)
        .expect("text requirements normalize");

    assert_eq!(chat, text);
    assert_eq!(
        chat.hard().required_skills,
        vec![SkillRequirement {
            skill_index: 2,
            minimum_proficiency: 0.8,
        }]
    );
    assert_eq!(
        chat.advisory().inferred_skills,
        vec![SkillRequirement {
            skill_index: 7,
            minimum_proficiency: 0.5,
        }]
    );
    assert_eq!(
        chat.hard().compliance,
        BTreeSet::from(["au-residency".to_string()])
    );
    assert_eq!(
        chat.hard().allowed_deployments,
        BTreeSet::from([DeploymentKind::Managed])
    );
    assert_eq!(
        chat.hard().denied_model_ids,
        BTreeSet::from(["legacy-a".to_string()])
    );
    assert_eq!(
        chat.hard().absolute_capability_ceiling,
        CapabilityBand::Advanced
    );
}

#[test]
fn malformed_wire_requirements_fail_before_snapshot_or_selection() {
    for body in [
        br#"{
            "model": "auto",
            "messages": [{"role": "user", "content": "hello"}],
            "routing_requirements": {
                "absolute_capability_ceiling": "unbounded"
            }
        }"#
        .as_slice(),
        br#"{
            "model": "auto",
            "messages": [{"role": "user", "content": "hello"}],
            "routing_requirements": {
                "allowed_deployments": []
            }
        }"#
        .as_slice(),
        br#"{
            "model": "auto",
            "messages": [{"role": "user", "content": "hello"}],
            "routing_requirements": {
                "required_skills": [
                    {"skill_index": 1, "minimum_proficiency": 0.7},
                    {"skill_index": 1, "minimum_proficiency": 0.8}
                ]
            }
        }"#
        .as_slice(),
    ] {
        let request = GenerationRequest::from_http("/v1/chat/completions", body)
            .expect("generation request shape parses");

        assert!(
            request.requirements(&[false; 12]).is_err(),
            "malformed explicit routing requirements reached selection"
        );
    }
}

#[test]
fn provider_payloads_drop_only_internal_routing_requirements() {
    let mut chat = match GenerationRequest::from_http(
        "/v1/chat/completions",
        br#"{
            "model": "auto",
            "messages": [{"role": "user", "content": "hello"}],
            "routing_requirements": {"compliance": ["au-residency"]},
            "provider_passthrough": {"priority": "high"}
        }"#,
    )
    .expect("chat request parses")
    {
        GenerationRequest::Chat { request, .. } => *request,
        GenerationRequest::Text { .. } => panic!("expected chat request"),
    };
    let mut text = match GenerationRequest::from_http(
        "/v1/completions",
        br#"{
            "model": "auto",
            "prompt": "hello",
            "routing_requirements": {"compliance": ["au-residency"]},
            "provider_passthrough": {"priority": "high"}
        }"#,
    )
    .expect("text request parses")
    {
        GenerationRequest::Text { request, .. } => request,
        GenerationRequest::Chat { .. } => panic!("expected text request"),
    };

    strip_internal_chat_extensions(&mut chat);
    strip_internal_completion_extensions(&mut text);

    for extra in [&chat.extra, &text.extra] {
        assert!(!extra.contains_key("routing_requirements"));
        assert_eq!(
            extra.get("provider_passthrough"),
            Some(&serde_json::json!({"priority": "high"}))
        );
    }
}
