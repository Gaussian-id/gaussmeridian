//! Security utilities for input validation, sanitization, and encryption
//!
//! This module provides comprehensive security utilities for:
//! - Input validation and sanitization
//! - SQL injection prevention
//! - XSS prevention
//! - SSRF prevention
//! - Audit logging
//! - Encryption utilities

use std::collections::HashMap;
// Tracing macros are used via tracing::error! and tracing::warn! macros

/// Input validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            is_valid: false,
            errors,
        }
    }
}

/// Input validator
pub struct InputValidator;

impl InputValidator {
    /// Validate and sanitize user input
    pub fn validate_input(input: &str, max_length: Option<usize>) -> ValidationResult {
        let mut errors = Vec::new();

        // Check for null bytes
        if input.contains('\0') {
            errors.push("Input contains null bytes".to_string());
        }

        // Check length
        if let Some(max) = max_length {
            if input.len() > max {
                errors.push(format!(
                    "Input exceeds maximum length of {} characters",
                    max
                ));
            }
        }

        // Check for control characters (except newline, tab, carriage return)
        if input
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t' && c != '\r')
        {
            errors.push("Input contains invalid control characters".to_string());
        }

        if errors.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(errors)
        }
    }

    /// Sanitize string to prevent XSS
    pub fn sanitize_for_xss(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
            .replace('/', "&#x2F;")
    }

    /// Validate URL to prevent SSRF.
    ///
    /// This is a literal-address and scheme check performed against the URL as written — it does
    /// NOT resolve hostnames, so a hostname that resolves to a private/internal address at
    /// connect time (DNS rebinding) is not caught here. Callers that fetch attacker-influenced
    /// URLs against untrusted infrastructure need connect-time IP validation on top of this, not
    /// instead of it; this check is the cheap, synchronous first line of defense against the
    /// common case of a literal private IP or `localhost` being supplied directly.
    pub fn validate_url_for_ssrf(url: &str) -> ValidationResult {
        let mut errors = Vec::new();

        // Parse URL
        if let Ok(parsed) = url::Url::parse(url) {
            // Check for private/localhost addresses. Use `Url::host()` (typed `url::Host`,
            // already IP-vs-domain classified by the parser) rather than `host_str()` + a manual
            // `str::parse::<IpAddr>()` — `host_str()` serializes an IPv6 host WITH its `[...]`
            // brackets (WHATWG URL host serialization), which fails to parse as a bare `IpAddr`
            // and would silently fall through to "not an IP, must be a hostname" for every IPv6
            // literal, `[::1]` included.
            if let Some(host) = parsed.host() {
                let is_private = match host {
                    url::Host::Ipv4(ip) => {
                        ip.is_loopback()
                            || ip.is_private()
                            || ip.is_link_local() // includes 169.254.169.254 cloud metadata
                            || ip.is_unspecified()
                    }
                    url::Host::Ipv6(ip) => {
                        ip.is_loopback()
                            || ip.is_unspecified()
                            || (ip.segments()[0] & 0xfe00) == 0xfc00 // fc00::/7 unique local
                            || (ip.segments()[0] & 0xffc0) == 0xfe80 // fe80::/10 link-local
                            || ip
                                .to_ipv4_mapped()
                                .is_some_and(|v4| v4.is_loopback() || v4.is_private() || v4.is_link_local())
                    }
                    // A domain name. `localhost` is the one worth a literal string check;
                    // anything else needs DNS resolution to classify, which this synchronous,
                    // string-only check deliberately does not do (see the doc comment above).
                    url::Host::Domain(domain) => domain.eq_ignore_ascii_case("localhost"),
                };
                if is_private {
                    errors.push(
                        "URL points to a private, loopback, or link-local address (SSRF prevention)"
                            .to_string(),
                    );
                }

                // Check for dangerous protocols
                match parsed.scheme() {
                    "http" | "https" => {}
                    _ => {
                        errors.push(format!("Unsupported URL scheme: {}", parsed.scheme()));
                    }
                }
            } else {
                errors.push("URL has no valid host".to_string());
            }
        } else {
            errors.push("Invalid URL format".to_string());
        }

        if errors.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(errors)
        }
    }

    /// Escape SQL string to prevent SQL injection
    pub fn escape_sql(input: &str) -> String {
        input
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            .replace('\0', "\\0")
    }

    /// Validate email format
    pub fn validate_email(email: &str) -> ValidationResult {
        let mut errors = Vec::new();

        if email.is_empty() {
            errors.push("Email cannot be empty".to_string());
        } else {
            // Basic email validation
            let parts: Vec<&str> = email.split('@').collect();
            if parts.len() != 2 {
                errors.push("Invalid email format".to_string());
            } else {
                let (local, domain) = (parts[0], parts[1]);
                if local.is_empty() || local.len() > 64 {
                    errors.push("Email local part is invalid".to_string());
                }
                if domain.is_empty() || !domain.contains('.') {
                    errors.push("Email domain is invalid".to_string());
                }
            }
        }

        if errors.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(errors)
        }
    }

    /// Validate API key format
    pub fn validate_api_key(key: &str) -> ValidationResult {
        let mut errors = Vec::new();

        if key.len() < 16 {
            errors.push("API key is too short (minimum 16 characters)".to_string());
        }

        if key.len() > 512 {
            errors.push("API key is too long (maximum 512 characters)".to_string());
        }

        // API keys should be base64-like or hex-like
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '=')
        {
            errors.push("API key contains invalid characters".to_string());
        }

        if errors.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(errors)
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub action: String,
    pub resource: String,
    pub status: AuditStatus,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuditStatus {
    Success,
    Failure,
    Denied,
}

