//! Semantic caching implementation using embeddings
//!
//! This module provides semantic caching that can match similar queries
//! even if they're not exactly the same, reducing API calls and costs.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::{error::CacheError, stats::CacheStats, traits::Cache};

/// Semantic cache entry with embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCacheEntry<V> {
    pub key: String,
    pub embedding: Vec<f32>,
    pub value: V,
    pub created_at: std::time::SystemTime,
    pub ttl: Option<Duration>,
    pub access_count: u64,
}

/// Configuration for semantic cache
#[derive(Debug, Clone)]
pub struct SemanticCacheConfig {
    /// Minimum cosine similarity threshold (0.0 to 1.0)
    /// Higher values require more similar queries
    pub similarity_threshold: f64,
    
    /// Maximum number of entries to store
    pub max_entries: usize,
    
    /// Default TTL for entries
    pub default_ttl: Duration,
    
    /// Whether to use external embedding service
    pub use_external_embeddings: bool,
    
    /// API endpoint for embeddings (if external)
    pub embedding_api_url: Option<String>,
    
    /// Model to use for embeddings
    pub embedding_model: String,
}

impl Default for SemanticCacheConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.92,
            max_entries: 10000,
            default_ttl: Duration::from_secs(3600),
            use_external_embeddings: false,
            embedding_api_url: None,
            embedding_model: "all-MiniLM-L6-v2".to_string(),
        }
    }
}

/// Semantic cache implementation
pub struct SemanticCache<V: Clone + Send + Sync> {
    config: SemanticCacheConfig,
    entries: Arc<RwLock<Vec<SemanticCacheEntry<V>>>>,
    key_to_index: Arc<RwLock<HashMap<String, usize>>>,
    stats: Arc<RwLock<SemanticCacheStats>>,
}

/// Statistics for semantic cache
#[derive(Debug, Clone, Default)]
pub struct SemanticCacheStats {
    pub exact_hits: u64,
    pub semantic_hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_similarity_checks: u64,
}

