//! Cache traits and interfaces

use async_trait::async_trait;
use std::time::Duration;

use crate::stats::{CacheStats, DetailedCacheStats};

/// Core cache trait for different cache implementations
#[async_trait]
pub trait Cache<K, V>: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get a value from cache
    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error>;

    /// Set a value in cache with optional TTL
    async fn set(&self, key: K, value: V, ttl: Option<Duration>) -> Result<(), Self::Error>;

    /// Delete a value from cache
    async fn delete(&self, key: &K) -> Result<(), Self::Error>;

    /// Clear all values from cache
    async fn clear(&self) -> Result<(), Self::Error>;

    /// Check if a key exists in cache
    async fn exists(&self, key: &K) -> Result<bool, Self::Error>;

    /// Get the number of items in cache
    async fn size(&self) -> Result<usize, Self::Error>;

    /// Get cache statistics
    async fn get_stats(&self) -> Result<CacheStats, Self::Error>;
}

/// Cache with statistics capabilities
#[async_trait]
pub trait CacheWithStats<K, V>: Cache<K, V> {
    /// Get detailed cache statistics
    fn get_detailed_stats(&self) -> DetailedCacheStats;

    /// Reset cache statistics
    fn reset_stats(&mut self);
}

/// Cache with eviction strategies
#[async_trait]
pub trait CacheWithEviction<K, V>: Cache<K, V> {
    /// Evict items using LRU strategy
    async fn evict_lru(&mut self, count: usize) -> Result<usize, Self::Error>;

    /// Evict items using LFU strategy
    async fn evict_lfu(&mut self, count: usize) -> Result<usize, Self::Error>;

    /// Evict items using FIFO strategy
    async fn evict_fifo(&mut self, count: usize) -> Result<usize, Self::Error>;

    /// Evict expired items
    async fn evict_expired(&mut self) -> Result<usize, Self::Error>;
}

/// Cache with warming capabilities
#[async_trait]
pub trait CacheWithWarming<K, V>: Cache<K, V> {
    /// Warm cache with predefined keys
    async fn warm_cache(&self, keys: Vec<K>) -> Result<usize, Self::Error>;

    /// Prefetch values for given keys
    async fn prefetch(
        &self,
        keys: Vec<K>,
        fetcher: Box<dyn Fn(K) -> V + Send + Sync>,
    ) -> Result<usize, Self::Error>;
}

/// Cache with distributed capabilities
#[async_trait]
pub trait DistributedCache<K, V>: Cache<K, V> {
    /// Get cache from multiple nodes
    async fn get_distributed(&self, key: &K) -> Result<Option<V>, Self::Error>;

    /// Set cache across multiple nodes
    async fn set_distributed(
        &self,
        key: K,
        value: V,
        ttl: Option<Duration>,
    ) -> Result<(), Self::Error>;

    /// Invalidate cache across all nodes
    async fn invalidate_distributed(&self, key: &K) -> Result<(), Self::Error>;
}

/// Cache with compression capabilities
#[async_trait]
pub trait CompressedCache<K, V>: Cache<K, V> {
    /// Get compressed value
    async fn get_compressed(&self, key: &K) -> Result<Option<Vec<u8>>, Self::Error>;

    /// Set compressed value
    async fn set_compressed(
        &self,
        key: K,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Result<(), Self::Error>;
}

/// Cache with encryption capabilities
#[async_trait]
pub trait EncryptedCache<K, V>: Cache<K, V> {
    /// Get encrypted value
    async fn get_encrypted(&self, key: &K, encryption_key: &[u8])
        -> Result<Option<V>, Self::Error>;

    /// Set encrypted value
    async fn set_encrypted(
        &self,
        key: K,
        value: V,
        encryption_key: &[u8],
        ttl: Option<Duration>,
    ) -> Result<(), Self::Error>;
}
