//! Legacy advisory skill-feature extractor for Meridian routing.
//!
//! BELLA (arXiv:2602.02386) informs the P3 destination, but this module does
//! not implement BELLA. It emits keyword-derived advisory features without
//! critic evidence, learned per-model proficiency, or held-out attribution.
//!
//! Twelve legacy dimensions are extracted from keyword and structural signals.
//! They remain advisory until P3 supplies governed taxonomy, uncertainty,
//! continuous proficiency, and skill-specific evaluator evidence.
//!
//! ## Runtime role
//! These features are advisory inputs only. They cannot exclude a candidate.
//! Only explicit, verified request requirements may populate hard skills and
//! participate in hard eligibility.
//!
//! P3 must replace or explicitly retire these priors; weights alone are not
//! enough to justify a BELLA claim.

use gaussmeridian_models::request::{Content, ContentPart, Message, Role};

// ─── Skill dimension constants ────────────────────────────────────────────────

/// Number of skill dimensions in the MVP capability vector.
pub const SKILL_DIMS: usize = 12;

/// Bit index 0 — Numerical and mathematical reasoning.
pub const SKILL_NUMERICAL_REASONING: usize = 0;
/// Bit index 1 — Code synthesis, debugging, and software engineering.
pub const SKILL_CODE_SYNTHESIS: usize = 1;
/// Bit index 2 — Temporal logic, scheduling, and chronological reasoning.
pub const SKILL_TEMPORAL_LOGIC: usize = 2;
/// Bit index 3 — Legal interpretation, regulatory analysis, compliance.
pub const SKILL_LEGAL_INTERPRETATION: usize = 3;
/// Bit index 4 — Medical knowledge, diagnosis, clinical reasoning.
pub const SKILL_MEDICAL_KNOWLEDGE: usize = 4;
/// Bit index 5 — Scientific analysis, research, hypothesis evaluation.
pub const SKILL_SCIENTIFIC_ANALYSIS: usize = 5;
/// Bit index 6 — Creative writing, narrative, fiction, poetry.
pub const SKILL_CREATIVE_WRITING: usize = 6;
/// Bit index 7 — Named entity extraction and information retrieval.
pub const SKILL_ENTITY_EXTRACTION: usize = 7;
/// Bit index 8 — Summarisation and condensation of long documents.
pub const SKILL_SUMMARISATION: usize = 8;
/// Bit index 9 — Translation between natural languages.
pub const SKILL_TRANSLATION: usize = 9;
/// Bit index 10 — Symbolic mathematics, equations, formal proofs.
pub const SKILL_MATH_SYMBOLIC: usize = 10;
/// Bit index 11 — Data analysis, statistics, pattern recognition.
pub const SKILL_DATA_ANALYSIS: usize = 11;

// ─── Capability matrix (seeded values per provider tier) ─────────────────────

/// Pre-seeded capability matrix for the 15-model catalog.
///
/// Each entry is `(model_name, skill_vector)`.
/// Used by the DB seed to populate `provider_model.skill_vector`.
///
/// Format: `[bool; SKILL_DIMS]` — `true` = model supports this skill.
/// Indices follow the `SKILL_*` constants defined above.
pub fn provider_capability_matrix() -> Vec<(&'static str, [bool; SKILL_DIMS])> {
    // Helper: all skills
    let all = [true; SKILL_DIMS];
    // No medical/legal (efficient general models)
    let no_med_legal = [true, true, true, false, false, true, true, true, true, true, true, true];
    // Code + math + data specialist (deepseek-v3, deepseek-r1)
    let code_math    = [true, true, false, false, false, true, false, false, false, false, true, true];
    // Pure code specialist (codestral, qwen2.5-coder)
    let code_only    = [false, true, false, false, false, false, false, false, false, false, true, false];
    // Multilingual + code (qwen models)
    let multilingual = [true, true, true, false, false, true, false, true, true, true, true, true];
    // OSS general (ollama/llama, ollama/mistral) — no legal/medical/temporal
    let oss_general  = [true, true, false, false, false, true, true, true, true, false, true, true];

    vec![
        // Flagship — all skills
        ("gpt-4o",               all),
        ("claude-sonnet-4-5",    all),
        ("gemini-2.5-pro",       all),
        // Efficient — no legal/medical
        ("gpt-4o-mini",          no_med_legal),
        ("claude-haiku-4-5",     no_med_legal),
        ("gemini-2.5-flash",     no_med_legal),
        ("o4-mini",              no_med_legal),
        // Specialist
        ("deepseek-v3",          code_math),
        ("qwen3-235b",           multilingual),
        ("codestral",            code_only),
        ("gemini-2.5-flash-lite", no_med_legal),
        // OSS
        ("llama-3.3-70b",        oss_general),
        ("qwen2.5-coder-32b",    code_only),
        ("mistral-nemo",         oss_general),
        ("deepseek-r1-8b",       code_math),
    ]
}

