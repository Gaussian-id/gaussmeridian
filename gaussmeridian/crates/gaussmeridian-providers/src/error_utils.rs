//! Error handling utilities for providers
//!
//! Provides secure error handling that prevents information leakage
//! and sanitizes error messages.

use gaussmeridian_models::ProviderError;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex to detect API keys in error messages
    /// Matches patterns like: "API key: sk-xxxx", "api_key=xxxx", "Bearer xxxx"
    static ref API_KEY_PATTERN: Regex = Regex::new(
        r"(?i)(api[\s_-]?key|bearer|token|secret|password|auth)[\s:=]+([a-zA-Z0-9_-]{20,})"
    ).unwrap();

    /// Shape-based fallback: catches known provider key prefixes wherever they appear in a
    /// message, independent of any preceding label. `API_KEY_PATTERN` above requires the secret
    /// to directly follow a label like "key:" or "token="; real upstream error prose ("Incorrect
    /// API key provided: sk-abc... You can find your key at...") often doesn't match that shape,
    /// so this catches the token by its own recognizable form instead.
    static ref KNOWN_KEY_PREFIX_PATTERN: Regex = Regex::new(
        r"\b(sk-ant-[A-Za-z0-9_-]{16,}|sk-[A-Za-z0-9_-]{16,}|AIza[A-Za-z0-9_-]{20,}|gsk_[A-Za-z0-9_-]{16,}|hf_[A-Za-z0-9_-]{16,}|xai-[A-Za-z0-9_-]{16,}|Bearer\s+[A-Za-z0-9._-]{20,}|\?key=[A-Za-z0-9_-]{16,})\b"
    ).unwrap();

    /// Regex to detect email addresses
    static ref EMAIL_PATTERN: Regex = Regex::new(
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
    ).unwrap();
}

/// Sanitize error messages to prevent information leakage
pub fn sanitize_error_message(message: &str) -> String {
    let mut sanitized = message.to_string();

    // Remove API keys and tokens that directly follow a labeled keyword
    sanitized = API_KEY_PATTERN
        .replace_all(&sanitized, "$1 [REDACTED]")
        .to_string();

    // Remove API keys by their own recognizable shape, regardless of surrounding prose
    sanitized = KNOWN_KEY_PREFIX_PATTERN
        .replace_all(&sanitized, "[REDACTED]")
        .to_string();

    // Remove email addresses
    sanitized = EMAIL_PATTERN
        .replace_all(&sanitized, "[REDACTED]")
        .to_string();

    // Limit message length to prevent excessive logging
    if sanitized.len() > 500 {
        sanitized.truncate(500);
        sanitized.push_str("... [truncated]");
    }

    sanitized
}

/// Convert HTTP status code to ProviderError with sanitized message
pub fn http_status_to_provider_error(
    status: reqwest::StatusCode,
    error_body: String,
) -> ProviderError {
    let sanitized_body = sanitize_error_message(&error_body);

    match status.as_u16() {
        401 => ProviderError::Authentication(format!("Unauthorized: {}", sanitized_body)),
        403 => ProviderError::Authentication(format!("Forbidden: {}", sanitized_body)),
        429 => ProviderError::RateLimit(format!("Rate limited: {}", sanitized_body)),
        400..=499 => ProviderError::BadRequest(format!("Bad request: {}", sanitized_body)),
        500..=599 => ProviderError::Internal(format!("Server error: {}", sanitized_body)),
        _ => {
            ProviderError::Unavailable(format!("Unexpected status {}: {}", status, sanitized_body))
        }
    }
}

/// Handle HTTP response errors safely
pub async fn handle_http_error(response: reqwest::Response) -> ProviderError {
    let status = response.status();
    let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| format!("HTTP {} - Unable to read error body", status));

    http_status_to_provider_error(status, error_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_api_key() {
        let message = "API key: sk-1234567890abcdefghijklmnopqrstuvwxyz";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("sk-1234567890"));
    }

    #[test]
    fn test_sanitize_key_in_unlabeled_prose() {
        // Realistic upstream error text with no "key:"/"key="-style label directly before the
        // token — the labeled-pattern regex alone misses this; the shape-based fallback catches it.
        let message =
            "Incorrect API key provided: sk-1234567890abcdefghijklmnop. You can find your key at https://example.com/keys";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("sk-1234567890"));
    }

    #[test]
    fn test_sanitize_query_param_key() {
        // Gemini-style auth: the key is a `?key=` query parameter, which surfaces in
        // `reqwest::Error`'s Display for pre-response network failures (DNS/TLS/timeout).
        let message = "Request failed: error sending request for url (https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key=AIzaSyDaGmWKa4JsXZ-HjGw7ISLan_giLwWXeAB): connection reset";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("AIzaSyDaGmWKa4JsXZ"));
    }

    #[test]
    fn test_sanitize_email() {
        let message = "Contact admin@example.com for support";
        let sanitized = sanitize_error_message(message);
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("admin@example.com"));
    }

    #[test]
    fn test_sanitize_long_message() {
        let message = "a".repeat(1000);
        let sanitized = sanitize_error_message(&message);
        assert!(sanitized.len() <= 520); // 500 + "... [truncated]"
        assert!(sanitized.contains("[truncated]"));
    }
}
