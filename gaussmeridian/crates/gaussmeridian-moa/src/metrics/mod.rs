use crate::error::{MoaResult, MoaError};
use std::{
    sync::Arc,
    collections::HashMap,
};
use tokio::sync::RwLock;
use tracing::{error, info};
use serde::{Serialize, Deserialize};

#[cfg(feature = "metrics")]
use prometheus::{
    Registry, Counter, Histogram, IntGauge, TextEncoder,
    Encoder, HistogramOpts,
};

const MAX_METRICS_HISTORY: usize = 1000;
const MAX_METRICS_CLEANUP_INTERVAL_SECS: u64 = 3600;
const DEFAULT_TYPE: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_response_time: f64,
    pub success_rate: f64,
    pub agent_metrics: HashMap<String, AgentMetricsSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricsSnapshot {
    pub response_times: Vec<f64>,
    pub success_rates: Vec<f64>,
    pub confidence_scores: Vec<f64>,
    pub avg_response_time: f64,
    pub avg_success_rate: f64,
    pub avg_confidence: f64,
}

#[cfg(feature = "metrics")]
pub struct MetricsRegistry {
    registry: Registry,
    request_counter: Counter,
    response_time: Histogram,
    error_counter: Counter,
    active_requests: IntGauge,
    agent_metrics: HashMap<String, PrometheusAgentMetrics>,
    history: Arc<RwLock<Vec<MetricsSnapshot>>>,
    _cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(not(feature = "metrics"))]
pub struct MetricsRegistry {
    request_count: AtomicU64,
    error_count: AtomicU64,
    active_requests: AtomicU64,
}

#[cfg(feature = "metrics")]
pub struct PrometheusAgentMetrics {
    pub request_counter: Counter,
    pub response_time: Histogram,
    pub error_counter: Counter,
}

#[cfg(feature = "metrics")]
impl PrometheusAgentMetrics {
    pub fn new(registry: &Registry, agent_id: &str) -> MoaResult<Self> {
        let request_counter = Counter::new(
            format!("agent_{}_requests_total", agent_id),
            format!("Total requests for agent {}", agent_id)
        ).map_err(|e| MoaError::metrics(format!("Failed to create request counter: {}", e), Some(Box::new(e))))?;
        
        let response_time = Histogram::with_opts(
            HistogramOpts::new(
                format!("agent_{}_response_time_seconds", agent_id),
                format!("Response time for agent {}", agent_id)
            )
        ).map_err(|e| MoaError::metrics(format!("Failed to create response time histogram: {}", e), Some(Box::new(e))))?;
        
        let error_counter = Counter::new(
            format!("agent_{}_errors_total", agent_id),
            format!("Total errors for agent {}", agent_id)
        ).map_err(|e| MoaError::metrics(format!("Failed to create error counter: {}", e), Some(Box::new(e))))?;

        registry.register(Box::new(request_counter.clone()))
            .map_err(|e| MoaError::metrics(format!("Failed to register request counter: {}", e), Some(Box::new(e))))?;
        registry.register(Box::new(response_time.clone()))
            .map_err(|e| MoaError::metrics(format!("Failed to register response time: {}", e), Some(Box::new(e))))?;
        registry.register(Box::new(error_counter.clone()))
            .map_err(|e| MoaError::metrics(format!("Failed to register error counter: {}", e), Some(Box::new(e))))?;

        Ok(Self {
            request_counter,
            response_time,
            error_counter,
        })
    }
}

