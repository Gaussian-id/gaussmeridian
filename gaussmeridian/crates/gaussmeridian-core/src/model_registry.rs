//! Model registry for managing model-to-provider mappings

use crate::provider_registry::ProviderRegistry;
use gaussmeridian_models::{CostInfo, Model, ModelCapabilities, ModelInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Model metadata cache with version tracking
#[derive(Debug, Clone)]
struct ModelMetadata {
    info: ModelInfo,
    version: String,
    last_updated: chrono::DateTime<chrono::Utc>,
    health_status: ModelHealthStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelHealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// Thread-safe model registry with metadata support
pub struct ModelRegistry {
    model_to_provider: Arc<RwLock<HashMap<String, String>>>,
    provider_to_models: Arc<RwLock<HashMap<String, Vec<Model>>>>,
    model_metadata: Arc<RwLock<HashMap<String, ModelMetadata>>>,
    provider_registry: Option<Arc<ProviderRegistry>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            model_to_provider: Arc::new(RwLock::new(HashMap::new())),
            provider_to_models: Arc::new(RwLock::new(HashMap::new())),
            model_metadata: Arc::new(RwLock::new(HashMap::new())),
            provider_registry: None,
        }
    }

    /// Create a new registry with provider registry reference for discovery
    pub fn with_provider_registry(provider_registry: Arc<ProviderRegistry>) -> Self {
        Self {
            model_to_provider: Arc::new(RwLock::new(HashMap::new())),
            provider_to_models: Arc::new(RwLock::new(HashMap::new())),
            model_metadata: Arc::new(RwLock::new(HashMap::new())),
            provider_registry: Some(provider_registry),
        }
    }

    pub async fn register_provider_models(&self, provider: &str, models: Vec<Model>) {
        let mut model_map = self.model_to_provider.write().await;
        let mut provider_map = self.provider_to_models.write().await;

        for model in &models {
            model_map.insert(model.id.clone(), provider.to_string());
        }
        provider_map.insert(provider.to_string(), models);

        info!(
            "Registered {} models for provider {}",
            provider_map.get(provider).map(|v| v.len()).unwrap_or(0),
            provider
        );
    }

    pub async fn get_provider_for_model(&self, model_id: &str) -> Option<String> {
        self.model_to_provider.read().await.get(model_id).cloned()
    }

    pub async fn list_all_models(&self) -> Vec<Model> {
        self.provider_to_models
            .read()
            .await
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    /// Get model metadata with capability detection and version tracking
    pub async fn get_model_metadata(&self, model_id: &str) -> Option<ModelInfo> {
        // Security: Validate model_id to prevent injection
        if model_id.is_empty() || model_id.len() > 256 {
            warn!("Invalid model_id provided: length {}", model_id.len());
            return None;
        }

        // Performance: Check cache first
        {
            let metadata_cache = self.model_metadata.read().await;
            if let Some(metadata) = metadata_cache.get(model_id) {
                // Return cached metadata if it's recent (less than 1 hour old)
                let age = chrono::Utc::now() - metadata.last_updated;
                if age.num_hours() < 1 {
                    return Some(metadata.info.clone());
                }
            }
        }

        // Try to get metadata from provider
        if let Some(provider_name) = self.get_provider_for_model(model_id).await {
            if let Some(provider_registry) = &self.provider_registry {
                if let Some(provider_entry) = provider_registry.get(&provider_name) {
                    // Try to get cost info from provider
                    let cost_info = match provider_entry.provider.get_cost_info(model_id).await {
                        Ok(cost) => cost,
                        Err(_) => CostInfo {
                            input_cost_per_1k_tokens: 0.0,
                            output_cost_per_1k_tokens: 0.0,
                            currency: "USD".to_string(),
                            model: model_id.to_string(),
                        },
                    };

                    // Detect capabilities from provider
                    let capabilities = provider_entry.provider.capabilities();
                    let model_capabilities = ModelCapabilities {
                        supports_streaming: capabilities.supports_streaming,
                        supports_functions: capabilities.supports_functions,
                        supports_vision: capabilities.supports_vision,
                        supports_embeddings: capabilities.supports_embeddings,
                    };

                    // Get context length from provider capabilities
                    let context_length = capabilities.max_context_length.unwrap_or(4096);

                    let model_info = ModelInfo {
                        id: model_id.to_string(),
                        name: model_id.to_string(),
                        context_length,
                        pricing: cost_info,
                        capabilities: model_capabilities,
                    };

                    // Cache the metadata
                    {
                        let mut metadata_cache = self.model_metadata.write().await;
                        metadata_cache.insert(
                            model_id.to_string(),
                            ModelMetadata {
                                info: model_info.clone(),
                                version: "1.0".to_string(), // Default version
                                last_updated: chrono::Utc::now(),
                                health_status: ModelHealthStatus::Unknown,
                            },
                        );
                    }

                    return Some(model_info);
                }
            }
        }

        // Fallback: check if we have the model registered
        let models = self.list_all_models().await;
        if let Some(model) = models.iter().find(|m| m.id == model_id) {
            // Create basic metadata from model
            let model_info = ModelInfo {
                id: model.id.clone(),
                name: model.id.clone(),
                context_length: 4096, // Default
                pricing: CostInfo {
                    input_cost_per_1k_tokens: 0.0,
                    output_cost_per_1k_tokens: 0.0,
                    currency: "USD".to_string(),
                    model: model.id.clone(),
                },
                capabilities: ModelCapabilities {
                    supports_streaming: true,
                    supports_functions: false,
                    supports_vision: false,
                    supports_embeddings: false,
                },
            };

            return Some(model_info);
        }

        None
    }

    /// Update model health status
    pub async fn update_model_health(&self, model_id: &str, is_healthy: bool) {
        let mut metadata_cache = self.model_metadata.write().await;
        if let Some(metadata) = metadata_cache.get_mut(model_id) {
            metadata.health_status = if is_healthy {
                ModelHealthStatus::Healthy
            } else {
                ModelHealthStatus::Unhealthy
            };
            metadata.last_updated = chrono::Utc::now();
        }
    }

    /// Get model version
    pub async fn get_model_version(&self, model_id: &str) -> Option<String> {
        self.model_metadata
            .read()
            .await
            .get(model_id)
            .map(|m| m.version.clone())
    }

    /// Update model version
    pub async fn update_model_version(&self, model_id: &str, version: String) {
        let mut metadata_cache = self.model_metadata.write().await;
        if let Some(metadata) = metadata_cache.get_mut(model_id) {
            metadata.version = version;
            metadata.last_updated = chrono::Utc::now();
        }
    }

    /// Check if model is healthy
    pub async fn is_model_healthy(&self, model_id: &str) -> bool {
        self.model_metadata
            .read()
            .await
            .get(model_id)
            .map(|m| m.health_status == ModelHealthStatus::Healthy)
            .unwrap_or(false)
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
