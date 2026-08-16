//! Authentication error types

/// Authentication errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid API key")]
    Invalid,
    #[error("Credential has expired")]
    Expired,
    #[error("Missing authorization header")]
    MissingHeader,
    #[error("Invalid authorization format")]
    InvalidFormat,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Service temporarily unavailable: {0}")]
    Unavailable(String),
}
