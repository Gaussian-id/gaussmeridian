//! Rate limiting functionality
//!
//! Provides sliding window rate limiting for API requests and tokens.
//! Uses async-compatible data structures for optimal performance in async contexts.

use crate::error::AuthError;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub current_requests: u32,
    pub current_tokens: u32,
    pub reset_at: chrono::DateTime<chrono::Utc>,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            tokens_per_minute: 100000,
            current_requests: 0,
            current_tokens: 0,
            reset_at: chrono::Utc::now() + chrono::Duration::minutes(1),
        }
    }
}

/// Rate limit state for a single key
struct RateLimitState {
    requests: RwLock<VecDeque<Instant>>,
    tokens: RwLock<VecDeque<(Instant, u32)>>,
    total_requests: AtomicU64,
    total_tokens: AtomicU64,
    total_rejected: AtomicU64,
}

impl RateLimitState {
    fn new() -> Self {
        Self {
            requests: RwLock::new(VecDeque::new()),
            tokens: RwLock::new(VecDeque::new()),
            total_requests: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            total_rejected: AtomicU64::new(0),
        }
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining_requests: u32,
    pub remaining_tokens: u32,
    pub reset_at: Instant,
    pub retry_after: Option<Duration>,
}

/// Rate limit configuration per key
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            tokens_per_minute: 100000,
            window_size: Duration::from_secs(60),
        }
    }
}

/// Rate Limiter using sliding window algorithm
///
/// This implementation uses DashMap for concurrent access and tokio::sync::RwLock
/// for async-compatible locking within each rate limit state.
pub struct RateLimiter {
    /// Per-key rate limit configurations
    configs: DashMap<String, RateLimitConfig>,
    /// Per-key rate limit states
    states: DashMap<String, RateLimitState>,
    /// Default configuration for unconfigured keys
    default_config: RateLimitConfig,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    /// Create a new rate limiter with default configuration
    pub fn new() -> Self {
        Self {
            configs: DashMap::new(),
            states: DashMap::new(),
            default_config: RateLimitConfig::default(),
        }
    }

    /// Create with custom default configuration
    pub fn with_default_config(config: RateLimitConfig) -> Self {
        Self {
            configs: DashMap::new(),
            states: DashMap::new(),
            default_config: config,
        }
    }

    /// Set rate limit configuration for a specific key
    pub fn set_config(&self, key: &str, config: RateLimitConfig) {
        self.configs.insert(key.to_string(), config);
    }

    /// Get configuration for a key (or default)
    fn get_config(&self, key: &str) -> RateLimitConfig {
        self.configs
            .get(key)
            .map(|c| c.clone())
            .unwrap_or_else(|| self.default_config.clone())
    }

