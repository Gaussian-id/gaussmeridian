//! JWT token handling

use crate::error::AuthError;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Option<String>,
    user_id: Option<String>,
    tenant_id: Option<String>,
    permissions: Option<Vec<String>>,
    exp: usize,
    iat: usize,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

/// JWT Manager
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    /// Default access-token lifetime in seconds, used when a caller does not pass an explicit
    /// `exp` claim. `new()` keeps this at 3600 (1 h) so pre-PRD-25 call sites are unaffected;
    /// `with_access_ttl` overrides it (prod default 900 s — see `app.rs`).
    access_ttl_secs: u64,
}

impl JwtManager {
    pub fn new(secret: &str) -> Self {
        let validation = Validation::new(Algorithm::HS256);
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
            access_ttl_secs: 3600,
        }
    }

    /// PRD-25 Phase 1 — override the default access-token TTL (seconds). Additive builder:
    /// leaves `new()`'s 3600 s default untouched, so the ~11 `JwtManager::new` call sites need
    /// no change. Only affects tokens minted without an explicit `exp` claim.
    pub fn with_access_ttl(mut self, secs: u64) -> Self {
        self.access_ttl_secs = secs;
        self
    }

    pub fn create_token(
        &self,
        claims_map: &HashMap<String, serde_json::Value>,
    ) -> Result<String, AuthError> {
        let now = chrono::Utc::now().timestamp() as usize;
        let exp = now + self.access_ttl_secs as usize; // configurable default (PRD-25); was 3600

        // Extract standard claims
        let sub = claims_map
            .get("sub")
            .or_else(|| claims_map.get("user_id"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let user_id = claims_map
            .get("user_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        let tenant_id = claims_map
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        let permissions = claims_map
            .get("permissions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        // Get expiration from claims or use default
        let exp = claims_map
            .get("exp")
            .and_then(|v| v.as_u64())
            .map(|e| e as usize)
            .unwrap_or(exp);

        // Store extra claims
        let mut extra = claims_map.clone();
        extra.remove("sub");
        extra.remove("user_id");
        extra.remove("tenant_id");
        extra.remove("permissions");
        extra.remove("exp");
        extra.remove("iat");

        let claims = Claims {
            sub,
            user_id,
            tenant_id,
            permissions,
            exp,
            iat: now,
            extra,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::Internal(format!("Failed to encode JWT: {}", e)))
    }

    pub fn validate_token(
        &self,
        token: &str,
    ) -> Result<HashMap<String, serde_json::Value>, AuthError> {
        let token_data =
            decode::<Claims>(token, &self.decoding_key, &self.validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::Expired,
                    jsonwebtoken::errors::ErrorKind::InvalidToken => AuthError::Invalid,
                    _ => AuthError::Invalid,
                }
            })?;

        let claims = token_data.claims;
        let mut result = HashMap::new();

        // Add standard claims
        if let Some(sub) = claims.sub {
            result.insert("sub".to_string(), serde_json::Value::String(sub));
        }
        if let Some(user_id) = claims.user_id {
            result.insert("user_id".to_string(), serde_json::Value::String(user_id));
        }
        if let Some(tenant_id) = claims.tenant_id {
            result.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id),
            );
        }
        if let Some(permissions) = claims.permissions {
            result.insert(
                "permissions".to_string(),
                serde_json::Value::Array(
                    permissions
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        // Add extra claims
        for (key, value) in claims.extra {
            result.insert(key, value);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    /// Decode a JWT's payload segment (no signature check) and return its `exp` claim.
    fn token_exp(token: &str) -> i64 {
        let payload_b64 = token.split('.').nth(1).expect("jwt has a payload segment");
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload_b64)
            .expect("payload decodes");
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("payload is json");
        v.get("exp").and_then(|e| e.as_i64()).expect("exp present")
    }

    #[test]
    fn new_defaults_access_ttl_to_3600() {
        let jwt = JwtManager::new("s");
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::Value::String("u1".to_string()));
        let now = chrono::Utc::now().timestamp();
        let token = jwt.create_token(&claims).expect("mint");
        let exp = token_exp(&token);
        // exp ≈ now + 3600 (allow a couple seconds of wall-clock drift during the test).
        assert!((exp - (now + 3600)).abs() <= 2, "default TTL must be 3600s; exp={exp}, now={now}");
    }

    #[test]
    fn with_access_ttl_overrides_the_default_exp() {
        let jwt = JwtManager::new("s").with_access_ttl(900);
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::Value::String("u1".to_string()));
        let now = chrono::Utc::now().timestamp();
        let token = jwt.create_token(&claims).expect("mint");
        let exp = token_exp(&token);
        assert!((exp - (now + 900)).abs() <= 2, "with_access_ttl(900) must set exp≈now+900; exp={exp}, now={now}");
    }

    #[test]
    fn explicit_exp_claim_still_wins_over_the_default() {
        // The BUG-05 regression relies on this: a far-past explicit `exp` must be honored so the
        // token reads as expired regardless of the configured default TTL.
        let jwt = JwtManager::new("s").with_access_ttl(900);
        let past = (chrono::Utc::now().timestamp() - 3600) as u64;
        let mut claims = HashMap::new();
        claims.insert("sub".to_string(), serde_json::Value::String("u1".to_string()));
        claims.insert("exp".to_string(), serde_json::Value::from(past));
        let token = jwt.create_token(&claims).expect("mint");
        assert_eq!(token_exp(&token), past as i64, "explicit exp claim must override the default");
    }
}
