//! Distributed rate limiter using SurrealDB
//!
//! Provides distributed rate limiting across multiple instances using SurrealDB
//! as a shared state backend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};

#[cfg(feature = "db")]
use gaussmeridian_db::{client::DatabaseClient, error::DatabaseError};

/// Rate limit entry in SurrealDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEntry {
    pub id: Option<String>,
    pub key: String,
    pub window_start: DateTime<Utc>,
    pub request_count: u32,
    pub token_count: u32,
    pub last_updated: DateTime<Utc>,
}

/// Distributed rate limiter configuration
#[derive(Debug, Clone)]
pub struct DistributedRateLimiterConfig {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub window_duration_secs: u64,
}

impl Default for DistributedRateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 1000,
            tokens_per_minute: 100000,
            window_duration_secs: 60,
        }
    }
}

/// Distributed rate limiter using SurrealDB
#[cfg(feature = "db")]
pub struct DistributedRateLimiter {
    db_client: DatabaseClient,
    config: DistributedRateLimiterConfig,
}

#[cfg(feature = "db")]
impl DistributedRateLimiter {
    pub fn new(db_client: DatabaseClient, config: DistributedRateLimiterConfig) -> Self {
        Self { db_client, config }
    }

    pub fn with_default_config(db_client: DatabaseClient) -> Self {
        Self::new(db_client, DistributedRateLimiterConfig::default())
    }

    /// Check if request is allowed under rate limit
    pub async fn check_rate_limit(
        &self,
        key: &str,
        tokens: u32,
    ) -> Result<RateLimitCheckResult, DatabaseError> {
        let now = Utc::now();
        let window_start =
            now - chrono::Duration::seconds(self.config.window_duration_secs as i64);

        // Escape key for SQL
        let escaped_key = key.replace("'", "''");

        // Get or create rate limit entry for this key
        let query = format!(
            "SELECT * FROM rate_limits WHERE key = '{}' AND window_start >= time::unix({}) LIMIT 1",
            escaped_key,
            window_start.timestamp()
        );

        let mut response = self.db_client.query(&query).await?;
        let entries: Option<Vec<RateLimitEntry>> = response.take(0)?;

        let (current_requests, current_tokens) = if let Some(entries) = entries {
            if let Some(entry) = entries.first() {
                (entry.request_count, entry.token_count)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        // Check limits
        let requests_allowed = current_requests < self.config.requests_per_minute;
        let tokens_allowed = current_tokens + tokens <= self.config.tokens_per_minute;
        let allowed = requests_allowed && tokens_allowed;

        if allowed {
            // Update or create entry
            let update_query = if current_requests > 0 {
                format!(
                    "UPDATE rate_limits SET request_count = {}, token_count = {}, last_updated = time::now() \
                     WHERE key = '{}' AND window_start >= time::unix({})",
                    current_requests + 1,
                    current_tokens + tokens,
                    escaped_key,
                    window_start.timestamp()
                )
            } else {
                // Create new entry
                format!(
                    "CREATE rate_limits SET key = '{}', window_start = time::now(), \
                     request_count = 1, token_count = {}, last_updated = time::now()",
                    escaped_key, tokens
                )
            };

            if let Err(e) = self.db_client.query(&update_query).await {
                error!("Failed to update rate limit entry: {}", e);
                // Don't fail the request, just log the error
                warn!("Rate limit tracking failed, allowing request");
            } else {
                debug!("Rate limit updated for key: {}", key);
            }
        }

        let remaining_requests = self
            .config
            .requests_per_minute
            .saturating_sub(current_requests);
        let remaining_tokens = self.config.tokens_per_minute.saturating_sub(current_tokens);

        Ok(RateLimitCheckResult {
            allowed,
            remaining_requests: if allowed {
                remaining_requests.saturating_sub(1)
            } else {
                remaining_requests
            },
            remaining_tokens: if allowed {
                remaining_tokens.saturating_sub(tokens)
            } else {
                remaining_tokens
            },
            reset_after_seconds: self.config.window_duration_secs,
        })
    }

    /// Get current rate limit status without updating
    pub async fn get_status(&self, key: &str) -> Result<RateLimitCheckResult, DatabaseError> {
        let now = Utc::now();
        let window_start =
            now - chrono::Duration::seconds(self.config.window_duration_secs as i64);

        let escaped_key = key.replace("'", "''");

        let query = format!(
            "SELECT * FROM rate_limits WHERE key = '{}' AND window_start >= time::unix({}) LIMIT 1",
            escaped_key,
            window_start.timestamp()
        );

        let mut response = self.db_client.query(&query).await?;
        let entries: Option<Vec<RateLimitEntry>> = response.take(0)?;

        let (current_requests, current_tokens) = if let Some(entries) = entries {
            if let Some(entry) = entries.first() {
                (entry.request_count, entry.token_count)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        let remaining_requests = self
            .config
            .requests_per_minute
            .saturating_sub(current_requests);
        let remaining_tokens = self.config.tokens_per_minute.saturating_sub(current_tokens);

        Ok(RateLimitCheckResult {
            allowed: true, // Status check doesn't block
            remaining_requests,
            remaining_tokens,
            reset_after_seconds: self.config.window_duration_secs,
        })
    }

    /// Clear old rate limit entries (cleanup job)
    pub async fn cleanup_old_entries(&self) -> Result<(), DatabaseError> {
        let cutoff = Utc::now()
            - chrono::Duration::seconds((self.config.window_duration_secs * 2) as i64);

        let query = format!(
            "DELETE FROM rate_limits WHERE window_start < time::unix({})",
            cutoff.timestamp()
        );

        self.db_client.query(&query).await?;
        debug!("Cleaned up old rate limit entries");

        Ok(())
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitCheckResult {
    pub allowed: bool,
    pub remaining_requests: u32,
    pub remaining_tokens: u32,
    pub reset_after_seconds: u64,
}

/// Rate limit headers for HTTP responses
#[derive(Debug, Clone)]
pub struct RateLimitHeaders {
    pub limit: u32,
    pub remaining: u32,
    pub reset: u64,
}

impl From<&RateLimitCheckResult> for RateLimitHeaders {
    fn from(result: &RateLimitCheckResult) -> Self {
        Self {
            limit: result.remaining_requests + if result.allowed { 1 } else { 0 },
            remaining: result.remaining_requests,
            reset: result.reset_after_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_check_result() {
        let result = RateLimitCheckResult {
            allowed: true,
            remaining_requests: 99,
            remaining_tokens: 9900,
            reset_after_seconds: 60,
        };

        assert!(result.allowed);
        assert_eq!(result.remaining_requests, 99);
        assert_eq!(result.remaining_tokens, 9900);
    }

    #[test]
    fn test_rate_limit_headers_from_result() {
        let result = RateLimitCheckResult {
            allowed: true,
            remaining_requests: 99,
            remaining_tokens: 9900,
            reset_after_seconds: 60,
        };

        let headers = RateLimitHeaders::from(&result);
        assert_eq!(headers.limit, 100);
        assert_eq!(headers.remaining, 99);
        assert_eq!(headers.reset, 60);
    }
}

