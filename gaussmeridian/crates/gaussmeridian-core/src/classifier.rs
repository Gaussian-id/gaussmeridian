//! Deterministic Meridian request-complexity estimation.
//!
//! This module is a product heuristic, not a CARROT predictor. CARROT predicts
//! conditional model outcomes and costs from learned evidence; the estimator
//! here produces a versioned routing input from inspectable prompt signals.
//!
//! ## Routing rule
//! ```text
//! complexity_score < τ_moa  → single-model routing path
//! complexity_score ≥ τ_moa  → moa_flagged = true  (logged; NOT dispatched in MVP)
//! τ_moa is per-project config (default 0.7)
//! ```

use crate::active_instruction::{
    analyze, ActiveInstructionEvidence, MERIDIAN_ACTIVE_INSTRUCTION_V1_VERSION,
    MERIDIAN_ACTIVE_INSTRUCTION_VERSION,
};
use gaussmeridian_models::request::{Content, ContentPart, Message};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Historical identifier retained for frozen P1 qualification and replay.
pub const MERIDIAN_COMPLEXITY_V2_VERSION: &str = "meridian-complexity/v2";
/// Historical active-instruction identifier retained for frozen evidence validation.
pub const MERIDIAN_COMPLEXITY_V3_VERSION: &str = "meridian-complexity/v3";
/// Active identifier stored with current deterministic Meridian complexity estimates.
pub const MERIDIAN_COMPLEXITY_VERSION: &str = "meridian-complexity/v4";

const SIGNAL_WEIGHTS: [f32; 6] = [0.10, 0.20, 0.20, 0.15, 0.15, 0.20];
const HIGH_DEMAND_OPERATORS: &[&str] = &[
    "analyse",
    "analyze",
    "assess",
    "calculate",
    "compare",
    "critique",
    "debug",
    "demonstrate",
    "derive",
    "design",
    "diagnose",
    "evaluate",
    "implement",
    "investigate",
    "optimize",
    "optimise",
    "prove",
    "solve",
    "synthesize",
    "synthesise",
    "verify",
];
const VERIFICATION_MARKERS: &[&str] = &[
    "benchmark",
    "correctness",
    "demonstrate",
    "derive",
    "prove",
    "test",
    "validate",
    "verify",
];
const ROUTINE_TASK_OPERATORS: &[&str] = &[
    "classify",
    "extract",
    "explain",
    "rewrite",
    "summarise",
    "summarize",
    "translate",
];
const FOLLOW_ON_BOUNDARY_WORDS: &[&str] = &[
    "and",
    "or",
    "but",
    "then",
    "next",
    "afterward",
    "subsequently",
    "also",
    "finally",
];

/// Stable signal identifiers used to explain and replay a complexity estimate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplexitySignalKind {
    InputLoad,
    TaskOperatorDemand,
    FormalObjectDemand,
    ConstraintCoupling,
    OutputVerificationContract,
    DemandInteraction,
}

const SIGNAL_KINDS: [ComplexitySignalKind; 6] = [
    ComplexitySignalKind::InputLoad,
    ComplexitySignalKind::TaskOperatorDemand,
    ComplexitySignalKind::FormalObjectDemand,
    ComplexitySignalKind::ConstraintCoupling,
    ComplexitySignalKind::OutputVerificationContract,
    ComplexitySignalKind::DemandInteraction,
];

/// One normalized signal and its weighted contribution to the final score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexitySignalEvidence {
    pub kind: ComplexitySignalKind,
    pub normalized_value: f32,
    pub weight: f32,
    pub contribution: f32,
}

/// Frozen evidence emitted by the deterministic complexity estimator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexityEvidence {
    pub estimator_version: String,
    pub score: f32,
    pub estimated_input_tokens: u32,
    pub signals: Vec<ComplexitySignalEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_instruction: Option<ActiveInstructionEvidence>,
}

/// Validation failures for frozen complexity evidence.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ComplexityEvidenceError {
    #[error("invalid complexity evidence field: {field}")]
    Invalid { field: &'static str },
}

impl ComplexityEvidence {
    /// Validate version, signal ordering, normalized values, and reconstructed score.
    pub fn validate(&self) -> Result<(), ComplexityEvidenceError> {
        match self.estimator_version.as_str() {
            MERIDIAN_COMPLEXITY_V2_VERSION => {
                if self.active_instruction.is_some() {
                    return Err(invalid_evidence("active_instruction"));
                }
            }
            MERIDIAN_COMPLEXITY_V3_VERSION | MERIDIAN_COMPLEXITY_VERSION => {
                let expected_analysis_version =
                    if self.estimator_version == MERIDIAN_COMPLEXITY_V3_VERSION {
                        MERIDIAN_ACTIVE_INSTRUCTION_V1_VERSION
                    } else {
                        MERIDIAN_ACTIVE_INSTRUCTION_VERSION
                    };
                if !self.active_instruction.as_ref().is_some_and(|evidence| {
                    evidence.analysis_version == expected_analysis_version && evidence.validate()
                }) {
                    return Err(invalid_evidence("active_instruction"));
                }
            }
            _ => return Err(invalid_evidence("estimator_version")),
        }
        if !is_unit_interval(self.score) {
            return Err(invalid_evidence("score"));
        }
        if self.signals.len() != SIGNAL_KINDS.len() {
            return Err(invalid_evidence("signals"));
        }

        let mut reconstructed_score = 0.0_f32;
        for ((signal, expected_kind), expected_weight) in
            self.signals.iter().zip(SIGNAL_KINDS).zip(SIGNAL_WEIGHTS)
        {
            if signal.kind != expected_kind {
                return Err(invalid_evidence("signals.kind"));
            }
            if !is_unit_interval(signal.normalized_value)
                || !approximately_equal(signal.weight, expected_weight)
                || !is_unit_interval(signal.contribution)
                || !approximately_equal(
                    signal.contribution,
                    signal.normalized_value * signal.weight,
                )
            {
                return Err(invalid_evidence("signals.math"));
            }
            reconstructed_score += signal.contribution;
        }
        if !approximately_equal(self.score, reconstructed_score) {
            return Err(invalid_evidence("score.reconstruction"));
        }
        Ok(())
    }
}

/// Result of classifying a request for routing purposes.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub evidence: ComplexityEvidence,
    /// True when `evidence.score >= tau_moa`.
    /// Logged at DEBUG level with a \[MoA-GATE\] marker. **Never dispatched in MVP.**
    pub moa_flagged: bool,
}

/// Stateless, deterministic estimator for Meridian routing complexity.
pub struct MeridianComplexityEstimator {
    version: ComplexityEstimatorVersion,
}

#[derive(Clone, Copy)]
enum ComplexityEstimatorVersion {
    HistoricalV2,
    ActiveV4,
}

impl Default for MeridianComplexityEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl MeridianComplexityEstimator {
    /// Create the active stateless deterministic complexity estimator.
    pub fn new() -> Self {
        Self {
            version: ComplexityEstimatorVersion::ActiveV4,
        }
    }

    /// Create the exact historical v2 estimator for frozen qualification and replay.
    pub fn historical_v2() -> Self {
        Self {
            version: ComplexityEstimatorVersion::HistoricalV2,
        }
    }

    /// Classify a chat completion request and return a [`ClassificationResult`].
    ///
    /// # Arguments
    /// * `messages` — the messages from the incoming `ChatCompletionRequest`
    /// * `tau_moa`  — per-project MoA complexity threshold (default 0.7)
    pub fn classify(&self, messages: &[Message], tau_moa: f32) -> ClassificationResult {
        let evidence = match self.version {
            ComplexityEstimatorVersion::HistoricalV2 => {
                self.estimate_v2(&extract_routing_text(messages))
            }
            ComplexityEstimatorVersion::ActiveV4 => self.estimate_v4(messages),
        };

        let moa_flagged = evidence.score >= tau_moa;
        if moa_flagged {
            tracing::debug!(
                complexity = evidence.score,
                tau_moa = tau_moa,
                tokens = evidence.estimated_input_tokens,
                estimator_version = evidence.estimator_version,
                "[MoA-GATE] moa_flagged=true — MoA dispatch BLOCKED (MVP constraint)"
            );
        }

        ClassificationResult {
            evidence,
            moa_flagged,
        }
    }

