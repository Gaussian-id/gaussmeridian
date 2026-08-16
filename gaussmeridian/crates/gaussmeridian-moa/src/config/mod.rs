pub mod settings;

use serde::{Deserialize, Serialize};
use validator::Validate;
use std::path::Path;
use crate::error::{MoaError, MoaResult};

/// Main configuration for MOA engine
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MoaConfig {
    /// Storage configuration
    #[validate]
    pub storage: StorageConfig,
    /// Number of processing layers
    #[validate(range(min = 1, max = 10))]
    pub layers: usize,
    /// MOA strategy configuration
    #[validate]
    pub strategy: MoaStrategy,
    /// Agent configuration
    #[validate]
    pub agents: Vec<AgentConfig>,
    /// Metrics configuration
    #[validate]
    pub metrics: MetricsConfig,
    /// Tracing configuration
    #[validate]
    pub tracing: TracingConfig,
    /// Security configuration
    #[validate]
    pub security: SecurityConfig,
    /// Resource limits configuration
    #[validate]
    pub resources: ResourceConfig,
}

/// Available MOA strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoaStrategy {
    /// Standard MOA with weighted aggregation
    Standard(StandardConfig),
    /// Sparse MOA with selective agent usage
    Sparse(SparseConfig),
    /// Self-MOA with recursive refinement
    SelfMoa(SelfMoaConfig),
    /// Collaborative MOA with agent interaction
    Collaborative(CollaborativeConfig),
    /// Adaptive MOA with learning
    Adaptive(AdaptiveConfig),
    /// Hierarchical MOA with layered processing
    Hierarchical(HierarchicalConfig),
    /// Ensemble MOA combining multiple strategies
    Ensemble(EnsembleConfig),
    /// Dynamic routing MOA
    DynamicRouting(DynamicRoutingConfig),
}

impl MoaStrategy {
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        match self {
            MoaStrategy::Standard(config) => config.validate(),
            MoaStrategy::Sparse(config) => config.validate(),
            MoaStrategy::SelfMoa(config) => config.validate(),
            MoaStrategy::Collaborative(config) => config.validate(),
            MoaStrategy::Adaptive(config) => config.validate(),
            MoaStrategy::Hierarchical(config) => config.validate(),
            MoaStrategy::Ensemble(config) => config.validate(),
            MoaStrategy::DynamicRouting(config) => config.validate(),
        }
    }
}

/// Standard MOA configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StandardConfig {
    /// Maximum concurrent requests
    #[validate(range(min = 1, max = 1000))]
    pub max_concurrent: usize,
    /// Request timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub timeout_secs: u64,
}

/// Sparse MOA configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SparseConfig {
    /// Number of agents to select
    #[validate(range(min = 1))]
    pub k: usize,
    /// Selection strategy
    pub selection: String,
    /// Minimum confidence threshold
    #[validate(range(min = 0.0, max = 1.0))]
    pub confidence_threshold: f32,
}

/// Self-MOA configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SelfMoaConfig {
    /// Number of self-refinement samples
    #[validate(range(min = 1, max = 100))]
    pub samples: usize,
    /// Maximum refinement rounds
    #[validate(range(min = 1, max = 10))]
    pub max_rounds: usize,
    /// Diversity threshold
    #[validate(range(min = 0.0, max = 1.0))]
    pub diversity_threshold: f32,
}

/// Collaborative MOA configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CollaborativeConfig {
    /// Maximum rounds of collaboration
    #[validate(range(min = 1, max = 10))]
    pub max_rounds: usize,
    /// Consensus threshold
    #[validate(range(min = 0.0, max = 1.0))]
    pub consensus_threshold: f32,
    /// Maximum concurrent collaborations
    #[validate(range(min = 1, max = 100))]
    pub max_concurrent: usize,
}

/// Adaptive MOA configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AdaptiveConfig {
    /// Learning rate
    #[validate(range(min = 0.0, max = 1.0))]
    pub learning_rate: f32,
    /// Temperature for exploration
    #[validate(range(min = 0.0, max = 10.0))]
    pub temperature: f32,
    /// History window size
    #[validate(range(min = 1, max = 10000))]
    pub history_window: usize,
    /// Maximum concurrent requests
    #[validate(range(min = 1, max = 1000))]
    pub max_concurrent: usize,
}

/// Hierarchical MOA configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct HierarchicalConfig {
    /// Number of layers
    #[validate(range(min = 1, max = 10))]
    pub layers: usize,
    /// Strategy per layer
    pub layer_strategies: Vec<String>,
    /// Maximum concurrent requests
    #[validate(range(min = 1, max = 1000))]
    pub max_concurrent: usize,
}

/// Ensemble MOA configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct EnsembleConfig {
    /// Strategies to combine
    #[validate]
    pub strategies: Vec<MoaStrategy>,
    /// Strategy weights
    #[validate(custom = "validate_weights")]
    pub weights: Vec<f32>,
    /// Maximum concurrent requests
    #[validate(range(min = 1, max = 1000))]
    pub max_concurrent: usize,
}

fn validate_weights(weights: &[f32]) -> Result<(), validator::ValidationError> {
    let sum: f32 = weights.iter().sum();
    if (sum - 1.0).abs() > 1e-6 {
        return Err(validator::ValidationError::new("weights must sum to 1.0"));
    }
    Ok(())
}

