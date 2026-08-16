//! Tests for the metrics system

use std::time::Duration;

use crate::collector::{BasicMetricsCollector, MetricsCollector};
use crate::health::{HealthChecker, HealthStatus};

#[test]
fn test_basic_metrics_collector() {
    let collector = BasicMetricsCollector::new();

    collector.record_request("test", Duration::from_millis(100), true);
    collector.record_token();
    collector.record_cache_hit();

    let metrics = collector.get_metrics().unwrap();
    assert!(metrics.contains("gaussmeridian_requests_total 1"));
    assert!(metrics.contains("gaussmeridian_requests_success 1"));
    assert!(metrics.contains("gaussmeridian_tokens_processed 1"));
    assert!(metrics.contains("gaussmeridian_cache_hits 1"));
}

#[tokio::test]
async fn test_health_checker() {
    let checker = HealthChecker::new();
    let health = checker.check_health().await;

    assert!(matches!(health.status, HealthStatus::Healthy));
}
