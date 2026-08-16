//! Redis-backed JWT revocation list (OD-006 — logout).
//!
//! GaussMeridian's JWTs are stateless: there was no server-side way to invalidate one before its
//! natural expiry, so `POST /v1/auth/logout` didn't exist. This adds a minimal revocation list:
//! on logout, hash the raw token and store the hash in Redis with a TTL equal to the token's own
//! remaining lifetime — no longer than that, since an expired JWT is already invalid on its own.
//! `auth_middleware_with_state` checks this list after JWT signature validation succeeds.
//!
//! No JWT schema change (no `jti` claim needed) — the token hash itself is the revocation key.

use crate::error::AuthError;
use deadpool_redis::{Config as DeadpoolConfig, Pool, Runtime};
use sha2::{Digest, Sha256};

/// Hash a raw JWT for use as a revocation-list key. Never store the raw token itself.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Redis-backed set of revoked JWT hashes, each expiring with the token it represents.
pub struct TokenRevocationList {
    pool: Pool,
}

impl TokenRevocationList {
    /// Create a new revocation list backed by Redis. Verifies connectivity with PING.
    pub async fn new(redis_url: &str) -> Result<Self, AuthError> {
        let cfg = DeadpoolConfig::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| AuthError::InvalidConfig(format!("Redis pool creation failed: {}", e)))?;
        let mut conn = pool
            .get()
            .await
            .map_err(|e| AuthError::InvalidConfig(format!("Redis pool unavailable: {}", e)))?;
        redis::cmd("PING")
            .query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| AuthError::InvalidConfig(format!("Redis PING failed: {}", e)))?;
        Ok(Self { pool })
    }

    /// Revoke a token hash for `ttl_seconds` (the token's own remaining lifetime).
    /// A `ttl_seconds <= 0` means the token is already expired — a no-op, not an error.
    pub async fn revoke(&self, token_hash: &str, ttl_seconds: i64) -> Result<(), AuthError> {
        if ttl_seconds <= 0 {
            return Ok(());
        }
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| AuthError::Unavailable(format!("Redis pool unavailable: {}", e)))?;
        redis::cmd("SET")
            .arg(format!("gr:revoked:{}", token_hash))
            .arg(1)
            .arg("EX")
            .arg(ttl_seconds)
            .query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| AuthError::Unavailable(format!("Redis SET failed: {}", e)))?;
        Ok(())
    }

    /// Check whether a token hash has been revoked.
    pub async fn is_revoked(&self, token_hash: &str) -> Result<bool, AuthError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| AuthError::Unavailable(format!("Redis pool unavailable: {}", e)))?;
        let exists: bool = redis::cmd("EXISTS")
            .arg(format!("gr:revoked:{}", token_hash))
            .query_async(&mut *conn)
            .await
            .map_err(|e| AuthError::Unavailable(format!("Redis EXISTS failed: {}", e)))?;
        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_token_is_deterministic_and_distinct() {
        let h1 = hash_token("token-a");
        let h2 = hash_token("token-a");
        let h3 = hash_token("token-b");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        // SHA-256 hex digest length
        assert_eq!(h1.len(), 64);
    }
}
