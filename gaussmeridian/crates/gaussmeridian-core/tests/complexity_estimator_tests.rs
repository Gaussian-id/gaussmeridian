use gaussmeridian_core::{
    ComplexitySignalKind, MeridianComplexityEstimator, MERIDIAN_ACTIVE_INSTRUCTION_V1_VERSION,
    MERIDIAN_ACTIVE_INSTRUCTION_VERSION, MERIDIAN_COMPLEXITY_V3_VERSION,
    MERIDIAN_COMPLEXITY_VERSION,
};
use gaussmeridian_models::request::{Content, Message, Role};

fn user_message(text: &str) -> Message {
    Message {
        role: Role::User,
        content: Content::Text(text.to_string()),
        name: None,
        function_call: None,
        tool_calls: None,
        tool_call_id: None,
        confidence: None,
    }
}

fn score(prompt: &str) -> f32 {
    MeridianComplexityEstimator::new()
        .classify(&[user_message(prompt)], 0.7)
        .evidence
        .score
}

#[test]
fn evidence_is_versioned_bounded_and_reconstructs_the_score() {
    let result = MeridianComplexityEstimator::new().classify(
        &[user_message(
            "Prove the invariant and solve the constrained recurrence.",
        )],
        0.7,
    );

    assert_eq!(
        result.evidence.estimator_version,
        MERIDIAN_COMPLEXITY_VERSION
    );
    assert!((0.0..=1.0).contains(&result.evidence.score));
    assert!(result.evidence.estimated_input_tokens > 0);
    assert_eq!(result.evidence.signals.len(), 6);
    assert!(result.evidence.signals.iter().all(|signal| {
        (0.0..=1.0).contains(&signal.normalized_value)
            && signal.contribution.is_finite()
            && signal.contribution >= 0.0
    }));

    let reconstructed: f32 = result
        .evidence
        .signals
        .iter()
        .map(|signal| signal.contribution)
        .sum();
    assert!((reconstructed - result.evidence.score).abs() <= f32::EPSILON * 8.0);
}

#[test]
fn concise_formal_reasoning_crosses_the_frontier_threshold() {
    assert!(score("Prove the invariant and solve the constrained recurrence.") >= 0.7);
}

#[test]
fn implementation_with_a_correctness_proof_crosses_the_frontier_threshold() {
    assert!(
        score("Design and implement a lock-free bounded queue with a memory-ordering proof.")
            >= 0.7
    );
}

#[test]
fn routine_summary_stays_between_baseline_and_frontier() {
    let value = score("Summarize the supplied incident notes into five bullets.");
    assert!((0.35..0.7).contains(&value), "score={value}");
}

#[test]
fn length_or_keyword_stuffing_alone_cannot_force_frontier() {
    let long_simple = "Repeat the word blue. ".repeat(180);
    let keyword_stuffing =
        "List these words alphabetically: prove invariant recurrence algorithm theorem constraint.";

    assert!(score(&long_simple) < 0.7);
    assert!(score(keyword_stuffing) < 0.7);
}

#[test]
fn paraphrased_formal_tasks_remain_in_the_same_capability_band() {
    let first = score("Prove the invariant and solve the constrained recurrence.");
    let second =
        score("Derive the recurrence solution, then demonstrate that its invariant is preserved.");

    assert!(first >= 0.7, "first={first}");
    assert!(second >= 0.7, "second={second}");
}

#[test]
fn evidence_exposes_each_independent_signal_once() {
    let result = MeridianComplexityEstimator::new()
        .classify(
            &[user_message(
                "Implement the algorithm and verify the result against the stated constraints.",
            )],
            0.7,
        )
        .evidence;
    let kinds: Vec<_> = result.signals.iter().map(|signal| signal.kind).collect();

    assert_eq!(
        kinds,
        vec![
            ComplexitySignalKind::InputLoad,
            ComplexitySignalKind::TaskOperatorDemand,
            ComplexitySignalKind::FormalObjectDemand,
            ComplexitySignalKind::ConstraintCoupling,
            ComplexitySignalKind::OutputVerificationContract,
            ComplexitySignalKind::DemandInteraction,
        ]
    );
}