impl MetricsRegistry {
    #[cfg(feature = "metrics")]
    pub fn new() -> MoaResult<Self> {
        let registry = Registry::new();
        let request_counter = Counter::new(
            "moa_requests_total",
            "Total number of requests processed"
        ).map_err(|e| MoaError::metrics(format!("Failed to create request counter: {}", e), Some(Box::new(e))))?;
        
        let response_time = Histogram::with_opts(
            HistogramOpts::new(
                "moa_response_time_seconds".to_string(),
                "Response time in seconds".to_string()
            )
        ).map_err(|e| MoaError::metrics(format!("Failed to create response time histogram: {}", e), Some(Box::new(e))))?;
        
        let error_counter = Counter::new(
            "moa_errors_total",
            "Total number of errors"
        ).map_err(|e| MoaError::metrics(format!("Failed to create error counter: {}", e), Some(Box::new(e))))?;
        
        let active_requests = IntGauge::new(
            "moa_active_requests",
            "Number of currently active requests"
        ).map_err(|e| MoaError::metrics(format!("Failed to create active requests gauge: {}", e), Some(Box::new(e))))?;

        registry.register(Box::new(request_counter.clone()))
            .map_err(|e| MoaError::metrics(format!("Failed to register request counter: {}", e), Some(Box::new(e))))?;
        registry.register(Box::new(response_time.clone()))
            .map_err(|e| MoaError::metrics(format!("Failed to register response time: {}", e), Some(Box::new(e))))?;
        registry.register(Box::new(error_counter.clone()))
            .map_err(|e| MoaError::metrics(format!("Failed to register error counter: {}", e), Some(Box::new(e))))?;
        registry.register(Box::new(active_requests.clone()))
            .map_err(|e| MoaError::metrics(format!("Failed to register active requests: {}", e), Some(Box::new(e))))?;

        let agent_metrics = HashMap::new();
        let history = Arc::new(RwLock::new(Vec::with_capacity(MAX_METRICS_HISTORY)));

        // Start cleanup task
        let history_clone = Arc::clone(&history);
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(MAX_METRICS_CLEANUP_INTERVAL_SECS)
            );
            loop {
                interval.tick().await;
                if let Err(e) = Self::cleanup_old_metrics(&history_clone).await {
                    error!("Failed to cleanup old metrics: {}", e);
                }
            }
        });

        Ok(Self {
            registry,
            request_counter,
            response_time,
            error_counter,
            active_requests,
            agent_metrics,
            history,
            _cleanup_task: Some(cleanup_task),
        })
    }

    #[cfg(not(feature = "metrics"))]
    pub fn new() -> MoaResult<Self> {
        Ok(Self {
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            active_requests: AtomicU64::new(0),
        })
    }

    #[cfg(feature = "metrics")]
    pub fn record_request(&self) {
        self.request_counter.inc();
        self.active_requests.inc();
    }

    #[cfg(not(feature = "metrics"))]
    pub fn record_request(&self) {
        self.request_count.fetch_add(1, Ordering::SeqCst);
        self.active_requests.fetch_add(1, Ordering::SeqCst);
    }

    #[cfg(feature = "metrics")]
    pub fn record_response(&self, duration: std::time::Duration) {
        self.response_time.observe(duration.as_secs_f64());
        self.active_requests.dec();
    }

    #[cfg(not(feature = "metrics"))]
    pub fn record_response(&self, _duration: std::time::Duration) {
        self.active_requests.fetch_sub(1, Ordering::SeqCst);
    }

    #[cfg(feature = "metrics")]
    pub fn record_error(&self) {
        self.error_counter.inc();
        self.active_requests.dec();
    }

    #[cfg(not(feature = "metrics"))]
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::SeqCst);
        self.active_requests.fetch_sub(1, Ordering::SeqCst);
    }

    #[cfg(feature = "metrics")]
    pub fn get_metrics(&self) -> MoaResult<String> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer)
            .map_err(|e| MoaError::metrics(format!("Failed to encode metrics: {}", e), Some(Box::new(e))))?;
        String::from_utf8(buffer)
            .map_err(|e| MoaError::metrics(format!("Failed to convert metrics to string: {}", e), Some(Box::new(e))))
    }

    #[cfg(not(feature = "metrics"))]
    pub fn get_metrics(&self) -> MoaResult<String> {
        Ok(format!(
            "requests_total: {}\nerrors_total: {}\nactive_requests: {}",
            self.request_count.load(Ordering::SeqCst),
            self.error_count.load(Ordering::SeqCst),
            self.active_requests.load(Ordering::SeqCst)
        ))
    }

    async fn cleanup_old_metrics(history: &Arc<RwLock<Vec<MetricsSnapshot>>>) -> MoaResult<()> {
        let mut history = history.write().await;
        let now = chrono::Utc::now();
        history.retain(|snapshot| {
            (now - snapshot.timestamp).num_seconds() as u64 <= MAX_METRICS_CLEANUP_INTERVAL_SECS
        });
        Ok(())
    }
}

impl Drop for MetricsRegistry {
    fn drop(&mut self) {
        if let Some(task) = self._cleanup_task.take() {
            task.abort();
        }
        info!("Shutting down metrics registry");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn test_metrics_recording() -> MoaResult<()> {
        let registry = MetricsRegistry::new()?;

        // Record request
        registry.record_request();
        
        // Record completion
        let start = Instant::now();
        tokio::time::sleep(Duration::from_millis(100)).await;
        registry.record_response(start.elapsed());

        // Get metrics
        let metrics = registry.get_metrics()?;
        assert!(!metrics.is_empty());

        Ok(())
    }
} 