    fn estimate_v2(&self, text: &str) -> ComplexityEvidence {
        let lower = text.to_lowercase();
        let token_count = estimate_tokens(text);
        let enumerating = is_simple_enumeration_request(text);

        let input_load = input_load(token_count);
        let task_operator = task_operator_demand(text, &lower, enumerating);
        let formal_object = formal_object_demand(&lower);
        let constraint_coupling = constraint_coupling(&lower);
        let output_contract = output_verification_contract(text, &lower, enumerating);
        let interaction = if !enumerating
            && task_operator >= 0.75
            && formal_object >= 0.50
            && (constraint_coupling >= 0.50 || output_contract >= 0.75)
        {
            1.0
        } else {
            0.0
        };

        let values = [
            input_load,
            task_operator,
            formal_object,
            constraint_coupling,
            output_contract,
            interaction,
        ];
        let signals = SIGNAL_KINDS
            .into_iter()
            .zip(values)
            .zip(SIGNAL_WEIGHTS)
            .map(
                |((kind, normalized_value), weight)| ComplexitySignalEvidence {
                    kind,
                    normalized_value,
                    weight,
                    contribution: normalized_value * weight,
                },
            )
            .collect::<Vec<_>>();
        let score = signals.iter().map(|signal| signal.contribution).sum();

        let evidence = ComplexityEvidence {
            estimator_version: MERIDIAN_COMPLEXITY_V2_VERSION.to_string(),
            score,
            estimated_input_tokens: token_count,
            signals,
            active_instruction: None,
        };
        debug_assert!(evidence.validate().is_ok());
        evidence
    }

    fn estimate_v4(&self, messages: &[Message]) -> ComplexityEvidence {
        let analysis = analyze(messages);
        let text = analysis.active_text.as_str();
        let lower = text.to_lowercase();
        let token_count = estimate_tokens(&extract_routing_text(messages));
        let lexical_text = analysis.lexical_text.as_str();
        let values = if is_lexical_command_request(lexical_text) {
            if is_simple_enumeration_request(lexical_text) {
                lexical_enumeration_signal_values(analysis.evidence.active_instruction_tokens)
            } else {
                legacy_signal_values(lexical_text, estimate_tokens(lexical_text))
            }
        } else {
            let enumerating = is_simple_enumeration_request(text);
            let demanding_count = analysis.evidence.demanding_operators.len();
            let routine_count = analysis.evidence.routine_operators.len();
            let formal_count = analysis.evidence.formal_concepts.len();
            let input_load = active_input_load(
                analysis.evidence.active_instruction_tokens,
                demanding_count,
                formal_count,
            );
            let task_operator =
                active_task_operator_demand(text, enumerating, demanding_count, routine_count);
            let formal_object = formal_concept_demand(formal_count);
            let constraint_coupling =
                active_constraint_coupling(&lower, demanding_count, routine_count);
            let output_contract = active_output_verification_contract(
                text,
                &lower,
                enumerating,
                &analysis.evidence.demanding_operators,
                &analysis.evidence.routine_operators,
            );
            let interaction = demand_interaction(
                enumerating,
                task_operator,
                formal_object,
                constraint_coupling,
                output_contract,
            );
            [
                input_load,
                task_operator,
                formal_object,
                constraint_coupling,
                output_contract,
                interaction,
            ]
        };
        let signals = SIGNAL_KINDS
            .into_iter()
            .zip(values)
            .zip(SIGNAL_WEIGHTS)
            .map(
                |((kind, normalized_value), weight)| ComplexitySignalEvidence {
                    kind,
                    normalized_value,
                    weight,
                    contribution: normalized_value * weight,
                },
            )
            .collect::<Vec<_>>();
        let score = signals.iter().map(|signal| signal.contribution).sum();
        let evidence = ComplexityEvidence {
            estimator_version: MERIDIAN_COMPLEXITY_VERSION.to_string(),
            score,
            estimated_input_tokens: token_count,
            signals,
            active_instruction: Some(analysis.evidence),
        };
        debug_assert!(evidence.validate().is_ok());
        evidence
    }
}

fn lexical_enumeration_signal_values(active_instruction_tokens: u32) -> [f32; 6] {
    [
        active_input_load(active_instruction_tokens, 0, 0),
        0.50,
        0.0,
        0.0,
        0.0,
        0.0,
    ]
}

fn legacy_signal_values(text: &str, token_count: u32) -> [f32; 6] {
    let lower = text.to_lowercase();
    let enumerating = is_simple_enumeration_request(text);
    let task_operator = task_operator_demand(text, &lower, enumerating);
    let formal_object = formal_object_demand(&lower);
    let constraint_coupling = constraint_coupling(&lower);
    let output_contract = output_verification_contract(text, &lower, enumerating);
    [
        input_load(token_count),
        task_operator,
        formal_object,
        constraint_coupling,
        output_contract,
        demand_interaction(
            enumerating,
            task_operator,
            formal_object,
            constraint_coupling,
            output_contract,
        ),
    ]
}

fn demand_interaction(
    enumerating: bool,
    task_operator: f32,
    formal_object: f32,
    constraint_coupling: f32,
    output_contract: f32,
) -> f32 {
    if !enumerating
        && task_operator >= 0.75
        && formal_object >= 0.50
        && (constraint_coupling >= 0.50 || output_contract >= 0.75)
    {
        1.0
    } else {
        0.0
    }
}

fn active_input_load(
    active_instruction_tokens: u32,
    demanding_count: usize,
    formal_count: usize,
) -> f32 {
    let value = input_load(active_instruction_tokens);
    if demanding_count == 0 && formal_count == 0 {
        value.min(0.35)
    } else {
        value
    }
}

fn active_task_operator_demand(
    text: &str,
    enumerating: bool,
    demanding_count: usize,
    routine_count: usize,
) -> f32 {
    if enumerating {
        0.50
    } else if demanding_count > 0 {
        1.0
    } else if routine_count > 0 {
        0.75
    } else if text.contains('?') {
        0.20
    } else {
        0.0
    }
}

fn formal_concept_demand(formal_count: usize) -> f32 {
    match formal_count {
        0 => 0.0,
        1 => 0.50,
        2 => 0.80,
        _ => 1.0,
    }
}

fn active_constraint_coupling(lower: &str, demanding_count: usize, routine_count: usize) -> f32 {
    if demanding_count >= 2 {
        return 1.0;
    }
    let mut value = constraint_coupling(lower);
    let structured_routine = routine_count > 0
        && lower.contains(" into ")
        && ["bullet", "bullets", "json", "table"]
            .iter()
            .any(|marker| contains_word(lower, marker));
    if structured_routine {
        value = value.max(0.35);
    }
    if [" dengan ", " terhadap ", " sambil ", " sesuai "]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        value = value.max(0.70);
    }
    value
}

