//! Cache error types

use thiserror::Error;

/// Cache error types
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache key not found: {0}")]
    KeyNotFound(String),

    #[error("Cache is full")]
    CacheFull,

    #[error("Cache operation timeout")]
    Timeout,

    #[error("Cache serialization error: {0}")]
    SerializationError(String),

    #[error("Cache deserialization error: {0}")]
    DeserializationError(String),

    #[error("Cache compression error: {0}")]
    CompressionError(String),

    #[error("Cache decompression error: {0}")]
    DecompressionError(String),

    #[error("Cache encryption error: {0}")]
    EncryptionError(String),

    #[error("Cache decryption error: {0}")]
    DecryptionError(String),

    #[error("Cache configuration error: {0}")]
    ConfigurationError(String),

    #[error("Cache statistics error: {0}")]
    StatisticsError(String),

    #[error("Cache warming error: {0}")]
    WarmingError(String),

    #[error("Cache prefetch error: {0}")]
    PrefetchError(String),

    #[error("Cache eviction error: {0}")]
    EvictionError(String),

    #[error("Cache health check failed: {0}")]
    HealthCheckError(String),

    #[error("Cache monitoring error: {0}")]
    MonitoringError(String),

    #[error("Cache distributed operation error: {0}")]
    DistributedError(String),

    #[error("Cache consistency error: {0}")]
    ConsistencyError(String),

    #[error("Cache lock error: {0}")]
    LockError(String),

    #[error("Cache memory error: {0}")]
    MemoryError(String),

    #[error("Cache network error: {0}")]
    NetworkError(String),

    #[error("Cache authentication error: {0}")]
    AuthenticationError(String),

    #[error("Cache authorization error: {0}")]
    AuthorizationError(String),

    #[error("Cache rate limit exceeded")]
    RateLimitExceeded,

    #[error("Cache quota exceeded")]
    QuotaExceeded,

    #[error("Cache maintenance mode")]
    MaintenanceMode,

    #[error("Cache not available: {0}")]
    NotAvailable(String),

    #[error("Cache internal error: {0}")]
    InternalError(String),

    #[error("Cache validation error: {0}")]
    ValidationError(String),

    #[error("Cache version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("Cache migration error: {0}")]
    MigrationError(String),

    #[error("Cache backup error: {0}")]
    BackupError(String),

    #[error("Cache restore error: {0}")]
    RestoreError(String),

    #[error("Cache cleanup error: {0}")]
    CleanupError(String),

    #[error("Cache initialization error: {0}")]
    InitializationError(String),

    #[error("Cache shutdown error: {0}")]
    ShutdownError(String),
}

/// Redis-specific error types
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("Redis connection failed: {0}")]
    Connection(String),

    #[error("Redis operation failed: {0}")]
    Operation(String),

    #[error("Redis authentication failed: {0}")]
    Authentication(String),

    #[error("Redis cluster error: {0}")]
    Cluster(String),

    #[error("Redis pipeline error: {0}")]
    Pipeline(String),

    #[error("Redis transaction error: {0}")]
    Transaction(String),

    #[error("Redis pub/sub error: {0}")]
    PubSub(String),

    #[error("Redis Lua script error: {0}")]
    LuaScript(String),

    #[error("Redis memory error: {0}")]
    Memory(String),

    #[error("Redis replication error: {0}")]
    Replication(String),

    #[error("Redis persistence error: {0}")]
    Persistence(String),

    #[error("Redis sentinel error: {0}")]
    Sentinel(String),

    #[error("Redis timeout: {0}")]
    Timeout(String),

    #[error("Redis protocol error: {0}")]
    Protocol(String),

    #[error("Redis data type error: {0}")]
    DataType(String),

    #[error("Redis key space error: {0}")]
    KeySpace(String),

    #[error("Redis value error: {0}")]
    Value(String),

    #[error("Redis command error: {0}")]
    Command(String),

    #[error("Redis argument error: {0}")]
    Argument(String),

    #[error("Redis permission error: {0}")]
    Permission(String),

    #[error("Redis quota error: {0}")]
    Quota(String),

    #[error("Redis maintenance error: {0}")]
    Maintenance(String),

    #[error("Redis cluster node error: {0}")]
    ClusterNode(String),

    #[error("Redis cluster slot error: {0}")]
    ClusterSlot(String),

    #[error("Redis cluster hash error: {0}")]
    ClusterHash(String),

    #[error("Redis cluster migration error: {0}")]
    ClusterMigration(String),

    #[error("Redis cluster failover error: {0}")]
    ClusterFailover(String),
}

