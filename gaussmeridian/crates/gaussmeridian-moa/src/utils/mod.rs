pub mod embeddings;
pub mod similarity;
pub mod evaluation;

use crate::{models, MoaResult};

pub fn calculate_diversity_score(responses: &[models::AgentResponse]) -> MoaResult<f32> {
    if responses.len() < 2 {
        return Ok(0.0);
    }
    
    let mut total_distance = 0.0;
    let mut comparisons = 0;
    
    for i in 0..responses.len() {
        for j in (i + 1)..responses.len() {
            let distance = similarity::jaccard_distance(&responses[i].content, &responses[j].content);
            total_distance += distance;
            comparisons += 1;
        }
    }
    
    Ok(total_distance / comparisons as f32)
}

pub fn calculate_average_similarity(responses: &[models::AgentResponse]) -> MoaResult<f32> {
    if responses.len() < 2 {
        return Ok(1.0);
    }
    
    let mut total_similarity = 0.0;
    let mut comparisons = 0;
    
    for i in 0..responses.len() {
        for j in (i + 1)..responses.len() {
            let similarity = similarity::cosine_similarity(&responses[i].content, &responses[j].content)?;
            total_similarity += similarity;
            comparisons += 1;
        }
    }
    
    Ok(total_similarity / comparisons as f32)
}

use chrono::{DateTime, Duration, Utc};
use futures::{stream::FuturesUnordered, StreamExt};
use rand::{distributions::WeightedIndex, prelude::*, RngCore};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
    time::{Duration as StdDuration, Instant},
};
use tokio::{
    sync::{Mutex, RwLock, Semaphore},
    time::sleep,
};
use tracing::warn;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: usize,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Backoff factor
    pub backoff_factor: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_factor: 2.0,
        }
    }
}

/// Retry a future with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    f: F,
    config: &RetryConfig,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: Debug,
{
    let mut delay = config.initial_delay_ms;
    let mut attempt = 0;
    
    loop {
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                attempt += 1;
                if attempt >= config.max_retries {
                    return Err(e);
                }
                
                warn!("Attempt {} failed: {:?}, retrying in {}ms", attempt, e, delay);
                sleep(StdDuration::from_millis(delay)).await;
                
                delay = (delay as f32 * config.backoff_factor) as u64;
                delay = delay.min(config.max_delay_ms);
            }
        }
    }
}

/// Rate limiter
pub struct RateLimiter {
    /// Maximum requests per second
    rate: f64,
    /// Maximum burst size
    burst: usize,
    /// Token bucket
    tokens: Arc<Mutex<TokenBucket>>,
}

/// Token bucket for rate limiting
struct TokenBucket {
    /// Current number of tokens
    tokens: f64,
    /// Last update time
    last_update: Instant,
    /// Maximum number of tokens
    max_tokens: f64,
    /// Tokens per second
    tokens_per_sec: f64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(rate: f64, burst: usize) -> Self {
        Self {
            rate,
            burst,
            tokens: Arc::new(Mutex::new(TokenBucket {
                tokens: burst as f64,
                last_update: Instant::now(),
                max_tokens: burst as f64,
                tokens_per_sec: rate,
            })),
        }
    }
    
    /// Acquire permission to proceed
    pub async fn acquire(&self) -> bool {
        let mut bucket = self.tokens.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_update).as_secs_f64();
        
        bucket.tokens = (bucket.tokens + elapsed * bucket.tokens_per_sec)
            .min(bucket.max_tokens);
        bucket.last_update = now;
        
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
    
    /// Wait until permission is granted
    pub async fn wait(&self) {
        while !self.acquire().await {
            sleep(StdDuration::from_millis(10)).await;
        }
    }
}

/// Cache with TTL
pub struct Cache<K, V> {
    /// Cache data
    data: Arc<RwLock<HashMap<K, (V, DateTime<Utc>)>>>,
    /// Time to live
    ttl: Duration,
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new cache
    pub fn new(ttl: Duration) -> Self {
        let cache = Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        };
        
        // Start cleanup task
        let data = cache.data.clone();
        let ttl = cache.ttl;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(StdDuration::from_secs(60));
            loop {
                interval.tick().await;
                Self::cleanup(&data, ttl).await;
            }
        });
        
        cache
    }
    
    /// Get a value from the cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let data = self.data.read().await;
        data.get(key)
            .filter(|(_, timestamp)| {
                Utc::now() - *timestamp < self.ttl
            })
            .map(|(value, _)| value.clone())
    }
    
    /// Set a value in the cache
    pub async fn set(&self, key: K, value: V) {
        let mut data = self.data.write().await;
        data.insert(key, (value, Utc::now()));
    }
    
    /// Remove expired entries
    async fn cleanup(data: &RwLock<HashMap<K, (V, DateTime<Utc>)>>, ttl: Duration) {
        let mut data = data.write().await;
        let now = Utc::now();
        data.retain(|_, (_, timestamp)| now - *timestamp < ttl);
    }
}

/// Weighted random selection
#[derive(Debug)]
pub struct WeightedSelector<T> {
    /// Items with weights
    items: Vec<(T, f64)>,
}

