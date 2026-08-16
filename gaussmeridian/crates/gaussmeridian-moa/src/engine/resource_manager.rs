use crate::{
    config::MoaConfig,
    error::{MoaError, MoaResult},
    models::HealthStatus,
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, Semaphore},
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, info, warn, error};

const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// Manages system resources and caching
pub struct ResourceManager {
    config: Arc<MoaConfig>,
    request_limiter: Arc<Semaphore>,
    response_cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    _cleanup_task: Option<JoinHandle<()>>,
}

struct CacheEntry {
    data: Vec<u8>,
    expires_at: Instant,
}

impl ResourceManager {
    pub fn new(config: &Arc<MoaConfig>) -> MoaResult<Self> {
        let request_limiter = Arc::new(Semaphore::new(config.resources.max_concurrent_requests));
        let response_cache = Arc::new(RwLock::new(HashMap::new()));
        
        // Start cache cleanup task
        let cache = Arc::clone(&response_cache);
        let ttl = Duration::from_secs(config.resources.cache_ttl_secs as u64);
        let cleanup_task = tokio::spawn(async move {
            loop {
                sleep(CLEANUP_INTERVAL).await;
                Self::cleanup_expired_cache_entries(&cache, ttl).await;
            }
        });

        Ok(Self {
            config: Arc::clone(config),
            request_limiter,
            response_cache,
            _cleanup_task: Some(cleanup_task),
        })
    }

    pub async fn init(&self) -> MoaResult<()> {
        info!("Initializing resource manager...");
        Ok(())
    }

    pub async fn shutdown(&self) -> MoaResult<()> {
        info!("Shutting down resource manager...");
        if let Some(handle) = &self._cleanup_task {
            handle.abort();
        }
        Ok(())
    }

    pub async fn health_check(&self) -> MoaResult<HealthStatus> {
        let available_permits = self.request_limiter.available_permits();
        let max_permits = self.config.resources.max_concurrent_requests;
        
        if available_permits as f32 / max_permits as f32 >= 0.2 {
            Ok(HealthStatus::Healthy)
        } else if available_permits > 0 {
            Ok(HealthStatus::Degraded)
        } else {
            Ok(HealthStatus::Unhealthy)
        }
    }

    pub async fn detailed_health_check(&self) -> MoaResult<crate::models::ComponentHealthStatus> {
        let status = self.health_check().await?;
        let cache = self.response_cache.read().await;
        let cache_size = cache.len();
        let max_cache = self.config.resources.max_batch_size;
        let cache_usage = cache_size as f32 / max_cache as f32;
        
        let message = if cache_usage > 0.9 {
            Some(format!("Cache usage high: {:.1}%", cache_usage * 100.0))
        } else {
            None
        };

        Ok(crate::models::ComponentHealthStatus {
            status,
            message,
            timestamp: chrono::Utc::now(),
        })
    }

    pub async fn acquire_permit(&self) -> MoaResult<tokio::sync::SemaphorePermit> {
        match self.request_limiter.acquire().await {
            Ok(permit) => Ok(permit),
            Err(_) => Err(MoaError::resource("Failed to acquire request permit".to_string())),
        }
    }

    pub async fn get_cached(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.response_cache.read().await;
        cache.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.data.clone())
            } else {
                None
            }
        })
    }

    pub async fn cache_response(&self, key: String, data: Vec<u8>) -> MoaResult<()> {
        let mut cache = self.response_cache.write().await;
        let ttl = Duration::from_secs(self.config.resources.cache_ttl_secs as u64);
        
        cache.insert(key, CacheEntry {
            data,
            expires_at: Instant::now() + ttl,
        });

        // Check cache size
        if cache.len() > self.config.resources.max_batch_size {
            warn!("Cache size exceeds limit, triggering cleanup");
            drop(cache); // Release write lock before cleanup
            self.cleanup_cache().await?;
        }

        Ok(())
    }

    pub async fn cleanup_cache(&self) -> MoaResult<()> {
        let mut cache = self.response_cache.write().await;
        let now = Instant::now();
        cache.retain(|_, entry| entry.expires_at > now);
        Ok(())
    }

    async fn cleanup_expired_cache_entries(
        cache: &Arc<RwLock<HashMap<String, CacheEntry>>>,
        ttl: Duration,
    ) {
        let mut cache = cache.write().await;
        let now = Instant::now();
        let before_cleanup = cache.len();
        
        cache.retain(|_, entry| entry.expires_at > now);
        
        let removed = before_cleanup - cache.len();
        if removed > 0 {
            debug!("Cleaned up {} expired cache entries", removed);
        }
    }

    pub async fn check_capacity(&self) -> MoaResult<ResourceCapacity> {
        let available_permits = self.request_limiter.available_permits();
        let cache = self.response_cache.read().await;
        
        Ok(ResourceCapacity {
            available_permits,
            cache_size: cache.len(),
            cache_usage: cache.len() as f32 / self.config.resources.max_batch_size as f32,
        })
    }

    pub async fn apply_backpressure(&self, level: BackpressureLevel) -> MoaResult<()> {
        match level {
            BackpressureLevel::Low => {
                // Reduce cache TTL
                debug!("Applying low backpressure - reducing cache TTL");
            }
            BackpressureLevel::Medium => {
                // Trigger cache cleanup
                debug!("Applying medium backpressure - cleaning cache");
                self.cleanup_cache().await?;
            }
            BackpressureLevel::High => {
                // Clear cache and reduce permits
                debug!("Applying high backpressure - clearing cache");
                let mut cache = self.response_cache.write().await;
                cache.clear();
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ResourceCapacity {
    pub available_permits: usize,
    pub cache_size: usize,
    pub cache_usage: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum BackpressureLevel {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_management() {
        // Create test config
        let config = Arc::new(MoaConfig::default());
        
        // Create resource manager
        let manager = ResourceManager::new(&config).unwrap();
        
        // Test initialization
        manager.init().await.unwrap();
        
        // Test permit acquisition
        let permit = manager.acquire_permit().await.unwrap();
        drop(permit);
        
        // Test caching
        let key = "test_key".to_string();
        let data = vec![1, 2, 3];
        manager.cache_response(key.clone(), data.clone()).await.unwrap();
        
        // Test cache retrieval
        let cached = manager.get_cached(&key).await.unwrap();
        assert_eq!(cached, data);
        
        // Test cleanup
        manager.cleanup_cache().await.unwrap();
        
        // Test shutdown
        manager.shutdown().await.unwrap();
    }
} 