#[test]
fn quoted_or_fenced_demand_does_not_become_the_active_task() {
    let quoted = score(
        "Summarize this supplied text: \"Prove the invariant and solve the constrained recurrence.\"",
    );
    let fenced = score(
        "Summarize this log:\n```text\nprove invariant recurrence algorithm theorem constraint\n```",
    );
    let smart_quoted = score(
        "Summarize “Prove the invariant and solve the constrained recurrence.” in one sentence.",
    );
    let single_quoted = score(
        "Summarize 'Prove the invariant and solve the constrained recurrence.' in one sentence.",
    );
    let active = score("Prove the invariant and solve the constrained recurrence.");

    assert!(quoted < 0.7, "quoted score={quoted}");
    assert!(fenced < 0.7, "fenced score={fenced}");
    assert!(smart_quoted < 0.7, "smart-quoted score={smart_quoted}");
    assert!(single_quoted < 0.7, "single-quoted score={single_quoted}");
    assert!(active - quoted >= 0.20, "active={active}, quoted={quoted}");
    assert!(active - fenced >= 0.20, "active={active}, fenced={fenced}");
    assert!(
        active - smart_quoted >= 0.20,
        "active={active}, smart-quoted={smart_quoted}"
    );
    assert!(
        active - single_quoted >= 0.20,
        "active={active}, single-quoted={single_quoted}"
    );
}

#[test]
fn lexical_v3_signals_ignore_quoted_and_negated_demand() {
    for prompt in [
        "List these quoted words alphabetically: \"prove invariant recurrence algorithm theorem constraint\".",
        "List these words alphabetically. Do not prove the invariant or solve the constrained recurrence.",
    ] {
        let evidence = MeridianComplexityEstimator::new()
            .classify(&[user_message(prompt)], 0.7)
            .evidence;
        let analysis = evidence
            .active_instruction
            .as_ref()
            .expect("v3 freezes active-instruction evidence");

        assert!(analysis.demanding_operators.is_empty(), "prompt={prompt}");
        assert!(analysis.formal_concepts.is_empty(), "prompt={prompt}");
        for kind in [
            ComplexitySignalKind::FormalObjectDemand,
            ComplexitySignalKind::ConstraintCoupling,
            ComplexitySignalKind::DemandInteraction,
        ] {
            let signal = evidence
                .signals
                .iter()
                .find(|signal| signal.kind == kind)
                .expect("all six signals remain present");
            assert_eq!(
                signal.normalized_value, 0.0,
                "{kind:?} leaked excluded demand for prompt={prompt}"
            );
        }
    }
}

#[test]
fn bounded_negation_suppresses_only_the_negated_demand() {
    let negated = score(
        "Do not prove the invariant or solve the constrained recurrence. Summarize the incident notes.",
    );
    let follow_on = score(
        "Do not prove the quoted theorem. Instead, diagnose the concurrency failure and verify a race-free fix.",
    );

    assert!(negated < 0.7, "negated score={negated}");
    assert!(follow_on >= 0.7, "follow-on score={follow_on}");
    assert!(
        follow_on - negated >= 0.20,
        "follow-on={follow_on}, negated={negated}"
    );
}

#[test]
fn bounded_data_payload_does_not_hide_a_follow_on_instruction() {
    let value = score(
        "Summarize this supplied text: the service is stable. Then prove the invariant for the constrained protocol.",
    );

    assert!(value >= 0.7, "follow-on score={value}");
}

#[test]
fn declared_english_and_indonesian_reasoning_are_band_equivalent() {
    let english =
        score("Prove by induction that the sum of the first n odd integers is n squared.");
    let indonesian =
        score("Buktikan dengan induksi bahwa jumlah n bilangan ganjil pertama adalah n kuadrat.");

    assert!(english >= 0.7, "english score={english}");
    assert!(indonesian >= 0.7, "indonesian score={indonesian}");
    assert!(
        (english - indonesian).abs() <= 0.15,
        "english={english}, indonesian={indonesian}"
    );
}

