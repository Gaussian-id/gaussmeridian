pub mod database;
pub mod cache;

use crate::{
    models,
    error::{MoaResult, MoaError},
    config,
};
use redb::{
    Database, 
    TableDefinition,
    ReadableTable,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use once_cell::sync::OnceCell;
use uuid::Uuid;

const REQUESTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("requests");
const RESPONSES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("responses");
const METRICS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("metrics");

// Split generic operations into separate traits
#[async_trait]
pub trait StorageRaw: Send + Sync {
    async fn store_raw(&self, key: &str, value: &[u8]) -> MoaResult<()>;
    async fn load_raw(&self, key: &str) -> MoaResult<Option<Vec<u8>>>;
    async fn delete(&self, key: &str) -> MoaResult<()>;
    async fn exists(&self, key: &str) -> MoaResult<bool>;
    async fn clear(&self) -> MoaResult<()>;
}

#[async_trait]
pub trait StorageJson: StorageRaw {
    async fn store_json<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> MoaResult<()> {
        let bytes = serde_json::to_vec(value).map_err(|e| MoaError::Serialization {
            message: format!("Failed to serialize value: {}", e),
            source: Some(e),
        })?;
        self.store_raw(key, &bytes).await
    }
    
    async fn load_json<T: DeserializeOwned + Send>(&self, key: &str) -> MoaResult<Option<T>> {
        if let Some(bytes) = self.load_raw(key).await? {
            let value = serde_json::from_slice(&bytes).map_err(|e| MoaError::Serialization {
                message: format!("Failed to deserialize value: {}", e),
                source: Some(e),
            })?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

// Implement StorageJson for any type that implements StorageRaw
impl<T: ?Sized + StorageRaw> StorageJson for T {}

pub struct MoaStorage {
    db: Database,
}

impl MoaStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> MoaResult<Self> {
        let db = Database::create(path.as_ref()).map_err(|e| MoaError::storage(
            format!("Failed to create database: {}", e),
            Some(Box::new(e)),
        ))?;
        
        let write_txn = db.begin_write().map_err(|e| MoaError::storage(
            format!("Failed to begin write transaction: {}", e),
            Some(Box::new(e)),
        ))?;
        
        let result: MoaResult<()> = (|| {
            write_txn.open_table(REQUESTS_TABLE).map_err(|e| MoaError::storage(
                format!("Failed to open requests table: {}", e),
                Some(Box::new(e)),
            ))?;
            write_txn.open_table(RESPONSES_TABLE).map_err(|e| MoaError::storage(
                format!("Failed to open responses table: {}", e),
                Some(Box::new(e)),
            ))?;
            write_txn.open_table(METRICS_TABLE).map_err(|e| MoaError::storage(
                format!("Failed to open metrics table: {}", e),
                Some(Box::new(e)),
            ))?;
            Ok(())
        })();

        if let Err(e) = result {
            write_txn.abort()?;
            return Err(e);
        }

        write_txn.commit().map_err(|e| MoaError::storage(
            format!("Failed to commit transaction: {}", e),
            Some(Box::new(e)),
        ))?;
        
        Ok(Self { db })
    }
    
    pub async fn store_request(&self, request: &models::MoaRequest) -> MoaResult<()> {
        let key = request.id.to_string();
        let value = serde_json::to_vec(request).map_err(MoaError::from)?;
        let write_txn = self.db.begin_write()
            .map_err(|e| MoaError::storage(
                format!("Failed to begin write transaction: {}", e),
                Some(Box::new(e))
            ))?;
        {
            let mut table = write_txn.open_table(REQUESTS_TABLE)
                .map_err(|e| MoaError::storage(
                    format!("Failed to open requests table: {}", e),
                    Some(Box::new(e))
                ))?;
            table.insert(key.as_str(), value.as_slice())
                .map_err(|e| MoaError::storage(
                    format!("Failed to insert request: {}", e),
                    Some(Box::new(e))
                ))?;
        }
        write_txn.commit()
            .map_err(|e| MoaError::storage(
                format!("Failed to commit transaction: {}", e),
                Some(Box::new(e))
            ))?;
        Ok(())
    }
    
    pub async fn store_response(&self, response: &models::MoaResponse) -> MoaResult<()> {
        let key = response.id.to_string();
        let value = serde_json::to_vec(response).map_err(MoaError::from)?;
        let write_txn = self.db.begin_write()
            .map_err(|e| MoaError::storage(
                format!("Failed to begin write transaction: {}", e),
                Some(Box::new(e))
            ))?;
        {
            let mut table = write_txn.open_table(RESPONSES_TABLE)
                .map_err(|e| MoaError::storage(
                    format!("Failed to open responses table: {}", e),
                    Some(Box::new(e))
                ))?;
            table.insert(key.as_str(), value.as_slice())
                .map_err(|e| MoaError::storage(
                    format!("Failed to insert response: {}", e),
                    Some(Box::new(e))
                ))?;
        }
        write_txn.commit()
            .map_err(|e| MoaError::storage(
                format!("Failed to commit transaction: {}", e),
                Some(Box::new(e))
            ))?;
        Ok(())
    }
    
    pub async fn get_request(&self, id: &Uuid) -> MoaResult<Option<models::MoaRequest>> {
        let read_txn = self.db.begin_read()
            .map_err(|e| MoaError::storage(format!("GetRequest: BeginRead Txn Failed: {}", e), Some(Box::new(e))))?;
        let table = read_txn.open_table(REQUESTS_TABLE)
            .map_err(|e| MoaError::storage(format!("GetRequest: Open Table Failed: {}", e), Some(Box::new(e))))?;
        let key = id.to_string();

        let guard_option = table.get(key.as_str())
            .map_err(|e| MoaError::storage(format!("GetRequest: Get Failed: {}", e), Some(Box::new(e))))?;

        if let Some(guard) = guard_option {
            let value_bytes = guard.value().to_vec();
            drop(guard); // Ensure guard is dropped before further operations that might conflict with its lifetime
            let request = serde_json::from_slice(&value_bytes).map_err(MoaError::from)?;
            Ok(Some(request))
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_response(&self, id: &Uuid) -> MoaResult<Option<models::MoaResponse>> {
        let read_txn = self.db.begin_read()
            .map_err(|e| MoaError::storage(format!("GetResponse: BeginRead Txn Failed: {}", e), Some(Box::new(e))))?;
        let table = read_txn.open_table(RESPONSES_TABLE)
            .map_err(|e| MoaError::storage(format!("GetResponse: Open Table Failed: {}", e), Some(Box::new(e))))?;
        let key = id.to_string();

        let guard_option = table.get(key.as_str())
            .map_err(|e| MoaError::storage(format!("GetResponse: Get Failed: {}", e), Some(Box::new(e))))?;

        if let Some(guard) = guard_option {
            let value_bytes = guard.value().to_vec();
            drop(guard);
            let response = serde_json::from_slice(&value_bytes).map_err(MoaError::from)?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    async fn clear(&self) -> MoaResult<()> {
        let write_txn = self.db.begin_write()
            .map_err(|e| MoaError::storage(format!("Clear: Begin Txn Failed: {}", e), Some(Box::new(e))))?;
        
        // For redb 1.x, clear by deleting and re-opening the table.
        // This approach is suitable for redb v1.5.1.

        // Delete and re-open REQUESTS_TABLE
        write_txn.delete_table(REQUESTS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Delete REQUESTS_TABLE Failed: {}", e), Some(Box::new(e))))?;
        write_txn.open_table(REQUESTS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Re-open REQUESTS_TABLE Failed: {}", e), Some(Box::new(e))))?;

        // Delete and re-open RESPONSES_TABLE
        write_txn.delete_table(RESPONSES_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Delete RESPONSES_TABLE Failed: {}", e), Some(Box::new(e))))?;
        write_txn.open_table(RESPONSES_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Re-open RESPONSES_TABLE Failed: {}", e), Some(Box::new(e))))?;

        // Delete and re-open METRICS_TABLE
        write_txn.delete_table(METRICS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Delete METRICS_TABLE Failed: {}", e), Some(Box::new(e))))?;
        write_txn.open_table(METRICS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Re-open METRICS_TABLE Failed: {}", e), Some(Box::new(e))))?;
            
        write_txn.commit().map_err(|e| MoaError::storage(format!("Clear: Commit Failed: {}", e), Some(Box::new(e))))?;
        Ok(())
    }
}

// Storage manager
pub struct StorageManager {
    backend: Arc<dyn StorageRaw + Send + Sync>,
}

impl StorageManager {
    pub fn new_file_storage(base_dir_param: impl AsRef<PathBuf>, cache_ttl_secs: u64) -> MoaResult<Self> {
        // Ensure we get PathBuf, then convert to &Path for FileStorage::new
        let path_buf_ref = base_dir_param.as_ref(); // path_buf_ref is &PathBuf
        let storage = Arc::new(FileStorage::new(path_buf_ref.as_path(), cache_ttl_secs)?);
        Ok(Self {
            backend: storage,
        })
    }
    
    pub fn new_memory_storage() -> Self {
        let storage = Arc::new(MemoryStorage::new());
        Self {
            backend: storage,
        }
    }
    
    pub async fn store<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> MoaResult<()> {
        StorageJson::store_json(&*self.backend, key, value).await
    }
    
    pub async fn load<T: DeserializeOwned + Send>(&self, key: &str) -> MoaResult<Option<T>> {
        StorageJson::load_json(&*self.backend, key).await
    }
    
    pub async fn delete(&self, key: &str) -> MoaResult<()> {
        self.backend.delete(key).await
    }
    
    pub async fn exists(&self, key: &str) -> MoaResult<bool> {
        self.backend.exists(key).await
    }
    
    pub async fn clear(&self) -> MoaResult<()> {
        self.backend.clear().await
    }
}

// Re-export storage implementations
mod file_storage;
mod memory_storage;

pub use file_storage::FileStorage;
pub use memory_storage::MemoryStorage;

// Initialize storage
pub async fn init(config: &config::StorageConfig) -> MoaResult<()> {
    let storage_backend: Arc<dyn StorageRaw + Send + Sync> = match config.backend {
        config::StorageBackendType::File => {
            let base_dir_pathbuf = PathBuf::from(&config.path);
            let cache_ttl_secs = config.cleanup_interval_seconds;
            if !base_dir_pathbuf.exists() {
                std::fs::create_dir_all(&base_dir_pathbuf).map_err(|e| MoaError::storage(format!("Failed to create storage directory: {}", e.to_string()), Some(Box::new(e))))?;
            }
            let file_storage = FileStorage::new(base_dir_pathbuf.as_path(), cache_ttl_secs)
                .map_err(|e| MoaError::storage(format!("Failed to init FileStorage: {}", e), Some(e)))?;
            Arc::new(file_storage)
        }
        config::StorageBackendType::Memory => {
            let memory_storage = MemoryStorage::new();
            Arc::new(memory_storage)
        }
        config::StorageBackendType::Redb => {
            let _db_path = PathBuf::from(&config.path);
            return Err(MoaError::storage(
                format!("Redb storage backend through global STORAGE not fully supported without StorageRaw impl for MoaStorage."),
                None::<Box<dyn std::error::Error + Send + Sync>>
            ));
        }
    };

    if STORAGE.set(storage_backend).is_err() {
        return Err(MoaError::storage("Failed to set global storage".to_string(), None::<Box<dyn std::error::Error + Send + Sync>>));
    }
    Ok(())
}

static STORAGE: OnceCell<Arc<dyn StorageRaw>> = OnceCell::new();

pub fn get_storage() -> MoaResult<&'static Arc<dyn StorageRaw>> {
    STORAGE.get().ok_or_else(|| MoaError::storage("Storage not initialized".to_string(), None::<Box<dyn std::error::Error + Send + Sync>>))
}

#[async_trait]
pub trait StorageCore: Send + Sync {
    async fn store<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> MoaResult<()>;
    async fn retrieve<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> MoaResult<Option<T>>;
    async fn delete(&self, key: &str) -> MoaResult<()>;
    async fn exists(&self, key: &str) -> MoaResult<bool>;
    async fn clear(&self) -> MoaResult<()>;
}

#[async_trait]
impl StorageCore for MoaStorage {
    async fn store<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> MoaResult<()> {
        let bytes = serde_json::to_vec(value).map_err(MoaError::from)?;
        let write_txn = self.db.begin_write()
            .map_err(|e| MoaError::storage(format!("Store: Begin Txn Failed: {}", e), Some(Box::new(e))))?;
        {
            // Assuming a generic table for now, or key-based routing to specific tables
            let mut table = write_txn.open_table(REQUESTS_TABLE) // Example table
                .map_err(|e| MoaError::storage(format!("Store: Open Table Failed: {}", e), Some(Box::new(e))))?;
            table.insert(key, bytes.as_slice())
                .map_err(|e| MoaError::storage(format!("Store: Insert Failed: {}", e), Some(Box::new(e))))?;
        }
        write_txn.commit().map_err(|e| MoaError::storage(format!("Store: Commit Failed: {}", e), Some(Box::new(e))))?;
        Ok(())
    }

    async fn retrieve<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> MoaResult<Option<T>> {
        let read_txn = self.db.begin_read()
            .map_err(|e| MoaError::storage(format!("Retrieve: BeginRead Txn Failed: {}", e), Some(Box::new(e))))?;
        let table = read_txn.open_table(REQUESTS_TABLE) // Example table
            .map_err(|e| MoaError::storage(format!("Retrieve: Open Table Failed: {}", e), Some(Box::new(e))))?;
        
        let guard_option = table.get(key).map_err(|e| MoaError::storage(format!("Retrieve: Get Failed: {}", e), Some(Box::new(e))))?;

        if let Some(guard) = guard_option {
            let value_bytes = guard.value().to_vec();
            drop(guard);
            let value = serde_json::from_slice(&value_bytes).map_err(MoaError::from)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, key: &str) -> MoaResult<()> {
        let write_txn = self.db.begin_write()
            .map_err(|e| MoaError::storage(format!("Delete: Begin Txn Failed: {}", e), Some(Box::new(e))))?;
        {
            let mut table = write_txn.open_table(REQUESTS_TABLE) // Example table
                .map_err(|e| MoaError::storage(format!("Delete: Open Table Failed: {}", e), Some(Box::new(e))))?;
            table.remove(key)
                .map_err(|e| MoaError::storage(format!("Delete: Remove Failed: {}", e), Some(Box::new(e))))?;
        }
        write_txn.commit().map_err(|e| MoaError::storage(format!("Delete: Commit Failed: {}", e), Some(Box::new(e))))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> MoaResult<bool> {
        let read_txn = self.db.begin_read()
            .map_err(|e| MoaError::storage(format!("Exists: Begin Txn Failed: {}", e), Some(Box::new(e))))?;
        let table = read_txn.open_table(REQUESTS_TABLE) // Example table
            .map_err(|e| MoaError::storage(format!("Exists: Open Table Failed: {}", e), Some(Box::new(e))))?;
        let exists = table.get(key)
                        .map_err(|e| MoaError::storage(format!("Exists: Get Failed: {}", e), Some(Box::new(e))))?
                        .is_some();
        Ok(exists)
    }

    async fn clear(&self) -> MoaResult<()> {
        let write_txn = self.db.begin_write()
            .map_err(|e| MoaError::storage(format!("Clear: Begin Txn Failed: {}", e), Some(Box::new(e))))?;
        
        // For redb 1.x, clear by deleting and re-opening the table.
        // This approach is suitable for redb v1.5.1.

        // Delete and re-open REQUESTS_TABLE
        write_txn.delete_table(REQUESTS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Delete REQUESTS_TABLE Failed: {}", e), Some(Box::new(e))))?;
        write_txn.open_table(REQUESTS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Re-open REQUESTS_TABLE Failed: {}", e), Some(Box::new(e))))?;

        // Delete and re-open RESPONSES_TABLE
        write_txn.delete_table(RESPONSES_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Delete RESPONSES_TABLE Failed: {}", e), Some(Box::new(e))))?;
        write_txn.open_table(RESPONSES_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Re-open RESPONSES_TABLE Failed: {}", e), Some(Box::new(e))))?;

        // Delete and re-open METRICS_TABLE
        write_txn.delete_table(METRICS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Delete METRICS_TABLE Failed: {}", e), Some(Box::new(e))))?;
        write_txn.open_table(METRICS_TABLE)
            .map_err(|e| MoaError::storage(format!("Clear: Re-open METRICS_TABLE Failed: {}", e), Some(Box::new(e))))?;
            
        write_txn.commit().map_err(|e| MoaError::storage(format!("Clear: Commit Failed: {}", e), Some(Box::new(e))))?;
        Ok(())
    }
}