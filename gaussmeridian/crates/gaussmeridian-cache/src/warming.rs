//! Cache warming and prefetching

use async_trait::async_trait;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::interval;

use crate::{error::CacheError, traits::Cache};

/// Cache warming configuration
#[derive(Debug, Clone)]
pub struct WarmingConfig {
    pub enabled: bool,
    pub warming_interval: Duration,
    pub prefetch_threshold: f64,
    pub max_prefetch_keys: usize,
    pub warming_strategy: WarmingStrategy,
    pub prefetch_strategy: PrefetchStrategy,
}

/// Warming strategy
#[derive(Debug, Clone)]
pub enum WarmingStrategy {
    /// Warm cache on startup
    Startup,
    /// Warm cache periodically
    Periodic,
    /// Warm cache on demand
    OnDemand,
    /// Warm cache based on access patterns
    AccessPattern,
}

/// Prefetch strategy
#[derive(Debug, Clone)]
pub enum PrefetchStrategy {
    /// Prefetch next likely keys
    Next,
    /// Prefetch based on access frequency
    Frequency,
    /// Prefetch based on time patterns
    TimePattern,
    /// Prefetch based on user behavior
    UserBehavior,
}

/// Cache warmer
pub struct CacheWarmer<K, V> {
    config: WarmingConfig,
    access_patterns: Arc<Mutex<AccessPatterns<K>>>,
    warming_keys: Arc<Mutex<Vec<K>>>,
    warming_stats: Arc<Mutex<WarmingStats>>,
    _phantom: PhantomData<V>,
}

/// Access patterns for warming
struct AccessPatterns<K> {
    key_sequence: Vec<K>,
    key_frequency: HashMap<K, u64>,
    key_timestamps: HashMap<K, Vec<Instant>>,
    last_access: HashMap<K, Instant>,
}

/// Warming statistics
#[derive(Debug, Clone)]
pub struct WarmingStats {
    pub total_warmed: u64,
    pub successful_warms: u64,
    pub failed_warms: u64,
    pub last_warming: Option<Instant>,
    pub warming_duration: Duration,
    pub cache_hit_rate_before: f64,
    pub cache_hit_rate_after: f64,
}

impl Default for WarmingStats {
    fn default() -> Self {
        Self {
            total_warmed: 0,
            successful_warms: 0,
            failed_warms: 0,
            last_warming: None,
            warming_duration: Duration::ZERO,
            cache_hit_rate_before: 0.0,
            cache_hit_rate_after: 0.0,
        }
    }
}