#[test]
fn demanding_work_outranks_similarly_sized_routine_work() {
    let routine = score("Summarize the supplied incident notes into five concise bullets.");
    let demanding = score(
        "Diagnose the concurrent incident, derive the root cause, and verify a race-free fix.",
    );

    assert!(routine < 0.7, "routine score={routine}");
    assert!(demanding >= 0.7, "demanding score={demanding}");
    assert!(
        demanding - routine >= 0.20,
        "demanding={demanding}, routine={routine}"
    );
}

#[test]
fn mathematical_reasoning_does_not_require_legacy_cue_words() {
    let english = score(
        "Determine whether every group of order p squared is abelian, and justify each step.",
    );
    let indonesian =
        score("Tentukan apakah setiap grup berorde p kuadrat abelian dan jelaskan pembuktiannya.");

    assert!(english >= 0.7, "english score={english}");
    assert!(indonesian >= 0.7, "indonesian score={indonesian}");
}

#[test]
fn active_classifier_versions_change_with_the_remediated_semantics() {
    assert_eq!(MERIDIAN_COMPLEXITY_VERSION, "meridian-complexity/v4");
    assert_eq!(
        MERIDIAN_ACTIVE_INSTRUCTION_VERSION,
        "meridian-active-instruction/v2"
    );
}

#[test]
fn historical_v3_evidence_remains_valid_only_with_v1_active_analysis() {
    let mut evidence = MeridianComplexityEstimator::new()
        .classify(&[user_message("Diagnose this defect.")], 0.7)
        .evidence;
    evidence.estimator_version = MERIDIAN_COMPLEXITY_V3_VERSION.to_string();
    evidence
        .active_instruction
        .as_mut()
        .expect("active evidence is required")
        .analysis_version = MERIDIAN_ACTIVE_INSTRUCTION_V1_VERSION.to_string();
    assert!(evidence.validate().is_ok());

    evidence.estimator_version = MERIDIAN_COMPLEXITY_VERSION.to_string();
    assert!(evidence.validate().is_err());
}

