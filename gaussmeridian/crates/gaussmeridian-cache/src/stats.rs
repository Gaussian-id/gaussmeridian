//! Cache statistics and monitoring

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Basic cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f64,
    pub miss_rate: f64,
}

/// Detailed cache statistics
#[derive(Debug, Clone)]
pub struct DetailedCacheStats {
    pub basic: CacheStats,
    pub evictions: u64,
    pub expirations: u64,
    pub errors: u64,
    pub avg_get_time: Duration,
    pub avg_set_time: Duration,
    pub total_requests: u64,
    pub memory_usage: usize,
    pub last_reset: Instant,
}

/// Cache performance metrics
pub struct CacheMetrics {
    pub total_requests: AtomicU64,
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
    pub expirations: AtomicU64,
    pub errors: AtomicU64,
    pub current_size: AtomicUsize,
    pub max_size: AtomicUsize,
    pub total_get_time: AtomicU64, // nanoseconds
    pub total_set_time: AtomicU64, // nanoseconds
    pub last_reset: std::sync::Mutex<Instant>,
}

impl CacheMetrics {
    pub fn new(max_size: usize) -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            expirations: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            current_size: AtomicUsize::new(0),
            max_size: AtomicUsize::new(max_size),
            total_get_time: AtomicU64::new(0),
            total_set_time: AtomicU64::new(0),
            last_reset: std::sync::Mutex::new(Instant::now()),
        }
    }

    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_expiration(&self) {
        self.expirations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_get_time(&self, duration: Duration) {
        self.total_get_time
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn record_set_time(&self, duration: Duration) {
        self.total_set_time
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn set_current_size(&self, size: usize) {
        self.current_size.store(size, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        let miss_rate = 1.0 - hit_rate;

        CacheStats {
            hits,
            misses,
            size: self.current_size.load(Ordering::Relaxed),
            capacity: self.max_size.load(Ordering::Relaxed),
            hit_rate,
            miss_rate,
        }
    }

    pub fn get_detailed_stats(&self) -> DetailedCacheStats {
        let basic = self.get_stats();
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let total_get_time = self.total_get_time.load(Ordering::Relaxed);
        let total_set_time = self.total_set_time.load(Ordering::Relaxed);

        let avg_get_time = if self.hits.load(Ordering::Relaxed) > 0 {
            Duration::from_nanos(total_get_time / self.hits.load(Ordering::Relaxed))
        } else {
            Duration::ZERO
        };

        let avg_set_time = if total_requests > 0 {
            Duration::from_nanos(total_set_time / total_requests)
        } else {
            Duration::ZERO
        };

        DetailedCacheStats {
            basic,
            evictions: self.evictions.load(Ordering::Relaxed),
            expirations: self.expirations.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            avg_get_time,
            avg_set_time,
            total_requests,
            memory_usage: self.current_size.load(Ordering::Relaxed) * std::mem::size_of::<u64>(), // Rough estimate
            last_reset: *self.last_reset.lock().unwrap(),
        }
    }

    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.evictions.store(0, Ordering::Relaxed);
        self.expirations.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.total_get_time.store(0, Ordering::Relaxed);
        self.total_set_time.store(0, Ordering::Relaxed);
        *self.last_reset.lock().unwrap() = Instant::now();
    }
}

/// Cache health status
#[derive(Debug, Clone)]
pub enum CacheHealth {
    Healthy,
    Degraded { hit_rate: f64, error_rate: f64 },
    Unhealthy { error_rate: f64, last_error: String },
}

/// Cache performance analyzer
pub struct CacheAnalyzer {
    metrics: CacheMetrics,
    health_thresholds: HealthThresholds,
}

#[derive(Debug, Clone)]
pub struct HealthThresholds {
    pub min_hit_rate: f64,
    pub max_error_rate: f64,
    pub max_avg_get_time: Duration,
    pub max_avg_set_time: Duration,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            min_hit_rate: 0.8,
            max_error_rate: 0.05,
            max_avg_get_time: Duration::from_millis(10),
            max_avg_set_time: Duration::from_millis(50),
        }
    }
}

impl CacheAnalyzer {
    pub fn new(max_size: usize) -> Self {
        Self {
            metrics: CacheMetrics::new(max_size),
            health_thresholds: HealthThresholds::default(),
        }
    }

    pub fn with_thresholds(mut self, thresholds: HealthThresholds) -> Self {
        self.health_thresholds = thresholds;
        self
    }

    pub fn get_health(&self) -> CacheHealth {
        let stats = self.metrics.get_detailed_stats();
        let error_rate = if stats.total_requests > 0 {
            stats.errors as f64 / stats.total_requests as f64
        } else {
            0.0
        };

        if error_rate > self.health_thresholds.max_error_rate {
            CacheHealth::Unhealthy {
                error_rate,
                last_error: "High error rate".to_string(),
            }
        } else if stats.basic.hit_rate < self.health_thresholds.min_hit_rate {
            CacheHealth::Degraded {
                hit_rate: stats.basic.hit_rate,
                error_rate,
            }
        } else {
            CacheHealth::Healthy
        }
    }

    pub fn get_metrics(&self) -> &CacheMetrics {
        &self.metrics
    }

    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        let stats = self.metrics.get_detailed_stats();

        if stats.basic.hit_rate < 0.5 {
            recommendations
                .push("Consider increasing cache size or improving cache key strategy".to_string());
        }

        if stats.evictions > stats.total_requests / 10 {
            recommendations
                .push("High eviction rate - consider increasing cache capacity".to_string());
        }

        if stats.avg_get_time > Duration::from_millis(5) {
            recommendations
                .push("Slow cache access - consider optimizing cache implementation".to_string());
        }

        recommendations
    }
}
