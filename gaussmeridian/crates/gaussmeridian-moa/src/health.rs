use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use thiserror::Error;
use crate::storage::StorageBackend;
use crate::metrics::MetricsCollector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: SystemStatus,
    pub components: HashMap<String, ComponentStatus>,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SystemStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub name: String,
    pub status: ComponentHealth,
    pub message: Option<String>,
    pub last_check: DateTime<Utc>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ComponentHealth {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Error)]
pub enum HealthCheckError {
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Metrics error: {0}")]
    Metrics(String),
    
    #[error("Component check failed: {0}")]
    ComponentCheck(String),
}

pub struct HealthChecker {
    storage: Arc<dyn StorageBackend>,
    metrics: Arc<MetricsCollector>,
    status_cache: Arc<RwLock<HealthStatus>>,
    start_time: DateTime<Utc>,
}

impl HealthChecker {
    pub fn new(
        storage: Arc<dyn StorageBackend>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        let status = HealthStatus {
            status: SystemStatus::Healthy,
            components: HashMap::new(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: 0,
        };

        Self {
            storage,
            metrics,
            status_cache: Arc::new(RwLock::new(status)),
            start_time: Utc::now(),
        }
    }

    pub async fn start_monitoring(&self) {
        let status_cache = Arc::clone(&self.status_cache);
        let storage = Arc::clone(&self.storage);
        let metrics = Arc::clone(&self.metrics);
        let start_time = self.start_time;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                
                let mut status = status_cache.write().await;
                status.timestamp = Utc::now();
                status.uptime = (Utc::now() - start_time).num_seconds() as u64;

                // Check storage health
                let storage_status = Self::check_storage(&storage).await;
                status.components.insert("storage".to_string(), storage_status);

                // Check metrics health
                let metrics_status = Self::check_metrics(&metrics).await;
                status.components.insert("metrics".to_string(), metrics_status);

                // Update overall system status
                status.status = Self::determine_system_status(&status.components);
            }
        });
    }

    async fn check_storage(storage: &Arc<dyn StorageBackend>) -> ComponentStatus {
        let mut metrics = HashMap::new();
        let start = Utc::now();

        // Perform storage health check
        match storage.exists("health_check").await {
            Ok(_) => {
                let latency = (Utc::now() - start).num_milliseconds() as f64;
                metrics.insert("latency_ms".to_string(), latency);
                
                ComponentStatus {
                    name: "storage".to_string(),
                    status: ComponentHealth::Healthy,
                    message: None,
                    last_check: Utc::now(),
                    metrics,
                }
            }
            Err(e) => ComponentStatus {
                name: "storage".to_string(),
                status: ComponentHealth::Unhealthy,
                message: Some(e.to_string()),
                last_check: Utc::now(),
                metrics,
            }
        }
    }

    async fn check_metrics(metrics: &Arc<MetricsCollector>) -> ComponentStatus {
        let mut component_metrics = HashMap::new();
        
        // Get some basic metrics
        match metrics.get_aggregated_value("system_memory_usage").await {
            Ok(memory_usage) => {
                component_metrics.insert("memory_usage_mb".to_string(), memory_usage);
            }
            Err(_) => {}
        }

        ComponentStatus {
            name: "metrics".to_string(),
            status: ComponentHealth::Healthy,
            message: None,
            last_check: Utc::now(),
            metrics: component_metrics,
        }
    }

    fn determine_system_status(components: &HashMap<String, ComponentStatus>) -> SystemStatus {
        let mut unhealthy_count = 0;
        let mut degraded_count = 0;

        for status in components.values() {
            match status.status {
                ComponentHealth::Unhealthy => unhealthy_count += 1,
                ComponentHealth::Degraded => degraded_count += 1,
                ComponentHealth::Healthy => {}
            }
        }

        if unhealthy_count > 0 {
            SystemStatus::Unhealthy
        } else if degraded_count > 0 {
            SystemStatus::Degraded
        } else {
            SystemStatus::Healthy
        }
    }

    pub async fn get_health_status(&self) -> HealthStatus {
        self.status_cache.read().await.clone()
    }

    pub async fn check_component(&self, component: &str) -> Result<ComponentStatus, HealthCheckError> {
        match component {
            "storage" => Ok(Self::check_storage(&self.storage).await),
            "metrics" => Ok(Self::check_metrics(&self.metrics).await),
            _ => Err(HealthCheckError::ComponentCheck(
                format!("Unknown component: {}", component)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::metrics::MetricsConfig;
    use std::time::Duration;

    #[tokio::test]
    async fn test_health_checker() {
        let storage = Arc::new(MemoryStorage::new());
        let metrics_config = MetricsConfig {
            collection_interval: Duration::from_secs(1),
            retention_period: Duration::from_secs(3600),
            aggregation_window: Duration::from_secs(60),
            export_prometheus: false,
            prometheus_port: 9090,
        };
        let metrics = Arc::new(MetricsCollector::new(storage.clone(), metrics_config).unwrap());
        
        let health_checker = HealthChecker::new(storage, metrics);
        
        // Test initial status
        let status = health_checker.get_health_status().await;
        assert_eq!(status.status, SystemStatus::Healthy);
        
        // Test component check
        let storage_status = health_checker.check_component("storage").await.unwrap();
        assert_eq!(storage_status.status, ComponentHealth::Healthy);
        
        // Test unknown component
        let result = health_checker.check_component("unknown").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_system_status_determination() {
        let mut components = HashMap::new();
        
        // All healthy
        components.insert(
            "comp1".to_string(),
            ComponentStatus {
                name: "comp1".to_string(),
                status: ComponentHealth::Healthy,
                message: None,
                last_check: Utc::now(),
                metrics: HashMap::new(),
            }
        );
        
        assert_eq!(
            HealthChecker::determine_system_status(&components),
            SystemStatus::Healthy
        );
        
        // One degraded
        components.insert(
            "comp2".to_string(),
            ComponentStatus {
                name: "comp2".to_string(),
                status: ComponentHealth::Degraded,
                message: None,
                last_check: Utc::now(),
                metrics: HashMap::new(),
            }
        );
        
        assert_eq!(
            HealthChecker::determine_system_status(&components),
            SystemStatus::Degraded
        );
        
        // One unhealthy
        components.insert(
            "comp3".to_string(),
            ComponentStatus {
                name: "comp3".to_string(),
                status: ComponentHealth::Unhealthy,
                message: Some("Error".to_string()),
                last_check: Utc::now(),
                metrics: HashMap::new(),
            }
        );
        
        assert_eq!(
            HealthChecker::determine_system_status(&components),
            SystemStatus::Unhealthy
        );
    }
} 