// ─── Skill extraction ─────────────────────────────────────────────────────────

/// Extract the required skill vector from a set of messages.
///
/// Returns a `[bool; SKILL_DIMS]` bitmask where `true` means the query
/// requires that skill capability. At least `SKILL_SUMMARISATION` is always
/// set as a baseline (all models should support basic Q&A).
///
/// P3 must replace or explicitly retire this keyword-derived advisory prior.
pub fn extract_skill_vector(messages: &[Message]) -> [bool; SKILL_DIMS] {
    let text = concat_user_text(messages);
    let lower = text.to_lowercase();
    let mut skills = [false; SKILL_DIMS];

    // ── Numerical reasoning ────────────────────────────────────────────────────
    let num_kw = ["calculate", "compute", "equation", "probability", "statistics",
                  "integral", "derivative", "formula", "percentage", "average",
                  "variance", "standard deviation", "sum", "product", "matrix"];
    if num_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_NUMERICAL_REASONING] = true;
    }

    // ── Code synthesis ─────────────────────────────────────────────────────────
    let code_kw = ["```", "function", "implement", "algorithm", "debug", "refactor",
                   "class", "struct", "api", "sql", "python", "rust", "javascript",
                   "typescript", "code", "compile", "runtime", "async", "library",
                   "interface", "module", "package",
                   // implicit code signals
                   "make it work", "fix this", "bug", "error", "exception",
                   "fn ", "def ", "class ", "endpoint"];
    if code_kw.iter().any(|&kw| text.contains(kw) || lower.contains(kw)) {
        skills[SKILL_CODE_SYNTHESIS] = true;
    }

    // ── Temporal logic ─────────────────────────────────────────────────────────
    let temporal_kw = ["schedule", "timeline", "before", "after", "deadline",
                       "sequence", "chronological", "calendar", "duration",
                       "earliest", "latest", "concurrent", "overlap", "recurring"];
    if temporal_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_TEMPORAL_LOGIC] = true;
    }

    // ── Legal interpretation ───────────────────────────────────────────────────
    let legal_kw = ["law", "legal", "regulation", "clause", "contract", "statute",
                    "liability", "compliance", "gdpr", "jurisdiction", "tort",
                    "litigation", "indemnif", "arbitration", "court", "ruling",
                    "precedent", "fiduciary", "ip rights", "patent", "trademark",
                    // extended implicit legal signals
                    "negligence", "damages", "lawsuit", "regulatory",
                    "attorney", "rights", "obligation"];
    if legal_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_LEGAL_INTERPRETATION] = true;
    }

    // ── Medical knowledge ──────────────────────────────────────────────────────
    let medical_kw = ["diagnosis", "treatment", "symptom", "disease", "medication",
                      "clinical", "pathology", "prognosis", "therapy", "dosage",
                      "contraindication", "patient", "medical", "surgery", "drug",
                      // extended implicit medical signals
                      "prescription", "side effect", "condition"];
    if medical_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_MEDICAL_KNOWLEDGE] = true;
    }

    // ── Scientific analysis ────────────────────────────────────────────────────
    let sci_kw = ["experiment", "hypothesis", "research", "scientific", "physics",
                  "chemistry", "biology", "quantum", "molecule", "reaction",
                  "empirical", "laboratory", "theory", "observation", "catalyst"];
    if sci_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_SCIENTIFIC_ANALYSIS] = true;
    }

    // ── Creative writing ───────────────────────────────────────────────────────
    let creative_kw = ["story", "poem", "creative", "fiction", "narrative",
                       "character", "plot", "metaphor", "rhyme", "write a ",
                       "short story", "novel", "prose", "verse"];
    if creative_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_CREATIVE_WRITING] = true;
    }

    // ── Entity extraction ──────────────────────────────────────────────────────
    let entity_kw = ["extract", "identify all", "list all", "find all",
                     "named entity", "occurrence", "mention", "parse", "retrieve",
                     "entities from", "names in"];
    if entity_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_ENTITY_EXTRACTION] = true;
    }

    // ── Summarisation ──────────────────────────────────────────────────────────
    // Always true — every model must handle basic summarisation / Q&A.
    let sum_kw = ["summarize", "summarise", "tldr", "brief", "overview",
                  "key points", "condensed", "abstract", "summary"];
    // Set if explicitly requested OR as baseline
    skills[SKILL_SUMMARISATION] = sum_kw.iter().any(|&kw| lower.contains(kw)) || true;

    // ── Translation ────────────────────────────────────────────────────────────
    let trans_kw = ["translate", "translation", "in french", "in spanish",
                    "in german", "in japanese", "in chinese", "in arabic",
                    "in portuguese", "to english", "to french"];
    if trans_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_TRANSLATION] = true;
    }

    // ── Symbolic mathematics ───────────────────────────────────────────────────
    let math_kw = ["prove", "theorem", "integral", "derivative", "differential",
                   "matrix", "eigenvalue", "topology", "algebra", "calculus",
                   "fourier", "laplace", "gradient descent", "optimization"];
    if math_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_MATH_SYMBOLIC] = true;
    }

    // ── Data analysis ──────────────────────────────────────────────────────────
    let data_kw = ["dataset", "dataframe", "correlation", "regression",
                   "cluster", "visualization", "plot", "chart", "pandas",
                   "sql query", "data analysis", "time series", "forecast",
                   "anomaly", "distribution"];
    if data_kw.iter().any(|&kw| lower.contains(kw)) {
        skills[SKILL_DATA_ANALYSIS] = true;
    }

    skills
}