impl<K, V> CacheWarmer<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new cache warmer
    pub fn new(config: WarmingConfig) -> Self {
        Self {
            config,
            access_patterns: Arc::new(Mutex::new(AccessPatterns {
                key_sequence: Vec::new(),
                key_frequency: HashMap::new(),
                key_timestamps: HashMap::new(),
                last_access: HashMap::new(),
            })),
            warming_keys: Arc::new(Mutex::new(Vec::new())),
            warming_stats: Arc::new(Mutex::new(WarmingStats::default())),
            _phantom: PhantomData,
        }
    }

    /// Start warming process
    pub async fn start_warming<C>(
        &self,
        cache: Arc<C>,
        fetcher: Arc<dyn CacheFetcher<K, V> + Send + Sync>,
    ) where
        C: Cache<K, V> + Send + Sync + 'static,
    {
        if !self.config.enabled {
            return;
        }

        match self.config.warming_strategy {
            WarmingStrategy::Startup => {
                self.warm_on_startup(cache.clone(), fetcher.clone()).await;
            }
            WarmingStrategy::Periodic => {
                self.warm_periodically(cache.clone(), fetcher.clone()).await;
            }
            WarmingStrategy::OnDemand => {
                // On-demand warming is handled by the cache itself
            }
            WarmingStrategy::AccessPattern => {
                self.warm_by_access_pattern(cache.clone(), fetcher.clone())
                    .await;
            }
        }
    }

    /// Warm cache on startup
    async fn warm_on_startup<C>(
        &self,
        cache: Arc<C>,
        fetcher: Arc<dyn CacheFetcher<K, V> + Send + Sync>,
    ) where
        C: Cache<K, V> + Send + Sync + 'static,
    {
        let start = Instant::now();
        let keys = self.warming_keys.lock().await;

        let mut stats = self.warming_stats.lock().await;
        stats.last_warming = Some(start);

        let hit_rate_before = cache.get_stats().await.map(|s| s.hit_rate).unwrap_or(0.0);

        let mut warmed = 0;
        let mut successful = 0;

        for key in keys.iter() {
            warmed += 1;
            if let Ok(Some(value)) = fetcher.fetch(key.clone()).await {
                if cache.set(key.clone(), value, None).await.is_ok() {
                    successful += 1;
                }
            }
        }

        stats.total_warmed += warmed;
        stats.successful_warms += successful;
        stats.failed_warms += warmed - successful;
        stats.warming_duration = start.elapsed();

        let hit_rate_after = cache.get_stats().await.map(|s| s.hit_rate).unwrap_or(0.0);

        stats.cache_hit_rate_before = hit_rate_before;
        stats.cache_hit_rate_after = hit_rate_after;
    }

    /// Warm cache periodically
    async fn warm_periodically<C>(
        &self,
        cache: Arc<C>,
        fetcher: Arc<dyn CacheFetcher<K, V> + Send + Sync>,
    ) where
        C: Cache<K, V> + Send + Sync + 'static,
    {
        let mut interval = interval(self.config.warming_interval);

        loop {
            interval.tick().await;
            self.warm_on_startup(cache.clone(), fetcher.clone()).await;
        }
    }

    /// Warm cache based on access patterns
    async fn warm_by_access_pattern<C>(
        &self,
        cache: Arc<C>,
        fetcher: Arc<dyn CacheFetcher<K, V> + Send + Sync>,
    ) where
        C: Cache<K, V> + Send + Sync + 'static,
    {
        let patterns = self.access_patterns.lock().await;

        // Analyze access patterns and predict next likely keys
        let predicted_keys = self.predict_keys::<V>(&patterns).await;

        // Warm predicted keys
        for key in predicted_keys {
            if let Ok(Some(value)) = fetcher.fetch(key.clone()).await {
                let _ = cache.set(key, value, None).await;
            }
        }
    }

    /// Record access pattern
    pub async fn record_access(&self, key: K) {
        let mut patterns = self.access_patterns.lock().await;
        let now = Instant::now();

        patterns.key_sequence.push(key.clone());
        *patterns.key_frequency.entry(key.clone()).or_insert(0) += 1;
        patterns
            .key_timestamps
            .entry(key.clone())
            .or_insert_with(Vec::new)
            .push(now);
        patterns.last_access.insert(key, now);

        // Keep only recent history
        if patterns.key_sequence.len() > 1000 {
            patterns.key_sequence.remove(0);
        }
    }

    /// Predict next likely keys based on access patterns
    async fn predict_keys<V2>(&self, patterns: &AccessPatterns<K>) -> Vec<K>
    where
        V2: Clone + Send + Sync + 'static,
    {
        let mut predictions = Vec::new();

        // Predict based on frequency
        let mut freq_pairs: Vec<_> = patterns.key_frequency.iter().collect();
        freq_pairs.sort_by(|a, b| b.1.cmp(a.1));

        for (key, _) in freq_pairs.iter().take(10) {
            predictions.push((*key).clone());
        }

        // Predict based on sequence patterns
        if patterns.key_sequence.len() >= 2 {
            let last_key = &patterns.key_sequence[patterns.key_sequence.len() - 1];

            // Find keys that frequently follow the last accessed key
            for i in 0..patterns.key_sequence.len() - 1 {
                if &patterns.key_sequence[i] == last_key && i + 1 < patterns.key_sequence.len() {
                    let next_key = &patterns.key_sequence[i + 1];
                    if !predictions.contains(next_key) {
                        predictions.push(next_key.clone());
                    }
                }
            }
        }

        // Limit predictions
        predictions.truncate(self.config.max_prefetch_keys);
        predictions
    }

    /// Add keys for warming
    pub async fn add_warming_keys(&self, keys: Vec<K>) {
        let mut warming_keys = self.warming_keys.lock().await;
        warming_keys.extend(keys);
    }

    /// Get warming statistics
    pub async fn get_stats(&self) -> WarmingStats {
        self.warming_stats.lock().await.clone()
    }

    /// Prefetch keys based on current cache state
    pub async fn prefetch_keys<C>(
        &self,
        cache: Arc<C>,
        keys: Vec<K>,
        fetcher: Arc<dyn CacheFetcher<K, V> + Send + Sync>,
    ) -> Result<usize, CacheError>
    where
        C: Cache<K, V> + Send + Sync + 'static,
        CacheError: From<C::Error>,
    {
        let mut prefetched = 0;

        for key in keys {
            // Check if key is already in cache
            if cache.get(&key).await.map_err(CacheError::from)?.is_none() {
                // Fetch and cache the value
                if let Ok(Some(value)) = fetcher.fetch(key.clone()).await {
                    cache
                        .set(key, value, None)
                        .await
                        .map_err(CacheError::from)?;
                    prefetched += 1;
                }
            }
        }

        Ok(prefetched)
    }
}

