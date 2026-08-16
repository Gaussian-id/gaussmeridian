//! Redis cache implementation

use async_trait::async_trait;
use deadpool_redis::{Connection, Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{
    config::{CompressionConfig, EncryptionConfig, RedisConfig},
    error::CacheError,
    stats::{CacheMetrics, CacheStats},
    traits::{Cache, CacheWithStats, CompressedCache, DistributedCache},
};

/// Advanced Redis cache implementation
pub struct RedisCache<K, V> {
    pool: Pool,
    config: RedisConfig,
    metrics: CacheMetrics,
    compression: CompressionConfig,
    encryption: EncryptionConfig,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> RedisCache<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    /// Create a new Redis cache
    pub async fn new(config: RedisConfig) -> Result<Self, CacheError> {
        let pool_config = deadpool_redis::Config::from_url(&config.url);
        let pool = pool_config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| CacheError::NetworkError(format!("Failed to create Redis pool: {}", e)))?;

        // Test connection
        let _conn = pool.get().await.map_err(|e| {
            CacheError::NetworkError(format!("Failed to get Redis connection: {}", e))
        })?;

        let metrics = CacheMetrics::new(10000); // Redis doesn't have a fixed size limit

        Ok(Self {
            pool,
            config,
            metrics,
            compression: CompressionConfig::default(),
            encryption: EncryptionConfig::default(),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Create with compression
    pub fn with_compression(mut self, compression: CompressionConfig) -> Self {
        self.compression = compression;
        self
    }

    /// Create with encryption
    pub fn with_encryption(mut self, encryption: EncryptionConfig) -> Self {
        self.encryption = encryption;
        self
    }

    /// Get Redis connection
    async fn get_connection(&self) -> Result<Connection, CacheError> {
        self.pool
            .get()
            .await
            .map_err(|e| CacheError::NetworkError(format!("Failed to get Redis connection: {}", e)))
    }

    /// Serialize key to Redis format
    fn serialize_key(&self, key: &K) -> Result<String, CacheError> {
        serde_json::to_string(key)
            .map_err(|e| CacheError::SerializationError(format!("Failed to serialize key: {}", e)))
    }

    /// Serialize value to Redis format
    fn serialize_value(&self, value: &V) -> Result<String, CacheError> {
        serde_json::to_string(value).map_err(|e| {
            CacheError::SerializationError(format!("Failed to serialize value: {}", e))
        })
    }

    /// Deserialize value from Redis format
    fn deserialize_value(&self, data: &str) -> Result<V, CacheError> {
        serde_json::from_str(data).map_err(|e| {
            CacheError::DeserializationError(format!("Failed to deserialize value: {}", e))
        })
    }

    /// Compress data if enabled
    fn compress_data(&self, data: &str) -> Result<Vec<u8>, CacheError> {
        if !self.compression.enabled || data.len() < self.compression.min_size {
            return Ok(data.as_bytes().to_vec());
        }

        #[cfg(feature = "compression")]
        {
            match self.compression.algorithm {
                crate::config::CompressionAlgorithm::Gzip => {
                    use flate2::write::GzEncoder;
                    use flate2::Compression;
                    use std::io::Write;

                    let mut encoder = GzEncoder::new(
                        Vec::new(),
                        Compression::new(self.compression.compression_level as u32),
                    );
                    encoder.write_all(data.as_bytes()).map_err(|e| {
                        CacheError::CompressionError(format!("Gzip compression failed: {}", e))
                    })?;
                    encoder.finish().map_err(|e| {
                        CacheError::CompressionError(format!("Gzip compression failed: {}", e))
                    })
                }
                crate::config::CompressionAlgorithm::Lz4 => {
                    use lz4::block::compress;
                    compress(data.as_bytes(), None, false).map_err(|e| {
                        CacheError::CompressionError(format!("LZ4 compression failed: {}", e))
                    })
                }
                crate::config::CompressionAlgorithm::Snappy => {
                    use snap::raw::Encoder;
                    let mut encoder = Encoder::new();
                    encoder.compress_vec(data.as_bytes()).map_err(|e| {
                        CacheError::CompressionError(format!("Snappy compression failed: {}", e))
                    })
                }
                crate::config::CompressionAlgorithm::Zstd => {
                    use zstd::bulk::compress;
                    compress(data.as_bytes(), self.compression.compression_level as i32).map_err(
                        |e| CacheError::CompressionError(format!("Zstd compression failed: {}", e)),
                    )
                }
                crate::config::CompressionAlgorithm::Brotli => {
                    use brotli::enc::BrotliEncoderParams;
                    use brotli::BrotliCompress;
                    let params = BrotliEncoderParams::default();
                    let mut output = Vec::new();
                    BrotliCompress(&mut data.as_bytes(), &mut output, &params).map_err(|e| {
                        CacheError::CompressionError(format!("Brotli compression failed: {}", e))
                    })?;
                    Ok(output)
                }
            }
        }

        #[cfg(not(feature = "compression"))]
        {
            Ok(data.as_bytes().to_vec())
        }
    }

    /// Decompress data if compressed
    fn decompress_data(&self, data: &[u8]) -> Result<String, CacheError> {
        // Try to detect if data is compressed by checking magic bytes
        if data.len() < 2 {
            return Ok(String::from_utf8_lossy(data).to_string());
        }

        #[cfg(feature = "compression")]
        {
            // Check for compression signatures
            if data.starts_with(b"\x1f\x8b") {
                // Gzip
                use flate2::read::GzDecoder;
                use std::io::Read;

                let mut decoder = GzDecoder::new(data);
                let mut output = String::new();
                decoder.read_to_string(&mut output).map_err(|e| {
                    CacheError::DecompressionError(format!("Gzip decompression failed: {}", e))
                })?;
                Ok(output)
            } else if data.starts_with(b"\x04\x22\x4d\x18") {
                // LZ4
                use lz4::block::decompress;
                let decompressed = decompress(data, None).map_err(|e| {
                    CacheError::DecompressionError(format!("LZ4 decompression failed: {}", e))
                })?;
                String::from_utf8(decompressed).map_err(|e| {
                    CacheError::DecompressionError(format!(
                        "Invalid UTF-8 after LZ4 decompression: {}",
                        e
                    ))
                })
            } else if data.starts_with(b"\xff\x06\x00\x00\x73\x4e\x61\x50\x70\x59") {
                // Snappy
                use snap::raw::Decoder;
                let mut decoder = Decoder::new();
                let decompressed = decoder.decompress_vec(data).map_err(|e| {
                    CacheError::DecompressionError(format!("Snappy decompression failed: {}", e))
                })?;
                String::from_utf8(decompressed).map_err(|e| {
                    CacheError::DecompressionError(format!(
                        "Invalid UTF-8 after Snappy decompression: {}",
                        e
                    ))
                })
            } else if data.starts_with(b"\x28\xb5\x2f\xfd") {
                // Zstd
                use zstd::bulk::decompress;
                let decompressed = decompress(data, 0).map_err(|e| {
                    CacheError::DecompressionError(format!("Zstd decompression failed: {}", e))
                })?;
                String::from_utf8(decompressed).map_err(|e| {
                    CacheError::DecompressionError(format!(
                        "Invalid UTF-8 after Zstd decompression: {}",
                        e
                    ))
                })
            } else {
                // Assume uncompressed
                String::from_utf8(data.to_vec())
                    .map_err(|e| CacheError::DecompressionError(format!("Invalid UTF-8: {}", e)))
            }
        }

        #[cfg(not(feature = "compression"))]
        {
            String::from_utf8(data.to_vec())
                .map_err(|e| CacheError::DecompressionError(format!("Invalid UTF-8: {}", e)))
        }
    }

    /// Encrypt data if enabled
    fn encrypt_data(&self, data: &[u8], _key: &[u8]) -> Result<Vec<u8>, CacheError> {
        // Placeholder implementation - in real usage, implement proper encryption
        Ok(data.to_vec())
    }

    /// Decrypt data if encrypted
    fn decrypt_data(&self, data: &[u8], _key: &[u8]) -> Result<Vec<u8>, CacheError> {
        // Placeholder implementation - in real usage, implement proper decryption
        Ok(data.to_vec())
    }

    /// Execute operation with retry logic
    async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T, CacheError>
    where
        F: Fn()
                -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, CacheError>> + Send>>
            + Send
            + Sync,
    {
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_millis(100 * attempts as u64)).await;
                }
            }
        }
    }

    /// Get Redis statistics
    pub async fn get_redis_stats(&self) -> Result<RedisStats, CacheError> {
        let mut conn = self.get_connection().await?;

        let info: String = redis::cmd("INFO")
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::NetworkError(format!("Failed to get Redis info: {}", e)))?;

        // Parse INFO output to extract statistics
        let mut stats = RedisStats::default();

        for line in info.lines() {
            if line.starts_with("used_memory:") {
                stats.used_memory = line
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("connected_clients:") {
                stats.connected_clients = line
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("total_commands_processed:") {
                stats.total_commands = line
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("keyspace_hits:") {
                stats.keyspace_hits = line
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("keyspace_misses:") {
                stats.keyspace_misses = line
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }

        Ok(stats)
    }
}

/// Redis statistics
#[derive(Debug, Clone, Default)]
pub struct RedisStats {
    pub used_memory: usize,
    pub connected_clients: usize,
    pub total_commands: u64,
    pub keyspace_hits: u64,
    pub keyspace_misses: u64,
}

#[async_trait]
impl<K, V> Cache<K, V> for RedisCache<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    type Error = CacheError;

    async fn get(&self, key: &K) -> Result<Option<V>, Self::Error> {
        let start = std::time::Instant::now();

        let key_str = self.serialize_key(key)?;
        let mut conn = self.get_connection().await?;

        let result: Option<Vec<u8>> = redis::cmd("GET")
            .arg(&key_str)
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::NetworkError(format!("Redis GET failed: {}", e)))?;

        if let Some(data) = result {
            let decrypted = self.decrypt_data(&data, &self.encryption.key)?;
            let decompressed = self.decompress_data(&decrypted)?;
            let value = self.deserialize_value(&decompressed)?;

            self.metrics.record_hit();
            self.metrics.record_get_time(start.elapsed());
            Ok(Some(value))
        } else {
            self.metrics.record_miss();
            self.metrics.record_get_time(start.elapsed());
            Ok(None)
        }
    }

    async fn set(&self, key: K, value: V, ttl: Option<Duration>) -> Result<(), Self::Error> {
        let start = std::time::Instant::now();

        let key_str = self.serialize_key(&key)?;
        let value_str = self.serialize_value(&value)?;
        let compressed = self.compress_data(&value_str)?;
        let encrypted = self.encrypt_data(&compressed, &self.encryption.key)?;

        let ttl_seconds = ttl.map(|d| d.as_secs() as i64);

        let mut conn = self.get_connection().await?;

        if let Some(ttl) = ttl_seconds {
            let _: () = redis::cmd("SETEX")
                .arg(&key_str)
                .arg(ttl)
                .arg(&encrypted)
                .query_async(&mut *conn)
                .await
                .map_err(|e| CacheError::NetworkError(format!("Redis SETEX failed: {}", e)))?;
        } else {
            let _: () = redis::cmd("SET")
                .arg(&key_str)
                .arg(&encrypted)
                .query_async(&mut *conn)
                .await
                .map_err(|e| CacheError::NetworkError(format!("Redis SET failed: {}", e)))?;
        }

        self.metrics.record_set_time(start.elapsed());
        Ok(())
    }

    async fn delete(&self, key: &K) -> Result<(), Self::Error> {
        let key_str = self.serialize_key(key)?;
        let mut conn = self.get_connection().await?;

        let _: () = redis::cmd("DEL")
            .arg(&key_str)
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::NetworkError(format!("Redis DEL failed: {}", e)))?;

        Ok(())
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        let mut conn = self.get_connection().await?;

        let _: () = redis::cmd("FLUSHDB")
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::NetworkError(format!("Redis FLUSHDB failed: {}", e)))?;

        Ok(())
    }

    async fn exists(&self, key: &K) -> Result<bool, Self::Error> {
        let key_str = self.serialize_key(key)?;
        let mut conn = self.get_connection().await?;

        let result: i32 = redis::cmd("EXISTS")
            .arg(&key_str)
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::NetworkError(format!("Redis EXISTS failed: {}", e)))?;

        Ok(result > 0)
    }

    async fn size(&self) -> Result<usize, Self::Error> {
        let mut conn = self.get_connection().await?;

        let result: i32 = redis::cmd("DBSIZE")
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::NetworkError(format!("Redis DBSIZE failed: {}", e)))?;

        Ok(result as usize)
    }

    async fn get_stats(&self) -> Result<CacheStats, Self::Error> {
        Ok(self.metrics.get_stats())
    }
}

