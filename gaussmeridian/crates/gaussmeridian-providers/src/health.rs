//! Provider health monitoring and management

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::common::*;
use gaussmeridian_core::LLMProvider;
// gaussmeridian_models types are used via gaussmeridian_core

/// Health monitor for providers
pub struct HealthMonitor {
    providers: Arc<
        RwLock<
            HashMap<
                String,
                Arc<dyn LLMProvider<Error = gaussmeridian_models::ProviderError> + Send + Sync>,
            >,
        >,
    >,
    health_status: Arc<RwLock<HashMap<String, ProviderHealth>>>,
    check_interval: Duration,
    unhealthy_threshold: u32,
    recovery_threshold: u32,
}

impl HealthMonitor {
    pub fn new(check_interval: Duration) -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            health_status: Arc::new(RwLock::new(HashMap::new())),
            check_interval,
            unhealthy_threshold: 3,
            recovery_threshold: 2,
        }
    }

    /// Add a provider to monitor
    pub async fn add_provider(
        &self,
        name: String,
        provider: Arc<dyn LLMProvider<Error = gaussmeridian_models::ProviderError> + Send + Sync>,
    ) {
        self.providers.write().await.insert(name.clone(), provider);
        self.health_status
            .write()
            .await
            .insert(name, ProviderHealth::Unknown);
    }

    /// Remove a provider from monitoring
    pub async fn remove_provider(&self, name: &str) {
        self.providers.write().await.remove(name);
        self.health_status.write().await.remove(name);
    }

    /// Get health status for all providers
    pub async fn get_all_health(&self) -> HashMap<String, ProviderHealth> {
        self.health_status.read().await.clone()
    }

    /// Get healthy providers
    pub async fn get_healthy_providers(&self) -> Vec<String> {
        let health_status = self.health_status.read().await;
        health_status
            .iter()
            .filter_map(|(name, health)| {
                if matches!(health, ProviderHealth::Healthy) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Start health monitoring
    pub async fn start_monitoring(&self) {
        let providers = self.providers.clone();
        let health_status = self.health_status.clone();
        let check_interval = self.check_interval;
        let _unhealthy_threshold = self.unhealthy_threshold;
        let _recovery_threshold = self.recovery_threshold;

        tokio::spawn(async move {
            let mut interval = interval(check_interval);

            loop {
                interval.tick().await;

                let providers_to_check = providers.read().await.clone();
                let mut new_health_status = health_status.read().await.clone();

                for (name, provider) in providers_to_check {
                    let health = Self::check_provider_health(&provider).await;
                    new_health_status.insert(name, health);
                }

                *health_status.write().await = new_health_status;
            }
        });
    }

    /// Check individual provider health
    async fn check_provider_health(
        provider: &Arc<dyn LLMProvider<Error = gaussmeridian_models::ProviderError> + Send + Sync>,
    ) -> ProviderHealth {
        let start = Instant::now();

        match provider.health_check().await {
            Ok(()) => {
                let latency = start.elapsed();
                if latency > Duration::from_secs(5) {
                    ProviderHealth::Degraded {
                        latency,
                        error_rate: 0.0,
                    }
                } else {
                    ProviderHealth::Healthy
                }
            }
            Err(e) => ProviderHealth::Unhealthy {
                last_error: e.to_string(),
                consecutive_failures: 1,
            },
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub interval: Duration,
    pub timeout: Duration,
    pub unhealthy_threshold: u32,
    pub recovery_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            unhealthy_threshold: 3,
            recovery_threshold: 2,
        }
    }
}

/// Custom health check trait
#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check(
        &self,
        provider: &dyn LLMProvider<Error = gaussmeridian_models::ProviderError>,
    ) -> Result<bool, String>;

    fn name(&self) -> &'static str;
}

/// Latency health check
pub struct LatencyHealthCheck {
    max_latency: Duration,
}

impl LatencyHealthCheck {
    pub fn new(max_latency: Duration) -> Self {
        Self { max_latency }
    }
}

#[async_trait]
impl HealthCheck for LatencyHealthCheck {
    async fn check(
        &self,
        provider: &dyn LLMProvider<Error = gaussmeridian_models::ProviderError>,
    ) -> Result<bool, String> {
        let start = Instant::now();

        match provider.health_check().await {
            Ok(()) => {
                let latency = start.elapsed();
                if latency <= self.max_latency {
                    Ok(true)
                } else {
                    Err(format!("Latency too high: {:?}", latency))
                }
            }
            Err(e) => Err(format!("Health check failed: {}", e)),
        }
    }