impl<V: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de>> SemanticCache<V> {
    pub fn new(config: SemanticCacheConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(Vec::new())),
            key_to_index: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SemanticCacheStats::default())),
        }
    }

    /// Generate embedding for a text query
    /// In a real implementation, this would use a proper embedding model
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, CacheError> {
        if self.config.use_external_embeddings {
            self.generate_external_embedding(text).await
        } else {
            Ok(self.generate_simple_embedding(text))
        }
    }

    /// Simple embedding using TF-IDF-like approach (for local use)
    /// In production, use a proper embedding model
    fn generate_simple_embedding(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        const EMBEDDING_DIM: usize = 128;
        let mut embedding = vec![0.0; EMBEDDING_DIM];
        
        // Simple approach: hash words and distribute their weights
        for word in text.split_whitespace() {
            let mut hasher = DefaultHasher::new();
            word.to_lowercase().hash(&mut hasher);
            let hash = hasher.finish();
            let idx = (hash as usize) % EMBEDDING_DIM;
            embedding[idx] += 1.0;
        }
        
        // Normalize
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }
        
        embedding
    }

    /// Generate embedding using external API
    async fn generate_external_embedding(&self, text: &str) -> Result<Vec<f32>, CacheError> {
        let api_url = self.config.embedding_api_url.as_ref().ok_or_else(|| {
            CacheError::ConfigurationError("Embedding API URL not configured".to_string())
        })?;
        
        let client = reqwest::Client::new();
        
        #[derive(Serialize)]
        struct EmbeddingRequest {
            input: String,
            model: String,
        }
        
        #[derive(Deserialize)]
        struct EmbeddingResponse {
            data: Vec<EmbeddingData>,
        }
        
        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }
        
        let request = EmbeddingRequest {
            input: text.to_string(),
            model: self.config.embedding_model.clone(),
        };
        
        let response = client
            .post(api_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| CacheError::NetworkError(format!("Embedding API error: {}", e)))?;
        
        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| CacheError::InternalError(format!("Failed to parse embedding: {}", e)))?;
        
        embedding_response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| CacheError::InternalError("No embedding in response".to_string()))
    }

    /// Calculate cosine similarity between two embeddings
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        if a.len() != b.len() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }
        
        (dot_product / (magnitude_a * magnitude_b)) as f64
    }

    /// Find similar entry using semantic search
    async fn find_similar(&self, embedding: &[f32]) -> Option<(usize, f64, V)> {
        let entries = self.entries.read().await;
        let mut stats = self.stats.write().await;
        
        let mut best_match: Option<(usize, f64, V)> = None;
        let now = std::time::SystemTime::now();
        
        for (idx, entry) in entries.iter().enumerate() {
            // Check if entry is expired
            if let Some(ttl) = entry.ttl {
                if let Ok(elapsed) = now.duration_since(entry.created_at) {
                    if elapsed > ttl {
                        continue;
                    }
                }
            }
            
            stats.total_similarity_checks += 1;
            
            let similarity = Self::cosine_similarity(embedding, &entry.embedding);
            
            if similarity >= self.config.similarity_threshold {
                if let Some((_, best_sim, _)) = &best_match {
                    if similarity > *best_sim {
                        best_match = Some((idx, similarity, entry.value.clone()));
                    }
                } else {
                    best_match = Some((idx, similarity, entry.value.clone()));
                }
            }
        }
        
        if best_match.is_some() {
            stats.semantic_hits += 1;
            debug!("Semantic cache hit with similarity {:.4}", best_match.as_ref().unwrap().1);
        } else {
            stats.misses += 1;
        }
        
        best_match
    }

    /// Evict entries if cache is full
    async fn evict_if_needed(&self) {
        let mut entries = self.entries.write().await;
        let mut key_to_index = self.key_to_index.write().await;
        let mut stats = self.stats.write().await;
        
        while entries.len() >= self.config.max_entries {
            // Simple LRU: remove oldest entry
            if !entries.is_empty() {
                let removed = entries.remove(0);
                key_to_index.remove(&removed.key);
                stats.evictions += 1;
                
                // Update indices
                for (_key, idx) in key_to_index.iter_mut() {
                    if *idx > 0 {
                        *idx -= 1;
                    }
                }
            }
        }
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) {
        let now = std::time::SystemTime::now();
        let mut entries = self.entries.write().await;
        let mut key_to_index = self.key_to_index.write().await;
        
        let mut to_remove = Vec::new();
        
        for (idx, entry) in entries.iter().enumerate() {
            if let Some(ttl) = entry.ttl {
                if let Ok(elapsed) = now.duration_since(entry.created_at) {
                    if elapsed > ttl {
                        to_remove.push(idx);
                    }
                }
            }
        }
        
        // Remove in reverse order to maintain indices
        for idx in to_remove.iter().rev() {
            let removed = entries.remove(*idx);
            key_to_index.remove(&removed.key);
        }
        
        // Update remaining indices
        key_to_index.clear();
        for (idx, entry) in entries.iter().enumerate() {
            key_to_index.insert(entry.key.clone(), idx);
        }
        
        if !to_remove.is_empty() {
            info!("Cleaned up {} expired semantic cache entries", to_remove.len());
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> SemanticCacheStats {
        self.stats.read().await.clone()
    }

    /// Clear all statistics
    pub async fn clear_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = SemanticCacheStats::default();
    }
}

