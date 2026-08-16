//! OutcomeGate — Tower Layer 7 validation logic (M3).
//!
//! The OutcomeGate determines `r_binary` for each provider response:
//!   - `r_binary = 1`: response passed the validator → charge the request
//!   - `r_binary = 0`: response failed → zero-charge, trigger retry if attempts remain
//!
//! # Validators
//!
//! | Type         | Description                                               | MVP |
//! |--------------|-----------------------------------------------------------|-----|
//! | `None`       | Always returns `r_binary = 1`. Default.                  | ✅  |
//! | `JsonSchema` | Validates provider JSON body against a JSON Schema.      | ✅  |
//! | `Confidence` | Checks a `"confidence"` field in the response body.      | ✅  |
//! | `Webhook`    | POSTs the body to an external URL; 2xx = pass.           | ✅  |
//! | `UnitTest`   | Runs Python-style assert strings (stub for MVP).         | ✅  |
//!
//! Placement: called inline inside `provider_middleware_with_state` after the
//! provider returns a successful HTTP 2xx response (before cost/ledger write).

use std::time::Duration;

// ─── Public result type ───────────────────────────────────────────────────────

/// Result from one OutcomeGate evaluation.
#[derive(Debug, Clone)]
pub struct GateResult {
    /// 1 = validator passed, 0 = validator rejected the response.
    pub r_binary: i64,
    /// Human-readable outcome stored in `ledger_entry.validator_result`.
    /// Values: `"skipped"`, `"passed"`, `"failed:<reason>"`.
    pub detail:   String,
}

impl GateResult {
    fn pass() -> Self {
        Self { r_binary: 1, detail: "passed".into() }
    }

    fn skip() -> Self {
        Self { r_binary: 1, detail: "skipped".into() }
    }

    fn fail(reason: impl Into<String>) -> Self {
        Self { r_binary: 0, detail: format!("failed:{}", reason.into()) }
    }
}

// ─── OutcomeGate ─────────────────────────────────────────────────────────────

/// OutcomeGate evaluates a provider response body against the configured validator.
///
/// Constructed from `ProjectSettingsExt.validator_type` and `.validator_config`
/// in `provider_middleware_with_state`. Each request gets a fresh `OutcomeGate`
/// (cheap — just two clones from extensions).
pub struct OutcomeGate {
    validator_type:   String,
    validator_config: serde_json::Value,
}

impl OutcomeGate {
    /// Create an OutcomeGate from the project's validator settings.
    pub fn new(validator_type: impl Into<String>, validator_config: serde_json::Value) -> Self {
        Self {
            validator_type:   validator_type.into(),
            validator_config,
        }
    }

    /// Evaluate the provider response body and return `GateResult`.
    ///
    /// `response_body` is the raw JSON string returned by the provider
    /// (e.g. an OpenAI-compatible `ChatCompletion` object).
    pub async fn evaluate(&self, response_body: &str) -> GateResult {
        match self.validator_type.as_str() {
            "none" | "" => self.validate_none(),
            "json_schema"  => self.validate_json_schema(response_body),
            "confidence"   => self.validate_confidence(response_body),
            "webhook"      => self.validate_webhook(response_body).await,
            "unit_test"    => self.validate_unit_test(response_body),
            unknown => {
                // Unknown validator — log and pass to avoid blocking production
                tracing::warn!(validator = %unknown, "OutcomeGate: unknown validator type, defaulting to pass");
                GateResult::skip()
            }
        }
    }

    // ─── Validator: None ─────────────────────────────────────────────────────

    fn validate_none(&self) -> GateResult {
        GateResult::skip()
    }

    // ─── Validator: JsonSchema ────────────────────────────────────────────────

