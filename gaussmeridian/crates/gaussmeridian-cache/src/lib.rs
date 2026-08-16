//! Caching implementations for GaussMeridian
//!
//! This crate provides comprehensive caching implementations including:
//! - In-memory caching with multiple eviction strategies (LRU, LFU, FIFO)
//! - Redis caching with connection pooling and clustering
//! - Cache key generation and management
//! - Cache statistics and monitoring
//! - Cache warming and prefetching
//! - Distributed caching with consistency guarantees

pub mod config;
pub mod error;
pub mod memory;
pub mod moka_l1;
pub mod redis;
pub mod semantic;
pub mod stats;
pub mod strategies;
pub mod traits;
pub mod warming;

pub use moka_l1::{L1CacheEntry, MokaL1Cache};

pub use config::{
    AlertThresholds,
    CacheConfig,
    CacheType,
    CompressionAlgorithm,
    CompressionConfig,
    EncryptionAlgorithm,
    EncryptionConfig,
    LogLevel,
    MemoryConfig,
    MonitoringConfig,
    PerformanceConfig,
    RedisConfig,
};
pub use error::*;
pub use memory::*;
pub use redis::*;
pub use semantic::{SemanticCache, SemanticCacheConfig, SemanticCacheEntry, SemanticCacheStats};
pub use stats::*;
pub use strategies::{
    ARCStrategy,
    ClockStrategy,
    EvictionStrategy as CacheEvictionStrategy,
    FIFOStrategy,
    LFUStrategy,
    LRUStrategy,
    RandomStrategy,
    TTLStrategy,
};
pub use traits::*;
pub use warming::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_memory_cache() {
        let cache = MemoryCache::new(100, Duration::from_secs(60));

        // Test set and get
        cache
            .set("key1".to_string(), "value1".to_string(), None)
            .await
            .unwrap();
        let value = cache.get(&"key1".to_string()).await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Test delete
        cache.delete(&"key1".to_string()).await.unwrap();
        let value = cache.get(&"key1".to_string()).await.unwrap();
        assert_eq!(value, None);

        // Test exists
        cache
            .set("key2".to_string(), "value2".to_string(), None)
            .await
            .unwrap();
        assert!(cache.exists(&"key2".to_string()).await.unwrap());
        assert!(!cache.exists(&"key3".to_string()).await.unwrap());

        // Test size
        assert_eq!(cache.size().await.unwrap(), 1);

        // Test clear
        cache.clear().await.unwrap();
        assert_eq!(cache.size().await.unwrap(), 0);
    }
}