fn active_output_verification_contract(
    text: &str,
    lower: &str,
    enumerating: bool,
    demanding_operators: &[String],
    routine_operators: &[String],
) -> f32 {
    if output_verification_contract(text, lower, enumerating) > 0.0 {
        return 1.0;
    }
    if enumerating {
        return 0.0;
    }
    let strict_structured_output = [
        "return only json",
        "return json",
        "respond only with json",
        "output only json",
        "kembalikan hanya json",
        "kembalikan json",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    if strict_structured_output {
        return 0.35;
    }
    let explicit_verification = ["buktikan", "justify", "pembukti", "validasi", "verifikasi"]
        .iter()
        .any(|marker| lower.contains(marker));
    let explained_proof = routine_operators
        .iter()
        .any(|operator| operator == "jelaskan")
        && lower.contains("bukti");
    let demanding_verification = demanding_operators.iter().any(|operator| {
        matches!(
            operator.as_str(),
            "buktikan" | "demonstrate" | "derive" | "prove" | "verify" | "verifikasi"
        )
    });
    if explicit_verification || explained_proof || demanding_verification {
        1.0
    } else {
        0.0
    }
}

fn input_load(token_count: u32) -> f32 {
    match token_count {
        0 => 0.0,
        1..=20 => 0.10,
        21..=60 => 0.35,
        61..=150 => 0.60,
        151..=400 => 0.80,
        _ => 1.0,
    }
}

fn task_operator_demand(text: &str, lower: &str, enumerating: bool) -> f32 {
    if enumerating {
        return 0.50;
    }

    if contains_any_word(lower, HIGH_DEMAND_OPERATORS) {
        return 1.0;
    }

    if ROUTINE_TASK_OPERATORS
        .iter()
        .any(|word| contains_word(lower, word))
    {
        return 1.0;
    }

    if text.contains('?') {
        0.20
    } else {
        0.0
    }
}

fn formal_object_demand(lower: &str) -> f32 {
    const FORMAL_OBJECTS: &[&str] = &[
        "algorithm",
        "arbitration",
        "bounded",
        "complexity",
        "concurrency",
        "consensus",
        "constraint",
        "correctness",
        "cryptographic",
        "derivative",
        "distributed",
        "gdpr",
        "indemnification",
        "invariant",
        "jurisdiction",
        "liability",
        "lock-free",
        "memory-ordering",
        "proof",
        "recurrence",
        "regulatory",
        "theorem",
    ];
    match FORMAL_OBJECTS
        .iter()
        .filter(|word| lower.contains(**word))
        .count()
    {
        0 => 0.0,
        1 => 0.50,
        2 => 0.80,
        _ => 1.0,
    }
}

fn constraint_coupling(lower: &str) -> f32 {
    const COUPLING_MARKERS: &[&str] = &[
        " against ",
        " although ",
        " bounded ",
        " constrained ",
        " correctness ",
        " edge case",
        " in the context ",
        " preserving ",
        " pursuant ",
        " subject to ",
        " under ",
        " whereas ",
        " while ",
        " with ",
    ];
    match COUPLING_MARKERS
        .iter()
        .filter(|marker| lower.contains(**marker))
        .count()
    {
        0 => 0.0,
        1 => 0.70,
        _ => 1.0,
    }
}

fn output_verification_contract(text: &str, lower: &str, enumerating: bool) -> f32 {
    if enumerating {
        return 0.0;
    }

    if contains_any_word(lower, VERIFICATION_MARKERS) {
        return 1.0;
    }

    let structured_output = lower.contains(" into ")
        && (contains_word(lower, "bullet")
            || contains_word(lower, "bullets")
            || contains_word(lower, "table")
            || contains_word(lower, "json"));
    if structured_output || text.contains("```") {
        1.0
    } else {
        0.0
    }
}

fn is_simple_enumeration_request(text: &str) -> bool {
    let command = strip_polite_prefix(text.trim_start());
    let command_lower = command.to_lowercase();
    if !["list ", "repeat ", "sort ", "alphabetize "]
        .iter()
        .any(|prefix| command_lower.starts_with(prefix))
    {
        return false;
    }

    if let Some((instruction, payload)) = command.split_once(':') {
        let instruction_lower = instruction.to_lowercase();
        if contains_any_word(&instruction_lower, HIGH_DEMAND_OPERATORS)
            || contains_any_word(&instruction_lower, VERIFICATION_MARKERS)
        {
            return false;
        }

        let explicitly_lexical = ["word", "words", "term", "terms", "keyword", "keywords"]
            .iter()
            .any(|item| contains_word(&instruction_lower, item));
        let contains_demand = contains_demand_operator(&payload.to_lowercase());
        if contains_follow_on_task(payload) || (!explicitly_lexical && contains_demand) {
            return false;
        }
    } else if contains_any_word(&command_lower, HIGH_DEMAND_OPERATORS)
        || contains_any_word(&command_lower, VERIFICATION_MARKERS)
    {
        return false;
    }
    true
}

fn is_lexical_command_request(text: &str) -> bool {
    let command = strip_polite_prefix(text.trim_start()).to_lowercase();
    ["list ", "repeat ", "sort ", "alphabetize "]
        .iter()
        .any(|prefix| command.starts_with(prefix))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ClauseBoundary {
    PayloadStart,
    Semicolon,
    Sentence,
}

fn contains_follow_on_task(payload: &str) -> bool {
    if is_explicit_quoted_lexical_sequence(payload) {
        return false;
    }

    let mut boundary = ClauseBoundary::PayloadStart;
    let mut saw_lexical_context = false;

    for line in payload.split(|character| matches!(character, '\n' | '\r')) {
        let (line, marked_lexical_item) =
            if let Some(item) = strip_explicit_lexical_item_marker(line) {
                (item, true)
            } else {
                (line, false)
            };
        if marked_lexical_item {
            saw_lexical_context = true;
        }
        if line_contains_follow_on_task(
            line,
            boundary,
            &mut saw_lexical_context,
            marked_lexical_item,
        ) {
            return true;
        }
        boundary = ClauseBoundary::Sentence;
    }
    false
}

fn line_contains_follow_on_task(
    line: &str,
    initial_boundary: ClauseBoundary,
    saw_lexical_context: &mut bool,
    mut first_clause_is_marked_item: bool,
) -> bool {
    if is_explicit_quoted_lexical_sequence(line) {
        *saw_lexical_context = true;
        return false;
    }

    let mut clause_start = 0;
    let mut boundary = initial_boundary;

    for (index, character) in unquoted_separator_indices(line, &[';', '.', '?', '!']) {
        let next_boundary = match character {
            ';' => ClauseBoundary::Semicolon,
            '.' | '?' | '!' => ClauseBoundary::Sentence,
            _ => unreachable!("separator set contains only clause boundaries"),
        };

        if clause_is_follow_on_task(
            &line[clause_start..index],
            boundary,
            saw_lexical_context,
            first_clause_is_marked_item,
        ) {
            return true;
        }
        first_clause_is_marked_item = false;
        clause_start = index + character.len_utf8();
        boundary = next_boundary;
    }

    clause_is_follow_on_task(
        &line[clause_start..],
        boundary,
        saw_lexical_context,
        first_clause_is_marked_item,
    )
}

fn clause_is_follow_on_task(
    clause: &str,
    boundary: ClauseBoundary,
    saw_lexical_context: &mut bool,
    marked_lexical_item: bool,
) -> bool {
    if is_explicit_quoted_lexical_sequence(clause) {
        *saw_lexical_context = true;
        return false;
    }

    if clause_contains_explicit_task_instruction(clause, marked_lexical_item) {
        return true;
    }

    if marked_lexical_item {
        *saw_lexical_context = true;
        return false;
    }

    let words = clause_head_words(clause);
    if words.is_empty() {
        return false;
    }

    let direct_task = instruction_prefix_len(&words) == 0 && task_instruction_starts_at(&words);
    if boundary != ClauseBoundary::PayloadStart && direct_task && *saw_lexical_context {
        return true;
    }

    if !direct_task && clause_supplies_plain_lexical_context(&words) {
        *saw_lexical_context = true;
    }
    false
}

fn strip_explicit_lexical_item_marker(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let first = line.chars().next()?;
    let remainder = &line[first.len_utf8()..];

    if matches!(first, '-' | '*' | '+' | '•') {
        return remainder
            .strip_prefix(char::is_whitespace)
            .filter(|item| !item.trim().is_empty());
    }

    if !first.is_ascii_digit() {
        return None;
    }
    let digit_count = line
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .map(char::len_utf8)
        .sum::<usize>();
    let remainder = &line[digit_count..];
    let Some(remainder) = remainder
        .strip_prefix('.')
        .or_else(|| remainder.strip_prefix(')'))
    else {
        return None;
    };
    remainder
        .strip_prefix(char::is_whitespace)
        .filter(|item| !item.trim().is_empty())
}

fn is_explicit_quoted_lexical_item(clause: &str) -> bool {
    strip_explicit_quoted_lexical_item_prefix(clause)
        .is_some_and(|remainder| remainder.trim().is_empty())
}

fn strip_explicit_quoted_lexical_item_prefix(clause: &str) -> Option<&str> {
    let clause = clause.trim_start();
    let characters = indexed_characters(clause);
    let item_end = parse_quoted_lexical_item(&characters, 0)?;
    let byte_end = characters
        .get(item_end)
        .map_or(clause.len(), |character| character.byte_index);
    Some(&clause[byte_end..])
}

fn strip_fail_safe_quoted_item_prefix(clause: &str) -> Option<&str> {
    let clause = clause.trim_start();
    let opening_quote = clause.chars().next()?;
    quote_closer(opening_quote)?;

    let content_start = opening_quote.len_utf8();
    let mut scanner = QuoteScanner::default();
    for (index, character) in clause.char_indices() {
        match scanner.observe(clause, index, character) {
            QuotePosition::Closed => {
                if content_start >= index || clause[content_start..index].trim().is_empty() {
                    return None;
                }
                return Some(&clause[index + character.len_utf8()..]);
            }
            QuotePosition::Outside if index == 0 => return None,
            _ => {}
        }
    }
    None
}

fn is_explicit_quoted_lexical_sequence(clause: &str) -> bool {
    let clause = clause.trim();
    let characters = indexed_characters(clause);
    let mut cursor = 0;
    loop {
        let Some(after_item) = parse_quoted_lexical_item(&characters, cursor) else {
            return false;
        };
        match quoted_sequence_continuation(&characters, after_item) {
            Some(QuotedSequenceContinuation::End) => return true,
            Some(QuotedSequenceContinuation::Item(next_item)) => cursor = next_item,
            None => return false,
        }
    }
}

#[derive(Clone, Copy)]
struct IndexedCharacter {
    byte_index: usize,
    value: char,
}

#[derive(Clone, Copy)]
struct NestedQuote {
    closer: char,
    pending_plural_closer: bool,
}

fn indexed_characters(text: &str) -> Vec<IndexedCharacter> {
    text.char_indices()
        .map(|(byte_index, value)| IndexedCharacter { byte_index, value })
        .collect()
}

fn parse_quoted_lexical_item(
    characters: &[IndexedCharacter],
    opening_index: usize,
) -> Option<usize> {
    let opening_quote = characters.get(opening_index)?.value;
    let closing_quote = quote_closer(opening_quote)?;
    let mut preceding_backslashes = 0;
    let mut has_content = false;
    let mut nested_quotes: Vec<NestedQuote> = Vec::new();

    'characters: for index in opening_index + 1..characters.len() {
        let character = characters[index].value;
        let escaped = preceding_backslashes % 2 == 1;
        preceding_backslashes = if character == '\\' {
            preceding_backslashes + 1
        } else {
            0
        };

        if escaped {
            has_content |= !character.is_whitespace();
            continue;
        }
        if character == '\\' {
            has_content = true;
            continue;
        }

        loop {
            let Some(nested_quote) = nested_quotes.last().copied() else {
                break;
            };
            let parent_closer = if nested_quotes.len() > 1 {
                nested_quotes[nested_quotes.len() - 2].closer
            } else {
                closing_quote
            };

            if character == nested_quote.closer {
                if indexed_word_apostrophe(characters, index) {
                    has_content = true;
                    continue 'characters;
                }
                if indexed_plural_possessive(characters, index) {
                    // `students’` can be either a possessive or the end of `‘students’`.
                    // Defer that one-bit decision until a parent closer or a same-style
                    // sibling opener resolves it, without rescanning the remaining text.
                    nested_quotes.last_mut()?.pending_plural_closer = true;
                    has_content = true;
                    continue 'characters;
                }
                nested_quotes.pop();
                has_content = true;
                continue 'characters;
            }

            if nested_quote.pending_plural_closer && character == parent_closer {
                nested_quotes.pop();
                continue;
            }

            if indexed_quote_starts_nested_item(characters, index, character) {
                let next_closer = quote_closer(character)?;
                if next_closer == nested_quote.closer {
                    if nested_quote.pending_plural_closer {
                        nested_quotes.pop();
                        continue;
                    }
                    return None;
                }
                nested_quotes.push(NestedQuote {
                    closer: next_closer,
                    pending_plural_closer: false,
                });
            }
            has_content |= !character.is_whitespace();
            continue 'characters;
        }

        if character != closing_quote {
            if indexed_quote_starts_nested_item(characters, index, character) {
                let nested_closer = quote_closer(character)?;
                if nested_closer == closing_quote {
                    return None;
                }
                nested_quotes.push(NestedQuote {
                    closer: nested_closer,
                    pending_plural_closer: false,
                });
            }
            has_content |= !character.is_whitespace();
            continue;
        }

        if indexed_word_apostrophe(characters, index) {
            has_content = true;
            continue;
        }
        if indexed_plural_possessive(characters, index)
            && !quoted_item_boundary_is_unambiguous(characters, index + 1)
        {
            has_content = true;
            continue;
        }
        if indexed_quote_starts_nested_item(characters, index, character) {
            return None;
        }
        return has_content.then_some(index + 1);
    }

    None
}

fn indexed_word_apostrophe(characters: &[IndexedCharacter], index: usize) -> bool {
    matches!(characters[index].value, '\'' | '’')
        && characters
            .get(index.wrapping_sub(1))
            .is_some_and(|previous| previous.value.is_alphanumeric())
        && characters
            .get(index + 1)
            .is_some_and(|next| next.value.is_alphanumeric())
}

fn indexed_plural_possessive(characters: &[IndexedCharacter], index: usize) -> bool {
    matches!(characters[index].value, '\'' | '’')
        && characters
            .get(index.wrapping_sub(1))
            .is_some_and(|previous| matches!(previous.value, 's' | 'S'))
        && characters
            .get(index + 1)
            .is_some_and(|next| next.value.is_whitespace())
}

fn indexed_quote_can_open(characters: &[IndexedCharacter], index: usize, quote: char) -> bool {
    if matches!(quote, '“' | '‘' | '«' | '‹') {
        return true;
    }
    if !matches!(quote, '"' | '\'' | '`') {
        return false;
    }

    characters
        .get(index.wrapping_sub(1))
        .is_none_or(|previous| !previous.value.is_alphanumeric() && previous.value != '_')
}

fn indexed_quote_starts_nested_item(
    characters: &[IndexedCharacter],
    index: usize,
    quote: char,
) -> bool {
    indexed_quote_can_open(characters, index, quote)
        && characters
            .get(index + 1)
            .is_some_and(|next| next.value.is_alphanumeric())
}

enum QuotedSequenceContinuation {
    End,
    Item(usize),
}

fn quoted_sequence_continuation(
    characters: &[IndexedCharacter],
    cursor: usize,
) -> Option<QuotedSequenceContinuation> {
    let mut cursor = skip_horizontal_whitespace(characters, cursor);
    if cursor == characters.len() {
        return Some(QuotedSequenceContinuation::End);
    }

    let mut saw_separator = false;
    if matches!(characters[cursor].value, '.' | '?' | '!') {
        while cursor < characters.len() && matches!(characters[cursor].value, '.' | '?' | '!') {
            cursor += 1;
        }
        saw_separator = true;
        cursor = skip_all_whitespace(characters, cursor);
        if cursor == characters.len() {
            return Some(QuotedSequenceContinuation::End);
        }
    }

    if cursor < characters.len() && matches!(characters[cursor].value, ',' | ';') {
        cursor += 1;
        saw_separator = true;
    }
    if cursor < characters.len() && matches!(characters[cursor].value, '\r' | '\n') {
        saw_separator = true;
    }
    cursor = skip_all_whitespace(characters, cursor);

    if let Some(after_connector) = ascii_connector_end(characters, cursor) {
        cursor = skip_all_whitespace(characters, after_connector);
        saw_separator = true;
    }

    if !saw_separator
        || !characters
            .get(cursor)
            .is_some_and(|character| quote_closer(character.value).is_some())
    {
        return None;
    }
    Some(QuotedSequenceContinuation::Item(cursor))
}

fn quoted_item_boundary_is_unambiguous(characters: &[IndexedCharacter], cursor: usize) -> bool {
    quoted_sequence_continuation(characters, cursor).is_some()
}

fn skip_horizontal_whitespace(characters: &[IndexedCharacter], mut cursor: usize) -> usize {
    while characters.get(cursor).is_some_and(|character| {
        character.value.is_whitespace() && !matches!(character.value, '\r' | '\n')
    }) {
        cursor += 1;
    }
    cursor
}

fn skip_all_whitespace(characters: &[IndexedCharacter], mut cursor: usize) -> usize {
    while characters
        .get(cursor)
        .is_some_and(|character| character.value.is_whitespace())
    {
        cursor += 1;
    }
    cursor
}

fn ascii_connector_end(characters: &[IndexedCharacter], cursor: usize) -> Option<usize> {
    const CONNECTORS: [&[char]; 2] = [&['a', 'n', 'd'], &['o', 'r']];

    for connector in CONNECTORS {
        let end = cursor + connector.len();
        let matches = characters.get(cursor..end).is_some_and(|candidate| {
            candidate
                .iter()
                .zip(connector.iter())
                .all(|(character, expected)| character.value.eq_ignore_ascii_case(expected))
        });
        if matches
            && characters
                .get(end)
                .is_some_and(|character| character.value.is_whitespace())
        {
            return Some(end);
        }
    }
    None
}

fn clause_contains_explicit_task_instruction(
    clause: &str,
    skip_first_explicit_segment: bool,
) -> bool {
    let comma_segments = split_unquoted(clause, &[',']);

    for (segment_index, segment) in comma_segments.iter().enumerate() {
        if is_explicit_quoted_lexical_item(segment) {
            continue;
        }
        if strip_fail_safe_quoted_item_prefix(segment)
            .map(instruction_words)
            .is_some_and(|words| contains_task_operator_with_object(&words))
        {
            return true;
        }
        let words = instruction_words(segment);
        if words.is_empty() {
            continue;
        }

        let prefix_length = instruction_prefix_len(&words);
        let explicit_prefix = prefix_length > 0;
        if !(skip_first_explicit_segment && segment_index == 0)
            && explicit_prefix
            && task_instruction_starts_at(&words[prefix_length..])
        {
            return true;
        }
    }

    comma_segments
        .iter()
        .enumerate()
        .any(|(segment_index, segment)| {
            if is_explicit_quoted_lexical_item(segment) {
                return false;
            }
            let words = instruction_words(segment);
            contains_follow_on_task_after_boundary(&words, segment_index > 0)
        })
}

fn contains_task_operator_with_object(words: &[&str]) -> bool {
    words
        .iter()
        .take(words.len().saturating_sub(1))
        .any(|word| is_task_operator(word))
}

fn contains_follow_on_task_after_boundary(words: &[&str], has_prior_segment: bool) -> bool {
    let mut cursor = 0;
    let mut follows_lexical_data = false;

    while cursor < words.len() {
        if is_follow_on_boundary_word(words[cursor]) {
            follows_lexical_data |= has_prior_segment || cursor > 0;
            cursor += 1;
            continue;
        }

        let polite_prefix = polite_prefix_len_once(&words[cursor..]);
        if polite_prefix > 0 {
            cursor += polite_prefix;
            continue;
        }

        if follows_lexical_data
            && is_task_operator(words[cursor])
            && words.get(cursor + 1).is_some()
        {
            return true;
        }

        follows_lexical_data = false;
        cursor += 1;
    }

    false
}

fn clause_head_words(clause: &str) -> Vec<&str> {
    instruction_words(
        split_unquoted(clause, &[','])
            .into_iter()
            .next()
            .unwrap_or_default(),
    )
}

fn split_unquoted<'a>(text: &'a str, separators: &[char]) -> Vec<&'a str> {
    let mut segments = Vec::new();
    let mut segment_start = 0;
    for (index, separator) in unquoted_separator_indices(text, separators) {
        segments.push(&text[segment_start..index]);
        segment_start = index + separator.len_utf8();
    }
    segments.push(&text[segment_start..]);
    segments
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QuotePosition {
    Outside,
    Opened,
    Inside,
    Closed,
}

