//! Metrics and observability for GaussMeridian
//!
//! This crate provides comprehensive metrics collection including:
//! - Request/response metrics
//! - Performance metrics
//! - Provider health metrics
//! - Cache performance metrics
//! - Prometheus integration
//! - Distributed tracing

pub mod collector;
pub mod health;
pub mod performance;
pub mod prometheus;

pub use collector::{BasicMetricsCollector, MetricsCollector};
pub use health::{HealthCheck, HealthChecker, HealthStatus};
pub use performance::PerformanceMetrics;
pub use prometheus::PrometheusMetricsCollector;

/// Main metrics collector that combines all functionality
pub struct MainMetricsCollector {
    basic: BasicMetricsCollector,
    health_checker: HealthChecker,
}

impl MainMetricsCollector {
    pub fn new(_prometheus_enabled: bool) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            basic: BasicMetricsCollector::new(),
            health_checker: HealthChecker::new(),
        })
    }

    pub fn record_request(&self, operation: &str, duration: std::time::Duration, success: bool) {
        self.basic.record_request(operation, duration, success);
    }

    pub fn record_token(&self) {
        self.basic.record_token();
    }

    pub fn record_cache_hit(&self) {
        self.basic.record_cache_hit();
    }

    pub fn record_cache_miss(&self) {
        self.basic.record_cache_miss();
    }

    pub fn inc_active_requests(&self) {
        self.basic.inc_active_requests();
    }

    pub fn dec_active_requests(&self) {
        self.basic.dec_active_requests();
    }

    pub fn get_cache_counts(&self) -> (u64, u64) {
        self.basic.get_cache_counts()
    }

    pub fn get_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.basic.get_metrics()
    }

    pub async fn check_health(&self) -> HealthCheck {
        self.health_checker.check_health().await
    }
}

#[cfg(test)]
mod tests;