#[async_trait]
impl<K, V> CacheWithStats<K, V> for RedisCache<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    fn get_detailed_stats(&self) -> crate::stats::DetailedCacheStats {
        self.metrics.get_detailed_stats()
    }

    fn reset_stats(&mut self) {
        self.metrics.reset();
    }
}

#[async_trait]
impl<K, V> DistributedCache<K, V> for RedisCache<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    async fn get_distributed(
        &self,
        key: &K,
    ) -> Result<Option<V>, <RedisCache<K, V> as Cache<K, V>>::Error> {
        self.get(key).await
    }

    async fn set_distributed(
        &self,
        key: K,
        value: V,
        ttl: Option<Duration>,
    ) -> Result<(), <RedisCache<K, V> as Cache<K, V>>::Error> {
        self.set(key, value, ttl).await
    }

    async fn invalidate_distributed(
        &self,
        key: &K,
    ) -> Result<(), <RedisCache<K, V> as Cache<K, V>>::Error> {
        self.delete(key).await
    }
}

#[async_trait]
impl<K, V> CompressedCache<K, V> for RedisCache<K, V>
where
    K: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
    V: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    async fn get_compressed(
        &self,
        key: &K,
    ) -> Result<Option<Vec<u8>>, <RedisCache<K, V> as Cache<K, V>>::Error> {
        let mut conn = self.pool.get().await.map_err(|e| {
            CacheError::NetworkError(format!("Failed to get connection: {}", e))
        })?;
        let key_str = serde_json::to_string(key).map_err(|e| {
            CacheError::SerializationError(format!("Failed to serialize key: {}", e))
        })?;
        let result: Option<Vec<u8>> = conn
            .get(&key_str)
            .await
            .map_err(|e| CacheError::NetworkError(format!("Failed to get from Redis: {}", e)))?;
        match result {
            Some(data) => {
                let decompressed = self.decompress_data(&data)?;
                Ok(Some(decompressed.into_bytes()))
            }
            None => Ok(None),
        }
    }

    async fn set_compressed(
        &self,
        key: K,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Result<(), <RedisCache<K, V> as Cache<K, V>>::Error> {
        let mut conn =
            self.pool.get().await.map_err(|e| {
                CacheError::NetworkError(format!("Failed to get connection: {}", e))
            })?;
        let key_str = serde_json::to_string(&key).map_err(|e| {
            CacheError::SerializationError(format!("Failed to serialize key: {}", e))
        })?;
        // Convert Vec<u8> to &str for compression
        let value_str = std::str::from_utf8(&value).map_err(|e| {
            CacheError::SerializationError(format!("Failed to convert value to str: {}", e))
        })?;
        let compressed = self.compress_data(value_str)?;
        if let Some(ttl) = ttl {
            let ttl_secs = ttl.as_secs();
            conn.set_ex::<_, _, ()>(&key_str, compressed, ttl_secs)
                .await
                .map_err(|e| CacheError::NetworkError(format!("Failed to set in Redis: {}", e)))?;
        } else {
            conn.set::<_, _, ()>(&key_str, compressed)
                .await
                .map_err(|e| CacheError::NetworkError(format!("Failed to set in Redis: {}", e)))?;
        }
        Ok(())
    }
}
