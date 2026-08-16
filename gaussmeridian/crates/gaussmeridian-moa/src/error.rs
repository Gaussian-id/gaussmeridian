use std::fmt;
use thiserror::Error;
use serde_json;
use reqwest;
use redb;
use crate::security;
use toml;
use std::result;
use tokio::task::JoinError;
use validator;

/// Comprehensive error type for the MoA system
#[derive(Debug, Error)]
pub enum MoaError {
    /// Configuration related errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Storage backend errors
    #[error("Storage error: {message}")]
    Storage {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Network and API communication errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        #[source]
        source: Option<reqwest::Error>,
    },
    
    /// Data serialization/deserialization errors
    #[error("Data serialization error: {message}")]
    Serialization {
        message: String,
        #[source]
        source: Option<serde_json::Error>,
    },
    
    /// Agent-specific errors
    #[error("Agent error ({agent_id}): {message}")]
    Agent {
        message: String,
        agent_id: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Strategy execution errors
    #[error("Strategy error: {message}")]
    Strategy {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Resource management errors
    #[error("Resource error ({resource_type}): {message}")]
    Resource {
        message: String,
        resource_type: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Security errors
    #[error("Security error: {message}")]
    Security {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Timeout errors
    #[error("Timeout error ({duration:?}): {message}")]
    Timeout {
        message: String,
        duration: std::time::Duration,
    },
    
    /// Not found errors
    #[error("Not found: {message}{}", .context.as_ref().map(|s| format!(" ({})", s)).unwrap_or_default())]
    NotFound {
        message: String,
        context: Option<String>,
    },
    
    /// API errors from external services
    #[error("API Error from {service}: Status {status} - {message}")]
    ApiError {
        service: String,
        status: u16,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    /// Internal errors
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Metrics errors
    #[error("Metrics error: {message}")]
    Metrics {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Task errors
    #[error("Task error: {message}")]
    Task {
        message: String,
        #[source]
        source: Option<JoinError>,
    },

    /// Validation errors
    #[error("Validation error{}: {message}", .field.as_ref().map(|f| format!(" in field '{}'", f)).unwrap_or_default())]
    Validation {
        message: String,
        field: Option<String>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Embedding errors
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Other errors
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Convenience type alias for Results with MoaError
pub type MoaResult<T> = result::Result<T, MoaError>;

impl MoaError {
    /// Creates a new Config error
    pub fn config<S: Into<String>>(message: S, source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>) -> Self {
        MoaError::Config {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Storage error
    pub fn storage<S: Into<String>>(message: S, source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>) -> Self {
        MoaError::Storage {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Network error
    pub fn network<S: Into<String>>(message: S, source: Option<reqwest::Error>) -> Self {
        MoaError::Network {
            message: message.into(),
            source,
        }
    }

    /// Creates a new Agent error
    pub fn agent<S: Into<String>>(
        message: S,
        agent_id: S,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        MoaError::Agent {
            message: message.into(),
            agent_id: agent_id.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Strategy error
    pub fn strategy<E>(message: impl Into<String>, source: Option<E>) -> Self 
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        MoaError::Strategy {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Resource error
    pub fn resource<S: Into<String>>(
        message: S,
        resource_type: S,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        MoaError::Resource {
            message: message.into(),
            resource_type: resource_type.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Security error
    pub fn security<S: Into<String>>(message: S, source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>) -> Self {
        MoaError::Security {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Timeout error
    pub fn timeout(message: impl Into<String>, duration: std::time::Duration) -> Self {
        MoaError::Timeout {
            message: message.into(),
            duration,
        }
    }

    /// Creates a new NotFound error
    pub fn not_found(message: impl Into<String>, context: Option<impl Into<String>>) -> Self {
        MoaError::NotFound {
            message: message.into(),
            context: context.map(Into::into),
        }
    }

    /// Creates a new API error
    pub fn api<E>(service: impl Into<String>, status: u16, message: impl Into<String>, source: Option<E>) -> Self 
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        MoaError::ApiError {
            service: service.into(),
            status,
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Internal error
    pub fn internal<S: Into<String>>(message: S, source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>) -> Self {
        MoaError::Internal {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Metrics error
    pub fn metrics<S: Into<String>>(message: S, source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>) -> Self {
        MoaError::Metrics {
            message: message.into(),
            source: source.map(Into::into),
        }
    }

    /// Creates a new Task error
    pub fn task<S: Into<String>>(message: S, source: Option<JoinError>) -> Self {
        MoaError::Task {
            message: message.into(),
            source,
        }
    }

    /// Creates a new Validation error
    pub fn validation<S: Into<String>>(
        message: S,
        field: Option<S>,
        source: Option<impl Into<Box<dyn std::error::Error + Send + Sync>>>,
    ) -> Self {
        MoaError::Validation {
            message: message.into(),
            field: field.map(Into::into),
            source: source.map(Into::into),
        }
    }

    /// Checks if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            MoaError::Network { .. } |
            MoaError::Resource { .. } |
            MoaError::Task { .. }
        )
    }

    /// Checks if the error is fatal
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            MoaError::Config { .. } |
            MoaError::Security { .. } |
            MoaError::Internal { .. }
        )
    }

    /// Returns the type of the error
    pub fn error_type(&self) -> &'static str {
        match self {
            MoaError::Config { .. } => "config",
            MoaError::Storage { .. } => "storage",
            MoaError::Network { .. } => "network",
            MoaError::Serialization { .. } => "serialization",
            MoaError::Agent { .. } => "agent",
            MoaError::Strategy { .. } => "strategy",
            MoaError::Resource { .. } => "resource",
            MoaError::Security { .. } => "security",
            MoaError::Timeout { .. } => "timeout",
            MoaError::NotFound { .. } => "not_found",
            MoaError::ApiError { .. } => "api_error",
            MoaError::Internal { .. } => "internal",
            MoaError::Metrics { .. } => "metrics",
            MoaError::Task { .. } => "task",
            MoaError::Validation { .. } => "validation",
            MoaError::Embedding(_) => "embedding",
            MoaError::Other(_) => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceErrorKind {
    Limit,
    Timeout,
    Processing,
    Memory,
    Disk,
    Network,
}

impl fmt::Display for ResourceErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceErrorKind::Limit => write!(f, "Resource limit exceeded"),
            ResourceErrorKind::Timeout => write!(f, "Operation timed out"),
            ResourceErrorKind::Processing => write!(f, "Processing error"),
            ResourceErrorKind::Memory => write!(f, "Memory allocation error"),
            ResourceErrorKind::Disk => write!(f, "Disk space error"),
            ResourceErrorKind::Network => write!(f, "Network resource error"),
        }
    }
}

// Implement From traits for common error types
impl From<std::io::Error> for MoaError {
    fn from(err: std::io::Error) -> Self {
        MoaError::internal(err.to_string(), Some(err))
    }
}

impl From<reqwest::Error> for MoaError {
    fn from(err: reqwest::Error) -> Self {
        MoaError::network(err.to_string(), Some(err))
    }
}

impl From<toml::de::Error> for MoaError {
    fn from(err: toml::de::Error) -> Self {
        MoaError::config(err.to_string(), Some(err))
    }
}

impl From<redb::Error> for MoaError {
    fn from(err: redb::Error) -> Self {
        MoaError::storage(err.to_string(), Some(err))
    }
}

impl From<redb::StorageError> for MoaError {
    fn from(err: redb::StorageError) -> Self {
        MoaError::storage(err.to_string(), Some(err))
    }
}

impl From<redb::DatabaseError> for MoaError {
    fn from(err: redb::DatabaseError) -> Self {
        MoaError::storage(err.to_string(), Some(err))
    }
}

impl From<redb::TableError> for MoaError {
    fn from(err: redb::TableError) -> Self {
        MoaError::storage(err.to_string(), Some(Box::new(err)))
    }
}

impl From<security::SecurityError> for MoaError {
    fn from(err: security::SecurityError) -> Self {
        MoaError::security(err.to_string(), Some(err))
    }
}

impl From<rustyline::error::ReadlineError> for MoaError {
    fn from(err: rustyline::error::ReadlineError) -> Self {
        MoaError::internal(err.to_string(), Some(err))
    }
}

impl From<JoinError> for MoaError {
    fn from(err: JoinError) -> Self {
        MoaError::task(err.to_string(), Some(err))
    }
}

impl From<serde_json::Error> for MoaError {
    fn from(err: serde_json::Error) -> Self {
        MoaError::Serialization {
            message: err.to_string(),
            source: Some(err),
        }
    }
}

impl From<validator::ValidationErrors> for MoaError {
    fn from(err: validator::ValidationErrors) -> Self {
        MoaError::Validation {
            message: err.to_string(),
            field: None,
            source: Some(Box::new(err)),
        }
    }
}

impl From<validator::ValidationError> for MoaError {
    fn from(err: validator::ValidationError) -> Self {
        MoaError::Validation {
            message: err.to_string(),
            field: None,
            source: Some(Box::new(err)),
        }
    }
}

impl From<tokio::sync::AcquireError> for MoaError {
    fn from(err: tokio::sync::AcquireError) -> Self {
        MoaError::resource(
            format!("Failed to acquire semaphore permit: {}", err),
            "Semaphore".to_string(),
            Some(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err.to_string())) as Box<dyn std::error::Error + Send + Sync>)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_creation_and_display() {
        let err = MoaError::timeout("Operation took too long", Duration::from_secs(30));
        assert!(err.to_string().contains("Operation took too long"));

        let err_agent = MoaError::agent("Failed to process".to_string(), "test_agent".to_string(), None::<Box<dyn std::error::Error + Send + Sync>>);
        assert_eq!(err_agent.to_string(), "Agent error (test_agent): Failed to process");

        let err_resource = MoaError::resource(
            "Too many requests".to_string(), // message
            ResourceErrorKind::Limit.to_string(), // resource_type
            None::<Box<dyn std::error::Error + Send + Sync>>
        );
        assert!(err_resource.to_string().contains("Too many requests"));
        assert!(err_resource.to_string().contains("Resource limit exceeded")); // Display for ResourceErrorKind::Limit
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let moa_err_io: MoaError = io_err.into();
        assert!(moa_err_io.to_string().contains("file not found"));

        // Use a valid variant of security::SecurityError
        let security_err_concrete = security::SecurityError::KeyManagement("invalid key".to_string());
        let moa_err_sec: MoaError = security_err_concrete.into();
        assert!(moa_err_sec.to_string().contains("invalid key"));
    }

    #[test]
    fn test_error_creation() {
        let err_config = MoaError::config("test error".to_string(), None::<Box<dyn std::error::Error + Send + Sync>>);
        assert_eq!(err_config.error_type(), "config");
        assert!(!err_config.is_retryable());
        assert!(err_config.is_fatal());

        let err_network = MoaError::network("test error".to_string(), None);
        assert_eq!(err_network.error_type(), "network");
        assert!(err_network.is_retryable());
        assert!(!err_network.is_fatal());
    }

    #[test]
    fn test_error_display() {
        let err_validation = MoaError::validation(
            "Invalid input".to_string(),
            Some("field".to_string()),
            None::<Box<dyn std::error::Error + Send + Sync>>,
        );
        assert!(err_validation.to_string().contains("Invalid input"));
        assert!(err_validation.to_string().contains("in field 'field'"));
    }
}