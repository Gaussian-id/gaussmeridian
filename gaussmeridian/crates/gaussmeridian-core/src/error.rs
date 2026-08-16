//! Error types for the core router

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GaussMeridianError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Cache error: {0}")]
    CacheError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}