    /// Get or create state for a key
    fn get_or_create_state(&self, key: &str) -> dashmap::mapref::one::Ref<'_, String, RateLimitState> {
        if !self.states.contains_key(key) {
            self.states.insert(key.to_string(), RateLimitState::new());
        }
        self.states.get(key).unwrap()
    }

    /// Check rate limit for a key with token count
    pub async fn check_rate_limit(&self, key: &str, request_tokens: u32) -> Result<RateLimitResult, AuthError> {
        let config = self.get_config(key);
        let state = self.get_or_create_state(key);

        let now = Instant::now();
        let window_start = now - config.window_size;

        // Clean up and count requests
        let current_requests = {
            let mut requests = state.requests.write().await;
            
            // Remove expired entries
            while let Some(front) = requests.front() {
                if *front < window_start {
                    requests.pop_front();
                } else {
                    break;
                }
            }
            
            requests.len() as u32
        };

        // Clean up and count tokens
        let current_tokens = {
            let mut tokens = state.tokens.write().await;
            
            // Remove expired entries
            while let Some(front) = tokens.front() {
                if front.0 < window_start {
                    tokens.pop_front();
                } else {
                    break;
                }
            }
            
            tokens.iter().map(|(_, t)| t).sum::<u32>()
        };

        // Check limits
        let requests_allowed = current_requests < config.requests_per_minute;
        let tokens_allowed = (current_tokens + request_tokens) <= config.tokens_per_minute;
        let allowed = requests_allowed && tokens_allowed;

        // Calculate reset time
        let reset_at = now + config.window_size;
        let retry_after = if !allowed {
            // Find the oldest entry that would need to expire
            let requests = state.requests.read().await;
            requests.front().map(|oldest| {
                let expiry = *oldest + config.window_size;
                if expiry > now {
                    expiry - now
                } else {
                    Duration::ZERO
                }
            })
        } else {
            None
        };

        if allowed {
            // Record the request
            {
                let mut requests = state.requests.write().await;
                requests.push_back(now);
            }
            {
                let mut tokens = state.tokens.write().await;
                tokens.push_back((now, request_tokens));
            }
            
            state.total_requests.fetch_add(1, Ordering::Relaxed);
            state.total_tokens.fetch_add(request_tokens as u64, Ordering::Relaxed);
            
            debug!(
                "Rate limit check passed for {}: {}/{} requests, {}/{} tokens",
                key,
                current_requests + 1,
                config.requests_per_minute,
                current_tokens + request_tokens,
                config.tokens_per_minute
            );
        } else {
            state.total_rejected.fetch_add(1, Ordering::Relaxed);
            
            warn!(
                "Rate limit exceeded for {}: {}/{} requests, {}/{} tokens",
                key, current_requests, config.requests_per_minute, current_tokens, config.tokens_per_minute
            );
        }

        Ok(RateLimitResult {
            allowed,
            remaining_requests: config.requests_per_minute.saturating_sub(current_requests + if allowed { 1 } else { 0 }),
            remaining_tokens: config.tokens_per_minute.saturating_sub(current_tokens + if allowed { request_tokens } else { 0 }),
            reset_at,
            retry_after,
        })
    }

    /// Simple rate limit check (returns error if exceeded)
    pub async fn check(&self, key: &str, tokens: u32) -> Result<(), AuthError> {
        let result = self.check_rate_limit(key, tokens).await?;
        if result.allowed {
            Ok(())
        } else {
            Err(AuthError::RateLimitExceeded)
        }
    }

    /// Check if rate limit is exceeded without consuming a request
    pub async fn is_limited(&self, key: &str) -> bool {
        let config = self.get_config(key);
        
        if let Some(state) = self.states.get(key) {
            let now = Instant::now();
            let window_start = now - config.window_size;

            let requests = state.requests.read().await;
            let current_requests = requests.iter().filter(|t| **t >= window_start).count() as u32;

            current_requests >= config.requests_per_minute
        } else {
            false
        }
    }

    /// Get current rate limit status for a key
    pub async fn get_status(&self, key: &str) -> RateLimitStatus {
        let config = self.get_config(key);
        
        if let Some(state) = self.states.get(key) {
            let now = Instant::now();
            let window_start = now - config.window_size;

            let requests = state.requests.read().await;
            let current_requests = requests.iter().filter(|t| **t >= window_start).count() as u32;
            drop(requests);

            let tokens = state.tokens.read().await;
            let current_tokens: u32 = tokens
                .iter()
                .filter(|(t, _)| *t >= window_start)
                .map(|(_, tok)| tok)
                .sum();

            RateLimitStatus {
                requests_used: current_requests,
                requests_limit: config.requests_per_minute,
                tokens_used: current_tokens,
                tokens_limit: config.tokens_per_minute,
                total_requests: state.total_requests.load(Ordering::Relaxed),
                total_tokens: state.total_tokens.load(Ordering::Relaxed),
                total_rejected: state.total_rejected.load(Ordering::Relaxed),
                reset_at: now + config.window_size,
            }
        } else {
            RateLimitStatus {
                requests_used: 0,
                requests_limit: config.requests_per_minute,
                tokens_used: 0,
                tokens_limit: config.tokens_per_minute,
                total_requests: 0,
                total_tokens: 0,
                total_rejected: 0,
                reset_at: Instant::now() + config.window_size,
            }
        }
    }

    /// Clear rate limit state for a key
    pub async fn clear(&self, key: &str) {
        self.states.remove(key);
    }

    /// Clear all rate limit states
    pub async fn clear_all(&self) {
        self.states.clear();
    }

    /// Number of distinct keys currently tracked. Exposed so a caller can spawn periodic
    /// `evict_stale` maintenance and observe whether it's keeping the map bounded.
    pub fn tracked_key_count(&self) -> usize {
        self.states.len()
    }

    /// Remove every tracked key whose most recent recorded activity has already fallen outside
    /// its own sliding window (or that has no activity at all). `DashMap` never shrinks on its
    /// own, and a key is created for any client identifier seen (see
    /// `RateLimiter::check`/`get_or_create_state`) — an attacker who can cause many distinct
    /// identifiers to be checked (e.g. by rotating an untrusted caller-supplied header) can
    /// otherwise grow this map without bound. Checking "is the deque empty" instead of "is the
    /// newest entry still within the window" would miss exactly that attack: a key checked only
    /// once keeps a single entry sitting in its deque forever, because the lazy expiry in
    /// `check_rate_limit` only runs the next time that SAME key is checked — which, for a
    /// rotate-once, never-again identifier, is never. A key whose window has fully elapsed is
    /// safe to drop: the next request for it just recreates a fresh empty state, observably
    /// identical to reusing the one being evicted. Intended to be called periodically (e.g. every
    /// few minutes) from a background task, not on the request path.
    pub async fn evict_stale(&self) -> usize {
        // `try_read` (never `.await`) deliberately: holding a DashMap shard's internal lock
        // across an awaited inner lock would stall unrelated keys hashing to the same shard for
        // the duration. A key whose inner lock is momentarily contended is by definition not
        // idle right now, so skipping it this pass (it'll be reconsidered next pass) is correct,
        // not just an optimization.
        let now = Instant::now();
        let stale_keys: Vec<String> = self
            .states
            .iter()
            .filter_map(|entry| {
                let window = self.get_config(entry.key()).window_size;
                let window_start = now.checked_sub(window).unwrap_or(now);

                let requests = entry.value().requests.try_read().ok()?;
                let requests_stale = requests.back().is_none_or(|newest| *newest < window_start);
                drop(requests);

                let tokens = entry.value().tokens.try_read().ok()?;
                let tokens_stale = tokens
                    .back()
                    .is_none_or(|(newest, _)| *newest < window_start);
                drop(tokens);

                (requests_stale && tokens_stale).then(|| entry.key().clone())
            })
            .collect();
        let evicted = stale_keys.len();
        for key in stale_keys {
            self.states.remove(&key);
        }
        evicted
    }

    /// Get HTTP headers for rate limit response
    pub async fn get_headers(&self, key: &str) -> RateLimitHeaders {
        let status = self.get_status(key).await;
        RateLimitHeaders {
            limit: status.requests_limit,
            remaining: status.requests_limit.saturating_sub(status.requests_used),
            reset: status.reset_at.elapsed().as_secs() as i64, // Time until reset
        }
    }
}

/// Rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub requests_used: u32,
    pub requests_limit: u32,
    pub tokens_used: u32,
    pub tokens_limit: u32,
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_rejected: u64,
    pub reset_at: Instant,
}

/// HTTP rate limit headers
#[derive(Debug, Clone)]
pub struct RateLimitHeaders {
    pub limit: u32,
    pub remaining: u32,
    pub reset: i64,
}

impl RateLimitHeaders {
    /// Convert to HTTP header tuples
    pub fn to_header_pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            ("X-RateLimit-Limit", self.limit.to_string()),
            ("X-RateLimit-Remaining", self.remaining.to_string()),
            ("X-RateLimit-Reset", self.reset.to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 10,
            tokens_per_minute: 1000,
            window_size: Duration::from_secs(60),
        });

        for _ in 0..5 {
            let result = limiter.check_rate_limit("test_key", 10).await.unwrap();
            assert!(result.allowed);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 3,
            tokens_per_minute: 1000,
            window_size: Duration::from_secs(60),
        });

        // First 3 should pass
        for _ in 0..3 {
            let result = limiter.check_rate_limit("test_key", 10).await.unwrap();
            assert!(result.allowed);
        }

        // 4th should be blocked
        let result = limiter.check_rate_limit("test_key", 10).await.unwrap();
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_token_limit() {
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 100,
            tokens_per_minute: 100,
            window_size: Duration::from_secs(60),
        });

        // Request with 50 tokens - should pass
        let result = limiter.check_rate_limit("test_key", 50).await.unwrap();
        assert!(result.allowed);

        // Request with 60 tokens - should be blocked (50 + 60 > 100)
        let result = limiter.check_rate_limit("test_key", 60).await.unwrap();
        assert!(!result.allowed);

        // Request with 40 tokens - should pass (50 + 40 = 90)
        let result = limiter.check_rate_limit("test_key", 40).await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_evict_stale_removes_a_key_checked_only_once() {
        // Regression for the gap the naive "deque is empty" version had: `check_rate_limit`'s
        // lazy expiry only runs on the NEXT check for the SAME key, so a key seen exactly once
        // (the shape of a rotate-once-per-request attack) never has its single entry cleared by
        // the normal request path — eviction has to notice the window elapsed on its own.
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 10,
            tokens_per_minute: 1000,
            window_size: Duration::from_millis(20),
        });

        limiter.check_rate_limit("one-shot-key", 1).await.unwrap();
        assert_eq!(limiter.tracked_key_count(), 1);

        tokio::time::sleep(Duration::from_millis(40)).await;

        let evicted = limiter.evict_stale().await;
        assert_eq!(evicted, 1);
        assert_eq!(limiter.tracked_key_count(), 0);
    }

    #[tokio::test]
    async fn test_evict_stale_keeps_a_key_with_recent_activity() {
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 10,
            tokens_per_minute: 1000,
            window_size: Duration::from_secs(60),
        });

        limiter.check_rate_limit("active-key", 1).await.unwrap();

        let evicted = limiter.evict_stale().await;
        assert_eq!(evicted, 0);
        assert_eq!(limiter.tracked_key_count(), 1);
    }

    #[tokio::test]
    async fn test_rate_limiter_separate_keys() {
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 2,
            tokens_per_minute: 1000,
            window_size: Duration::from_secs(60),
        });

        // Key 1 uses both requests
        limiter.check_rate_limit("key1", 10).await.unwrap();
        limiter.check_rate_limit("key1", 10).await.unwrap();

        // Key 1 should be blocked
        let result = limiter.check_rate_limit("key1", 10).await.unwrap();
        assert!(!result.allowed);

        // Key 2 should still work
        let result = limiter.check_rate_limit("key2", 10).await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_status() {
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 10,
            tokens_per_minute: 1000,
            window_size: Duration::from_secs(60),
        });

        limiter.check_rate_limit("test_key", 50).await.unwrap();
        limiter.check_rate_limit("test_key", 30).await.unwrap();

        let status = limiter.get_status("test_key").await;
        assert_eq!(status.requests_used, 2);
        assert_eq!(status.tokens_used, 80);
        assert_eq!(status.total_requests, 2);
        assert_eq!(status.total_tokens, 80);
    }

    #[tokio::test]
    async fn test_rate_limiter_clear() {
        let limiter = RateLimiter::with_default_config(RateLimitConfig {
            requests_per_minute: 2,
            tokens_per_minute: 1000,
            window_size: Duration::from_secs(60),
        });

        limiter.check_rate_limit("test_key", 10).await.unwrap();
        limiter.check_rate_limit("test_key", 10).await.unwrap();

        // Should be blocked
        let result = limiter.check_rate_limit("test_key", 10).await.unwrap();
        assert!(!result.allowed);

        // Clear and retry
        limiter.clear("test_key").await;

        let result = limiter.check_rate_limit("test_key", 10).await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn test_rate_limiter_custom_config() {
        let limiter = RateLimiter::new();

        // Set custom config for a specific key
        limiter.set_config("premium", RateLimitConfig {
            requests_per_minute: 1000,
            tokens_per_minute: 1000000,
            window_size: Duration::from_secs(60),
        });

        // Premium key should have higher limits
        for _ in 0..100 {
            let result = limiter.check_rate_limit("premium", 100).await.unwrap();
            assert!(result.allowed);
        }
    }
}