/// Audit logger
pub struct AuditLogger {
    enabled: bool,
}

impl AuditLogger {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn log(&self, entry: AuditLogEntry) {
        if !self.enabled {
            return;
        }

        // Log to structured logging
        match entry.status {
            AuditStatus::Success => {
                tracing::info!(
                    timestamp = %entry.timestamp,
                    user_id = ?entry.user_id,
                    action = %entry.action,
                    resource = %entry.resource,
                    ip_address = ?entry.ip_address,
                    "Audit log: {} - {}", entry.action, entry.resource
                );
            }
            AuditStatus::Failure => {
                tracing::warn!(
                    timestamp = %entry.timestamp,
                    user_id = ?entry.user_id,
                    action = %entry.action,
                    resource = %entry.resource,
                    ip_address = ?entry.ip_address,
                    "Audit log (failure): {} - {}", entry.action, entry.resource
                );
            }
            AuditStatus::Denied => {
                tracing::warn!(
                    timestamp = %entry.timestamp,
                    user_id = ?entry.user_id,
                    action = %entry.action,
                    resource = %entry.resource,
                    ip_address = ?entry.ip_address,
                    "Audit log (denied): {} - {}", entry.action, entry.resource
                );
            }
        }
    }

    pub fn log_access(
        &self,
        user_id: Option<String>,
        resource: &str,
        action: &str,
        status: AuditStatus,
        ip_address: Option<String>,
    ) {
        let entry = AuditLogEntry {
            timestamp: chrono::Utc::now(),
            user_id,
            action: action.to_string(),
            resource: resource.to_string(),
            status,
            ip_address,
            user_agent: None,
            details: HashMap::new(),
        };
        self.log(entry);
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(true)
    }
}

/// Encryption utilities
pub struct EncryptionUtils;

