use thiserror::Error;

#[derive(Error, Debug)]
pub enum GaussMeridianError {
    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Concurrency error: {0}")]
    Concurrency(String),
}

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Provider unavailable: {0}")]
    Unavailable(String),

    #[error("Provider timeout: {0}")]
    Timeout(String),

    #[error("Provider authentication failed: {0}")]
    Authentication(String),

    #[error("Provider rate limit: {0}")]
    RateLimit(String),

    #[error("Provider bad request: {0}")]
    BadRequest(String),

    #[error("Provider internal error: {0}")]
    Internal(String),
}

impl From<GaussMeridianError> for crate::shared::ErrorResponse {
    fn from(error: GaussMeridianError) -> Self {
        let (message, error_type, code) = match error {
            GaussMeridianError::Authentication(msg) => (msg, "authentication_error", Some("401")),
            GaussMeridianError::Authorization(msg) => (msg, "authorization_error", Some("403")),
            GaussMeridianError::InvalidRequest(msg) => (msg, "invalid_request_error", Some("400")),
            GaussMeridianError::ModelNotFound(msg) => (msg, "model_not_found_error", Some("404")),
            GaussMeridianError::RateLimit(msg) => (msg, "rate_limit_error", Some("429")),
            GaussMeridianError::Timeout(msg) => (msg, "timeout_error", Some("408")),
            _ => (error.to_string(), "internal_error", Some("500")),
        };

        crate::shared::ErrorResponse {
            error: crate::shared::ErrorDetail {
                message,
                r#type: error_type.to_string(),
                param: None,
                code: code.map(String::from),
            },
        }
    }
}
