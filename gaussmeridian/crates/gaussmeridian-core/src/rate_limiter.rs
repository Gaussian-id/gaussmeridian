//! Rate limiting functionality
//!
//! Implements sliding window rate limiting algorithm for requests and tokens.

use dashmap::DashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter for API requests
pub struct RateLimiter {
    limits: DashMap<String, RateLimit>,
    request_windows: DashMap<String, RwLock<VecDeque<Instant>>>,
    token_windows: DashMap<String, RwLock<VecDeque<(Instant, u32)>>>,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub window_size: Duration,
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining_requests: u32,
    pub remaining_tokens: u32,
    pub reset_at: Instant,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: DashMap::new(),
            request_windows: DashMap::new(),
            token_windows: DashMap::new(),
        }
    }

    /// Set rate limit for a provider/key
    pub fn set_limit(&self, key: &str, limit: RateLimit) {
        self.limits.insert(key.to_string(), limit);
    }

    /// Check rate limit with sliding window algorithm
    pub async fn check_rate_limit(&self, key: &str, tokens: u32) -> Result<RateLimitResult, ()> {
        let limit = self
            .limits
            .get(key)
            .map(|r| r.clone())
            .unwrap_or_else(|| RateLimit {
                requests_per_minute: 1000,
                tokens_per_minute: 100000,
                window_size: Duration::from_secs(60),
            });

        let now = Instant::now();
        let window_start = now - limit.window_size;

        // Get or create request window for this key
        let request_window = self
            .request_windows
            .entry(key.to_string())
            .or_insert_with(|| RwLock::new(VecDeque::new()));

        // Get or create token window for this key
        let token_window = self
            .token_windows
            .entry(key.to_string())
            .or_insert_with(|| RwLock::new(VecDeque::new()));

        // Clean up old entries from request window
        let mut req_window = request_window.write().await;
        while let Some(front) = req_window.front() {
            if *front < window_start {
                req_window.pop_front();
            } else {
                break;
            }
        }

        // Clean up old entries from token window
        let mut tok_window = token_window.write().await;
        while let Some(front) = tok_window.front() {
            if front.0 < window_start {
                tok_window.pop_front();
            } else {
                break;
            }
        }

        // Count requests and tokens in current window
        let current_requests = req_window.len() as u32;
        let current_tokens: u32 = tok_window.iter().map(|(_, tokens)| tokens).sum();

        // Check if request would exceed limits
        let requests_allowed = current_requests < limit.requests_per_minute;
        let tokens_allowed = (current_tokens + tokens) <= limit.tokens_per_minute;
        let allowed = requests_allowed && tokens_allowed;

        if allowed {
            // Add current request to windows
            req_window.push_back(now);
            tok_window.push_back((now, tokens));
        }

        let remaining_requests = limit.requests_per_minute.saturating_sub(current_requests);
        let remaining_tokens = limit.tokens_per_minute.saturating_sub(current_tokens);
        let reset_at = if let Some(oldest) = req_window.front() {
            *oldest + limit.window_size
        } else {
            now + limit.window_size
        };

        Ok(RateLimitResult {
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
            reset_at,
        })
    }

    /// Check rate limit (simple boolean version for backward compatibility)
    pub async fn check_rate_limit_simple(&self, key: &str, tokens: u32) -> Result<bool, ()> {
        let result = self.check_rate_limit(key, tokens).await?;
        Ok(result.allowed)
    }

    /// Get rate limit headers for HTTP responses
    pub async fn get_rate_limit_headers(&self, key: &str) -> Option<RateLimitHeaders> {
        let status = self.get_status(key).await?;
        Some(RateLimitHeaders {
            limit: status.remaining_requests + (if status.allowed { 1 } else { 0 }),
            remaining: status.remaining_requests,
            reset: status
                .reset_at
                .duration_since(std::time::Instant::now())
                .as_secs(),
        })
    }

    /// Clear rate limit data for a key (useful for testing or manual resets)
    pub async fn clear(&self, key: &str) {
        self.request_windows.remove(key);
        self.token_windows.remove(key);
    }

    /// Get statistics about rate limiter usage
    pub fn get_stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            tracked_keys: self.limits.len(),
            active_request_windows: self.request_windows.len(),
            active_token_windows: self.token_windows.len(),
        }
    }

    /// Get current rate limit status
    pub async fn get_status(&self, key: &str) -> Option<RateLimitResult> {
        let limit = self.limits.get(key)?.clone();
        let now = Instant::now();
        let window_start = now - limit.window_size;

        let request_window = self.request_windows.get(key)?;
        let token_window = self.token_windows.get(key)?;

        let mut req_window = request_window.write().await;
        let mut tok_window = token_window.write().await;

        // Clean up old entries
        while let Some(front) = req_window.front() {
            if *front < window_start {
                req_window.pop_front();
            } else {
                break;
            }
        }

        while let Some(front) = tok_window.front() {
            if front.0 < window_start {
                tok_window.pop_front();
            } else {
                break;
            }
        }

        let current_requests = req_window.len() as u32;
        let current_tokens: u32 = tok_window.iter().map(|(_, tokens)| tokens).sum();

        let remaining_requests = limit.requests_per_minute.saturating_sub(current_requests);
        let remaining_tokens = limit.tokens_per_minute.saturating_sub(current_tokens);
        let reset_at = if let Some(oldest) = req_window.front() {
            *oldest + limit.window_size
        } else {
            now + limit.window_size
        };

        Some(RateLimitResult {
            allowed: true, // Status check doesn't block
            remaining_requests,
            remaining_tokens,
            reset_at,
        })
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limit headers for HTTP responses
#[derive(Debug, Clone)]
pub struct RateLimitHeaders {
    pub limit: u32,
    pub remaining: u32,
    pub reset: u64,
}

/// Rate limiter statistics
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub tracked_keys: usize,
    pub active_request_windows: usize,
    pub active_token_windows: usize,
}
