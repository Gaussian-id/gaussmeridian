use std::{
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{PathBuf, Path},
    sync::Arc,
    time::{Duration, Instant},
};
use async_trait::async_trait;
use dashmap::DashMap;
use tokio::sync::Semaphore;
use crate::error::MoaResult;
use super::StorageRaw;

pub struct FileStorage {
    base_dir: PathBuf,
    cache: Arc<DashMap<String, CacheEntry>>,
    cache_ttl: Duration,
    concurrency_limiter: Arc<Semaphore>,
}

struct CacheEntry {
    data: Vec<u8>,
    last_access: Instant,
}

impl FileStorage {
    pub fn new(base_dir: impl AsRef<Path>, cache_ttl_secs: u64) -> MoaResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)?;
        
        let storage = Self {
            base_dir,
            cache: Arc::new(DashMap::new()),
            cache_ttl: Duration::from_secs(cache_ttl_secs),
            concurrency_limiter: Arc::new(Semaphore::new(32)),
        };
        
        storage.start_cache_cleanup();
        Ok(storage)
    }
    
    fn get_file_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", key))
    }
    
    fn start_cache_cleanup(&self) {
        let cache = self.cache.clone();
        let ttl = self.cache_ttl;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let now = Instant::now();
                cache.retain(|_, entry| now.duration_since(entry.last_access) < ttl);
            }
        });
    }
}

#[async_trait]
impl StorageRaw for FileStorage {
    async fn store_raw(&self, key: &str, value: &[u8]) -> MoaResult<()> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        self.cache.insert(key.to_string(), CacheEntry {
            data: value.to_vec(),
            last_access: Instant::now(),
        });
        
        let file = File::create(self.get_file_path(key))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, value)?;
        
        Ok(())
    }
    
    async fn load_raw(&self, key: &str) -> MoaResult<Option<Vec<u8>>> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        if let Some(mut entry) = self.cache.get_mut(key) {
            entry.last_access = Instant::now();
            return Ok(Some(entry.data.clone()));
        }
        
        let path = self.get_file_path(key);
        if !path.exists() {
            return Ok(None);
        }
        
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let value: Vec<u8> = serde_json::from_reader(reader)?;
        
        self.cache.insert(key.to_string(), CacheEntry {
            data: value.clone(),
            last_access: Instant::now(),
        });
        
        Ok(Some(value))
    }
    
    async fn delete(&self, key: &str) -> MoaResult<()> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        self.cache.remove(key);
        
        let path = self.get_file_path(key);
        if path.exists() {
            fs::remove_file(path)?;
        }
        
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> MoaResult<bool> {
        let _permit = self.concurrency_limiter.acquire().await?;
        Ok(self.get_file_path(key).exists())
    }
    
    async fn clear(&self) -> MoaResult<()> {
        let _permit = self.concurrency_limiter.acquire().await?;
        
        self.cache.clear();
        
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::remove_file(entry.path())?;
            }
        }
        
        Ok(())
    }
} 