#[derive(Default)]
struct QuoteScanner {
    active_quote_closer: Option<char>,
    preceding_backslashes: usize,
}

impl QuoteScanner {
    fn observe(&mut self, text: &str, index: usize, character: char) -> QuotePosition {
        let escaped = self.preceding_backslashes % 2 == 1;
        self.preceding_backslashes = if character == '\\' {
            self.preceding_backslashes + 1
        } else {
            0
        };

        if let Some(closing_quote) = self.active_quote_closer {
            if quote_closes_at(text, index, character, closing_quote, escaped) {
                self.active_quote_closer = None;
                QuotePosition::Closed
            } else {
                QuotePosition::Inside
            }
        } else if !escaped && quote_can_open(text, index, character) {
            self.active_quote_closer = quote_closer(character);
            QuotePosition::Opened
        } else {
            QuotePosition::Outside
        }
    }

    fn has_unclosed_quote(&self) -> bool {
        self.active_quote_closer.is_some()
    }
}

fn unquoted_separator_indices(text: &str, separators: &[char]) -> Vec<(usize, char)> {
    let mut separators_outside_quotes = Vec::new();
    let mut pending_unmatched_quote_separators = Vec::new();
    let mut scanner = QuoteScanner::default();

    for (index, character) in text.char_indices() {
        match scanner.observe(text, index, character) {
            QuotePosition::Opened => {
                pending_unmatched_quote_separators.clear();
            }
            QuotePosition::Inside => {
                if separators.contains(&character) {
                    pending_unmatched_quote_separators.push((index, character));
                }
            }
            QuotePosition::Closed => {
                pending_unmatched_quote_separators.clear();
            }
            QuotePosition::Outside if separators.contains(&character) => {
                separators_outside_quotes.push((index, character));
            }
            QuotePosition::Outside => {}
        }
    }

    if scanner.has_unclosed_quote() {
        separators_outside_quotes.extend(pending_unmatched_quote_separators);
    }
    separators_outside_quotes
}

