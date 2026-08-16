//! In-memory cache implementations

use async_trait::async_trait;
use std::collections::BTreeMap;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::{
    config::{EvictionStrategy, MemoryConfig},
    error::CacheError,
    stats::{CacheAnalyzer, CacheMetrics, CacheStats},
    traits::{Cache, CacheWithEviction, CacheWithStats, CacheWithWarming},
};

/// Advanced in-memory cache with multiple eviction strategies
pub struct MemoryCache<K, V> {
    storage: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    config: MemoryConfig,
    metrics: CacheMetrics,
    analyzer: CacheAnalyzer,
    eviction_queue: Arc<Mutex<EvictionQueue<K>>>,
    access_order: Arc<Mutex<VecDeque<K>>>,
    frequency_map: Arc<Mutex<HashMap<K, u64>>>,
    expiration_queue: Arc<Mutex<BTreeMap<Instant, Vec<K>>>>,
}

/// Cache entry with metadata
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
    expires_at: Option<Instant>,
    size: usize,
}

/// Eviction queue for different strategies
struct EvictionQueue<K> {
    lru_queue: VecDeque<K>,
    lfu_map: HashMap<K, u64>,
    fifo_queue: VecDeque<K>,
    clock_hand: usize,
    clock_references: Vec<bool>,
    clock_keys: Vec<K>,
}

impl<K: Clone + Eq + Hash> EvictionQueue<K> {
    fn new() -> Self {
        Self {
            lru_queue: VecDeque::new(),
            lfu_map: HashMap::new(),
            fifo_queue: VecDeque::new(),
            clock_hand: 0,
            clock_references: Vec::new(),
            clock_keys: Vec::new(),
        }
    }

    fn add_to_lru(&mut self, key: K) {
        self.lru_queue.push_front(key);
    }

    fn update_lru(&mut self, key: &K) -> bool {
        if let Some(pos) = self.lru_queue.iter().position(|k| k == key) {
            let k = self.lru_queue.remove(pos).unwrap();
            self.lru_queue.push_front(k);
            true
        } else {
            false
        }
    }

    fn get_lru(&mut self) -> Option<K> {
        self.lru_queue.pop_back()
    }

    fn add_to_lfu(&mut self, key: K) {
        self.lfu_map.insert(key, 1);
    }

    fn update_lfu(&mut self, key: &K) -> bool {
        if let Some(freq) = self.lfu_map.get_mut(key) {
            *freq += 1;
            true
        } else {
            false
        }
    }

    fn get_lfu(&mut self) -> Option<K> {
        if let Some((key, &_freq)) = self.lfu_map.iter().min_by_key(|(_, &freq)| freq) {
            let key = key.clone();
            self.lfu_map.remove(&key);
            Some(key)
        } else {
            None
        }
    }

    fn add_to_fifo(&mut self, key: K) {
        self.fifo_queue.push_back(key);
    }

    fn get_fifo(&mut self) -> Option<K> {
        self.fifo_queue.pop_front()
    }

    fn add_to_clock(&mut self, key: K) {
        self.clock_keys.push(key);
        self.clock_references.push(true);
    }

    fn update_clock(&mut self, key: &K) -> bool {
        if let Some(pos) = self.clock_keys.iter().position(|k| k == key) {
            self.clock_references[pos] = true;
            true
        } else {
            false
        }
    }

    fn get_clock(&mut self) -> Option<K> {
        if self.clock_keys.is_empty() {
            return None;
        }

        loop {
            if self.clock_references[self.clock_hand] {
                // Give second chance
                self.clock_references[self.clock_hand] = false;
                self.clock_hand = (self.clock_hand + 1) % self.clock_keys.len();
            } else {
                // Evict this entry
                let key = self.clock_keys.remove(self.clock_hand);
                self.clock_references.remove(self.clock_hand);
                if self.clock_hand >= self.clock_keys.len() && !self.clock_keys.is_empty() {
                    self.clock_hand = 0;
                }
                return Some(key);
            }
        }
    }
}

