use std::{
    collections::HashMap,
    sync::Arc,
};
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::error::MoaResult;
use super::StorageRaw;

pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StorageRaw for MemoryStorage {
    async fn store_raw(&self, key: &str, value: &[u8]) -> MoaResult<()> {
        self.data.write().await.insert(key.to_string(), value.to_vec());
        Ok(())
    }
    
    async fn load_raw(&self, key: &str) -> MoaResult<Option<Vec<u8>>> {
        Ok(self.data.read().await.get(key).cloned())
    }
    
    async fn delete(&self, key: &str) -> MoaResult<()> {
        self.data.write().await.remove(key);
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> MoaResult<bool> {
        Ok(self.data.read().await.contains_key(key))
    }
    
    async fn clear(&self) -> MoaResult<()> {
        self.data.write().await.clear();
        Ok(())
    }
} 