fn quote_closes_at(
    text: &str,
    index: usize,
    character: char,
    closing_quote: char,
    escaped: bool,
) -> bool {
    character == closing_quote && !escaped && !is_word_apostrophe(text, index, character)
}

fn is_word_apostrophe(text: &str, index: usize, character: char) -> bool {
    if !matches!(character, '\'' | '’') {
        return false;
    }

    let previous_is_word = text[..index]
        .chars()
        .next_back()
        .is_some_and(char::is_alphanumeric);
    let next_is_word = text[index + character.len_utf8()..]
        .chars()
        .next()
        .is_some_and(char::is_alphanumeric);
    previous_is_word && next_is_word
}

fn quote_can_open(text: &str, index: usize, quote: char) -> bool {
    if matches!(quote, '“' | '‘' | '«' | '‹') {
        return true;
    }
    if !matches!(quote, '"' | '\'' | '`') {
        return false;
    }

    text[..index]
        .chars()
        .next_back()
        .is_none_or(|character| !character.is_alphanumeric() && character != '_')
}

fn quote_closer(opening_quote: char) -> Option<char> {
    match opening_quote {
        '"' | '\'' | '`' => Some(opening_quote),
        '“' => Some('”'),
        '‘' => Some('’'),
        '«' => Some('»'),
        '‹' => Some('›'),
        _ => None,
    }
}

fn clause_supplies_plain_lexical_context(words: &[&str]) -> bool {
    let words = strip_instruction_prefixes(words);
    words.first().is_some_and(|word| !is_task_operator(word))
}

fn task_instruction_starts_at(words: &[&str]) -> bool {
    let words = strip_instruction_prefixes(words);
    let Some((operator, object_words)) = words.split_first() else {
        return false;
    };

    // A bare operator can itself be lexical list data. Clause or coordinator
    // syntax supplies the imperative boundary; one object word is then enough.
    is_task_operator(operator) && !object_words.is_empty()
}

fn is_task_operator(word: &str) -> bool {
    HIGH_DEMAND_OPERATORS
        .iter()
        .chain(VERIFICATION_MARKERS)
        .chain(ROUTINE_TASK_OPERATORS)
        .any(|candidate| word.eq_ignore_ascii_case(candidate))
}

fn instruction_words(text: &str) -> Vec<&str> {
    text.split(|character: char| !is_instruction_word_character(character))
        .filter(|word| !word.is_empty())
        .collect()
}

fn is_instruction_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '-'
}

fn strip_instruction_prefixes<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    &words[instruction_prefix_len(words)..]
}

fn instruction_prefix_len(words: &[&str]) -> usize {
    let mut consumed = 0;
    while consumed < words.len() {
        let remaining = &words[consumed..];
        let prefix_length = if remaining
            .first()
            .is_some_and(|word| is_follow_on_boundary_word(word))
        {
            1
        } else {
            polite_prefix_len(remaining)
        };
        if prefix_length == 0 {
            break;
        }
        consumed += prefix_length;
    }
    consumed
}

fn polite_prefix_len(words: &[&str]) -> usize {
    let mut consumed = 0;
    while consumed < words.len() {
        let prefix_length = polite_prefix_len_once(&words[consumed..]);
        if prefix_length == 0 {
            break;
        }
        consumed += prefix_length;
    }
    consumed
}

fn polite_prefix_len_once(words: &[&str]) -> usize {
    match words {
        [word, ..] if matches_ignore_ascii_case(word, &["please", "kindly"]) => 1,
        [modal, you, ..]
            if matches_ignore_ascii_case(modal, &["can", "could", "will", "would"])
                && you.eq_ignore_ascii_case("you") =>
        {
            2
        }
        _ => 0,
    }
}

fn is_follow_on_boundary_word(word: &str) -> bool {
    FOLLOW_ON_BOUNDARY_WORDS
        .iter()
        .any(|candidate| word.eq_ignore_ascii_case(candidate))
}

fn matches_ignore_ascii_case(word: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| word.eq_ignore_ascii_case(candidate))
}

fn contains_demand_operator(text: &str) -> bool {
    contains_any_word(text, HIGH_DEMAND_OPERATORS) || contains_any_word(text, VERIFICATION_MARKERS)
}

fn strip_polite_prefix(text: &str) -> &str {
    let words = instruction_words(text);
    let prefix_length = polite_prefix_len(&words);
    if prefix_length == 0 {
        return text;
    }

    let mut remainder = text;
    for _ in 0..prefix_length {
        remainder = remainder
            .trim_start_matches(|character: char| !is_instruction_word_character(character));
        let word_length = remainder
            .chars()
            .take_while(|character| is_instruction_word_character(*character))
            .map(char::len_utf8)
            .sum::<usize>();
        remainder = &remainder[word_length..];
    }
    remainder.trim_start_matches(|character: char| !is_instruction_word_character(character))
}

fn contains_any_word(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| contains_word(text, word))
}

fn contains_word(text: &str, word: &str) -> bool {
    text.split(|character: char| !character.is_alphanumeric() && character != '-')
        .any(|candidate| candidate == word)
}

/// Estimate token count (whitespace tokenisation approximation).
///
/// Rule of thumb: 1 token ≈ 0.75 words (GPT tokeniser average).
pub fn estimate_tokens(text: &str) -> u32 {
    let words = text.split_whitespace().count();
    ((words as f32) / 0.75).round() as u32
}

/// Concatenate all textual request context, including instructions and conversation history.
fn extract_routing_text(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| extract_content_text(&m.content))
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn approximately_equal(left: f32, right: f32) -> bool {
    (left - right).abs() <= 1e-6
}

fn invalid_evidence(field: &'static str) -> ComplexityEvidenceError {
    ComplexityEvidenceError::Invalid { field }
}

