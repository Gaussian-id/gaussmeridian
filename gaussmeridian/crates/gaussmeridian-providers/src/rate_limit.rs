//! Rate limiting and quota management for providers

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter for provider requests
pub struct ProviderRateLimiter {
    limits: HashMap<String, RateLimit>,
    usage: Arc<RwLock<HashMap<String, UsageTracker>>>,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub requests_per_hour: Option<u32>,
    pub tokens_per_hour: Option<u32>,
    pub burst_size: u32,
    pub window_size: Duration,
}

/// Usage tracker for rate limiting
struct UsageTracker {
    requests: Vec<Instant>,
    tokens: Vec<(Instant, u32)>,
    last_reset: Instant,
    window_size: Duration,
}

impl UsageTracker {
    fn new(window_size: Duration) -> Self {
        Self {
            requests: Vec::new(),
            tokens: Vec::new(),
            last_reset: Instant::now(),
            window_size,
        }
    }

    fn cleanup_old_entries(&mut self) {
        let now = Instant::now();
        let cutoff = now - self.window_size;

        self.requests.retain(|&time| time > cutoff);
        self.tokens.retain(|(time, _)| *time > cutoff);
    }

    fn can_make_request(&mut self, limit: &RateLimit) -> bool {
        self.cleanup_old_entries();
        self.requests.len() < limit.requests_per_minute as usize
    }

    fn can_use_tokens(&mut self, limit: &RateLimit, tokens: u32) -> bool {
        self.cleanup_old_entries();

        let total_tokens: u32 = self.tokens.iter().map(|(_, count)| count).sum();
        total_tokens + tokens <= limit.tokens_per_minute
    }

    fn record_request(&mut self) {
        self.requests.push(Instant::now());
    }

    fn record_tokens(&mut self, tokens: u32) {
        self.tokens.push((Instant::now(), tokens));
    }
}

impl ProviderRateLimiter {
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_provider(&mut self, provider_name: String, limit: RateLimit) {
        self.limits.insert(provider_name, limit);
    }

    pub async fn check_rate_limit(
        &self,
        provider_name: &str,
        tokens: u32,
    ) -> Result<bool, RateLimitError> {
        let limit = self
            .limits
            .get(provider_name)
            .ok_or(RateLimitError::ProviderNotFound)?;

        let mut usage = self.usage.write().await;
        let tracker = usage
            .entry(provider_name.to_string())
            .or_insert_with(|| UsageTracker::new(limit.window_size));

        if !tracker.can_make_request(limit) {
            return Err(RateLimitError::RequestLimitExceeded);
        }

        if !tracker.can_use_tokens(limit, tokens) {
            return Err(RateLimitError::TokenLimitExceeded);
        }

        tracker.record_request();
        tracker.record_tokens(tokens);

        Ok(true)
    }

    pub async fn get_usage(&self, provider_name: &str) -> Option<UsageInfo> {
        let usage = self.usage.read().await;
        let tracker = usage.get(provider_name)?;
        let limit = self.limits.get(provider_name)?;

        Some(UsageInfo {
            requests_used: tracker.requests.len() as u32,
            requests_limit: limit.requests_per_minute,
            tokens_used: tracker.tokens.iter().map(|(_, count)| count).sum(),
            tokens_limit: limit.tokens_per_minute,
            window_size: limit.window_size,
        })
    }
}

/// Rate limit error
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Provider not found")]
    ProviderNotFound,
    #[error("Request rate limit exceeded")]
    RequestLimitExceeded,
    #[error("Token rate limit exceeded")]
    TokenLimitExceeded,
    #[error("Hourly request limit exceeded")]
    HourlyRequestLimitExceeded,
    #[error("Hourly token limit exceeded")]
    HourlyTokenLimitExceeded,
}

/// Usage information
#[derive(Debug, Clone)]
pub struct UsageInfo {
    pub requests_used: u32,
    pub requests_limit: u32,
    pub tokens_used: u32,
    pub tokens_limit: u32,
    pub window_size: Duration,
}

/// Quota manager for cost tracking
pub struct QuotaManager {
    quotas: HashMap<String, Quota>,
    usage: Arc<RwLock<HashMap<String, QuotaUsage>>>,
}