    /// Validate `response_body` against a JSON Schema stored in `validator_config["schema"]`.
    ///
    /// The response is expected to be an OpenAI-compatible ChatCompletion object.
    /// Only `choices[0].message.content` is validated against the schema if it is
    /// itself a JSON string; the full response body is validated otherwise.
    fn validate_json_schema(&self, response_body: &str) -> GateResult {
        // [1] Parse the response body
        let body: serde_json::Value = match serde_json::from_str(response_body) {
            Ok(v) => v,
            Err(e) => return GateResult::fail(format!("response_body is not valid JSON: {}", e)),
        };

        // [2] Extract the schema from validator_config
        let schema = match self.validator_config.get("schema") {
            Some(s) => s.clone(),
            None => return GateResult::fail("validator_config.schema is missing"),
        };

        // [3] Extract the content to validate — prefer choices[0].message.content as JSON
        let target: serde_json::Value = {
            let content_str = body
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str());

            match content_str.and_then(|s| serde_json::from_str(s).ok()) {
                Some(parsed) => parsed,
                None => body.clone(),
            }
        };

        // [4] Basic structural validation: check required top-level keys if specified
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for key in required {
                if let Some(key_str) = key.as_str() {
                    if target.get(key_str).is_none() {
                        return GateResult::fail(format!("missing required field: {}", key_str));
                    }
                }
            }
        }

        // [5] Check type constraints for declared properties
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (field, spec) in props {
                if let Some(value) = target.get(field) {
                    if let Some(expected_type) = spec.get("type").and_then(|t| t.as_str()) {
                        let type_matches = match expected_type {
                            "string"  => value.is_string(),
                            "number"  => value.is_number(),
                            "integer" => value.is_i64() || value.is_u64(),
                            "boolean" => value.is_boolean(),
                            "array"   => value.is_array(),
                            "object"  => value.is_object(),
                            "null"    => value.is_null(),
                            _         => true,
                        };
                        if !type_matches {
                            return GateResult::fail(format!(
                                "field '{}' expected type '{}', got '{}'",
                                field, expected_type, json_type_name(value)
                            ));
                        }
                    }
                }
            }
        }

        GateResult::pass()
    }

    // ─── Validator: Confidence ────────────────────────────────────────────────

    /// Check that the provider's response includes a `"confidence"` field
    /// (top-level or inside `choices[0].message`) meeting a minimum threshold.
    ///
    /// Config: `{ "threshold": 0.8, "temperature": 1.0 }`. `threshold` defaults to 0.7.
    /// `temperature` (Meridian V2 candidate 1) calibrates the raw self-reported confidence
    /// before the threshold comparison via [`calibrate_confidence`]; it defaults to `1.0`
    /// (identity), so a config that omits it behaves exactly as before.
    fn validate_confidence(&self, response_body: &str) -> GateResult {
        let threshold = self.validator_config
            .get("threshold")
            .and_then(|t| t.as_f64())
            .unwrap_or(0.7);

        let temperature = self.validator_config
            .get("temperature")
            .and_then(|t| t.as_f64())
            .unwrap_or(1.0);

        let body: serde_json::Value = match serde_json::from_str(response_body) {
            Ok(v) => v,
            Err(e) => return GateResult::fail(format!("response_body is not valid JSON: {}", e)),
        };

        match extract_confidence(&body) {
            None => GateResult::fail("confidence field not found in response"),
            Some(raw) => {
                let calibrated = calibrate_confidence(raw, temperature);
                if calibrated < threshold {
                    GateResult::fail(format!(
                        "calibrated confidence {:.3} (raw {:.3}, T={:.2}) below threshold {:.3}",
                        calibrated, raw, temperature, threshold
                    ))
                } else {
                    GateResult::pass()
                }
            }
        }
    }

    // ─── Validator: Webhook ───────────────────────────────────────────────────

    /// POST the response body to an external webhook URL.
    /// A 2xx HTTP response from the webhook = pass; anything else = fail.
    ///
    /// Config: `{ "url": "https://...", "timeout_ms": 2000 }`.
    /// Non-fatal on network error: returns `fail` to trigger retry logic.
    async fn validate_webhook(&self, response_body: &str) -> GateResult {
        let url = match self.validator_config.get("url").and_then(|u| u.as_str()) {
            Some(u) => u.to_string(),
            None => return GateResult::fail("validator_config.url is missing for webhook validator"),
        };

        let timeout_ms = self.validator_config
            .get("timeout_ms")
            .and_then(|t| t.as_u64())
            .unwrap_or(2000);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_default();

        let result = client
            .post(&url)
            .header("content-type", "application/json")
            .body(response_body.to_string())
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => GateResult::pass(),
            Ok(resp) => GateResult::fail(format!("webhook returned HTTP {}", resp.status())),
            Err(e) => GateResult::fail(format!("webhook request failed: {}", e)),
        }
    }

    // ─── Validator: UnitTest ──────────────────────────────────────────────────

    /// Evaluate a list of assert expressions against fields in the parsed response.
    ///
    /// Config: `{ "asserts": ["choices[0].message.content != \"\"", ...] }`.
    ///
    /// MVP implementation: only non-empty content assertion is evaluated.
    /// Full expression evaluation deferred to M4 when a sandboxed engine is available.
    fn validate_unit_test(&self, response_body: &str) -> GateResult {
        let asserts = match self.validator_config
            .get("asserts")
            .and_then(|a| a.as_array())
        {
            Some(a) => a.clone(),
            None => return GateResult::fail("validator_config.asserts is missing or not an array"),
        };

        let body: serde_json::Value = match serde_json::from_str(response_body) {
            Ok(v) => v,
            Err(e) => return GateResult::fail(format!("response_body is not valid JSON: {}", e)),
        };

        // MVP: evaluate a simplified set of assertions
        for assert in &asserts {
            let expr = match assert.as_str() {
                Some(s) => s,
                None => continue,
            };

            // Supported: check that choices[0].message.content is non-empty
            if expr.contains("choices[0].message.content") && expr.contains("!= \"\"") {
                let content = body
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("");

                if content.is_empty() {
                    return GateResult::fail(format!("assert failed: {}", expr));
                }
            }

            // Unknown expression patterns pass (sandbox evaluation deferred to M4)
        }

        GateResult::pass()
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Temperature-scale a raw self-reported confidence into a calibrated one.
///
/// Meridian V2 candidate 1 (from BLACK SEA OP. PARALLAX F4 / UCCI arXiv:2605.18796):
/// a raw LLM-reported `confidence` field is systematically miscalibrated, so gating a
/// billing decision (`R_binary`) directly on it is unreliable. This applies temperature
/// scaling (Guo et al. 2017, "On Calibration of Modern Neural Networks") in logit space:
///
/// ```text
/// calibrated = σ( logit(raw) / T )
/// ```
///
/// - `T = 1.0` is the identity — unchanged behavior, and the default, so existing
///   `confidence` validators keep working exactly as before.
/// - `T > 1.0` pulls confidence toward 0.5, tempering the overconfidence that is the
///   documented failure mode — a spuriously-high raw score can drop below threshold.
/// - `T < 1.0` sharpens. `0.5` is the fixed point for every `T`, and the map is monotonic
///   in `raw`, so ordering is preserved.
///
/// A non-positive or NaN temperature is treated as disabled (identity), never a panic.
pub fn calibrate_confidence(raw: f64, temperature: f64) -> f64 {
    // Disabled / degenerate temperature → identity (still clamped to a valid probability).
    if !(temperature > 0.0) || (temperature - 1.0).abs() < f64::EPSILON {
        return raw.clamp(0.0, 1.0);
    }
    // Clamp away from the 0/1 asymptotes so `logit` stays finite.
    let p = raw.clamp(1e-6, 1.0 - 1e-6);
    let logit = (p / (1.0 - p)).ln();
    let scaled = logit / temperature;
    1.0 / (1.0 + (-scaled).exp())
}

/// Extract a self-reported confidence from an OpenAI-compatible response body: a top-level
/// `confidence`, else `choices[0].message.confidence`. Shared by the OutcomeGate confidence
/// validator and cascade routing so both read the signal the same way.
pub fn extract_confidence(body: &serde_json::Value) -> Option<f64> {
    body.get("confidence")
        .and_then(|c| c.as_f64())
        .or_else(|| {
            body.get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("confidence"))
                .and_then(|c| c.as_f64())
        })
}

fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null    => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_)  => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn gate(validator_type: &str, config: serde_json::Value) -> OutcomeGate {
        OutcomeGate::new(validator_type, config)
    }

    fn chat_body(content: &str) -> String {
        json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": content},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30}
        }).to_string()
    }

    // Test 1 — None validator always returns r_binary=1
    #[tokio::test]
    async fn test_none_validator_always_passes() {
        let result = gate("none", json!({})).evaluate("any body").await;
        assert_eq!(result.r_binary, 1);
        assert_eq!(result.detail, "skipped");
    }

    // Test 2 — Empty validator_type falls through to None
    #[tokio::test]
    async fn test_empty_type_defaults_to_none() {
        let result = gate("", json!({})).evaluate("{}").await;
        assert_eq!(result.r_binary, 1);
    }

    // Test 3 — JsonSchema pass: required field present
    #[tokio::test]
    async fn test_json_schema_passes_when_required_field_present() {
        let schema = json!({
            "schema": {
                "required": ["answer"],
                "properties": { "answer": { "type": "string" } }
            }
        });
        // Content is a JSON string that will be re-parsed
        let body = json!({
            "choices": [{"message": {"content": "{\"answer\": \"hello\"}"}}]
        }).to_string();
        let result = gate("json_schema", schema).evaluate(&body).await;
        assert_eq!(result.r_binary, 1, "Should pass: answer field present");
    }

    // Test 4 — JsonSchema fail: required field missing
    #[tokio::test]
    async fn test_json_schema_fails_when_required_field_missing() {
        let schema = json!({
            "schema": {
                "required": ["answer"],
                "properties": { "answer": { "type": "string" } }
            }
        });
        let body = json!({
            "choices": [{"message": {"content": "{\"wrong_field\": \"hello\"}"}}]
        }).to_string();
        let result = gate("json_schema", schema).evaluate(&body).await;
        assert_eq!(result.r_binary, 0, "Should fail: answer field missing");
        assert!(result.detail.contains("missing required field"), "Detail: {}", result.detail);
    }

    // Test 5 — Confidence pass: confidence >= threshold
    #[tokio::test]
    async fn test_confidence_passes_above_threshold() {
        let body = json!({"confidence": 0.9}).to_string();
        let result = gate("confidence", json!({"threshold": 0.8})).evaluate(&body).await;
        assert_eq!(result.r_binary, 1);
    }

    // Test 6 — Confidence fail: confidence < threshold
    #[tokio::test]
    async fn test_confidence_fails_below_threshold() {
        let body = json!({"confidence": 0.6}).to_string();
        let result = gate("confidence", json!({"threshold": 0.8})).evaluate(&body).await;
        assert_eq!(result.r_binary, 0);
        assert!(result.detail.contains("below threshold"), "Detail: {}", result.detail);
    }

    // Test 7 — Confidence fail: field absent
    #[tokio::test]
    async fn test_confidence_fails_when_field_absent() {
        let body = chat_body("Hello there");
        let result = gate("confidence", json!({"threshold": 0.7})).evaluate(&body).await;
        assert_eq!(result.r_binary, 0);
        assert!(result.detail.contains("not found"), "Detail: {}", result.detail);
    }

    // Test 8 — UnitTest pass: non-empty content assertion
    #[tokio::test]
    async fn test_unit_test_passes_non_empty_content() {
        let config = json!({
            "asserts": ["choices[0].message.content != \"\""]
        });
        let body = chat_body("Some response text");
        let result = gate("unit_test", config).evaluate(&body).await;
        assert_eq!(result.r_binary, 1);
    }

    // Test 9 — UnitTest fail: empty content assertion
    #[tokio::test]
    async fn test_unit_test_fails_empty_content() {
        let config = json!({
            "asserts": ["choices[0].message.content != \"\""]
        });
        let body = chat_body("");
        let result = gate("unit_test", config).evaluate(&body).await;
        assert_eq!(result.r_binary, 0);
        assert!(result.detail.contains("assert failed"), "Detail: {}", result.detail);
    }

    // ─── Meridian V2 candidate 1 — confidence calibration ─────────────────────

    #[test]
    fn calibrate_is_identity_at_temperature_one() {
        for raw in [0.1, 0.5, 0.73, 0.9, 0.99] {
            assert!((calibrate_confidence(raw, 1.0) - raw).abs() < 1e-9, "raw={raw}");
        }
    }

    #[test]
    fn calibrate_treats_disabled_temperature_as_identity() {
        // Non-positive / NaN temperature must never panic and must fall back to identity.
        assert_eq!(calibrate_confidence(0.8, 0.0), 0.8);
        assert_eq!(calibrate_confidence(0.8, -3.0), 0.8);
        assert_eq!(calibrate_confidence(0.8, f64::NAN), 0.8);
    }

    #[test]
    fn calibrate_keeps_half_as_the_fixed_point() {
        for t in [0.5, 1.0, 2.0, 5.0] {
            assert!((calibrate_confidence(0.5, t) - 0.5).abs() < 1e-9, "T={t}");
        }
    }

    #[test]
    fn calibrate_tempers_overconfidence_toward_half() {
        // T>1 pulls a high raw score down toward 0.5, but never past it.
        let c = calibrate_confidence(0.95, 3.0);
        assert!(c < 0.95 && c > 0.5, "got {c}");
        // ...and pulls a low raw score up toward 0.5.
        let c_low = calibrate_confidence(0.05, 3.0);
        assert!(c_low > 0.05 && c_low < 0.5, "got {c_low}");
    }

    #[test]
    fn calibrate_is_monotonic_in_raw() {
        let t = 2.5;
        let mut prev = calibrate_confidence(0.0, t);
        for step in 1..=100 {
            let cur = calibrate_confidence(step as f64 / 100.0, t);
            assert!(cur >= prev, "not monotonic at step {step}: {cur} < {prev}");
            prev = cur;
        }
    }

    #[test]
    fn calibrate_handles_the_zero_and_one_asymptotes_without_nan() {
        assert!(calibrate_confidence(0.0, 2.0).is_finite());
        assert!(calibrate_confidence(1.0, 2.0).is_finite());
    }

    // Test — a spuriously overconfident raw score that passes at T=1 is caught once tempered.
    #[tokio::test]
    async fn test_confidence_calibration_can_flip_a_spurious_pass_to_fail() {
        let body = json!({"confidence": 0.9}).to_string();

        // T=1 (default behavior): 0.9 >= 0.8 → pass.
        let uncalibrated = gate("confidence", json!({"threshold": 0.8})).evaluate(&body).await;
        assert_eq!(uncalibrated.r_binary, 1);

        // T=3 tempers 0.9 below 0.8 → fail, catching the overconfidence.
        let calibrated = gate("confidence", json!({"threshold": 0.8, "temperature": 3.0}))
            .evaluate(&body)
            .await;
        assert_eq!(calibrated.r_binary, 0);
        assert!(calibrated.detail.contains("calibrated confidence"), "detail: {}", calibrated.detail);
    }

    // Test — omitting temperature preserves the exact pre-V2 behavior (backward compatibility).
    #[tokio::test]
    async fn test_confidence_without_temperature_is_backward_compatible() {
        let body = json!({"confidence": 0.85}).to_string();
        let result = gate("confidence", json!({"threshold": 0.8})).evaluate(&body).await;
        assert_eq!(result.r_binary, 1);
    }
}