fn extract_content_text(content: &Content) -> String {
    match content {
        Content::Text(s) => s.clone(),
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gaussmeridian_models::request::{Content, Message, Role};

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

    fn user_msg(text: &str) -> Message {
        message(Role::User, text)
    }

    #[test]
    fn test_simple_query_low_complexity() {
        let cb = MeridianComplexityEstimator::new();
        let msgs = vec![user_msg("What is the capital of France?")];
        let result = cb.classify(&msgs, 0.7);
        assert!(
            result.evidence.score < 0.3,
            "Simple factual query should score < 0.3, got {}",
            result.evidence.score
        );
        assert!(!result.moa_flagged);
    }

    #[test]
    fn test_complex_legal_query_high_complexity() {
        let cb = MeridianComplexityEstimator::new();
        let msgs = vec![user_msg(
            "Analyse the jurisdictional implications of cross-border data transfers \
             under GDPR Article 46(2)(c) in the context of a multi-party SaaS agreement \
             with US-based sub-processors. Evaluate the liability and indemnification clauses.",
        )];
        let result = cb.classify(&msgs, 0.7);
        assert!(
            result.evidence.score > 0.7,
            "Complex legal query should score > 0.7, got {}",
            result.evidence.score
        );
        assert!(result.moa_flagged);
    }

    #[test]
    fn test_code_query_moderate_complexity() {
        let cb = MeridianComplexityEstimator::new();
        let msgs = vec![user_msg(
            "Write a Rust function to implement a binary search algorithm. \
             The function should handle edge cases and return the index.",
        )];
        let result = cb.classify(&msgs, 0.7);
        assert!(
            result.evidence.score >= 0.2,
            "Code query should have moderate complexity, got {}",
            result.evidence.score
        );
    }

    #[test]
    fn test_multi_domain_scores_higher_than_single() {
        let cb = MeridianComplexityEstimator::new();
        let single = cb.classify(
            &[user_msg("Explain distributed consensus algorithms.")],
            0.7,
        );
        let multi = cb.classify(
            &[user_msg(
                "Explain the statistical mechanics of financial derivatives pricing \
                 and its implications for portfolio hedging under medical malpractice liability.",
            )],
            0.7,
        );
        assert!(
            multi.evidence.score > single.evidence.score,
            "Multi-domain query should score higher: multi={} single={}",
            multi.evidence.score,
            single.evidence.score
        );
    }

    #[test]
    fn test_moa_flag_respects_tau() {
        let cb = MeridianComplexityEstimator::new();
        let msgs = vec![user_msg(
            "Analyse the jurisdictional implications of cross-border GDPR transfers.",
        )];
        let low_tau = cb.classify(&msgs, 0.3);
        let high_tau = cb.classify(&msgs, 0.95);
        assert!(low_tau.moa_flagged, "Should be flagged with tau=0.3");
        assert!(!high_tau.moa_flagged, "Should not be flagged with tau=0.95");
    }

    #[test]
    fn test_token_estimate_reasonable() {
        let text = "hello world foo bar baz"; // 5 words ≈ 6-7 tokens
        let tokens = estimate_tokens(text);
        assert!(
            tokens >= 5 && tokens <= 10,
            "Expected 5-10 tokens, got {}",
            tokens
        );
    }

    #[test]
    fn test_score_clamps_to_one() {
        let cb = MeridianComplexityEstimator::new();
        // Extremely complex query with all signals maxed
        let msgs = vec![user_msg(
            "Implement a distributed consensus algorithm in Rust that handles \
             Byzantine fault tolerance? Compare the cryptographic properties of \
             PBFT vs Tendermint. Calculate the complexity O(n²) implications \
             for financial arbitrage derivatives under GDPR liability constraints. \
             Because furthermore additionally moreover step 1 step 2 step 3 finally.",
        )];
        let result = cb.classify(&msgs, 0.7);
        assert!(
            result.evidence.score <= 1.0,
            "Score must not exceed 1.0, got {}",
            result.evidence.score
        );
        assert!(result.evidence.score > 0.7);
    }

    #[test]
    fn complex_system_instruction_is_not_hidden_by_trivial_user_acknowledgement() {
        let result = MeridianComplexityEstimator::new().classify(
            &[
                message(
                    Role::System,
                    "Prove the safety invariant for a distributed consensus algorithm under \
                     Byzantine faults, then verify the bounded recurrence and edge cases.",
                ),
                user_msg("Proceed."),
            ],
            0.7,
        );

        assert!(
            result.evidence.score >= 0.7,
            "system instructions are routing work, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn assistant_history_contributes_to_context_load() {
        let estimator = MeridianComplexityEstimator::new();
        let current_only = estimator.classify(&[user_msg("Continue.")], 0.7);
        let with_history = estimator.classify(
            &[
                message(Role::Assistant, &"context ".repeat(180)),
                user_msg("Continue."),
            ],
            0.7,
        );

        assert!(
            with_history.evidence.estimated_input_tokens
                > current_only.evidence.estimated_input_tokens,
            "all textual context must count toward request load"
        );
    }

    #[test]
    fn polite_list_of_keywords_remains_a_simple_enumeration() {
        let result = MeridianComplexityEstimator::new().classify(
            &[user_msg(
                "Please list these words alphabetically: implement prove invariant recurrence \
                 algorithm theorem constraint.",
            )],
            0.7,
        );

        assert!(
            result.evidence.score < 0.5,
            "listed vocabulary is not requested work, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn list_prefix_cannot_suppress_downstream_proof_work() {
        let result = MeridianComplexityEstimator::new().classify(
            &[user_msg(
                "List the assumptions, then prove the safety invariant and solve the constrained \
                 recurrence with a correctness proof.",
            )],
            0.7,
        );

        assert!(
            result.evidence.score >= 0.7,
            "downstream proof work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn colon_list_payload_cannot_suppress_downstream_proof_work() {
        let result = MeridianComplexityEstimator::new().classify(
            &[user_msg(
                "List the assumptions: then prove the safety invariant and solve the constrained \
                 recurrence with a correctness proof.",
            )],
            0.7,
        );

        assert!(
            result.evidence.score >= 0.7,
            "colon-delimited downstream work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn lexical_list_prefix_cannot_suppress_follow_on_proof_work() {
        let result = MeridianComplexityEstimator::new().classify(
            &[user_msg(
                "List the words: then prove the safety invariant and solve the constrained \
                 recurrence with a correctness proof.",
            )],
            0.7,
        );

        assert!(
            result.evidence.score >= 0.7,
            "a lexical list prefix must not hide a follow-on proof task, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn lexical_list_sentence_boundary_cannot_suppress_follow_on_proof_work() {
        let result = MeridianComplexityEstimator::new().classify(
            &[user_msg(
                "List the words alphabetically: alpha beta.\n\
                 Prove the safety invariant and solve the constrained recurrence with a \
                 correctness proof.",
            )],
            0.7,
        );

        assert!(
            result.evidence.score >= 0.7,
            "sentence-delimited proof work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn lexical_list_conjunction_cannot_suppress_follow_on_proof_work() {
        let result = MeridianComplexityEstimator::new().classify(
            &[user_msg(
                "List the words alphabetically: alpha beta, and prove the safety invariant \
                 with a correctness proof.",
            )],
            0.7,
        );

        assert!(
            result.evidence.score >= 0.7,
            "conjoined proof work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn lexical_enumeration_is_bounded_to_one_sentence() {
        assert!(
            !is_simple_enumeration_request(
                "List the words alphabetically: alpha beta. Explain distributed consensus."
            ),
            "a second instruction sentence is outside the lexical enumeration payload"
        );
    }

    #[test]
    fn lexical_only_semicolon_payload_remains_a_simple_enumeration() {
        assert_simple_lexical_enumeration(
            "List the words alphabetically: derive; prove; theorem; solve; invariant.",
        );
    }

    #[test]
    fn lexical_only_sentence_payload_remains_a_simple_enumeration() {
        assert_simple_lexical_enumeration(
            "List the words alphabetically: derive. prove. theorem. solve. invariant.",
        );
    }

    #[test]
    fn lexical_only_transition_words_remain_a_simple_enumeration() {
        assert_simple_lexical_enumeration(
            "List the words alphabetically: then, next, also, finally, derive, prove.",
        );
    }

    #[test]
    fn longer_transition_word_payload_remains_a_simple_enumeration() {
        assert_simple_lexical_enumeration(
            "List the words alphabetically: then, derive, prove, theorem, solve, invariant, \
             verify, benchmark.",
        );
    }

    #[test]
    fn leading_transition_follow_on_instruction_is_not_lexical_data() {
        let prompt = "List the words: then prove invariant.";
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

        assert!(
            !is_simple_enumeration_request(prompt),
            "a leading coordinator must expose the imperative that follows it"
        );
        assert!(
            result.evidence.score >= 0.6,
            "leading coordinated proof work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn leading_polite_follow_on_instruction_is_not_lexical_data() {
        let prompt = "List the words: please prove the safety invariant.";
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

        assert!(
            !is_simple_enumeration_request(prompt),
            "a leading polite prefix must expose the imperative that follows it"
        );
        assert!(
            result.evidence.score >= 0.6,
            "leading polite proof work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn leading_coordinator_and_polite_prefix_do_not_hide_follow_on_work() {
        let prompt = "List the words: then please prove invariant.";

        assert!(
            !is_simple_enumeration_request(prompt),
            "coordinator and polite prefixes must be consumed before the imperative"
        );
    }

    #[test]
    fn semicolon_delimited_multiword_phrases_remain_lexical_data() {
        assert_simple_lexical_enumeration(
            "List the terms alphabetically: derive proof; prove invariant; verify theorem.",
        );
    }

    #[test]
    fn lowercase_sentence_delimited_multiword_phrases_remain_lexical_data() {
        assert_simple_lexical_enumeration(
            "List the terms alphabetically: derive proof. prove invariant. verify theorem.",
        );
    }

    #[test]
    fn direct_follow_on_instruction_is_case_invariant() {
        let prompts = [
            "List the words: alpha. prove invariant.",
            "List the words: alpha. Prove invariant.",
        ];
        let scores = prompts.map(|prompt| {
            assert!(
                !is_simple_enumeration_request(prompt),
                "a direct follow-on task must not depend on capitalization: {prompt}"
            );
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score >= 0.6,
                "proof work must remain visible regardless of capitalization, prompt={prompt}, score={}",
                result.evidence.score
            );
            result.evidence.score
        });

        assert!(
            approximately_equal(scores[0], scores[1]),
            "superficial capitalization must not change complexity: lower={}, upper={}",
            scores[0],
            scores[1]
        );
    }

    #[test]
    fn lexical_phrase_list_is_case_and_delimiter_invariant() {
        for prompt in [
            "List the terms alphabetically: derive proof. prove invariant. verify theorem.",
            "List the terms alphabetically: Derive proof. Prove invariant. Verify theorem.",
            "List the terms alphabetically: derive proof; prove invariant; verify theorem.",
            "List the terms alphabetically: Derive proof; Prove invariant; Verify theorem.",
        ] {
            assert_simple_lexical_enumeration(prompt);
        }
    }

    #[test]
    fn mixed_operator_vocabulary_list_remains_lexical_data() {
        assert_simple_lexical_enumeration(
            "List the terms alphabetically: derive; prove invariant; verify theorem.",
        );
    }

    #[test]
    fn explicit_bulleted_lexical_items_remain_data() {
        for prompt in [
            "List the terms alphabetically:\n- alpha\n- prove invariant\n- verify theorem.",
            "List the terms alphabetically:\n- Alpha\n- Prove invariant\n- Verify theorem.",
        ] {
            assert_simple_lexical_enumeration(prompt);
        }
    }

    #[test]
    fn explicit_numbered_lexical_items_remain_data() {
        assert_simple_lexical_enumeration(
            "List the terms alphabetically:\n1. alpha\n2. prove invariant\n3. verify theorem.",
        );
    }

    #[test]
    fn explicit_quoted_lexical_items_remain_data() {
        assert_simple_lexical_enumeration(
            "List the terms alphabetically:\n\"alpha\"\n\"prove invariant\"\n\"verify theorem\"",
        );
    }

    #[test]
    fn punctuation_inside_quoted_lexical_items_remains_data() {
        assert_simple_lexical_enumeration(
            "List the terms alphabetically:\n\"alpha.\"\n\"prove invariant.\"\n\"verify theorem.\"",
        );
    }

    #[test]
    fn inline_quoted_lexical_items_remain_data() {
        for prompt in [
            "List the terms alphabetically: \"alpha\", \"then prove invariant\", \"verify theorem\"",
            "List the terms alphabetically: “alpha.”, “then prove invariant.”, “verify theorem.”",
        ] {
            assert_simple_lexical_enumeration(prompt);
        }
    }

    #[test]
    fn quoted_connectors_and_escaped_inner_quotes_remain_data() {
        for prompt in [
            r#"List the terms alphabetically: "alpha", and "prove invariant"."#,
            r#"List the terms alphabetically: "alpha", "then prove \"invariant\"", "verify theorem"."#,
        ] {
            assert_simple_lexical_enumeration(prompt);
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score < 0.3,
                "quoted lexical data must retain the baseline band, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn balanced_nested_quote_styles_remain_lexical_data() {
        for prompt in [
            r#"List the terms alphabetically: "alpha", "the phrase ‘then prove invariant’", "verify theorem"."#,
            "List the terms alphabetically: ‘alpha’, ‘the phrase “then prove invariant”’, ‘verify theorem’.",
        ] {
            assert_simple_lexical_enumeration(prompt);
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score < 0.3,
                "balanced nested quotation is lexical data, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn nested_smart_quote_after_plural_s_remains_lexical_data() {
        for prompt in [
            r#"List the terms alphabetically: "alpha", "the phrase ‘students’ then prove invariant", "verify theorem"."#,
            "List the terms alphabetically: “alpha”, “the phrase ‘students’ then prove invariant”, “verify theorem”.",
            r#"List the terms alphabetically: "alpha", "the phrase 'students' then prove invariant", "verify theorem"."#,
        ] {
            assert_simple_lexical_enumeration(prompt);
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score < 0.3,
                "a balanced nested smart quote must not become a possessive and leak lexical data, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn nested_plural_possessive_keeps_its_later_closer() {
        for prompt in [
            r#"List the terms alphabetically: "alpha", "the phrase ‘students’ theorem’", "verify theorem"."#,
            r#"List the terms alphabetically: "alpha", "the phrase 'students' theorem'", "verify theorem"."#,
        ] {
            assert_simple_lexical_enumeration(prompt);
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score < 0.3,
                "a plural possessive inside nested quotation must not close it early, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn unclosed_nested_quote_fails_toward_visible_work() {
        let prompt = r#"List the terms alphabetically: "alpha", "the phrase ‘then prove invariant", "verify theorem"."#;
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

        assert!(
            !is_simple_enumeration_request(prompt),
            "an unclosed nested quote must not hide requested work"
        );
        assert!(
            result.evidence.score >= 0.6,
            "malformed nested quotation must fail toward task detection, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn quoted_list_connectors_require_complete_quoted_operands() {
        for prompt in [
            r#"List the terms alphabetically: "theorem" and "prove invariant"."#,
            r#"List the terms alphabetically: "alpha", "beta" and "verify theorem"."#,
            r#"List the terms alphabetically: "theorem" or "prove invariant"."#,
        ] {
            assert_simple_lexical_enumeration(prompt);
        }

        for prompt in [
            r#"List the terms alphabetically: "alpha", or prove invariant."#,
            r#"List the terms alphabetically: "alpha" and verify theorem."#,
        ] {
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                !is_simple_enumeration_request(prompt),
                "an unquoted connector operand must remain requested work: {prompt}"
            );
            assert!(
                result.evidence.score >= 0.6,
                "unquoted connector work must remain visible, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn top_level_modal_and_polite_prefixes_share_one_iterative_grammar() {
        for prompt in [
            r#"Will you list the terms alphabetically: "derive", "prove invariant", "verify theorem"?"#,
            r#"Can you please list the terms alphabetically: "derive", "prove invariant", "verify theorem"?"#,
            r#"Would you kindly list the terms alphabetically: "derive", "prove invariant", "verify theorem"?"#,
        ] {
            assert_simple_lexical_enumeration(prompt);
        }
    }

    #[test]
    fn apostrophes_inside_single_quoted_items_remain_data() {
        for prompt in [
            r#"List the terms alphabetically: 'alpha'; 'don't; prove invariant'; 'verify theorem'."#,
            "List the terms alphabetically: ‘alpha’; ‘don’t; prove invariant’; ‘verify theorem’.",
            r#"List the terms alphabetically: 'alpha'; 'students' theorem; prove invariant'; 'verify theorem'."#,
            "List the terms alphabetically: ‘alpha’; ‘students’ theorem; prove invariant’; ‘verify theorem’.",
            r#"List the terms alphabetically: 'students' and 'teachers'."#,
            r#"List the terms alphabetically: 'students' and teachers' theorem'."#,
        ] {
            assert_simple_lexical_enumeration(prompt);
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score < 0.3,
                "a contraction apostrophe must not close its outer quoted item, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn plural_possessive_detection_does_not_hide_unquoted_work() {
        for prompt in [
            r#"List the terms alphabetically: 'students' and prove invariant."#,
            "List the terms alphabetically: ‘students’ or verify theorem.",
            r#"List the terms alphabetically: 'students' theorem; prove invariant"#,
        ] {
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                !is_simple_enumeration_request(prompt),
                "possessive handling must fail malformed or unquoted work toward task detection: {prompt}"
            );
            assert!(
                result.evidence.score >= 0.6,
                "possessive handling hid requested work, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn plural_possessive_unicode_prefix_is_panic_safe() {
        for prompt in [
            "List the terms alphabetically: 'students' 😀 and alpha'",
            "List the terms alphabetically: ‘students’ 🧪 or alpha’",
        ] {
            assert_simple_lexical_enumeration(prompt);
        }
    }

    #[test]
    fn ambiguous_plural_possessive_fails_toward_visible_work() {
        let prompt = "List the terms: 'class' separately prove invariant; 'verify theorem'.";
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

        assert!(
            !is_simple_enumeration_request(prompt),
            "an ambiguous closing quote must not swallow later unquoted work"
        );
        assert!(
            result.evidence.score >= 0.6,
            "ambiguous quote handling hid requested work, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn quoted_sequence_connectors_cross_semicolon_and_newline_boundaries() {
        for prompt in [
            r#"List the terms alphabetically: "theorem"; and "prove invariant"."#,
            "List the terms alphabetically: \"theorem\"\nor \"prove invariant\"",
        ] {
            assert_simple_lexical_enumeration(prompt);
        }

        for prompt in [
            r#"List the terms alphabetically: "theorem"; and prove invariant."#,
            "List the terms alphabetically: \"theorem\"\nor prove invariant",
        ] {
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                !is_simple_enumeration_request(prompt),
                "an unquoted cross-boundary operand must remain work: {prompt}"
            );
            assert!(
                result.evidence.score >= 0.6,
                "cross-boundary work was hidden, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn quoted_sequence_connectors_cross_sentence_boundaries() {
        for prompt in [
            r#"List the terms alphabetically: "theorem". And "prove invariant"."#,
            "List the terms alphabetically: “theorem”. Or “prove invariant”.",
        ] {
            assert_simple_lexical_enumeration(prompt);
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score < 0.3,
                "presentation punctuation must not change a fully quoted lexical list, prompt={prompt}, score={}",
                result.evidence.score
            );
        }

        for prompt in [
            r#"List the terms alphabetically: "theorem". And prove invariant."#,
            "List the terms alphabetically: “theorem”. Or verify theorem.",
        ] {
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                !is_simple_enumeration_request(prompt),
                "an unquoted sentence-boundary operand must remain requested work: {prompt}"
            );
            assert!(
                result.evidence.score >= 0.6,
                "unquoted sentence-boundary work was hidden, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn repeated_plural_possessives_remain_bounded_and_panic_safe() {
        let repeated = std::iter::repeat_n("students' theorem", 2_048)
            .collect::<Vec<_>>()
            .join(" ");
        let prompt = format!("List the terms alphabetically: '{repeated}'");
        let started = std::time::Instant::now();

        assert_simple_lexical_enumeration(&prompt);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "quoted lexical scanning must remain bounded on adversarial possessives"
        );
    }

    #[test]
    fn repeated_connector_prefixes_remain_bounded() {
        let repeated = "and ".repeat(10_000);
        let prompt = format!("List the terms alphabetically: \"alpha\" {repeated}beta");
        let started = std::time::Instant::now();

        assert_simple_lexical_enumeration(&prompt);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "connector-prefix scanning must remain bounded on untrusted prompt input"
        );
    }

    #[test]
    fn escaped_terminal_quote_does_not_hide_unclosed_work() {
        let prompt = concat!(r#"List the terms: alpha; "prove invariant"#, "\\\"");
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

        assert!(
            !is_simple_enumeration_request(prompt),
            "an escaped terminal quote is not an outer closing quote"
        );
        assert!(
            result.evidence.score >= 0.6,
            "malformed quoted work must fail toward task detection, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn quoted_operator_item_does_not_hide_unquoted_follow_on_work() {
        let prompt = "List the quoted word: \"derive\". prove invariant.";
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

        assert!(
            !is_simple_enumeration_request(prompt),
            "a quoted lexical operator must establish data before the unquoted task"
        );
        assert!(
            result.evidence.score >= 0.6,
            "unquoted proof work must remain visible after a quoted item, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn explicit_item_marker_does_not_hide_same_line_follow_on_work() {
        for prompt in [
            "List the terms alphabetically:\n- \"alpha\". prove invariant.",
            "List the terms alphabetically:\n1. \"alpha\"; then verify theorem.",
        ] {
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

            assert!(
                !is_simple_enumeration_request(prompt),
                "a list marker must not shield an unquoted same-line task: {prompt}"
            );
            assert!(
                result.evidence.score >= 0.6,
                "same-line work must remain visible after a marked item, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn unmarked_task_after_explicit_lexical_items_remains_visible() {
        let prompt =
            "List the terms alphabetically:\n- alpha\n- prove invariant\nthen verify theorem.";
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

        assert!(
            !is_simple_enumeration_request(prompt),
            "an unmarked follow-on instruction must terminate the explicit lexical list"
        );
        assert!(
            result.evidence.score >= 0.6,
            "follow-on verification work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn semicolon_delimited_follow_on_instruction_is_detected() {
        for prompt in [
            "List the words: alpha; prove invariant.",
            "List the words: alpha; Prove invariant.",
        ] {
            assert!(
                !is_simple_enumeration_request(prompt),
                "a semicolon-delimited follow-on task must remain visible: {prompt}"
            );
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
            assert!(
                result.evidence.score >= 0.6,
                "semicolon-delimited proof work must affect routing, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn concise_follow_on_instruction_is_not_lexical_data() {
        let result = MeridianComplexityEstimator::new()
            .classify(&[user_msg("List the words: alpha. Prove invariant.")], 0.7);

        assert!(
            !is_simple_enumeration_request("List the words: alpha. Prove invariant."),
            "a concise second imperative must terminate the lexical payload"
        );
        assert!(
            result.evidence.score >= 0.6,
            "concise proof work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn polite_follow_on_instruction_is_not_lexical_data() {
        let result = MeridianComplexityEstimator::new().classify(
            &[user_msg(
                "List the words: alpha. Please prove the safety invariant.",
            )],
            0.7,
        );

        assert!(
            !is_simple_enumeration_request(
                "List the words: alpha. Please prove the safety invariant."
            ),
            "a clause-level polite prefix must not hide a follow-on task"
        );
        assert!(
            result.evidence.score >= 0.6,
            "polite proof work must remain visible, score={}",
            result.evidence.score
        );
    }

    #[test]
    fn later_polite_or_modal_comma_instruction_is_not_lexical_data() {
        for prompt in [
            "List the terms: alpha, beta, please prove invariant.",
            "List the terms: alpha, beta, kindly verify theorem.",
            "List the terms: alpha, beta, could you prove invariant.",
        ] {
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

            assert!(
                !is_simple_enumeration_request(prompt),
                "a later polite or modal clause must expose its task: {prompt}"
            );
            assert!(
                result.evidence.score >= 0.6,
                "later explicit work must affect routing, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn will_you_follow_on_instruction_is_not_lexical_data() {
        for prompt in [
            "List the terms: alpha. Will you prove invariant?",
            "List the terms: alpha, beta, will you prove invariant?",
        ] {
            let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);

            assert!(
                !is_simple_enumeration_request(prompt),
                "a will-you modal clause must expose its task: {prompt}"
            );
            assert!(
                result.evidence.score >= 0.6,
                "will-you proof work must affect routing, prompt={prompt}, score={}",
                result.evidence.score
            );
        }
    }

    #[test]
    fn coordinated_concise_follow_on_instruction_is_not_lexical_data() {
        assert!(
            !is_simple_enumeration_request(
                "list the words alphabetically: alpha, and prove invariant."
            ),
            "a coordinated imperative with a concise object must terminate lexical data"
        );
    }

    #[test]
    fn comma_delimited_operator_phrases_remain_lexical_data() {
        assert_simple_lexical_enumeration(
            "List the terms alphabetically: then, derive proof, prove invariant, \
             verify theorem.",
        );
    }

    #[test]
    fn lexical_only_conjunction_payload_remains_a_simple_enumeration() {
        assert_simple_lexical_enumeration("List the words alphabetically: alpha, beta, and prove.");
    }

    #[test]
    fn lexical_only_multiline_payload_remains_a_simple_enumeration() {
        assert_simple_lexical_enumeration(
            "List the words alphabetically:\nderive\nprove\ntheorem\nsolve\ninvariant",
        );
    }

    fn assert_simple_lexical_enumeration(prompt: &str) {
        assert!(
            is_simple_enumeration_request(prompt),
            "lexical vocabulary must retain the enumeration contract"
        );
        let result = MeridianComplexityEstimator::new().classify(&[user_msg(prompt)], 0.7);
        assert!(
            result.evidence.score < 0.5,
            "lexical vocabulary is not requested work, score={}",
            result.evidence.score
        );
    }
}
