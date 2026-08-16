use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    time::Instant,
};
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::Semaphore;
use crate::error::StorageResult;
use super::StorageBackend;

pub struct FileStorage {
    base_dir: PathBuf,
    cache: DashMap<String, CacheEntry>,
    concurrency_limiter: Semaphore,
    cache_ttl_secs: u64,
}

struct CacheEntry {
    data: Vec<u8>,
    last_access: Instant,
}

impl FileStorage {
    pub fn new(base_dir: impl AsRef<Path>, cache_ttl_secs: u64) -> StorageResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)?;
        
        Ok(Self {
            base_dir,
            cache: DashMap::new(),
            concurrency_limiter: Semaphore::new(32), // Configurable concurrency limit
            cache_ttl_secs,
        })
    }
    
    fn get_file_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", key))
    }
}

#[async_trait]
impl StorageBackend for FileStorage {
    async fn store_raw(&self, key: &str, value: Vec<u8>) -> StorageResult<()> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        // Update cache
        self.cache.insert(key.to_string(), CacheEntry {
            data: value.clone(),
            last_access: Instant::now(),
        });
        
        // Write to file
        let file = File::create(self.get_file_path(key))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &value)?;
        
        Ok(())
    }
    
    async fn load_raw(&self, key: &str) -> StorageResult<Option<Vec<u8>>> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        // Check cache first
        if let Some(entry) = self.cache.get_mut(key) {
            entry.last_access = Instant::now();
            return Ok(Some(entry.data.clone()));
        }
        
        // Read from file
        let path = self.get_file_path(key);
        if !path.exists() {
            return Ok(None);
        }
        
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let value: Vec<u8> = serde_json::from_reader(reader)?;
        
        // Update cache
        self.cache.insert(key.to_string(), CacheEntry {
            data: value.clone(),
            last_access: Instant::now(),
        });
        
        Ok(Some(value))
    }
    
    async fn delete(&self, key: &str) -> StorageResult<()> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        self.cache.remove(key);
        let path = self.get_file_path(key);
        if path.exists() {
            fs::remove_file(path)?;
        }
        
        Ok(())
    }
    
    async fn clear(&self) -> StorageResult<()> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        self.cache.clear();
        fs::remove_dir_all(&self.base_dir)?;
        fs::create_dir_all(&self.base_dir)?;
        
        Ok(())
    }
} 