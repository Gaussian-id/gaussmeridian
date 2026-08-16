//! Health monitoring types and provider scoring functions.
//!
//! Contains both the existing health status types (M1) and the EWMA/H(t)/score
//! functions required by SelectionMiddleware (M2).

use chrono::{DateTime, Utc};
use gaussmeridian_models::ProviderCapabilities;

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub status: HealthStatus,
    pub last_check: DateTime<Utc>,
    pub response_time: Option<std::time::Duration>,
    pub error_rate: f64,
}

#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub health: ProviderHealth,
    pub capabilities: ProviderCapabilities,
    pub cost_info: Option<gaussmeridian_models::CostInfo>,
    pub response_time: Option<std::time::Duration>,
}

// ─── M2 Provider Scoring ──────────────────────────────────────────────────────

/// Smoothing factor for quality score EWMA (α = 0.1).
///
/// Each new outcome shifts the running average by 10%. Balances responsiveness
/// to recent failures with stability against transient noise.
pub const EWMA_ALPHA: f32 = 0.1;

/// Minimum cost denominator — prevents division by zero for zero-cost OSS models.
const MIN_COST_DENOMINATOR: f64 = 0.001;

/// Update the EWMA quality score with a new binary outcome.
///
/// Formula: `Q_new = α × r_binary + (1 − α) × Q_current`
///
/// Source: GaussMeridian Provider Selection Algorithm (Obsidian note 06).
/// Scores are seeded at 0.85 (neutral prior) and converge toward real performance
/// from M3 onwards when the outcome gate produces `r_binary` values.
///
/// # Arguments
/// * `current`      — current EWMA quality score ∈ \[0.0, 1.0\]
/// * `new_r_binary` — outcome: `1.0` = validator passed, `0.0` = validator failed
/// * `alpha`        — smoothing factor; pass `EWMA_ALPHA` for the standard value
pub fn update_quality_ewma(current: f32, new_r_binary: f32, alpha: f32) -> f32 {
    (alpha * new_r_binary + (1.0 - alpha) * current).clamp(0.0, 1.0)
}

/// Compute the availability health score H(t) ∈ \[0.0, 1.0\].
///
/// Formula: `H(t) = (1 − E(t)) × (1 − R(t)) × S(t)`
///
/// Source: GaussMeridian Provider Selection Algorithm (Obsidian note 06).
///
/// # Arguments
/// * `error_rate`   — fraction of recent requests that failed ∈ \[0, 1\]
/// * `latency_norm` — normalised P99 latency: `0.0` = fast, `1.0` = unacceptably slow
/// * `cb_weight`    — circuit breaker weight from `CircuitState::scoring_weight()`:
///                    `closed=1.0`, `half_open=0.5`, `open=0.0`
pub fn health_score(error_rate: f32, latency_norm: f32, cb_weight: f32) -> f32 {
    ((1.0 - error_rate) * (1.0 - latency_norm) * cb_weight).clamp(0.0, 1.0)
}

/// Compute the legacy deterministic provider score for candidate ranking.
///
/// Formula (Cobb-Douglas): `score(p) = Q(p)^(1-λ) × (1/C(p))^λ × H(t)`
///
/// This is a GaussMeridian product heuristic, not the learned xRouter policy or its reward
/// objective. Paper-faithful xRouter work belongs to PRD-26 P5.
///
/// Higher score = preferred candidate. Score of `0.0` means the candidate is excluded
/// (circuit open, or zero availability).
///
/// # Arguments
/// * `quality`           — `Q(p)`: EWMA quality score ∈ \[0, 1\]
/// * `cost_per_m_output` — `C(p)`: cost per million output tokens in USD
/// * `availability`      — `H(t)`: health score from [`health_score`]
/// * `lambda`            — `λ`: project cost-sensitivity ∈ \[0, 1\]
///                         (`1.0` = cost-first, `0.0` = quality-first)
pub fn provider_score(quality: f64, cost_per_m_output: f64, availability: f64, lambda: f64) -> f64 {
    if availability == 0.0 {
        return 0.0;  // circuit open — explicit exclusion
    }
    let quality_term = quality.powf(1.0 - lambda);
    let cost_term = (1.0_f64 / cost_per_m_output.max(MIN_COST_DENOMINATOR)).powf(lambda);
    (quality_term * cost_term * availability).max(0.0)
}

#[cfg(test)]
mod scoring_tests {
    use super::*;

    #[test]
    fn test_ewma_success_increases_score() {
        let updated = update_quality_ewma(0.85, 1.0, EWMA_ALPHA);
        assert!(updated > 0.85, "Success should increase EWMA, got {}", updated);
    }

    #[test]
    fn test_ewma_failure_decreases_score() {
        let updated = update_quality_ewma(0.85, 0.0, EWMA_ALPHA);
        assert!(updated < 0.85, "Failure should decrease EWMA, got {}", updated);
    }

    #[test]
    fn test_ewma_convergence_to_one_after_50_successes() {
        let mut score = 0.85_f32;
        for _ in 0..50 {
            score = update_quality_ewma(score, 1.0, EWMA_ALPHA);
        }
        assert!(score > 0.99, "Score should converge toward 1.0 after 50 successes, got {}", score);
    }

    #[test]
    fn test_ewma_convergence_to_zero_after_100_failures() {
        let mut score = 0.85_f32;
        for _ in 0..100 {
            score = update_quality_ewma(score, 0.0, EWMA_ALPHA);
        }
        assert!(score < 0.01, "Score should converge toward 0.0 after 100 failures, got {}", score);
    }

    #[test]
    fn test_health_score_perfect_is_one() {
        let h = health_score(0.0, 0.0, 1.0);
        assert!((h - 1.0).abs() < 1e-6, "Perfect health should be 1.0, got {}", h);
    }

    #[test]
    fn test_health_score_open_circuit_is_zero() {
        let h = health_score(0.0, 0.0, 0.0);
        assert_eq!(h, 0.0, "Open circuit must give zero health score");
    }

    #[test]
    fn test_health_score_half_open_circuit() {
        let h = health_score(0.0, 0.0, 0.5);
        assert!((h - 0.5).abs() < 1e-6, "Half-open should give 0.5, got {}", h);
    }

    #[test]
    fn test_provider_score_zero_when_circuit_open() {
        let score = provider_score(0.9, 10.0, 0.0, 0.5);
        assert_eq!(score, 0.0, "Open circuit (availability=0) must give zero score");
    }

    #[test]
    fn test_provider_score_oss_zero_cost_no_panic() {
        let score = provider_score(0.85, 0.0, 1.0, 0.5);
        assert!(score > 0.0, "Zero-cost OSS model must produce positive score");
        assert!(score.is_finite(), "Score must be finite");
    }

    #[test]
    fn test_provider_score_high_lambda_prefers_cheap() {
        let cheap     = provider_score(0.85, 0.60,  1.0, 0.9); // gpt-4o-mini
        let expensive = provider_score(0.90, 15.00, 1.0, 0.9); // claude-sonnet
        assert!(cheap > expensive,
            "High lambda should prefer cheap: cheap={:.4} expensive={:.4}", cheap, expensive);
    }

    #[test]
    fn test_provider_score_positive_for_valid_inputs() {
        let score = provider_score(0.85, 10.0, 1.0, 0.5);
        assert!(score > 0.0 && score.is_finite());
    }
}
