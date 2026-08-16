//! Provider registry for managing LLM providers

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::traits::LLMProvider;

/// Provider status for health monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Provider entry with health and stats
pub struct ProviderEntry {
    pub name: String,
    pub provider: Arc<dyn LLMProvider<Error = gaussmeridian_models::ProviderError>>,
    pub status: Arc<Mutex<ProviderStatus>>,
    pub last_latency: Arc<Mutex<Option<Duration>>>,
    pub failure_count: Arc<AtomicUsize>,
}

/// Concurrent provider registry
pub struct ProviderRegistry {
    providers: dashmap::DashMap<String, Arc<ProviderEntry>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: dashmap::DashMap::new(),
        }
    }

    pub fn register(
        &self,
        name: String,
        provider: Arc<dyn LLMProvider<Error = gaussmeridian_models::ProviderError>>,
    ) {
        let entry = Arc::new(ProviderEntry {
            name: name.clone(),
            provider,
            status: Arc::new(Mutex::new(ProviderStatus::Unknown)),
            last_latency: Arc::new(Mutex::new(None)),
            failure_count: Arc::new(AtomicUsize::new(0)),
        });
        self.providers.insert(name, entry);
    }

    pub fn unregister(&self, name: &str) {
        self.providers.remove(name);
    }

    pub fn get(&self, name: &str) -> Option<Arc<ProviderEntry>> {
        self.providers.get(name).map(|e| e.value().clone())
    }

    pub fn all(&self) -> Vec<Arc<ProviderEntry>> {
        self.providers.iter().map(|e| e.value().clone()).collect()
    }

    pub async fn health_check_all(&self) {
        for entry in self.all() {
            let provider = entry.provider.clone();
            let status = entry.status.clone();
            let last_latency = entry.last_latency.clone();
            let failure_count = entry.failure_count.clone();
            tokio::spawn(async move {
                let start = std::time::Instant::now();
                let result = provider.health_check().await;
                let latency = start.elapsed();
                let mut status_guard = status.lock().await;
                let mut latency_guard = last_latency.lock().await;
                match result {
                    Ok(_) => {
                        *status_guard = ProviderStatus::Healthy;
                        *latency_guard = Some(latency);
                        failure_count.store(0, Ordering::Relaxed);
                    }
                    Err(_) => {
                        *status_guard = ProviderStatus::Unhealthy;
                        *latency_guard = None;
                        failure_count.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        }
    }

    /// List all models from all registered providers
    pub async fn all_models(&self) -> Vec<gaussmeridian_models::Model> {
        let mut all_models = Vec::new();
        let mut model_ids = std::collections::HashSet::new();

        // Collect models from all providers
        for entry in self.all() {
            match entry.provider.list_models().await {
                Ok(models) => {
                    for model in models {
                        // Deduplicate models by ID
                        if model_ids.insert(model.id.clone()) {
                            all_models.push(model);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to list models from provider {}: {}", entry.name, e);
                }
            }
        }

        info!(
            "Discovered {} unique models from {} providers",
            all_models.len(),
            self.providers.len()
        );
        all_models
    }

    /// Discover and register models from all providers
    pub async fn discover_models(&self) -> Result<usize, String> {
        let mut total_discovered = 0;

        for entry in self.all() {
            match entry.provider.list_models().await {
                Ok(models) => {
                    info!(
                        "Discovered {} models from provider {}",
                        models.len(),
                        entry.name
                    );
                    total_discovered += models.len();
                }
                Err(e) => {
                    warn!(
                        "Failed to discover models from provider {}: {}",
                        entry.name, e
                    );
                }
            }
        }

        Ok(total_discovered)
    }

    /// Get models for a specific provider
    pub async fn get_provider_models(
        &self,
        provider_name: &str,
    ) -> Result<Vec<gaussmeridian_models::Model>, String> {
        if let Some(entry) = self.get(provider_name) {
            entry
                .provider
                .list_models()
                .await
                .map_err(|e| format!("Failed to list models: {}", e))
        } else {
            Err(format!("Provider '{}' not found", provider_name))
        }
    }

    /// Validate that a model exists in at least one provider
    pub async fn validate_model(&self, model_id: &str) -> bool {
        for entry in self.all() {
            if entry.provider.supports_model(model_id).await {
                return true;
            }
        }
        false
    }
}