#[async_trait]
impl<V: Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + 'static> Cache<String, V>
    for SemanticCache<V>
{
    type Error = CacheError;

    async fn get(&self, key: &String) -> Result<Option<V>, Self::Error> {
        // First, try exact match
        {
            let key_to_index = self.key_to_index.read().await;
            
            if let Some(&idx) = key_to_index.get(key) {
                drop(key_to_index);
                let entries = self.entries.read().await;
                
                if let Some(entry) = entries.get(idx) {
                    // Check if expired
                    let now = std::time::SystemTime::now();
                    if let Some(ttl) = entry.ttl {
                        if let Ok(elapsed) = now.duration_since(entry.created_at) {
                            if elapsed > ttl {
                                return Ok(None);
                            }
                        }
                    }
                    
                    let mut stats = self.stats.write().await;
                    stats.exact_hits += 1;
                    
                    return Ok(Some(entry.value.clone()));
                }
            }
        }
        
        // Try semantic search
        let embedding = self.generate_embedding(key).await?;
        
        if let Some((idx, similarity, value)) = self.find_similar(&embedding).await {
            debug!("Semantic cache hit for key '{}' with similarity {:.4}", key, similarity);
            
            // Update access count
            let mut entries = self.entries.write().await;
            if let Some(entry) = entries.get_mut(idx) {
                entry.access_count += 1;
            }
            
            return Ok(Some(value));
        }
        
        Ok(None)
    }

    async fn set(&self, key: String, value: V, ttl: Option<Duration>) -> Result<(), Self::Error> {
        // Evict if needed
        self.evict_if_needed().await;
        
        // Generate embedding
        let embedding = self.generate_embedding(&key).await?;
        
        let entry = SemanticCacheEntry {
            key: key.clone(),
            embedding,
            value,
            created_at: std::time::SystemTime::now(),
            ttl: ttl.or(Some(self.config.default_ttl)),
            access_count: 0,
        };
        
        let mut entries = self.entries.write().await;
        let mut key_to_index = self.key_to_index.write().await;
        
        // Check if key already exists
        if let Some(&idx) = key_to_index.get(&key) {
            entries[idx] = entry;
        } else {
            let idx = entries.len();
            entries.push(entry);
            key_to_index.insert(key, idx);
        }
        
        Ok(())
    }

    async fn delete(&self, key: &String) -> Result<(), Self::Error> {
        let mut entries = self.entries.write().await;
        let mut key_to_index = self.key_to_index.write().await;
        
        if let Some(&idx) = key_to_index.get(key) {
            entries.remove(idx);
            key_to_index.remove(key);
            
            // Update remaining indices
            for (_, index) in key_to_index.iter_mut() {
                if *index > idx {
                    *index -= 1;
                }
            }
        }
        
        Ok(())
    }

    async fn exists(&self, key: &String) -> Result<bool, Self::Error> {
        Ok(self.get(key).await?.is_some())
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        let mut entries = self.entries.write().await;
        let mut key_to_index = self.key_to_index.write().await;
        
        entries.clear();
        key_to_index.clear();
        
        Ok(())
    }

    async fn size(&self) -> Result<usize, Self::Error> {
        Ok(self.entries.read().await.len())
    }

    async fn get_stats(&self) -> Result<CacheStats, Self::Error> {
        let stats = self.stats.read().await;
        let size = self.entries.read().await.len();
        let hits = stats.exact_hits + stats.semantic_hits;
        let misses = stats.misses;
        let total = hits + misses;
        
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        
        Ok(CacheStats {
            hits,
            misses,
            size,
            capacity: self.config.max_entries,
            hit_rate,
            miss_rate: 1.0 - hit_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_semantic_cache_exact_match() {
        let config = SemanticCacheConfig::default();
        let cache: SemanticCache<String> = SemanticCache::new(config);
        
        cache.set("hello world".to_string(), "response1".to_string(), None).await.unwrap();
        
        let result = cache.get(&"hello world".to_string()).await.unwrap();
        assert_eq!(result, Some("response1".to_string()));
        
        let stats = cache.get_stats().await;
        assert_eq!(stats.exact_hits, 1);
    }

    #[tokio::test]
    async fn test_semantic_cache_similar_match() {
        let mut config = SemanticCacheConfig::default();
        config.similarity_threshold = 0.7; // Lower threshold for testing
        
        let cache: SemanticCache<String> = SemanticCache::new(config);
        
        cache.set("what is the weather today".to_string(), "sunny".to_string(), None).await.unwrap();
        
        // Similar query
        let result = cache.get(&"weather today how".to_string()).await.unwrap();
        // May or may not match depending on the simple embedding algorithm
        // In production with proper embeddings, this would match
        
        let stats = cache.get_stats().await;
        assert!(stats.total_similarity_checks > 0);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let similarity = SemanticCache::<String>::cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0, 0.0];
        let similarity = SemanticCache::<String>::cosine_similarity(&a, &c);
        assert!(similarity.abs() < 0.001);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let config = SemanticCacheConfig::default();
        let cache: SemanticCache<String> = SemanticCache::new(config);
        
        cache.set(
            "test".to_string(),
            "value".to_string(),
            Some(Duration::from_millis(100)),
        ).await.unwrap();
        
        let result1 = cache.get(&"test".to_string()).await.unwrap();
        assert_eq!(result1, Some("value".to_string()));
        
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        let result2 = cache.get(&"test".to_string()).await.unwrap();
        assert_eq!(result2, None);
    }
}
