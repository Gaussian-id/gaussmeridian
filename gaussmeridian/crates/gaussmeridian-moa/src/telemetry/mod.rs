use metrics::{counter, gauge, histogram};
use opentelemetry::{
    trace::{Span, Tracer},
    KeyValue,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub request_count: u64,
    pub error_count: u64,
    pub average_latency_ms: f64,
    pub active_agents: u32,
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f64,
}

#[derive(Debug)]
pub struct TelemetryManager {
    metrics_store: RwLock<Vec<MetricsSnapshot>>,
    tracer: opentelemetry::sdk::trace::Tracer,
    start_time: Instant,
}

impl TelemetryManager {
    pub fn new() -> Self {
        let tracer = opentelemetry::sdk::trace::TracerProvider::builder()
            .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
            .build()
            .tracer("moa");

        Self {
            metrics_store: RwLock::new(Vec::new()),
            tracer,
            start_time: Instant::now(),
        }
    }

    pub async fn record_request(&self, duration_ms: f64, success: bool) {
        counter!("moa.requests.total", 1);
        histogram!("moa.requests.duration", duration_ms);
        
        if !success {
            counter!("moa.requests.errors", 1);
        }

        let mut store = self.metrics_store.write().await;
        if store.len() >= 1000 {
            store.remove(0); // Keep last 1000 snapshots
        }

        store.push(MetricsSnapshot {
            timestamp: chrono::Utc::now(),
            request_count: counter!("moa.requests.total"),
            error_count: counter!("moa.requests.errors"),
            average_latency_ms: histogram!("moa.requests.duration").mean(),
            active_agents: gauge!("moa.agents.active"),
            memory_usage_mb: self.get_memory_usage(),
            cpu_usage_percent: self.get_cpu_usage(),
        });
    }

    pub fn start_request_span(&self, request_id: &str) -> Span {
        self.tracer
            .start("moa.request")
            .with_attributes(vec![KeyValue::new("request_id", request_id.to_string())])
    }

    pub async fn get_metrics(&self, duration: std::time::Duration) -> Vec<MetricsSnapshot> {
        let store = self.metrics_store.read().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(duration).unwrap();
        store
            .iter()
            .filter(|snapshot| snapshot.timestamp >= cutoff)
            .cloned()
            .collect()
    }

    fn get_memory_usage(&self) -> u64 {
        // Implementation depends on platform
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
                if let Some(line) = content.lines().find(|l| l.starts_with("VmRSS:")) {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb.parse::<u64>() {
                            return kb / 1024; // Convert to MB
                        }
                    }
                }
            }
        }
        0
    }

    fn get_cpu_usage(&self) -> f64 {
        // Implementation depends on platform
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/self/stat") {
                let fields: Vec<&str> = content.split_whitespace().collect();
                if fields.len() >= 14 {
                    if let (Ok(utime), Ok(stime)) = (fields[13].parse::<u64>(), fields[14].parse::<u64>()) {
                        let total_time = utime + stime;
                        let elapsed = self.start_time.elapsed().as_secs();
                        if elapsed > 0 {
                            return (total_time as f64 / elapsed as f64) * 100.0;
                        }
                    }
                }
            }
        }
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_metrics_recording() {
        let manager = TelemetryManager::new();
        
        // Record some test metrics
        manager.record_request(100.0, true).await;
        manager.record_request(150.0, false).await;
        
        // Wait a bit
        sleep(std::time::Duration::from_millis(100)).await;
        
        // Get recent metrics
        let metrics = manager.get_metrics(std::time::Duration::from_secs(1)).await;
        
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].request_count, 1);
        assert_eq!(metrics[1].error_count, 1);
    }
} 