impl EncryptionUtils {
    /// Hash password using SHA256 (for non-critical hashing)
    /// Note: For password storage, use bcrypt or argon2 in production
    pub fn hash_password(password: &str) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Generate secure random token
    pub fn generate_token(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Hash a secret using **Argon2id** (PRD-21 Wave B / DR-010 D4, ratified 2026-07-16 —
    /// "same bar as `users.password_hash`"). **Discovery (superseded):** at the time this
    /// function was written, `users.password_hash` was hashed with `bcrypt`
    /// (`gaussmeridian-auth::AuthManager::hash_password`), not Argon2id — a deliberate deviation
    /// from "reuse the existing hasher" to honor the DR-010 ratification text, which names the
    /// algorithm explicitly. **Update:** Actual has since ruled that `users.password_hash`
    /// migrates to Argon2id too — `AuthManager::hash_password`/`verify_password` now use the
    /// same Argon2id defaults as this function (with verify-time detection of legacy
    /// bcrypt/SHA-256 rows and rehash-on-login), closing the gap this comment originally
    /// flagged. Mirrors `gaussmoa::security::KeyManager::hash_password`'s use of
    /// `Argon2::default()` (that crate's default IS Argon2id, version 0x13) — reimplemented
    /// here, standalone, rather than reused, because `KeyManager` requires an AES key file this
    /// call has no reason to need. Used for `project.access_secret` ("project password"); kept
    /// as its own function (not merged with `AuthManager::hash_password`) since the two guard
    /// different resources and `AuthManager` additionally owns legacy-hash detection and the
    /// rehash-on-login migration path.
    pub fn hash_secret_argon2id(secret: &str) -> Result<String, String> {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        use argon2::Argon2;

        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(secret.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| format!("argon2id hash failed: {}", e))
    }

    /// Verify a secret against an Argon2id hash produced by [`hash_secret_argon2id`]. Returns
    /// `Ok(false)` (not `Err`) for a non-matching secret — only a malformed/corrupt stored
    /// hash string is an `Err`, mirroring `gaussmoa::security::KeyManager::verify_password`.
    pub fn verify_secret_argon2id(secret: &str, hash: &str) -> Result<bool, String> {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};
        use argon2::Argon2;

        let parsed_hash = PasswordHash::new(hash).map_err(|e| format!("invalid argon2id hash: {}", e))?;
        Ok(Argon2::default().verify_password(secret.as_bytes(), &parsed_hash).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_validation() {
        let result = InputValidator::validate_input("test", Some(10));
        assert!(result.is_valid);

        let result = InputValidator::validate_input("test", Some(2));
        assert!(!result.is_valid);
    }

    #[test]
    fn test_xss_sanitization() {
        let input = "<script>alert('XSS')</script>";
        let sanitized = InputValidator::sanitize_for_xss(input);
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }

    #[test]
    fn test_ssrf_validation() {
        let result = InputValidator::validate_url_for_ssrf("http://localhost/test");
        assert!(!result.is_valid);

        let result = InputValidator::validate_url_for_ssrf("https://example.com/test");
        assert!(result.is_valid);
    }

    #[test]
    fn test_ssrf_validation_rejects_private_ip_literals() {
        for url in [
            "http://127.0.0.1/",
            "http://10.0.0.5/",
            "http://192.168.1.1/",
            "http://172.16.0.1/",
            "http://0.0.0.0/",
        ] {
            let result = InputValidator::validate_url_for_ssrf(url);
            assert!(!result.is_valid, "{url} should be rejected");
        }
    }

    #[test]
    fn test_ssrf_validation_rejects_cloud_metadata_and_link_local() {
        // 169.254.169.254 is the AWS/GCP/Azure instance-metadata address; the whole
        // 169.254.0.0/16 link-local range was previously unchecked entirely.
        let result =
            InputValidator::validate_url_for_ssrf("http://169.254.169.254/latest/meta-data/");
        assert!(!result.is_valid);
        let result = InputValidator::validate_url_for_ssrf("http://169.254.1.1/");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_ssrf_validation_rejects_ipv6_private_ranges() {
        for url in [
            "http://[::1]/",
            "http://[fc00::1]/",          // unique local
            "http://[fe80::1]/",          // link-local
            "http://[::ffff:127.0.0.1]/", // IPv4-mapped loopback
        ] {
            let result = InputValidator::validate_url_for_ssrf(url);
            assert!(!result.is_valid, "{url} should be rejected");
        }

        let result = InputValidator::validate_url_for_ssrf("http://[2001:4860:4860::8888]/"); // public
        assert!(result.is_valid);
    }

    #[test]
    fn test_email_validation() {
        let result = InputValidator::validate_email("test@example.com");
        assert!(result.is_valid);

        let result = InputValidator::validate_email("invalid-email");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_api_key_validation() {
        let result = InputValidator::validate_api_key("sk_test_1234567890abcdef");
        assert!(result.is_valid);

        let result = InputValidator::validate_api_key("short");
        assert!(!result.is_valid);
    }

    // ─── PRD-21 Wave B / DR-010 D4 — Argon2id project-password hashing ─────────────────────

    #[test]
    fn argon2id_hash_then_verify_round_trips_true() {
        let secret = "correct horse battery staple";
        let hash = EncryptionUtils::hash_secret_argon2id(secret).expect("hash succeeds");
        assert!(EncryptionUtils::verify_secret_argon2id(secret, &hash).expect("verify runs"));
    }

    #[test]
    fn argon2id_verify_rejects_the_wrong_secret() {
        let hash = EncryptionUtils::hash_secret_argon2id("correct horse battery staple").expect("hash succeeds");
        assert!(!EncryptionUtils::verify_secret_argon2id("wrong secret", &hash).expect("verify runs"));
    }

    #[test]
    fn argon2id_hash_never_equals_the_plaintext() {
        let secret = "correct horse battery staple";
        let hash = EncryptionUtils::hash_secret_argon2id(secret).expect("hash succeeds");
        assert_ne!(hash, secret, "the stored hash must never equal the plaintext secret");
        assert!(hash.starts_with("$argon2id$"), "must use the Argon2id variant, not argon2i/argon2d");
    }

    #[test]
    fn argon2id_hash_is_salted_so_two_hashes_of_the_same_secret_differ() {
        let secret = "correct horse battery staple";
        let hash1 = EncryptionUtils::hash_secret_argon2id(secret).expect("hash succeeds");
        let hash2 = EncryptionUtils::hash_secret_argon2id(secret).expect("hash succeeds");
        assert_ne!(hash1, hash2, "each hash must use a fresh random salt");
        assert!(EncryptionUtils::verify_secret_argon2id(secret, &hash1).expect("verify runs"));
        assert!(EncryptionUtils::verify_secret_argon2id(secret, &hash2).expect("verify runs"));
    }
}
