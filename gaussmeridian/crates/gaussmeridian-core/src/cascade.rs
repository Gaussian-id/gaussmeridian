//! Cascade / escalation routing.
//!
//! Meridian V2 candidate 3 (from BLACK SEA OP. PARALLAX F4): instead of committing to the single
//! highest-scored model per request, cascade routing tries the *cheapest viable* candidate first
//! and escalates to a more capable one only when the response's confidence is too low — the
//! pattern that, in the cited ICLR'25 result, cut cost ~85% while retaining ~95% of frontier
//! quality by sending only the genuinely-hard queries to the expensive model.
//!
//! This module holds the two pure decisions; the middleware owns the plumbing:
//!   1. [`order_candidates_cheapest_first`] — the ordering the selection stage should hand the
//!      provider stage when cascade is on (ascending cost, so index 0 is the cheapest try).
//!   2. [`should_escalate`] — after a candidate answers, whether to escalate to the next one.
//!
//! `should_escalate` reuses candidate 1's calibration ([`crate::outcome_gate::calibrate_confidence`])
//! so the escalation signal is the *calibrated* confidence, exactly the fix UCCI called for — an
//! uncalibrated raw score would make the escalation decision as unreliable as the gate it feeds.

use crate::outcome_gate::{calibrate_confidence, extract_confidence};

/// Server/project-level cascade configuration. `enabled = false` is the default no-op.
#[derive(Debug, Clone)]
pub struct CascadeConfig {
    pub enabled: bool,
    /// Escalate to the next candidate when the calibrated confidence is below this.
    pub confidence_threshold: f64,
    /// Temperature used to calibrate the raw confidence before the comparison (candidate 1).
    pub temperature: f64,
}

impl Default for CascadeConfig {
    fn default() -> Self {
        Self { enabled: false, confidence_threshold: 0.7, temperature: 1.0 }
    }
}

/// Return the indices of `costs` ordered cheapest-first (stable on ties). The selection stage
/// applies this to its already-filtered candidate list when cascade is enabled, so the provider
/// stage tries the cheapest model first and walks up the cost ladder as it escalates.
pub fn order_candidates_cheapest_first(costs: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..costs.len()).collect();
    idx.sort_by(|&a, &b| costs[a].partial_cmp(&costs[b]).unwrap_or(std::cmp::Ordering::Equal));
    idx
}

/// Decide whether to escalate to the next (more capable) candidate after this one answered.
///
/// Escalates only when cascade is enabled, another candidate remains to escalate *to*, the
/// response carries a confidence signal, and that signal — after calibration — is below the
/// configured threshold. A response with no confidence field is accepted (not escalated): there
/// is nothing to act on, and escalating blindly would defeat the cost saving.
pub fn should_escalate(config: &CascadeConfig, response_body: &str, has_more_candidates: bool) -> bool {
    if !config.enabled || !has_more_candidates {
        return false;
    }
    let Ok(body) = serde_json::from_str::<serde_json::Value>(response_body) else {
        return false;
    };
    let Some(raw) = extract_confidence(&body) else {
        return false;
    };
    calibrate_confidence(raw, config.temperature) < config.confidence_threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cfg(threshold: f64, temperature: f64) -> CascadeConfig {
        CascadeConfig { enabled: true, confidence_threshold: threshold, temperature }
    }

    #[test]
    fn orders_cheapest_first_stably() {
        // costs: index 0=$5, 1=$1, 2=$3, 3=$1
        assert_eq!(order_candidates_cheapest_first(&[5.0, 1.0, 3.0, 1.0]), vec![1, 3, 2, 0]);
    }

    #[test]
    fn disabled_cascade_never_escalates() {
        let c = CascadeConfig::default(); // enabled = false
        let body = json!({"confidence": 0.1}).to_string();
        assert!(!should_escalate(&c, &body, true));
    }

    #[test]
    fn does_not_escalate_when_no_more_candidates() {
        let body = json!({"confidence": 0.1}).to_string();
        assert!(!should_escalate(&cfg(0.7, 1.0), &body, false));
    }

    #[test]
    fn escalates_on_low_confidence() {
        let body = json!({"confidence": 0.4}).to_string();
        assert!(should_escalate(&cfg(0.7, 1.0), &body, true));
    }

    #[test]
    fn does_not_escalate_on_high_confidence() {
        let body = json!({"confidence": 0.95}).to_string();
        assert!(!should_escalate(&cfg(0.7, 1.0), &body, true));
    }

    #[test]
    fn accepts_response_with_no_confidence_signal() {
        // Nothing to act on → don't escalate (accept the cheap answer).
        let body = json!({"choices": [{"message": {"content": "hi"}}]}).to_string();
        assert!(!should_escalate(&cfg(0.7, 1.0), &body, true));
    }

    #[test]
    fn calibration_can_turn_a_borderline_accept_into_an_escalate() {
        // raw 0.75 clears a 0.7 threshold at T=1 (accept) but not once tempered at T=3 (escalate).
        let body = json!({"confidence": 0.75}).to_string();
        assert!(!should_escalate(&cfg(0.7, 1.0), &body, true));
        assert!(should_escalate(&cfg(0.7, 3.0), &body, true));
    }

    #[test]
    fn malformed_body_does_not_escalate() {
        assert!(!should_escalate(&cfg(0.7, 1.0), "not json", true));
    }
}