    fn name(&self) -> &'static str {
        "latency"
    }
}

/// Response time health check
pub struct ResponseTimeHealthCheck {
    max_response_time: Duration,
}

impl ResponseTimeHealthCheck {
    pub fn new(max_response_time: Duration) -> Self {
        Self { max_response_time }
    }
}

#[async_trait]
impl HealthCheck for ResponseTimeHealthCheck {
    async fn check(
        &self,
        _provider: &dyn LLMProvider<Error = gaussmeridian_models::ProviderError>,
    ) -> Result<bool, String> {
        // This would implement a more sophisticated response time check
        // For now, just return true
        Ok(true)
    }

    fn name(&self) -> &'static str {
        "response_time"
    }
}

/// Error rate health check
pub struct ErrorRateHealthCheck {
    max_error_rate: f64,
}

impl ErrorRateHealthCheck {
    pub fn new(max_error_rate: f64) -> Self {
        Self { max_error_rate }
    }
}

#[async_trait]
impl HealthCheck for ErrorRateHealthCheck {
    async fn check(
        &self,
        _provider: &dyn LLMProvider<Error = gaussmeridian_models::ProviderError>,
    ) -> Result<bool, String> {
        // This would check the provider's error rate
        // For now, just return true
        Ok(true)
    }

    fn name(&self) -> &'static str {
        "error_rate"
    }
}

/// Health alert manager
pub struct HealthAlertManager {
    alerts: Arc<RwLock<Vec<HealthAlert>>>,
    alert_handlers: Vec<Box<dyn AlertHandler + Send + Sync>>,
}

/// Health alert
#[derive(Debug, Clone)]
pub struct HealthAlert {
    pub provider_name: String,
    pub alert_type: AlertType,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub severity: AlertSeverity,
}

/// Alert type
#[derive(Debug, Clone)]
pub enum AlertType {
    ProviderUnhealthy,
    ProviderDegraded,
    ProviderRecovered,
    HighLatency,
    HighErrorRate,
    RateLimitExceeded,
}

/// Alert severity
#[derive(Debug, Clone)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Alert handler trait
#[async_trait]
pub trait AlertHandler: Send + Sync {
    async fn handle_alert(&self, alert: &HealthAlert);

    fn name(&self) -> &'static str;
}

/// Console alert handler
pub struct ConsoleAlertHandler;

#[async_trait]
impl AlertHandler for ConsoleAlertHandler {
    async fn handle_alert(&self, alert: &HealthAlert) {
        println!(
            "[{}] {} - {}: {}",
            alert.severity.as_str(),
            alert.timestamp,
            alert.provider_name,
            alert.message
        );
    }

    fn name(&self) -> &'static str {
        "console"
    }
}

/// Log alert handler
pub struct LogAlertHandler;

#[async_trait]
impl AlertHandler for LogAlertHandler {
    async fn handle_alert(&self, alert: &HealthAlert) {
        use tracing::{error, info, warn};

        match alert.severity {
            AlertSeverity::Critical | AlertSeverity::High => {
                error!(
                    provider = %alert.provider_name,
                    alert_type = ?alert.alert_type,
                    message = %alert.message,
                    "Provider health alert"
                );
            }
            AlertSeverity::Medium => {
                warn!(
                    provider = %alert.provider_name,
                    alert_type = ?alert.alert_type,
                    message = %alert.message,
                    "Provider health alert"
                );
            }
            AlertSeverity::Low => {
                info!(
                    provider = %alert.provider_name,
                    alert_type = ?alert.alert_type,
                    message = %alert.message,
                    "Provider health alert"
                );
            }
        }
    }

    fn name(&self) -> &'static str {
        "log"
    }
}

impl AlertSeverity {
    fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Low => "LOW",
            AlertSeverity::Medium => "MEDIUM",
            AlertSeverity::High => "HIGH",
            AlertSeverity::Critical => "CRITICAL",
        }
    }
}

impl HealthAlertManager {
    pub fn new() -> Self {
        Self {
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Box<dyn AlertHandler + Send + Sync>) {
        self.alert_handlers.push(handler);
    }

    pub async fn send_alert(&self, alert: HealthAlert) {
        // Store alert
        self.alerts.write().await.push(alert.clone());

        // Send to all handlers
        for handler in &self.alert_handlers {
            handler.handle_alert(&alert).await;
        }
    }

    pub async fn get_alerts(&self) -> Vec<HealthAlert> {
        self.alerts.read().await.clone()
    }

    pub async fn clear_alerts(&self) {
        self.alerts.write().await.clear();
    }
}