impl<T: Clone> WeightedSelector<T> {
    /// Create a new weighted selector
    pub fn new(items: Vec<(T, f64)>) -> Self {
        Self {
            items,
        }
    }
    
    /// Select an item based on weights
    pub fn select(&self, rng: &mut impl RngCore) -> Option<T> {
        if self.items.is_empty() {
            return None;
        }
        let weights: Vec<f64> = self.items.iter().map(|(_, w)| *w).collect();
        let dist = WeightedIndex::new(&weights).ok()?;
        Some(self.items[dist.sample(rng)].0.clone())
    }
    
    /// Select multiple items without replacement (naive implementation)
    pub fn select_multiple(&self, count: usize, rng: &mut impl RngCore) -> Vec<T> {
        if self.items.is_empty() || count == 0 {
            return Vec::new();
        }
        let mut selected_items = Vec::new();
        let mut available_items = self.items.clone();
        
        for _ in 0..count.min(available_items.len()) {
            if available_items.is_empty() { break; }
            let weights: Vec<f64> = available_items.iter().map(|(_, w)| *w).collect();
            if weights.iter().all(|&w| w == 0.0) { break; } // Avoid panic if all weights are zero
            let dist = WeightedIndex::new(&weights).unwrap();
            let selected_index = dist.sample(rng);
            selected_items.push(available_items.remove(selected_index).0);
        }
        selected_items
    }
    
    /// Update weights
    pub fn update_weights(&mut self, items: Vec<(T, f64)>) {
        self.items = items;
    }
}

/// Concurrent task executor with rate limiting
pub struct TaskExecutor<T> {
    /// Maximum concurrent tasks
    max_concurrent: usize,
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,
    /// Task results
    results: Arc<Mutex<Vec<T>>>,
}

impl<T: Send + 'static> TaskExecutor<T> {
    /// Create a new task executor
    pub fn new(max_concurrent: usize, rate: f64, burst: usize) -> Self {
        Self {
            max_concurrent,
            rate_limiter: RateLimiter::new(rate, burst),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            results: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Execute tasks concurrently
    pub async fn execute<F, Fut>(&self, tasks: Vec<F>) -> Vec<T>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = T> + Send,
        T: Clone,
    {
        let mut futures = FuturesUnordered::new();
        
        for task in tasks {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let rate_limiter = &self.rate_limiter;
            let results = self.results.clone();
            
            futures.push(async move {
                rate_limiter.wait().await;
                let result = task().await;
                results.lock().await.push(result);
                drop(permit);
            });
        }
        
        while let Some(_) = futures.next().await {}
        
        let results = self.results.lock().await;
        results.clone()
    }
}

/// JSON serialization helpers
pub mod json {
    use super::*;
    use serde_json::Value;
    use std::path::Path;
    
    /// Load JSON from file
    pub async fn load_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> std::io::Result<T> {
        let content = tokio::fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&content)?)
    }
    
    /// Save JSON to file
    pub async fn save_json<T: Serialize>(
        value: &T,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(value)?;
        tokio::fs::write(path, content).await
    }
    
    /// Merge JSON values
    pub fn merge_json(a: Value, b: Value) -> Value {
        match (a, b) {
            (Value::Object(mut a), Value::Object(b)) => {
                for (k, v) in b {
                    if let Some(av) = a.get_mut(&k) {
                        *av = merge_json(av.clone(), v);
                    } else {
                        a.insert(k, v);
                    }
                }
                Value::Object(a)
            }
            (_, b) => b,
        }
    }
}

/// String manipulation helpers
pub mod strings {
    /// Truncate string to max length with ellipsis
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }
    
    /// Split string into chunks
    pub fn chunk_string(s: &str, chunk_size: usize) -> Vec<String> {
        s.chars()
            .collect::<Vec<_>>()
            .chunks(chunk_size)
            .map(|c| c.iter().collect())
            .collect()
    }
}

/// Time helpers
pub mod time {
    use super::*;
    
    /// Format duration as human readable string
    pub fn format_duration(duration: StdDuration) -> String {
        let secs = duration.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
        }
    }
    
    /// Parse duration from string
    pub fn parse_duration(s: &str) -> Option<StdDuration> {
        let mut total_secs = 0;
        let mut num = 0;
        
        for c in s.chars() {
            match c {
                'h' => {
                    total_secs += num * 3600;
                    num = 0;
                }
                'm' => {
                    total_secs += num * 60;
                    num = 0;
                }
                's' => {
                    total_secs += num;
                    num = 0;
                }
                c if c.is_digit(10) => {
                    num = num * 10 + c.to_digit(10)? as u64;
                }
                _ => return None,
            }
        }
        
        Some(StdDuration::from_secs(total_secs))
    }
}

pub struct WeightedSelection {
    distribution: WeightedIndex<f64>
}

impl WeightedSelection {
    pub fn new(weights: Vec<f64>) -> Self {
        Self {
            distribution: WeightedIndex::new(&weights).unwrap()
        }
    }

    pub fn sample(&self) -> usize {
        self.distribution.sample(&mut thread_rng())
    }
}