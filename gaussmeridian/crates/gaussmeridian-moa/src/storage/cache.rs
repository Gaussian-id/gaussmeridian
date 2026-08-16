use crate::error::MoaResult;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

pub struct CacheEntry<T> {
    pub data: T,
    pub last_access: Instant,
}

pub struct Cache<T> {
    data: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    ttl: Duration,
}

impl<T: Clone + Send + Sync + 'static> Cache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    pub async fn get(&self, key: &str) -> Option<T> {
        let mut data = self.data.write().await;
        if let Some(entry) = data.get_mut(key) {
            if entry.last_access.elapsed() < self.ttl {
                entry.last_access = Instant::now();
                return Some(entry.data.clone());
            } else {
                data.remove(key);
            }
        }
        None
    }

    pub async fn set(&self, key: String, value: T) {
        let mut data = self.data.write().await;
        data.insert(key, CacheEntry {
            data: value,
            last_access: Instant::now(),
        });
    }

    pub async fn remove(&self, key: &str) {
        let mut data = self.data.write().await;
        data.remove(key);
    }

    pub async fn clear(&self) {
        let mut data = self.data.write().await;
        data.clear();
    }

    pub async fn cleanup(&self) {
        let mut data = self.data.write().await;
        data.retain(|_, entry| entry.last_access.elapsed() < self.ttl);
    }
}

pub async fn cached_get(key: String) -> MoaResult<Option<Vec<u8>>> {
    println!("cached_get called with key: {}", key);
    Ok(None)
}

pub async fn cached_set(key: String, value: Vec<u8>) -> MoaResult<()> {
    println!("cached_set called with key: {}, value_len: {}", key, value.len());
    let _ = value;
    Ok(())
}