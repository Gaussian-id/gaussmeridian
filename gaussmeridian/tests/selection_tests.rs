//! M2-VAL Selection Tests — legacy deterministic scoring and advisory skill-vector extraction.
//!
//! These are pure unit tests covering the component functions used by
//! `selection_middleware_with_state`. They do not exercise the HTTP stack.
//!
//! # Tests
//! - A: Code query sets the SKILL_CODE_SYNTHESIS bit
//! - B: Legal query sets the SKILL_LEGAL_INTERPRETATION bit (legacy advisory filter)
//! - C: λ flip (CORRECTNESS GATE) — Cobb-Douglas score reversal at λ=0.1 vs λ=0.9

use gaussmeridian_core::{
    extract_skill_vector, provider_score,
    skill::{SKILL_CODE_SYNTHESIS, SKILL_DIMS, SKILL_LEGAL_INTERPRETATION},
};
use gaussmeridian_models::request::{Content, Message, Role};

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

/// Minimal provider entry for hard-skill filtering tests.
struct FakeProvider {
    skill_vector: [bool; SKILL_DIMS],
}

impl FakeProvider {
    fn new_all_skills() -> Self {
        Self {
            skill_vector: [true; SKILL_DIMS],
        }
    }

    fn new_no_legal() -> Self {
        let mut sv = [true; SKILL_DIMS];
        sv[SKILL_LEGAL_INTERPRETATION] = false;
        Self { skill_vector: sv }
    }

    fn covers(&self, required: &[bool; SKILL_DIMS]) -> bool {
        required
            .iter()
            .enumerate()
            .all(|(i, &req)| !req || self.skill_vector[i])
    }
}

// ─── Test A — Code query selects code-capable providers ──────────────────────

/// Test A: A code-bearing query sets SKILL_CODE_SYNTHESIS.
///
/// A code-specialist with lower cost outscores a general provider at λ=0.5
/// (equal weighting) when cost difference is significant.
#[test]
fn test_code_query_selects_code_capable_provider() {
    let msgs = vec![user_msg(
        "Write a Rust function to implement binary search on a sorted Vec",
    )];
    let sv = extract_skill_vector(&msgs);

    // Skill bit must be set
    assert!(
        sv[SKILL_CODE_SYNTHESIS],
        "Code query must set SKILL_CODE_SYNTHESIS (index {})",
        SKILL_CODE_SYNTHESIS
    );

    // A code-specialist with lower cost ($0.20/M) outscores a general provider ($5.00/M)
    // at equal quality (0.90) with λ=0.5 (balanced scoring).
    let code_specialist_score = provider_score(0.90, 0.20, 1.0, 0.5);
    let general_provider_score = provider_score(0.90, 5.00, 1.0, 0.5);

    assert!(
        code_specialist_score > general_provider_score,
        "Balanced λ=0.5: cheaper code-specialist ({:.4}) must beat expensive general ({:.4})",
        code_specialist_score,
        general_provider_score
    );
}

// ─── Test B — Legal query excludes non-legal providers ───────────────────────

/// Test B: A GDPR legal query sets SKILL_LEGAL_INTERPRETATION; a provider
/// without that skill bit is excluded by the legacy advisory filter.
#[test]
fn test_legal_query_excludes_non_legal_provider() {
    let msgs = vec![user_msg(
        "Analyse the jurisdictional implications of cross-border data transfers \
         under GDPR Article 46(2)(c) for a multi-party SaaS",
    )];
    let sv = extract_skill_vector(&msgs);

    // Legal skill bit must be set
    assert!(
        sv[SKILL_LEGAL_INTERPRETATION],
        "GDPR legal query must set SKILL_LEGAL_INTERPRETATION (index {})",
        SKILL_LEGAL_INTERPRETATION
    );

    // A provider WITHOUT the legal skill must be excluded by the advisory coverage check
    let no_legal = FakeProvider::new_no_legal();
    let covers_legal_query = no_legal.covers(&sv);

    assert!(
        !covers_legal_query,
        "A provider without SKILL_LEGAL_INTERPRETATION must NOT cover a legal query"
    );

    // A provider WITH all skills must pass the coverage check
    let full_skills = FakeProvider::new_all_skills();
    let full_covers = full_skills.covers(&sv);

    assert!(
        full_covers,
        "A full-skill provider must cover the legal query"
    );
}

// ─── Test C — λ flip (CORRECTNESS GATE) ─────────────────────────────────────

/// Test D: Cobb-Douglas formula must invert provider preference when λ flips.
///
/// Provider pair chosen so that the crossover λ* ≈ 0.16 falls between the two
/// test points (λ=0.05 and λ=0.9), making both assertions provably correct:
///
/// - flagship:  quality=0.99, cost=$3.00/M, health=1.0 (high quality, moderate cost)
/// - budget:    quality=0.70, cost=$0.50/M, health=1.0 (lower quality, cheap)
///
/// Crossover λ* ≈ ln(0.99/0.70) / (ln(0.99/0.70) + ln(3.0/0.5)) ≈ 0.16
///   → λ=0.05 (quality-first): flagship must win  (λ < λ*)
///   → λ=0.9  (cost-first):    budget must win    (λ > λ*)
///
/// The OLD formula `(1.0 - lambda + 0.1)` applied the same multiplier to all
/// candidates, so ranking never changed with λ. The Cobb-Douglas fix must
/// produce genuine λ-sensitivity (preference inversion across λ*).
#[test]
fn test_lambda_flip_quality_vs_cost() {
    // λ = 0.05 — strong quality-first: flagship must win
    let flagship_q_first = provider_score(0.99, 3.0, 1.0, 0.05);
    let budget_q_first = provider_score(0.70, 0.5, 1.0, 0.05);

    assert!(
        flagship_q_first > budget_q_first,
        "Quality-first (λ=0.05): flagship ({:.6}) must beat budget ({:.6})",
        flagship_q_first,
        budget_q_first
    );

    // λ = 0.9 — strong cost-first: budget must win
    let flagship_cost_first = provider_score(0.99, 3.0, 1.0, 0.9);
    let budget_cost_first = provider_score(0.70, 0.5, 1.0, 0.9);

    assert!(
        budget_cost_first > flagship_cost_first,
        "Cost-first (λ=0.9): budget ({:.6}) must beat flagship ({:.6})",
        budget_cost_first,
        flagship_cost_first
    );
}