/// Cache fetcher trait
#[async_trait]
pub trait CacheFetcher<K, V>: Send + Sync {
    async fn fetch(&self, key: K) -> Result<Option<V>, CacheError>;
}

/// Default cache fetcher that returns None
pub struct NoOpFetcher;

#[async_trait]
impl<K: Send + Sync + 'static, V> CacheFetcher<K, V> for NoOpFetcher {
    async fn fetch(&self, _key: K) -> Result<Option<V>, CacheError> {
        Ok(None)
    }
}

/// HTTP-based cache fetcher
#[cfg(feature = "warming")]
pub struct HttpFetcher {
    client: reqwest::Client,
    base_url: String,
}

#[cfg(feature = "warming")]
impl HttpFetcher {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }
}

#[cfg(feature = "warming")]
#[async_trait]
impl<K, V> CacheFetcher<K, V> for HttpFetcher
where
    K: ToString + Send + Sync + 'static,
    V: for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
{
    async fn fetch(&self, key: K) -> Result<Option<V>, CacheError> {
        let url = format!("{}/{}", self.base_url, key.to_string());

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CacheError::NetworkError(format!("HTTP request failed: {}", e)))?;

        if response.status().is_success() {
            let value = response.json::<V>().await.map_err(|e| {
                CacheError::DeserializationError(format!("Failed to deserialize response: {}", e))
            })?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

/// Database-based cache fetcher
pub struct DatabaseFetcher {
    #[allow(dead_code)] // Kept for future database integration
    connection_string: String,
}

impl DatabaseFetcher {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }
}

#[async_trait]
impl<K: Send + Sync + 'static, V> CacheFetcher<K, V> for DatabaseFetcher
where
    K: ToString + Send + Sync,
    V: for<'de> serde::Deserialize<'de> + Send + Sync,
{
    async fn fetch(&self, _key: K) -> Result<Option<V>, CacheError> {
        // This is a placeholder - in a real implementation, you would
        // connect to the database and fetch the value
        Ok(None)
    }
}

/// File-based cache fetcher
pub struct FileFetcher {
    base_path: std::path::PathBuf,
}

impl FileFetcher {
    pub fn new(base_path: std::path::PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait]
impl<K: Send + Sync + 'static, V> CacheFetcher<K, V> for FileFetcher
where
    K: ToString + Send + Sync,
    V: for<'de> serde::Deserialize<'de> + Send + Sync,
{
    async fn fetch(&self, key: K) -> Result<Option<V>, CacheError> {
        let file_path = self.base_path.join(format!("{}.json", key.to_string()));

        if file_path.exists() {
            let content = tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| CacheError::InternalError(format!("Failed to read file: {}", e)))?;

            let value = serde_json::from_str::<V>(&content).map_err(|e| {
                CacheError::DeserializationError(format!("Failed to deserialize JSON: {}", e))
            })?;

            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

/// Warming manager for coordinating multiple warmers
pub struct WarmingManager<K, V> {
    warmers: Vec<Arc<CacheWarmer<K, V>>>,
    config: WarmingConfig,
    _phantom: PhantomData<V>,
}

impl<K, V> WarmingManager<K, V>
where
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(config: WarmingConfig) -> Self {
        Self {
            warmers: Vec::new(),
            config,
            _phantom: PhantomData,
        }
    }

    pub fn add_warmer(&mut self, warmer: Arc<CacheWarmer<K, V>>) {
        self.warmers.push(warmer);
    }

    pub async fn start_all<C>(
        &self,
        cache: Arc<C>,
        fetcher: Arc<dyn CacheFetcher<K, V> + Send + Sync>,
    ) where
        C: Cache<K, V> + Send + Sync + 'static,
    {
        for warmer in &self.warmers {
            warmer.start_warming(cache.clone(), fetcher.clone()).await;
        }
    }

    pub async fn get_combined_stats(&self) -> WarmingStats {
        let mut combined = WarmingStats::default();

        for warmer in &self.warmers {
            let stats = warmer.get_stats().await;
            combined.total_warmed += stats.total_warmed;
            combined.successful_warms += stats.successful_warms;
            combined.failed_warms += stats.failed_warms;
        }

        combined
    }
}
