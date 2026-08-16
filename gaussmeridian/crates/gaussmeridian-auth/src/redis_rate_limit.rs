//! Redis-backed sliding window rate limiter.
//!
//! Implements an atomic sliding window using a Redis sorted set + Lua script.
//! Atomic Lua execution prevents TOCTOU races in multi-instance deployments.
//! Falls back gracefully to the in-memory `RateLimiter` when Redis is unavailable
//! (controlled by the caller via `AppState.redis_rate_limiter: Option<Arc<RedisRateLimiter>>`).
//!
//! ## Algorithm
//! Key: `gr:rl:{client_id}` (Redis sorted set, member scored by timestamp ms)
//! Each request: ZREMRANGEBYSCORE (expire old entries) → ZCARD (count) → ZADD if allowed
//! The Lua script executes all three commands atomically on the Redis server.
//!
//! Default limit: 1 000 requests per 60-second window per client key.
//! Configurable at construction time; per-project overrides are planned for M3.

use crate::error::AuthError;
use deadpool_redis::{Config as DeadpoolConfig, Pool, Runtime};
use redis::Script;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Lua script: atomic sliding-window rate limit check + increment.
///
/// KEYS[1] = rate-limit key (e.g. "gr:rl:my-api-key")
/// ARGV[1] = current timestamp in milliseconds
/// ARGV[2] = window size in milliseconds (default 60 000)
/// ARGV[3] = request limit per window
///
/// Returns 1 if the request is allowed, 0 if rate limited.
const SLIDING_WINDOW_LUA: &str = r#"
local key = KEYS[1]
local now = tonumber(ARGV[1])
local window = tonumber(ARGV[2])
local limit = tonumber(ARGV[3])
local window_start = now - window
redis.call('ZREMRANGEBYSCORE', key, 0, window_start)
local count = redis.call('ZCARD', key)
if count < limit then
    redis.call('ZADD', key, now, now .. ':' .. math.random(1000000))
    redis.call('PEXPIRE', key, window + 1000)
    return 1
end
return 0
"#;

/// Sanitize a `client_id` coming from untrusted headers before embedding it in a Redis key.
///
/// Allows only ASCII alphanumerics, `.`, `-`, and `_`. Truncates to 256 characters.
/// This prevents crafted values from corrupting the Redis keyspace.
fn sanitize_client_id(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(256)
        .collect()
}

/// Redis-backed sliding-window rate limiter.
///
/// Uses a `deadpool-redis` connection pool for async, pooled Redis access.
/// Each instance is shared via `Arc` in `AppState`.
pub struct RedisRateLimiter {
    pool: Pool,
    requests_per_window: u32,
    window_ms: u64,
}

impl RedisRateLimiter {
    /// Create a new rate limiter backed by Redis.
    ///
    /// Verifies the connection with PING before returning.
    /// Returns `Err` if the connection cannot be established.
    pub async fn new(redis_url: &str, requests_per_minute: u32) -> Result<Self, AuthError> {
        let cfg = DeadpoolConfig::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| AuthError::InvalidConfig(format!("Redis pool creation failed: {}", e)))?;
        // Verify connectivity
        let mut conn = pool.get().await.map_err(|e| {
            AuthError::InvalidConfig(format!("Redis pool unavailable: {}", e))
        })?;
        redis::cmd("PING")
            .query_async::<_, ()>(&mut *conn)
            .await
            .map_err(|e| AuthError::InvalidConfig(format!("Redis PING failed: {}", e)))?;
        Ok(Self {
            pool,
            requests_per_window: requests_per_minute,
            window_ms: 60_000,
        })
    }

    /// Check rate limit for a client key.
    ///
    /// Returns `Ok(())` if the request is within the limit.
    /// Returns `Err(AuthError::RateLimitExceeded)` if the limit is exceeded.
    /// Returns `Err(AuthError::Unavailable)` on transient Redis errors (pool exhaustion,
    /// Lua script errors) — callers should fall back to the in-memory limiter.
    pub async fn check(&self, client_id: &str) -> Result<(), AuthError> {
        let safe_id = sanitize_client_id(client_id);
        let key = format!("gr:rl:{}", safe_id);
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);

        let mut conn = self.pool.get().await.map_err(|e| {
            warn!(client_id = %client_id, error = %e, "Redis pool exhausted — falling back to in-memory");
            AuthError::Unavailable(format!("Redis pool exhausted: {}", e))
        })?;

        let result: i64 = Script::new(SLIDING_WINDOW_LUA)
            .key(&key)
            .arg(now_ms)
            .arg(self.window_ms)
            .arg(self.requests_per_window)
            .invoke_async(&mut *conn)
            .await
            .map_err(|e| {
                warn!(client_id = %client_id, error = %e, "Redis Lua script error — falling back to in-memory");
                AuthError::Unavailable(format!("Redis script error: {}", e))
            })?;

        if result == 1 {
            debug!(client_id = %client_id, "Redis rate limit: allowed");
            Ok(())
        } else {
            warn!(client_id = %client_id, "Redis rate limit: exceeded");
            Err(AuthError::RateLimitExceeded)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests require a running Redis instance at localhost:6379.
    // Run with: cargo test -p gaussmeridian-auth redis_rate -- --nocapture --include-ignored
    // Skipped by default to avoid CI failures on machines without Redis.

    #[tokio::test]
    #[ignore = "requires Redis at localhost:6379"]
    async fn test_redis_allows_within_limit() {
        let limiter = RedisRateLimiter::new("redis://localhost:6379", 10)
            .await
            .expect("Redis not available");
        for _ in 0..5 {
            assert!(limiter.check("test_allows_within_limit").await.is_ok());
        }
    }

    #[tokio::test]
    #[ignore = "requires Redis at localhost:6379"]
    async fn test_redis_blocks_over_limit() {
        let limiter = RedisRateLimiter::new("redis://localhost:6379", 3)
            .await
            .expect("Redis not available");
        assert!(limiter.check("test_blocks_over_limit").await.is_ok());
        assert!(limiter.check("test_blocks_over_limit").await.is_ok());
        assert!(limiter.check("test_blocks_over_limit").await.is_ok());
        assert!(limiter.check("test_blocks_over_limit").await.is_err());
    }

    #[tokio::test]
    #[ignore = "requires Redis at localhost:6379"]
    async fn test_redis_separate_keys_independent() {
        let limiter = RedisRateLimiter::new("redis://localhost:6379", 2)
            .await
            .expect("Redis not available");
        assert!(limiter.check("test_sep_key_a").await.is_ok());
        assert!(limiter.check("test_sep_key_a").await.is_ok());
        assert!(limiter.check("test_sep_key_a").await.is_err()); // key_a exhausted
        assert!(limiter.check("test_sep_key_b").await.is_ok());  // key_b independent
    }
}
