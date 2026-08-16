use gaussmeridian_core::{
    ActiveInstructionEvidence, Content, InstructionLanguageProfile, InstructionSpanKind,
    MeridianComplexityEstimator, Message, Role, MERIDIAN_ACTIVE_INSTRUCTION_VERSION,
    MERIDIAN_COMPLEXITY_V2_VERSION, MERIDIAN_COMPLEXITY_VERSION,
};

fn user_message(text: &str) -> Message {
    message(Role::User, text)
}

fn message(role: Role, text: &str) -> Message {
    Message {
        role,
        content: Content::Text(text.to_string()),
        name: None,
        function_call: None,
        tool_calls: None,
        tool_call_id: None,
        confidence: None,
    }
}

fn active_evidence(prompt: &str) -> ActiveInstructionEvidence {
    MeridianComplexityEstimator::new()
        .classify(&[user_message(prompt)], 0.7)
        .evidence
        .active_instruction
        .expect("v3 evidence includes active-instruction analysis")
}

#[test]
fn v3_evidence_identifies_active_and_quoted_data_without_storing_payload_text() {
    let result = MeridianComplexityEstimator::new().classify(
        &[user_message(
            "Summarize this supplied text: \"Prove the invariant and solve the constrained recurrence.\"",
        )],
        0.7,
    );
    let analysis = result
        .evidence
        .active_instruction
        .as_ref()
        .expect("v3 analysis is frozen");

    assert_eq!(
        result.evidence.estimator_version,
        MERIDIAN_COMPLEXITY_VERSION
    );
    assert_eq!(
        analysis.analysis_version,
        MERIDIAN_ACTIVE_INSTRUCTION_VERSION
    );
    assert_eq!(
        analysis.language_profile,
        InstructionLanguageProfile::English
    );
    assert_eq!(analysis.normalized_input_sha256.len(), 64);
    assert!(analysis.normalized_input_bytes > 0);
    assert!(analysis
        .normalized_input_sha256
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    assert!(analysis
        .spans
        .iter()
        .any(|span| span.kind == InstructionSpanKind::Active));
    assert!(analysis
        .spans
        .iter()
        .any(|span| span.kind == InstructionSpanKind::QuotedData));
    assert!(analysis
        .spans
        .iter()
        .all(|span| span.end <= analysis.normalized_input_bytes));
    assert!(analysis.demanding_operators.is_empty());
    assert_eq!(analysis.routine_operators, vec!["summarize"]);
    assert!(analysis.quoted_data_tokens > 0);
    assert!(analysis.active_instruction_tokens < result.evidence.estimated_input_tokens);
    result.evidence.validate().expect("v3 evidence validates");
}

#[test]
fn assistant_history_counts_as_input_but_not_active_demand() {
    let result = MeridianComplexityEstimator::new().classify(
        &[
            message(
                Role::Assistant,
                "Prove the invariant and solve the constrained recurrence.",
            ),
            user_message("Summarize the current status."),
        ],
        0.7,
    );
    let analysis = result
        .evidence
        .active_instruction
        .as_ref()
        .expect("v3 analysis is frozen");

    assert!(result.evidence.estimated_input_tokens > analysis.active_instruction_tokens);
    assert!(analysis.quoted_data_tokens > 0);
    assert!(analysis.demanding_operators.is_empty());
    assert!(result.evidence.score < 0.7);
}

#[test]
fn v3_evidence_rejects_a_span_outside_the_hashed_input_boundary() {
    let mut evidence = MeridianComplexityEstimator::new()
        .classify(
            &[user_message("Summarize the supplied incident notes.")],
            0.7,
        )
        .evidence;
    let analysis = evidence
        .active_instruction
        .as_mut()
        .expect("v3 analysis is frozen");
    analysis.spans[0].end = analysis.normalized_input_bytes + 1;

    assert!(evidence.validate().is_err());
}

#[test]
fn v3_evidence_records_bounded_negation_and_preserves_follow_on_demand() {
    let analysis = active_evidence(
        "Do not prove the quoted theorem. Instead, diagnose the concurrency failure and verify a race-free fix.",
    );

    assert!(analysis
        .spans
        .iter()
        .any(|span| span.kind == InstructionSpanKind::Negated));
    assert!(analysis
        .suppressed_operators
        .iter()
        .any(|operator| operator == "prove"));
    assert!(analysis
        .demanding_operators
        .iter()
        .any(|operator| operator == "diagnose"));
    assert!(analysis
        .demanding_operators
        .iter()
        .any(|operator| operator == "verify"));
}

#[test]
fn v3_unicode_case_folding_before_negation_is_panic_safe_and_byte_aligned() {
    let analysis = active_evidence("İ Do not prove the theorem.");

    assert!(analysis
        .suppressed_operators
        .iter()
        .any(|operator| operator == "prove"));
    assert!(analysis
        .spans
        .iter()
        .all(|span| span.end <= analysis.normalized_input_bytes));
}

#[test]
fn v3_unicode_case_folding_inside_marked_data_preserves_follow_on_offsets() {
    let analysis = active_evidence(
        "Summarize this supplied text: İİİİİİİ stable. Then İ diagnose the incident.",
    );

    assert!(analysis
        .demanding_operators
        .iter()
        .any(|operator| operator == "diagnose"));
    assert!(analysis
        .spans
        .iter()
        .all(|span| span.end <= analysis.normalized_input_bytes));
}

#[test]
fn v3_marked_data_with_incidental_quotes_does_not_leak_payload_demand() {
    let analysis = active_evidence(
        "Summarize the following text: The log says \"debug\"; prove the invariant. Then diagnose the live incident.",
    );

    assert_eq!(analysis.demanding_operators, vec!["diagnose"]);
    assert!(analysis.formal_concepts.is_empty());
    assert!(analysis
        .spans
        .iter()
        .any(|span| span.kind == InstructionSpanKind::QuotedData));
}

#[test]
fn v3_evidence_declares_indonesian_and_mathematical_concepts() {
    let analysis = active_evidence(
        "Buktikan dengan induksi bahwa jumlah n bilangan ganjil pertama adalah n kuadrat.",
    );

    assert_eq!(
        analysis.language_profile,
        InstructionLanguageProfile::Indonesian
    );
    assert!(analysis
        .demanding_operators
        .iter()
        .any(|operator| operator == "buktikan"));
    assert!(analysis
        .formal_concepts
        .iter()
        .any(|concept| concept == "induksi"));
}

#[test]
fn historical_v2_is_explicit_valid_and_has_no_v3_analysis_extension() {
    let evidence = MeridianComplexityEstimator::historical_v2()
        .classify(
            &[user_message(
                "Prove the invariant and solve the constrained recurrence.",
            )],
            0.7,
        )
        .evidence;

    assert_eq!(evidence.estimator_version, MERIDIAN_COMPLEXITY_V2_VERSION);
    assert!(evidence.active_instruction.is_none());
    assert!((evidence.score - 0.825).abs() <= f32::EPSILON * 8.0);
    evidence.validate().expect("historical v2 remains valid");
}