/// Quota configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quota {
    pub daily_budget: f64,
    pub monthly_budget: f64,
    pub currency: String,
    pub alerts: Vec<QuotaAlert>,
}

/// Quota usage tracking
struct QuotaUsage {
    daily_spent: f64,
    monthly_spent: f64,
    last_reset: DateTime<Utc>,
}

/// Quota alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaAlert {
    pub threshold_percent: f64,
    pub alert_type: QuotaAlertType,
}

/// Quota alert type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuotaAlertType {
    Email,
    Webhook,
    Slack,
    Console,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: HashMap::new(),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_quota(&mut self, provider_name: String, quota: Quota) {
        self.quotas.insert(provider_name, quota);
    }

    pub async fn check_quota(&self, provider_name: &str, cost: f64) -> Result<bool, QuotaError> {
        let quota = self
            .quotas
            .get(provider_name)
            .ok_or(QuotaError::QuotaNotFound)?;

        let mut usage = self.usage.write().await;
        let usage_tracker = usage
            .entry(provider_name.to_string())
            .or_insert_with(|| QuotaUsage {
                daily_spent: 0.0,
                monthly_spent: 0.0,
                last_reset: Utc::now(),
            });

        let now = Utc::now();

        // Reset daily spending if it's a new day
        if now.date_naive() != usage_tracker.last_reset.date_naive() {
            usage_tracker.daily_spent = 0.0;
            usage_tracker.last_reset = now;
        }

        // Reset monthly spending if it's a new month
        if now.month() != usage_tracker.last_reset.month() {
            usage_tracker.monthly_spent = 0.0;
        }

        // Check daily budget
        if usage_tracker.daily_spent + cost > quota.daily_budget {
            return Err(QuotaError::DailyBudgetExceeded);
        }

        // Check monthly budget
        if usage_tracker.monthly_spent + cost > quota.monthly_budget {
            return Err(QuotaError::MonthlyBudgetExceeded);
        }

        usage_tracker.daily_spent += cost;
        usage_tracker.monthly_spent += cost;

        Ok(true)
    }

    pub async fn get_quota_usage(&self, provider_name: &str) -> Option<QuotaUsageInfo> {
        let quota = self.quotas.get(provider_name)?;
        let usage = self.usage.read().await;
        let usage_tracker = usage.get(provider_name)?;

        Some(QuotaUsageInfo {
            daily_spent: usage_tracker.daily_spent,
            daily_budget: quota.daily_budget,
            monthly_spent: usage_tracker.monthly_spent,
            monthly_budget: quota.monthly_budget,
            currency: quota.currency.clone(),
        })
    }
}

/// Quota error
#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    #[error("Quota not found")]
    QuotaNotFound,
    #[error("Daily budget exceeded")]
    DailyBudgetExceeded,
    #[error("Monthly budget exceeded")]
    MonthlyBudgetExceeded,
}

/// Quota usage information
#[derive(Debug, Clone)]
pub struct QuotaUsageInfo {
    pub daily_spent: f64,
    pub daily_budget: f64,
    pub monthly_spent: f64,
    pub monthly_budget: f64,
    pub currency: String,
}

/// Rate limit middleware
pub struct RateLimitMiddleware {
    rate_limiter: Arc<ProviderRateLimiter>,
    quota_manager: Arc<QuotaManager>,
}

impl RateLimitMiddleware {
    pub fn new(rate_limiter: Arc<ProviderRateLimiter>, quota_manager: Arc<QuotaManager>) -> Self {
        Self {
            rate_limiter,
            quota_manager,
        }
    }

    pub async fn check_limits(
        &self,
        provider_name: &str,
        tokens: u32,
        cost: f64,
    ) -> Result<(), MiddlewareError> {
        // Check rate limits
        self.rate_limiter
            .check_rate_limit(provider_name, tokens)
            .await
            .map_err(MiddlewareError::RateLimit)?;

        // Check quota
        self.quota_manager
            .check_quota(provider_name, cost)
            .await
            .map_err(MiddlewareError::Quota)?;

        Ok(())
    }
}

/// Middleware error
#[derive(Debug, thiserror::Error)]
pub enum MiddlewareError {
    #[error("Rate limit error: {0}")]
    RateLimit(#[from] RateLimitError),
    #[error("Quota error: {0}")]
    Quota(#[from] QuotaError),
}