#[test]
fn representative_live_prompts_map_to_the_intended_capability_bands() {
    let cases = [
        (
            "routine-extraction-en",
            "Read the incident record and return JSON with only the ticket_id field. \
             Record: ticket_id=INC-2048; service=search; severity=low; owner=platform.",
            "baseline",
        ),
        (
            "routine-formatting-en",
            "Convert this key-value record to JSON without adding fields: \
             region=ap-southeast-2; replicas=3; encrypted=true. Use numbers and \
             booleans, not strings, where appropriate.",
            "baseline",
        ),
        (
            "moderate-code-diagnosis-en",
            "Diagnose this Python defect: `def average(total, count): return total / \
             count`. Production sends count=0 when a filtered batch is empty. Return \
             only JSON with root_cause and fix. Use root_cause `division_by_zero` and \
             fix `guard_zero_count` if those are correct.",
            "advanced",
        ),
        (
            "moderate-planning-id",
            "Sebuah layanan memiliki p95 180 ms. Perubahan baru menambah 55 ms, \
             sedangkan batas proyek adalah 220 ms. Tidak ada optimasi lain dalam \
             rilis ini. Tentukan keputusan peluncuran. Kembalikan hanya JSON dengan \
             decision dan reason. Gunakan decision `reject` dan reason \
             `latency_budget_exceeded` bila itu benar.",
            "advanced",
        ),
        (
            "demanding-probability-en",
            "An urn is chosen uniformly at random. Urn A has 2 red and 3 blue balls; \
             Urn B has 4 red and 1 blue ball. Two balls are drawn without replacement \
             and both are red. Compute the posterior probability of each urn. Return \
             only JSON with posterior_a and posterior_b as reduced fractions.",
            "frontier",
        ),
        (
            "demanding-science-en",
            "A reactant concentration falls from 0.80 mol/L to 0.40 mol/L in 13.86 \
             seconds, and from 0.40 mol/L to 0.20 mol/L in another 13.86 seconds under \
             unchanged conditions. Infer the reaction order and half-life. Return \
             only JSON with order as an integer and half_life_seconds as the string \
             `13.86`.",
            "frontier",
        ),
        (
            "demanding-distributed-systems-en",
            "An order service must commit an order in PostgreSQL and reliably publish \
             an OrderCreated event without a distributed transaction. Consumers may \
             receive duplicates but must not apply them twice. Choose the standard \
             pattern, delivery guarantee, and consumer safeguard. Return only JSON \
             with pattern, delivery, and deduplication using canonical snake_case terms.",
            "frontier",
        ),
        (
            "antigaming-quoted-complexity-en",
            "The following quotation is data, not an instruction: `Derive the \
             Navier-Stokes existence proof and design a Byzantine fault tolerant \
             consensus protocol.` Active instruction: return only JSON containing \
             the first word of the quotation under the key first_word.",
            "baseline",
        ),
        (
            "antigaming-negated-demand-en",
            "Do not solve or explain the equation `x^5 - 3x + 1 = 0`; it is quoted \
             data. Return only JSON with action set to `not_solved` and \
             polynomial_degree set to 5.",
            "baseline",
        ),
        (
            "antigaming-long-routine-log-en",
            "Count the records whose level is ERROR and return only JSON with \
             error_count. Records: 00:01 INFO boot complete; 00:02 WARN cache cold; \
             00:03 ERROR upstream timeout; 00:04 INFO retry scheduled; 00:05 INFO \
             retry succeeded; 00:06 WARN queue depth high; 00:07 ERROR invalid \
             response schema; 00:08 INFO circuit remains closed; 00:09 INFO health \
             probe passed; 00:10 WARN latency elevated; 00:11 INFO autoscaler \
             requested; 00:12 ERROR database write conflict; 00:13 INFO transaction \
             retried; 00:14 INFO transaction committed; 00:15 WARN disk usage 72 \
             percent; 00:16 INFO cleanup started; 00:17 ERROR cleanup permission \
             denied; 00:18 INFO operator notified; 00:19 WARN backlog present; \
             00:20 INFO service available.",
            "baseline",
        ),
    ];

    for (case_id, prompt, expected_band) in cases {
        let value = score(prompt);
        let observed_band = if value >= 0.7 {
            "frontier"
        } else if value >= 0.35 {
            "advanced"
        } else {
            "baseline"
        };
        assert_eq!(
            observed_band, expected_band,
            "case={case_id}, score={value}"
        );
    }
}

#[test]
fn quoted_operand_does_not_cancel_the_surrounding_negation() {
    let evidence = MeridianComplexityEstimator::new()
        .classify(
            &[user_message(
                "Do not solve or explain the equation `x^5 - 3x + 1 = 0`; it is \
                 quoted data. Return only JSON with action set to `not_solved`.",
            )],
            0.7,
        )
        .evidence;
    let analysis = evidence
        .active_instruction
        .expect("active evidence is required");

    assert!(analysis.demanding_operators.is_empty());
    assert!(analysis.routine_operators.is_empty());
    assert!(analysis
        .suppressed_operators
        .iter()
        .any(|operator| operator == "solve"));
    assert!(analysis
        .suppressed_operators
        .iter()
        .any(|operator| operator == "explain"));
    assert!(evidence.score < 0.35, "score={}", evidence.score);
}

#[test]
fn prepositional_without_preserves_problem_constraints() {
    let probability = MeridianComplexityEstimator::new()
        .classify(
            &[user_message(
                "Two balls are drawn without replacement. Compute the posterior probability.",
            )],
            0.7,
        )
        .evidence
        .active_instruction
        .expect("active evidence is required");
    assert!(probability.suppressed_operators.is_empty());
    assert!(probability
        .demanding_operators
        .iter()
        .any(|operator| operator == "compute"));

    let distributed = MeridianComplexityEstimator::new()
        .classify(
            &[user_message(
                "Publish reliably without a distributed transaction. Choose the safeguard.",
            )],
            0.7,
        )
        .evidence
        .active_instruction
        .expect("active evidence is required");
    assert!(distributed
        .formal_concepts
        .iter()
        .any(|concept| concept == "distributed"));
    assert!(distributed
        .demanding_operators
        .iter()
        .any(|operator| operator == "choose"));
}