/// Memory cache error types
#[derive(Debug, Error)]
pub enum MemoryCacheError {
    #[error("Memory allocation failed: {0}")]
    Allocation(String),

    #[error("Memory lock failed: {0}")]
    Lock(String),

    #[error("Memory corruption: {0}")]
    Corruption(String),

    #[error("Memory overflow: {0}")]
    Overflow(String),

    #[error("Memory underflow: {0}")]
    Underflow(String),

    #[error("Memory fragmentation: {0}")]
    Fragmentation(String),

    #[error("Memory leak detected: {0}")]
    Leak(String),

    #[error("Memory access violation: {0}")]
    AccessViolation(String),

    #[error("Memory page fault: {0}")]
    PageFault(String),

    #[error("Memory stack overflow: {0}")]
    StackOverflow(String),

    #[error("Memory heap corruption: {0}")]
    HeapCorruption(String),

    #[error("Memory double free: {0}")]
    DoubleFree(String),

    #[error("Memory use after free: {0}")]
    UseAfterFree(String),

    #[error("Memory buffer overflow: {0}")]
    BufferOverflow(String),

    #[error("Memory null pointer: {0}")]
    NullPointer(String),

    #[error("Memory uninitialized: {0}")]
    Uninitialized(String),

    #[error("Memory alignment error: {0}")]
    Alignment(String),

    #[error("Memory boundary error: {0}")]
    Boundary(String),

    #[error("Memory size mismatch: {0}")]
    SizeMismatch(String),
}

/// Error conversion implementations
impl From<RedisError> for CacheError {
    fn from(err: RedisError) -> Self {
        match err {
            RedisError::Connection(msg) => CacheError::NetworkError(msg),
            RedisError::Authentication(msg) => CacheError::AuthenticationError(msg),
            RedisError::Operation(msg) => CacheError::InternalError(msg),
            RedisError::Timeout(_msg) => CacheError::Timeout,
            RedisError::Quota(_msg) => CacheError::QuotaExceeded,
            RedisError::Maintenance(_msg) => CacheError::MaintenanceMode,
            _ => CacheError::InternalError(format!("Redis error: {:?}", err)),
        }
    }
}

impl From<MemoryCacheError> for CacheError {
    fn from(err: MemoryCacheError) -> Self {
        match err {
            MemoryCacheError::Allocation(msg) => CacheError::MemoryError(msg),
            MemoryCacheError::Lock(msg) => CacheError::LockError(msg),
            MemoryCacheError::Corruption(msg) => CacheError::InternalError(msg),
            MemoryCacheError::Overflow(msg) => CacheError::MemoryError(msg),
            MemoryCacheError::Underflow(msg) => CacheError::MemoryError(msg),
            _ => CacheError::InternalError(format!("Memory cache error: {:?}", err)),
        }
    }
}

impl From<std::io::Error> for CacheError {
    fn from(err: std::io::Error) -> Self {
        CacheError::InternalError(format!("IO error: {}", err))
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(err: serde_json::Error) -> Self {
        CacheError::SerializationError(format!("JSON error: {}", err))
    }
}

impl From<tokio::time::error::Elapsed> for CacheError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        CacheError::Timeout
    }
}
