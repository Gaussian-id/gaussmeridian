//! Performance metrics types

use std::time::Duration;

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub requests_per_second: f64,
    pub average_response_time: Duration,
    pub error_rate: f64,
    pub active_connections: u32,
}
