use std::collections::HashMap;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::error::StorageResult;
use super::StorageBackend;

pub struct MemoryStorage {
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn store_raw(&self, key: &str, value: Vec<u8>) -> StorageResult<()> {
        self.data.write().await.insert(key.to_string(), value);
        Ok(())
    }
    
    async fn load_raw(&self, key: &str) -> StorageResult<Option<Vec<u8>>> {
        if let Some(data) = self.data.read().await.get(key) {
            Ok(Some(data.clone()))
        } else {
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str) -> StorageResult<()> {
        self.data.write().await.remove(key);
        Ok(())
    }
    
    async fn clear(&self) -> StorageResult<()> {
        self.data.write().await.clear();
        Ok(())
    }
} 