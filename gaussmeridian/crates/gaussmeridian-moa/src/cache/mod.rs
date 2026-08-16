use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, sync::Arc, time::Duration};
use tokio::sync::RwLock;

/// Generic cache implementation with TTL support
pub struct Cache<K, V>
where
    K: Eq + Hash + Clone + Serialize + DeserializeOwned,
    V: Clone + Serialize + DeserializeOwned,
{
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    ttl: Duration,
}

/// Cache entry with value and expiration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<V> {
    pub value: V,
    pub expiry: DateTime<Utc>,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone + Serialize + DeserializeOwned,
    V: Clone + Serialize + DeserializeOwned,
{
    /// Create a new cache with specified TTL
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Get a value from the cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if entry.expiry > Utc::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Set a value in the cache
    pub async fn set(&self, key: K, value: V) {
        let mut entries = self.entries.write().await;
        let expiry = Utc::now() + chrono::Duration::from_std(self.ttl).unwrap();
        entries.insert(key, CacheEntry { value, expiry });
    }

    /// Remove a value from the cache
    pub async fn remove(&self, key: &K) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// Clear all expired entries
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let now = Utc::now();
        entries.retain(|_, entry| entry.expiry > now);
    }

    /// Clear the entire cache
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }
}

/// Serializable cache implementation
pub struct SerializableCache<K, V>
where
    K: Eq + Hash + Clone + Serialize + DeserializeOwned,
    V: Clone + Serialize + DeserializeOwned,
{
    inner: Cache<K, V>,
}

impl<K, V> SerializableCache<K, V>
where
    K: Eq + Hash + Clone + Serialize + DeserializeOwned,
    V: Clone + Serialize + DeserializeOwned,
{
    /// Create a new serializable cache
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Cache::new(ttl),
        }
    }

    /// Save cache to disk
    pub async fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let entries = self.inner.entries.read().await;
        let file = std::fs::File::create(path)?;
        serde_json::to_writer(file, &*entries)?;
        Ok(())
    }

    /// Load cache from disk
    pub async fn load(&self, path: &std::path::Path) -> std::io::Result<()> {
        let file = std::fs::File::open(path)?;
        let loaded: HashMap<K, CacheEntry<V>> = serde_json::from_reader(file)?;
        let mut entries = self.inner.entries.write().await;
        *entries = loaded;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache: Cache<String, String> = Cache::new(Duration::from_secs(1));

        cache.set("key1".to_string(), "value1".to_string()).await;
        assert_eq!(
            cache.get(&"key1".to_string()).await,
            Some("value1".to_string())
        );

        cache.remove(&"key1".to_string()).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache: Cache<String, String> = Cache::new(Duration::from_millis(100));

        cache.set("key1".to_string(), "value1".to_string()).await;
        assert_eq!(
            cache.get(&"key1".to_string()).await,
            Some("value1".to_string())
        );

        sleep(Duration::from_millis(200)).await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache: Cache<String, String> = Cache::new(Duration::from_millis(100));

        cache.set("key1".to_string(), "value1".to_string()).await;
        cache.set("key2".to_string(), "value2".to_string()).await;

        sleep(Duration::from_millis(200)).await;
        cache.cleanup().await;

        assert_eq!(cache.get(&"key1".to_string()).await, None);
        assert_eq!(cache.get(&"key2".to_string()).await, None);
    }
}
