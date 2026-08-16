//! Cache configuration and settings

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of items in cache
    pub max_size: usize,

    /// Default time-to-live for cached items
    pub ttl: Duration,

    /// Time-to-idle (how long item can be idle before eviction)
    pub tti: Option<Duration>,

    /// Cache type (memory, redis, etc.)
    pub cache_type: CacheType,

    /// Eviction strategy
    pub eviction_strategy: EvictionStrategy,

    /// Compression settings
    pub compression: CompressionConfig,

    /// Encryption settings
    pub encryption: EncryptionConfig,

    /// Performance settings
    pub performance: PerformanceConfig,

    /// Monitoring settings
    pub monitoring: MonitoringConfig,
}

/// Cache type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheType {
    Memory,
    Redis {
        url: String,
        pool_size: usize,
        timeout: Duration,
    },
    Hybrid {
        memory_config: MemoryConfig,
        redis_config: RedisConfig,
    },
}

/// Memory cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_size: usize,
    pub ttl: Duration,
    pub eviction_strategy: EvictionStrategy,
    pub enable_stats: bool,
}

/// Redis cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
    pub timeout: Duration,
    pub retry_attempts: usize,
    pub retry_delay: Duration,
    pub enable_clustering: bool,
    pub cluster_nodes: Vec<String>,
    pub password: Option<String>,
    pub database: Option<u8>,
}

/// Eviction strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionStrategy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// First In, First Out
    FIFO,
    /// Random eviction
    Random,
    /// Time-based eviction
    TTL,
    /// Adaptive replacement cache
    ARC,
    /// Clock algorithm
    Clock,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub min_size: usize,
    pub compression_level: u8,
}

/// Compression algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    Lz4,
    Snappy,
    Zstd,
    Brotli,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub algorithm: EncryptionAlgorithm,
    pub key: Vec<u8>,
    pub key_rotation_interval: Option<Duration>,
}

/// Encryption algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    AES256,
    ChaCha20,
    XChaCha20,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub enable_async_operations: bool,
    pub batch_size: usize,
    pub concurrency_limit: usize,
    pub enable_prefetching: bool,
    pub prefetch_threshold: f64,
    pub enable_warming: bool,
    pub warming_interval: Duration,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub enable_health_checks: bool,
    pub health_check_interval: Duration,
    pub enable_alerts: bool,
    pub alert_thresholds: AlertThresholds,
    pub enable_logging: bool,
    pub log_level: LogLevel,
}

/// Alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub min_hit_rate: f64,
    pub max_error_rate: f64,
    pub max_latency: Duration,
    pub max_memory_usage: f64,
}

/// Log level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,
            ttl: Duration::from_secs(3600),
            tti: None,
            cache_type: CacheType::Memory,
            eviction_strategy: EvictionStrategy::LRU,
            compression: CompressionConfig::default(),
            encryption: EncryptionConfig::default(),
            performance: PerformanceConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: CompressionAlgorithm::Lz4,
            min_size: 1024,
            compression_level: 6,
        }
    }
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: EncryptionAlgorithm::AES256,
            key: vec![0; 32], // Default key for AES256
            key_rotation_interval: None,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_async_operations: true,
            batch_size: 100,
            concurrency_limit: 10,
            enable_prefetching: false,
            prefetch_threshold: 0.8,
            enable_warming: false,
            warming_interval: Duration::from_secs(300),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(60),
            enable_alerts: false,
            alert_thresholds: AlertThresholds::default(),
            enable_logging: true,
            log_level: LogLevel::Info,
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            min_hit_rate: 0.8,
            max_error_rate: 0.05,
            max_latency: Duration::from_millis(100),
            max_memory_usage: 0.9,
        }
    }
}

impl CacheConfig {
    /// Create a new cache configuration
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            ..Default::default()
        }
    }

    /// Create a memory-only cache configuration
    pub fn memory(max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            cache_type: CacheType::Memory,
            ..Default::default()
        }
    }

    /// Create a Redis cache configuration
    pub fn redis(url: String, max_size: usize, ttl: Duration) -> Self {
        Self {
            max_size,
            ttl,
            cache_type: CacheType::Redis {
                url,
                pool_size: 10,
                timeout: Duration::from_secs(5),
            },
            ..Default::default()
        }
    }

    /// Create a hybrid cache configuration
    pub fn hybrid(memory_config: MemoryConfig, redis_config: RedisConfig) -> Self {
        Self {
            max_size: memory_config.max_size,
            ttl: memory_config.ttl,
            cache_type: CacheType::Hybrid {
                memory_config,
                redis_config,
            },
            ..Default::default()
        }
    }

    /// Enable compression
    pub fn with_compression(mut self, algorithm: CompressionAlgorithm) -> Self {
        self.compression.enabled = true;
        self.compression.algorithm = algorithm;
        self
    }

    /// Enable encryption
    pub fn with_encryption(mut self, algorithm: EncryptionAlgorithm) -> Self {
        self.encryption.enabled = true;
        self.encryption.algorithm = algorithm;
        self
    }

    /// Set eviction strategy
    pub fn with_eviction_strategy(mut self, strategy: EvictionStrategy) -> Self {
        self.eviction_strategy = strategy;
        self
    }

    /// Enable monitoring
    pub fn with_monitoring(mut self, monitoring: MonitoringConfig) -> Self {
        self.monitoring = monitoring;
        self
    }
}