fn concat_user_text(messages: &[Message]) -> String {
    messages
        .iter()
        .filter(|m| matches!(m.role, Role::User))
        .map(|m| content_to_text(&m.content))
        .collect::<Vec<_>>()
        .join(" ")
}

fn content_to_text(content: &Content) -> String {
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

    fn user_msg(text: &str) -> Message {
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

    #[test]
    fn test_code_query_sets_code_synthesis() {
        let msgs = vec![user_msg("Write a Python function to sort a list")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_CODE_SYNTHESIS], "Code query should set SKILL_CODE_SYNTHESIS");
        assert!(!sv[SKILL_NUMERICAL_REASONING], "Code query should not set numerical reasoning");
    }

    #[test]
    fn test_legal_query_sets_legal_interpretation() {
        let msgs = vec![user_msg("Analyse the GDPR compliance requirements for data transfers under this contract")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_LEGAL_INTERPRETATION], "Legal query should set SKILL_LEGAL_INTERPRETATION");
    }

    #[test]
    fn test_math_query_sets_numerical_and_symbolic() {
        let msgs = vec![user_msg("Calculate the integral of x^2 and prove the theorem")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_NUMERICAL_REASONING], "Math query should set numerical reasoning");
        assert!(sv[SKILL_MATH_SYMBOLIC], "Math query with 'prove' should set symbolic math");
    }

    #[test]
    fn test_creative_query_sets_creative_writing() {
        let msgs = vec![user_msg("Write a short story about a robot learning to paint")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_CREATIVE_WRITING], "Creative query should set SKILL_CREATIVE_WRITING");
    }

    #[test]
    fn test_summarisation_always_set() {
        let msgs = vec![user_msg("What is the capital of France?")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_SUMMARISATION], "SKILL_SUMMARISATION must always be set as baseline");
    }

    #[test]
    fn test_code_query_does_not_set_medical() {
        let msgs = vec![user_msg("Implement a binary search tree in Rust")];
        let sv = extract_skill_vector(&msgs);
        assert!(!sv[SKILL_MEDICAL_KNOWLEDGE], "Code query should not set medical knowledge");
        assert!(!sv[SKILL_LEGAL_INTERPRETATION], "Code query should not set legal interpretation");
    }

    #[test]
    fn test_capability_matrix_has_15_entries() {
        let matrix = provider_capability_matrix();
        assert_eq!(matrix.len(), 15, "Capability matrix must have 15 entries");
    }

    #[test]
    fn test_flagship_models_have_all_skills() {
        let matrix = provider_capability_matrix();
        for &name in &["gpt-4o", "claude-sonnet-4-5", "gemini-2.5-pro"] {
            let entry = matrix.iter().find(|(n, _)| *n == name).expect(name);
            assert!(
                entry.1.iter().all(|&b| b),
                "{} should have all skills set to true",
                name
            );
        }
    }

    #[test]
    fn test_codestral_specialises_in_code() {
        let matrix = provider_capability_matrix();
        let (_, sv) = matrix.iter().find(|(n, _)| *n == "codestral").expect("codestral");
        assert!(sv[SKILL_CODE_SYNTHESIS], "codestral must have code synthesis");
        assert!(!sv[SKILL_LEGAL_INTERPRETATION], "codestral should not have legal");
        assert!(!sv[SKILL_MEDICAL_KNOWLEDGE], "codestral should not have medical");
    }

    // ── Legacy advisory keyword coverage tests ──────────────────────────────────

    #[test]
    fn test_implicit_legal_liability() {
        let msgs = vec![user_msg("What's my liability if a contractor gets hurt on my property?")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_LEGAL_INTERPRETATION],
            "Implicit legal query with 'liability' must set SKILL_LEGAL_INTERPRETATION");
    }

    #[test]
    fn test_implicit_code_make_it_work() {
        let msgs = vec![user_msg("make it work: const x = arr.find(i => i.id === target)")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_CODE_SYNTHESIS],
            "Implicit code query with 'make it work' must set SKILL_CODE_SYNTHESIS");
    }

    #[test]
    fn test_implicit_medical_medication() {
        let msgs = vec![user_msg("What medication should I take for a persistent headache?")];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_MEDICAL_KNOWLEDGE],
            "Implicit medical query with 'medication' must set SKILL_MEDICAL_KNOWLEDGE");
    }

    #[test]
    fn test_numeric_simple_not_numerical_reasoning() {
        // "How many days in a year?" is a factual recall question, not a calculation.
        // The advisory extractor uses keyword matching; this query contains no numeric
        // reasoning keywords, so SKILL_NUMERICAL_REASONING must not be set.
        let msgs = vec![user_msg("How many days in a year?")];
        let sv = extract_skill_vector(&msgs);
        assert!(!sv[SKILL_NUMERICAL_REASONING],
            "Simple factual question must not set SKILL_NUMERICAL_REASONING");
    }

    #[test]
    fn test_translation_detected() {
        let msgs = vec![user_msg(
            "Translate this paragraph to French: The router selects models based on complexity",
        )];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_TRANSLATION],
            "Translation query must set SKILL_TRANSLATION");
    }

    #[test]
    fn test_entity_extraction_detected() {
        let msgs = vec![user_msg(
            "Find all company names mentioned in this text: Apple, Microsoft and Google announced a partnership",
        )];
        let sv = extract_skill_vector(&msgs);
        assert!(sv[SKILL_ENTITY_EXTRACTION],
            "Entity extraction query must set SKILL_ENTITY_EXTRACTION");
    }
}