/// Dynamic routing configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DynamicRoutingConfig {
    /// Maximum concurrent requests
    #[validate(range(min = 1, max = 1000))]
    pub max_concurrent: usize,
    /// Exploration rate
    #[validate(range(min = 0.0, max = 1.0))]
    pub epsilon: f32,
    /// History window size
    #[validate(range(min = 1, max = 10000))]
    pub window_size: usize,
}

/// Types of agents supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentType {
    LLM,
    RuleBased,
    Retrieval,
    Custom(String),
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,
    /// Agent type
    pub agent_type: AgentType,
    /// Agent role
    pub role: AgentRole,
    /// Agent capabilities
    pub capabilities: Vec<String>,
    /// Specific configuration for the agent type
    pub config: serde_json::Value,
    /// Agent timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub timeout_secs: u64,
    /// Maximum retries
    #[validate(range(min = 0, max = 10))]
    pub max_retries: u32,
}

/// Agent roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentRole {
    /// Primary response generation
    Primary,
    /// Secondary response generation
    Secondary,
    /// Fallback response generation
    Fallback,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics endpoint
    pub endpoint: String,
    /// Collection interval in seconds
    #[validate(range(min = 1, max = 3600))]
    pub interval: u64,
    /// Maximum metrics history size
    #[validate(range(min = 100, max = 1000000))]
    pub history_size: usize,
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct TracingConfig {
    /// Enable tracing
    pub enabled: bool,
    /// Jaeger endpoint
    pub jaeger_endpoint: String,
    /// Service name
    pub service_name: String,
    /// Log level
    pub log_level: String,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SecurityConfig {
    /// Path to key file
    pub key_path: String,
    /// Enable encryption
    pub encryption_enabled: bool,
    /// Key rotation interval in days
    #[validate(range(min = 1, max = 365))]
    pub key_rotation_days: u32,
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ResourceConfig {
    /// Maximum concurrent requests
    #[validate(range(min = 1, max = 10000))]
    pub max_concurrent_requests: usize,
    /// Request timeout in seconds
    #[validate(range(min = 1, max = 3600))]
    pub request_timeout_secs: u64,
    /// Maximum batch size
    #[validate(range(min = 1, max = 1000))]
    pub max_batch_size: usize,
    /// Cache TTL in seconds
    #[validate(range(min = 1, max = 86400))]
    pub cache_ttl_secs: u64,
}

/// Storage Backend Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackendType {
    File,
    Redb,
    Memory,
}

/// Storage Configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StorageConfig {
    pub backend: StorageBackendType,
    pub path: String,
    #[validate(range(min = 60, max = 86400))]
    pub cleanup_interval_seconds: u64,
    #[validate(range(min = 1, max = 1000000))]
    pub max_size_mb: u64,
}

pub fn load_config(path: impl AsRef<Path>) -> MoaResult<MoaConfig> {
    let config_str = std::fs::read_to_string(path)?;
    let config: MoaConfig = toml::from_str(&config_str)?;
    
    // Validate configuration
    config.validate().map_err(|e| MoaError::config(
        format!("Configuration validation failed: {}", e),
        None::<std::io::Error>
    ))?;
    
    Ok(config)
}

impl Default for MoaConfig {
    fn default() -> Self {
        Self {
            layers: 3,
            strategy: MoaStrategy::Standard(StandardConfig {
                max_concurrent: 10,
                timeout_secs: 30,
            }),
            agents: Vec::new(),
            metrics: MetricsConfig {
                enabled: true,
                endpoint: "http://localhost:8080/metrics".to_string(),
                interval: 60,
                history_size: 10000,
            },
            tracing: TracingConfig {
                enabled: true,
                jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
                service_name: "moa-engine".to_string(),
                log_level: "info".to_string(),
            },
            storage: StorageConfig {
                backend: StorageBackendType::File,
                path: "./moa_storage".to_string(),
                cleanup_interval_seconds: 3600,
                max_size_mb: 1000,
            },
            security: SecurityConfig {
                key_path: "gaussmoa_keys.json".to_string(),
                encryption_enabled: true,
                key_rotation_days: 30,
            },
            resources: ResourceConfig {
                max_concurrent_requests: 100,
                request_timeout_secs: 30,
                max_batch_size: 10,
                cache_ttl_secs: 3600,
            },
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            agent_type: AgentType::LLM,
            role: AgentRole::Primary,
            capabilities: vec![],
            config: serde_json::json!({}),
            max_retries: 3,
            timeout_secs: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let mut config = MoaConfig::default();
        assert!(config.validate().is_ok());

        // Test invalid layers
        config.layers = 0;
        assert!(config.validate().is_err());
        config.layers = 3;

        // Test invalid weights
        if let MoaStrategy::Ensemble(ref mut ensemble) = config.strategy {
            ensemble.weights = vec![0.3, 0.3, 0.3]; // Sum < 1.0
            assert!(config.validate().is_err());
        }
    }

    #[test]
    fn test_resource_limits() {
        let config = MoaConfig::default();
        assert!(config.resources.max_concurrent_requests > 0);
        assert!(config.resources.request_timeout_secs > 0);
        assert!(config.resources.max_batch_size > 0);
        assert!(config.resources.cache_ttl_secs > 0);
    }
}