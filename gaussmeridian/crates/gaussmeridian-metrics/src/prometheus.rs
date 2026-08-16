//! Prometheus metrics integration

use std::time::Duration;

use crate::collector::{BasicMetricsCollector, MetricsCollector};

/// Prometheus metrics collector
pub struct PrometheusMetricsCollector {
    basic: BasicMetricsCollector,
}

impl PrometheusMetricsCollector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            basic: BasicMetricsCollector::new(),
        })
    }
}

impl MetricsCollector for PrometheusMetricsCollector {
    fn record_request(&self, operation: &str, duration: Duration, success: bool) {
        self.basic.record_request(operation, duration, success);
    }

    fn record_token(&self) {
        self.basic.record_token();
    }

    fn record_cache_hit(&self) {
        self.basic.record_cache_hit();
    }

    fn record_cache_miss(&self) {
        self.basic.record_cache_miss();
    }

    fn inc_active_requests(&self) {
        self.basic.inc_active_requests();
    }

    fn dec_active_requests(&self) {
        self.basic.dec_active_requests();
    }

    fn get_metrics(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.basic.get_metrics()
    }
}
