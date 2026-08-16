//! Metrics collector implementations

use std::sync::Arc;
use std::time::Duration;

/// Metrics collector trait
pub trait MetricsCollector: Send + Sync + 'static {
    fn record_request(&self, operation: &str, duration: Duration, success: bool);
    fn record_token(&self);
    fn record_cache_hit(&self);
    fn record_cache_miss(&self);
    fn inc_active_requests(&self);
    fn dec_active_requests(&self);
    fn get_metrics(&self) -> Result<String, Box<dyn std::error::Error>>;
}

/// Basic metrics collector implementation
#[derive(Clone)]
pub struct BasicMetricsCollector {
    requests_total: Arc<std::sync::atomic::AtomicU64>,
    requests_success: Arc<std::sync::atomic::AtomicU64>,
    requests_failed: Arc<std::sync::atomic::AtomicU64>,
    tokens_processed: Arc<std::sync::atomic::AtomicU64>,
    cache_hits: Arc<std::sync::atomic::AtomicU64>,
    cache_misses: Arc<std::sync::atomic::AtomicU64>,
    active_requests: Arc<std::sync::atomic::AtomicU64>,
    request_durations: Arc<std::sync::Mutex<Vec<Duration>>>,
}

impl BasicMetricsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_cache_counts(&self) -> (u64, u64) {
        (
            self.cache_hits.load(std::sync::atomic::Ordering::Relaxed),
            self.cache_misses.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

impl Default for BasicMetricsCollector {
    fn default() -> Self {
        Self {
            requests_total: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            requests_success: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            requests_failed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            tokens_processed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cache_hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cache_misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            active_requests: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            request_durations: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}

impl MetricsCollector for BasicMetricsCollector {
    fn record_request(&self, _operation: &str, duration: Duration, success: bool) {
        self.requests_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if success {
            self.requests_success
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.requests_failed
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        if let Ok(mut durations) = self.request_durations.lock() {
            durations.push(duration);
            // Keep only last 1000 durations
            if durations.len() > 1000 {
                durations.remove(0);
            }
        }
    }

    fn record_token(&self) {
        self.tokens_processed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_cache_hit(&self) {
        self.cache_hits
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_cache_miss(&self) {
        self.cache_misses
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn inc_active_requests(&self) {
        self.active_requests
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn dec_active_requests(&self) {
        self.active_requests
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn get_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        let total = self
            .requests_total
            .load(std::sync::atomic::Ordering::Relaxed);
        let success = self
            .requests_success
            .load(std::sync::atomic::Ordering::Relaxed);
        let failed = self
            .requests_failed
            .load(std::sync::atomic::Ordering::Relaxed);
        let tokens = self
            .tokens_processed
            .load(std::sync::atomic::Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(std::sync::atomic::Ordering::Relaxed);
        let active = self
            .active_requests
            .load(std::sync::atomic::Ordering::Relaxed);

        let avg_duration = if let Ok(durations) = self.request_durations.lock() {
            if durations.is_empty() {
                0.0
            } else {
                durations.iter().map(|d| d.as_millis() as f64).sum::<f64>() / durations.len() as f64
            }
        } else {
            0.0
        };

        let metrics = format!(
            "# HELP gaussmeridian_requests_total Total number of requests\n\
            # TYPE gaussmeridian_requests_total counter\n\
            gaussmeridian_requests_total {}\n\
            # HELP gaussmeridian_requests_success Total number of successful requests\n\
            # TYPE gaussmeridian_requests_success counter\n\
            gaussmeridian_requests_success {}\n\
            # HELP gaussmeridian_requests_failed Total number of failed requests\n\
            # TYPE gaussmeridian_requests_failed counter\n\
            gaussmeridian_requests_failed {}\n\
            # HELP gaussmeridian_tokens_processed Total number of tokens processed\n\
            # TYPE gaussmeridian_tokens_processed counter\n\
            gaussmeridian_tokens_processed {}\n\
            # HELP gaussmeridian_cache_hits Total number of cache hits\n\
            # TYPE gaussmeridian_cache_hits counter\n\
            gaussmeridian_cache_hits {}\n\
            # HELP gaussmeridian_cache_misses Total number of cache misses\n\
            # TYPE gaussmeridian_cache_misses counter\n\
            gaussmeridian_cache_misses {}\n\
            # HELP gaussmeridian_active_requests Current number of active requests\n\
            # TYPE gaussmeridian_active_requests gauge\n\
            gaussmeridian_active_requests {}\n\
            # HELP gaussmeridian_average_response_time_ms Average response time in milliseconds\n\
            # TYPE gaussmeridian_average_response_time_ms gauge\n\
            gaussmeridian_average_response_time_ms {}\n",
            total, success, failed, tokens, cache_hits, cache_misses, active, avg_duration
        );

        Ok(metrics)
    }
}
