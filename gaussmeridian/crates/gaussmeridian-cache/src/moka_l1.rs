//! Moka-backed L1 in-process exact-match cache.
//!
//! `MokaL1Cache` is the authoritative L1 cache for GaussMeridian. It wraps
//! `moka::future::Cache` which provides:
//! - Lock-free concurrent reads
//! - Time-to-live (TTL) and time-to-idle (TTI) eviction
//! - Bounded capacity with automatic LRU eviction
//! - Async-native API (no Mutex needed)
//!
//! # Cache key
//! For chat completions: SHA-256 of `(model, sorted_messages_json)` → hex string.
//! Key derivation is done by the caller (CacheMiddleware) before lookup.
//!
//! # Semantics
//! - `get` → `None` on miss; `Some(value)` on hit (increments hit counter)
//! - `set` → inserts with the configured TTL
//! - All operations are O(1) amortised

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;

/// Cached response body alongside routing metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1CacheEntry {
    /// Full JSON response body from the provider
    pub response_body: String,
    /// Model that produced this response
    pub model: String,
    /// Provider that produced this response
    pub provider: String,
}

/// Moka-backed in-process exact-match cache (L1).
///
/// Keyed by a `String` cache key (hex SHA-256 of normalised request). Values are
/// `L1CacheEntry`. All Moka eviction is async and non-blocking.
#[derive(Clone)]
pub struct MokaL1Cache {
    inner: Cache<String, L1CacheEntry>,
}

impl MokaL1Cache {
    /// Create a new L1 cache.
    ///
    /// `max_capacity` — maximum number of entries before LRU eviction.
    /// `ttl`          — maximum time-to-live per entry.
    pub fn new(max_capacity: u64, ttl: Duration) -> Self {
        let inner = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(ttl)
            .build();

        Self { inner }
    }

    /// Look up a cached response by exact key.
    pub async fn get(&self, key: &str) -> Option<L1CacheEntry> {
        let result = self.inner.get(key).await;
        if result.is_some() {
            debug!(cache = "l1", key = %key, "cache hit");
        }
        result
    }

    /// Insert a response into the cache.
    pub async fn set(&self, key: String, entry: L1CacheEntry) {
        debug!(cache = "l1", key = %key, "cache insert");
        self.inner.insert(key, entry).await;
    }

    /// Invalidate a specific key (e.g. after provider error on that key).
    pub async fn invalidate(&self, key: &str) {
        self.inner.invalidate(key).await;
    }

    /// Drain the entire cache (e.g. on config reload).
    pub async fn clear(&self) {
        self.inner.invalidate_all();
        // Allow background tasks to run
        self.inner.run_pending_tasks().await;
    }

    /// Number of entries currently in the cache.
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// Estimated size in number of entries (Moka reports weighted or entry count).
    pub fn weighted_size(&self) -> u64 {
        self.inner.weighted_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_hit_miss() {
        let cache = MokaL1Cache::new(100, Duration::from_secs(60));

        let entry = L1CacheEntry {
            response_body: r#"{"id":"test"}"#.to_string(),
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
        };

        assert!(cache.get("key1").await.is_none());
        cache.set("key1".to_string(), entry.clone()).await;
        let hit = cache.get("key1").await;
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().model, "gpt-4o");
    }

    #[tokio::test]
    async fn test_invalidate() {
        let cache = MokaL1Cache::new(100, Duration::from_secs(60));
        let entry = L1CacheEntry {
            response_body: "{}".to_string(),
            model: "claude-3".to_string(),
            provider: "anthropic".to_string(),
        };
        cache.set("key2".to_string(), entry).await;
        assert!(cache.get("key2").await.is_some());
        cache.invalidate("key2").await;
        assert!(cache.get("key2").await.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = MokaL1Cache::new(100, Duration::from_secs(60));
        let entry = L1CacheEntry {
            response_body: "{}".to_string(),
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
        };
        cache.set("a".to_string(), entry.clone()).await;
        cache.set("b".to_string(), entry).await;
        cache.clear().await;
        assert_eq!(cache.entry_count(), 0);
    }
}