impl<K, V> MemoryCache<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new memory cache
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        let config = MemoryConfig {
            max_size,
            ttl,
            eviction_strategy: EvictionStrategy::LRU,
            enable_stats: true,
        };
        Self::with_config(config)
    }

    /// Create a new memory cache with configuration
    pub fn with_config(config: MemoryConfig) -> Self {
        let metrics = CacheMetrics::new(config.max_size);
        let analyzer = CacheAnalyzer::new(config.max_size);

        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics,
            analyzer,
            eviction_queue: Arc::new(Mutex::new(EvictionQueue::new())),
            access_order: Arc::new(Mutex::new(VecDeque::new())),
            frequency_map: Arc::new(Mutex::new(HashMap::new())),
            expiration_queue: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.metrics.get_stats()
    }

    /// Get detailed cache statistics
    pub fn get_detailed_stats(&self) -> crate::stats::DetailedCacheStats {
        self.metrics.get_detailed_stats()
    }

    /// Get cache health status
    pub fn get_health(&self) -> crate::stats::CacheHealth {
        self.analyzer.get_health()
    }

    /// Get cache recommendations
    pub fn get_recommendations(&self) -> Vec<String> {
        self.analyzer.get_recommendations()
    }

    /// Evict items based on the configured strategy
    async fn evict_if_needed(&self) -> Result<(), CacheError> {
        let current_size = {
            let storage = self.storage.read().unwrap();
            storage.len()
        };

        if current_size >= self.config.max_size {
            let eviction_count = (current_size - self.config.max_size + 1).min(10);

            match self.config.eviction_strategy {
                EvictionStrategy::LRU => {
                    self.evict_lru(eviction_count).await?;
                    Ok(())
                }
                EvictionStrategy::LFU => {
                    self.evict_lfu(eviction_count).await?;
                    Ok(())
                }
                EvictionStrategy::FIFO => {
                    self.evict_fifo(eviction_count).await?;
                    Ok(())
                }
                EvictionStrategy::Random => {
                    self.evict_random(eviction_count).await?;
                    Ok(())
                }
                EvictionStrategy::TTL => {
                    self.evict_expired().await?;
                    Ok(())
                }
                EvictionStrategy::ARC => {
                    self.evict_arc(eviction_count).await?;
                    Ok(())
                }
                EvictionStrategy::Clock => {
                    self.evict_clock(eviction_count).await?;
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Evict using LRU strategy
    async fn evict_lru(&self, count: usize) -> Result<(), CacheError> {
        let mut _evicted = 0;
        for _ in 0..count {
            if let Some(key) = self.eviction_queue.lock().unwrap().get_lru() {
                if self.storage.write().unwrap().remove(&key).is_some() {
                    _evicted += 1;
                }
            } else {
                break;
            }
        }
        self.metrics.record_eviction();
        Ok(())
    }

    /// Evict using LFU strategy
    async fn evict_lfu(&self, count: usize) -> Result<(), CacheError> {
        let mut _evicted = 0;
        for _ in 0..count {
            if let Some(key) = self.eviction_queue.lock().unwrap().get_lfu() {
                if self.storage.write().unwrap().remove(&key).is_some() {
                    self.eviction_queue.lock().unwrap().lfu_map.remove(&key);
                    _evicted += 1;
                }
            } else {
                break;
            }
        }
        self.metrics.record_eviction();
        Ok(())
    }

    /// Evict using FIFO strategy
    async fn evict_fifo(&self, count: usize) -> Result<(), CacheError> {
        let mut _evicted = 0;
        for _ in 0..count {
            if let Some(key) = self.eviction_queue.lock().unwrap().get_fifo() {
                if self.storage.write().unwrap().remove(&key).is_some() {
                    _evicted += 1;
                }
            } else {
                break;
            }
        }
        self.metrics.record_eviction();
        Ok(())
    }

    /// Evict expired items
    async fn evict_expired(&self) -> Result<(), CacheError> {
        let mut evicted = 0;
        let now = std::time::Instant::now();
        let mut storage = self.storage.write().unwrap();

        let expired_keys: Vec<_> = storage
            .iter()
            .filter_map(|(key, entry)| {
                if let Some(expiry) = entry.expires_at {
                    if now >= expiry {
                        Some(key.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            if storage.remove(&key).is_some() {
                evicted += 1;
            }
        }

        if evicted > 0 {
            self.metrics.record_expiration();
        }

        Ok(())
    }

    /// Evict random items
    async fn evict_random(&self, count: usize) -> Result<(), CacheError> {
        let mut storage = self.storage.write().unwrap();

        let keys: Vec<K> = storage.keys().cloned().collect();
        let mut rng = fastrand::Rng::new();

        for _ in 0..count.min(keys.len()) {
            if let Some(key) = keys.get(rng.usize(..keys.len())) {
                storage.remove(key);
            }
        }

        self.metrics.record_eviction();
        Ok(())
    }

    /// Evict using ARC (Adaptive Replacement Cache) strategy
    async fn evict_arc(&self, count: usize) -> Result<(), CacheError> {
        // Simplified ARC implementation
        self.evict_lru(count).await?;
        Ok(())
    }

    /// Evict using Clock algorithm
    async fn evict_clock(&self, count: usize) -> Result<(), CacheError> {
        let mut queue = self.eviction_queue.lock().unwrap();

        for _ in 0..count {
            if let Some(key) = queue.get_clock() {
                let mut storage = self.storage.write().unwrap();
                storage.remove(&key);
            }
        }

        self.metrics.record_eviction();
        Ok(())
    }

    /// Calculate entry size
    fn calculate_size(&self, value: &V) -> usize {
        std::mem::size_of::<V>() + std::mem::size_of_val(value)
    }
}

#[async_trait]
impl<K, V> Cache<K, V> for MemoryCache<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    type Error = CacheError;

    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
        let start = Instant::now();

        // Clean up expired entries first
        self.evict_expired().await?;

        // First, check if the key exists and get its value
        let value = {
            let storage = self.storage.read().unwrap();
            if let Some(entry) = storage.get(key) {
                // Check if expired
                if let Some(expires_at) = entry.expires_at {
                    if Instant::now() > expires_at {
                        self.metrics.record_miss();
                        self.metrics.record_get_time(start.elapsed());
                        return Ok(None);
                    }
                }
                Some(entry.value.clone())
            } else {
                None
            }
        };

        // If we found a value, update access metadata
        if let Some(_) = &value {
            // Update access metadata in a separate scope
            {
                let mut storage = self.storage.write().unwrap();
                if let Some(entry) = storage.get_mut(key) {
                    entry.last_accessed = Instant::now();
                    entry.access_count += 1;
                }
            }

            // Update eviction queues
            {
                let mut queue = self.eviction_queue.lock().unwrap();
                match self.config.eviction_strategy {
                    EvictionStrategy::LRU => {
                        queue.update_lru(key);
                    }
                    EvictionStrategy::LFU => {
                        queue.update_lfu(key);
                    }
                    EvictionStrategy::FIFO => { /* FIFO doesn't update on access */ }
                    EvictionStrategy::Random => { /* Random doesn't update on access */ }
                    EvictionStrategy::TTL => { /* TTL doesn't update on access */ }
                    EvictionStrategy::ARC => { /* ARC updates handled separately */ }
                    EvictionStrategy::Clock => {
                        queue.update_clock(key);
                    }
                }
            }

            self.metrics.record_hit();
            self.metrics.record_get_time(start.elapsed());
        } else {
            self.metrics.record_miss();
            self.metrics.record_get_time(start.elapsed());
        }

        Ok(value)
    }

    async fn set(&self, key: K, value: V, ttl: Option<Duration>) -> Result<(), Self::Error> {
        let start = Instant::now();

        // Evict if needed
        self.evict_if_needed().await?;

        let size = self.calculate_size(&value);
        let expires_at = ttl.map(|duration| Instant::now() + duration);

        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            expires_at,
            size,
        };

        let mut storage = self.storage.write().unwrap();
        storage.insert(key.clone(), entry);

        // Update eviction queues
        drop(storage);
        {
            let mut queue = self.eviction_queue.lock().unwrap();
            match self.config.eviction_strategy {
                EvictionStrategy::LRU => {
                    queue.add_to_lru(key.clone());
                }
                EvictionStrategy::LFU => {
                    queue.add_to_lfu(key.clone());
                }
                EvictionStrategy::FIFO => {
                    queue.add_to_fifo(key.clone());
                }
                EvictionStrategy::Random => { /* Random strategy doesn't use queue */ }
                EvictionStrategy::TTL => { /* TTL strategy doesn't use queue */ }
                EvictionStrategy::ARC => { /* ARC strategy doesn't use queue */ }
                EvictionStrategy::Clock => {
                    queue.add_to_clock(key.clone());
                }
            }
        }

        // Update expiration queue
        if let Some(expires_at) = expires_at {
            let mut exp_queue = self.expiration_queue.lock().unwrap();
            exp_queue
                .entry(expires_at)
                .or_insert_with(Vec::new)
                .push(key);
        }

        self.metrics.record_set_time(start.elapsed());
        self.metrics
            .set_current_size(self.storage.read().unwrap().len());

        Ok(())
    }

    async fn delete(&self, key: &K) -> Result<(), Self::Error> {
        let mut storage = self.storage.write().unwrap();
        storage.remove(key);
        self.metrics.set_current_size(storage.len());
        Ok(())
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        let mut storage = self.storage.write().unwrap();
        storage.clear();

        let mut queue = self.eviction_queue.lock().unwrap();
        queue.lru_queue.clear();
        queue.lfu_map.clear();
        queue.fifo_queue.clear();
        queue.clock_keys.clear();
        queue.clock_references.clear();
        queue.clock_hand = 0;

        let mut exp_queue = self.expiration_queue.lock().unwrap();
        exp_queue.clear();

        self.metrics.set_current_size(0);
        Ok(())
    }

    async fn exists(&self, key: &K) -> Result<bool, Self::Error> {
        let storage = self.storage.read().unwrap();
        Ok(storage.contains_key(key))
    }

    async fn size(&self) -> Result<usize, Self::Error> {
        let storage = self.storage.read().unwrap();
        Ok(storage.len())
    }

    async fn get_stats(&self) -> Result<CacheStats, Self::Error> {
        Ok(self.get_stats())
    }
}

#[async_trait]
impl<K, V> CacheWithStats<K, V> for MemoryCache<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn get_detailed_stats(&self) -> crate::stats::DetailedCacheStats {
        self.get_detailed_stats()
    }

    fn reset_stats(&mut self) {
        self.metrics.reset();
    }
}

#[async_trait]
impl<K, V> CacheWithEviction<K, V> for MemoryCache<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn evict_lru(&mut self, count: usize) -> Result<usize, Self::Error> {
        let mut evicted = 0;
        for _ in 0..count {
            if let Some(key) = self.eviction_queue.lock().unwrap().get_lru() {
                if self.storage.write().unwrap().remove(&key).is_some() {
                    evicted += 1;
                }
            } else {
                break;
            }
        }
        Ok(evicted)
    }

    async fn evict_lfu(&mut self, count: usize) -> Result<usize, Self::Error> {
        let mut evicted = 0;
        for _ in 0..count {
            if let Some(key) = self.eviction_queue.lock().unwrap().get_lfu() {
                if self.storage.write().unwrap().remove(&key).is_some() {
                    self.eviction_queue.lock().unwrap().lfu_map.remove(&key);
                    evicted += 1;
                }
            } else {
                break;
            }
        }
        Ok(evicted)
    }

    async fn evict_fifo(&mut self, count: usize) -> Result<usize, Self::Error> {
        let mut evicted = 0;
        for _ in 0..count {
            if let Some(key) = self.eviction_queue.lock().unwrap().get_fifo() {
                if self.storage.write().unwrap().remove(&key).is_some() {
                    evicted += 1;
                }
            } else {
                break;
            }
        }
        Ok(evicted)
    }

    async fn evict_expired(&mut self) -> Result<usize, Self::Error> {
        let mut evicted = 0;
        let now = std::time::Instant::now();
        let mut storage = self.storage.write().unwrap();

        let expired_keys: Vec<_> = storage
            .iter()
            .filter_map(|(key, entry)| {
                if let Some(expiry) = entry.expires_at {
                    if now >= expiry {
                        Some(key.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        for key in expired_keys {
            if storage.remove(&key).is_some() {
                evicted += 1;
            }
        }

        Ok(evicted)
    }
}

#[async_trait]
impl<K, V> CacheWithWarming<K, V> for MemoryCache<K, V>
where
    K: Clone + Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn warm_cache(&self, keys: Vec<K>) -> Result<usize, Self::Error> {
        // This is a placeholder - in a real implementation, you would
        // fetch values for these keys from a data source
        Ok(keys.len())
    }

    async fn prefetch(
        &self,
        keys: Vec<K>,
        _fetcher: Box<dyn Fn(K) -> V + Send + Sync>,
    ) -> Result<usize, Self::Error> {
        // This is a placeholder - in a real implementation, you would
        // use the fetcher to get values for keys not in cache
        Ok(keys.len())
    }
}
