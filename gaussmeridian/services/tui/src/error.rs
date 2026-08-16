//! Unified error handling for GaussMeridian TUI
//!
//! This module provides consistent error types that match the backend API error format
//! for seamless integration between the TUI and GaussMeridian server.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Error type categories (matches backend ErrorType)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// Invalid request format or parameters
    InvalidRequestError,
    /// Authentication failed
    AuthenticationError,
    /// Authorization/permission denied
    PermissionError,
    /// Resource not found
    NotFoundError,
    /// Rate limit exceeded
    RateLimitError,
    /// Server-side error
    ServerError,
    /// Service temporarily unavailable
    ServiceUnavailableError,
    /// Validation error
    ValidationError,
    /// Conflict (e.g., duplicate resource)
    ConflictError,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorType::InvalidRequestError => write!(f, "invalid_request_error"),
            ErrorType::AuthenticationError => write!(f, "authentication_error"),
            ErrorType::PermissionError => write!(f, "permission_error"),
            ErrorType::NotFoundError => write!(f, "not_found_error"),
            ErrorType::RateLimitError => write!(f, "rate_limit_error"),
            ErrorType::ServerError => write!(f, "server_error"),
            ErrorType::ServiceUnavailableError => write!(f, "service_unavailable_error"),
            ErrorType::ValidationError => write!(f, "validation_error"),
            ErrorType::ConflictError => write!(f, "conflict_error"),
        }
    }
}

/// Specific error codes for programmatic handling (matches backend ErrorCode)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    // Authentication errors
    InvalidApiKey,
    ExpiredApiKey,
    MissingApiKey,
    InvalidToken,
    ExpiredToken,
    InvalidCredentials,
    AccountDisabled,

    // Authorization errors
    InsufficientPermissions,
    TenantAccessDenied,
    ResourceAccessDenied,

    // Validation errors
    InvalidEmail,
    WeakPassword,
    InvalidUsername,
    MissingRequiredField,
    InvalidFieldFormat,
    EmptyPrompt,
    EmptyInput,
    BatchTooLarge,

    // Resource errors
    ModelNotFound,
    ProviderNotFound,
    UserNotFound,
    ApiKeyNotFound,
    TenantNotFound,
    RequestNotFound,

    // Conflict errors
    EmailAlreadyExists,
    UsernameAlreadyExists,
    ApiKeyAlreadyExists,

    // Rate limiting
    RateLimitExceeded,
    QuotaExceeded,
    DailyLimitExceeded,

    // Server errors
    InternalError,
    DatabaseError,
    ProviderError,
    ConfigurationError,

    // Service errors
    ServiceUnavailable,
    ProviderUnavailable,
    MaintenanceMode,

    // Request errors
    PayloadTooLarge,
    UnsupportedMediaType,
    RequestTimeout,

    // Network errors (TUI-specific)
    NetworkError,
    ConnectionRefused,
    DnsError,

    // Generic
    Unknown,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_else(|_| "unknown".to_string());
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// API error response structure (matches backend ApiErrorResponse)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

/// Detailed error information from API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    /// Human-readable error message
    pub message: String,
    /// Error type category
    #[serde(rename = "type")]
    pub error_type: ErrorType,
    /// Specific error code
    pub code: ErrorCode,
    /// Optional parameter that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    /// Request ID for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Main API error type for the TUI
#[derive(Debug, Error)]
pub struct ApiError {
    /// HTTP status code
    pub status: u16,
    /// Error type category
    pub error_type: ErrorType,
    /// Specific error code
    pub code: ErrorCode,
    /// Human-readable message
    pub message: String,
    /// Optional parameter that caused the error
    pub param: Option<String>,
    /// Request ID for debugging
    #[allow(dead_code)]
    pub request_id: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.status, self.code, self.message)
    }
}

impl ApiError {
    /// Create a new API error
    pub fn new(
        status: u16,
        error_type: ErrorType,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            error_type,
            code,
            message: message.into(),
            param: None,
            request_id: None,
        }
    }

    /// Add parameter information to the error
    pub fn with_param(mut self, param: impl Into<String>) -> Self {
        self.param = Some(param.into());
        self
    }

    /// Add request ID to the error
    #[allow(dead_code)]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Create an error from an HTTP response
    pub async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status().as_u16();
        
        // Try to parse the error response body
        if let Ok(error_response) = response.json::<ApiErrorResponse>().await {
            return Self {
                status,
                error_type: error_response.error.error_type,
                code: error_response.error.code,
                message: error_response.error.message,
                param: error_response.error.param,
                request_id: error_response.error.request_id,
            };
        }

        // Fallback to status-based error
        Self::from_status(status, None)
    }

    /// Create an error from HTTP status code
    pub fn from_status(status: u16, status_text: Option<&str>) -> Self {
        match status {
            400 => Self::new(
                status,
                ErrorType::InvalidRequestError,
                ErrorCode::Unknown,
                status_text.unwrap_or("Bad request"),
            ),
            401 => Self::new(
                status,
                ErrorType::AuthenticationError,
                ErrorCode::InvalidCredentials,
                status_text.unwrap_or("Authentication required"),
            ),
            403 => Self::new(
                status,
                ErrorType::PermissionError,
                ErrorCode::InsufficientPermissions,
                status_text.unwrap_or("Permission denied"),
            ),
            404 => Self::new(
                status,
                ErrorType::NotFoundError,
                ErrorCode::Unknown,
                status_text.unwrap_or("Resource not found"),
            ),
            409 => Self::new(
                status,
                ErrorType::ConflictError,
                ErrorCode::Unknown,
                status_text.unwrap_or("Conflict"),
            ),
            429 => Self::new(
                status,
                ErrorType::RateLimitError,
                ErrorCode::RateLimitExceeded,
                status_text.unwrap_or("Rate limit exceeded"),
            ),
            500 => Self::new(
                status,
                ErrorType::ServerError,
                ErrorCode::InternalError,
                status_text.unwrap_or("Internal server error"),
            ),
            503 => Self::new(
                status,
                ErrorType::ServiceUnavailableError,
                ErrorCode::ServiceUnavailable,
                status_text.unwrap_or("Service unavailable"),
            ),
            _ => Self::new(
                status,
                ErrorType::ServerError,
                ErrorCode::Unknown,
                status_text.unwrap_or("An error occurred"),
            ),
        }
    }

    /// Create a network error
    pub fn network_error(message: impl Into<String>) -> Self {
        Self::new(
            0,
            ErrorType::ServerError,
            ErrorCode::NetworkError,
            message,
        )
    }

    /// Create a connection refused error
    pub fn connection_refused() -> Self {
        Self::new(
            0,
            ErrorType::ServiceUnavailableError,
            ErrorCode::ConnectionRefused,
            "Connection refused. Is the server running?",
        )
    }

    /// Create a timeout error
    pub fn timeout() -> Self {
        Self::new(
            408,
            ErrorType::ServerError,
            ErrorCode::RequestTimeout,
            "Request timed out",
        )
    }

    /// Check if this is an authentication error
    #[allow(dead_code)]
    pub fn is_auth_error(&self) -> bool {
        self.error_type == ErrorType::AuthenticationError || self.status == 401
    }

    /// Check if this is a permission error
    #[allow(dead_code)]
    pub fn is_permission_error(&self) -> bool {
        self.error_type == ErrorType::PermissionError || self.status == 403
    }

    /// Check if this is a validation error
    #[allow(dead_code)]
    pub fn is_validation_error(&self) -> bool {
        self.error_type == ErrorType::ValidationError
            || self.error_type == ErrorType::InvalidRequestError
    }

    /// Check if this is a rate limit error
    pub fn is_rate_limit_error(&self) -> bool {
        self.error_type == ErrorType::RateLimitError || self.status == 429
    }

    /// Check if this is a server error (should be retried)
    pub fn is_server_error(&self) -> bool {
        self.error_type == ErrorType::ServerError || self.status >= 500
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        self.is_server_error()
            || self.is_rate_limit_error()
            || self.code == ErrorCode::NetworkError
            || self.code == ErrorCode::RequestTimeout
    }

    /// Get a user-friendly error message
    #[allow(dead_code)]
    pub fn user_message(&self) -> String {
        match self.code {
            ErrorCode::InvalidApiKey => "Your API key is invalid. Please check your settings.".to_string(),
            ErrorCode::ExpiredApiKey => "Your API key has expired. Please generate a new one.".to_string(),
            ErrorCode::InvalidToken | ErrorCode::ExpiredToken => "Your session has expired. Please sign in again.".to_string(),
            ErrorCode::InvalidCredentials => "Invalid email or password. Please try again.".to_string(),
            ErrorCode::AccountDisabled => "Your account has been disabled. Please contact support.".to_string(),
            ErrorCode::InsufficientPermissions => "You do not have permission to perform this action.".to_string(),
            ErrorCode::InvalidEmail => "Please enter a valid email address.".to_string(),
            ErrorCode::WeakPassword => "Password must be at least 8 characters long.".to_string(),
            ErrorCode::EmailAlreadyExists => "This email is already registered.".to_string(),
            ErrorCode::UsernameAlreadyExists => "This username is already taken.".to_string(),
            ErrorCode::RateLimitExceeded => "Too many requests. Please wait a moment and try again.".to_string(),
            ErrorCode::QuotaExceeded => "You have exceeded your usage quota.".to_string(),
            ErrorCode::ServiceUnavailable => "Service is temporarily unavailable. Please try again later.".to_string(),
            ErrorCode::ConnectionRefused => "Cannot connect to server. Is GaussMeridian running?".to_string(),
            ErrorCode::NetworkError => "Network error. Please check your connection.".to_string(),
            ErrorCode::RequestTimeout => "Request timed out. Please try again.".to_string(),
            _ => self.message.clone(),
        }
    }
}

/// Convert reqwest errors to ApiError
impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return Self::timeout();
        }
        if err.is_connect() {
            return Self::connection_refused();
        }
        Self::network_error(err.to_string())
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_status() {
        let err = ApiError::from_status(401, None);
        assert_eq!(err.status, 401);
        assert!(err.is_auth_error());
    }

    #[test]
    fn test_error_retryable() {
        let server_err = ApiError::from_status(500, None);
        assert!(server_err.is_retryable());

        let rate_limit_err = ApiError::from_status(429, None);
        assert!(rate_limit_err.is_retryable());

        let auth_err = ApiError::from_status(401, None);
        assert!(!auth_err.is_retryable());
    }

    #[test]
    fn test_user_message() {
        let err = ApiError::new(
            401,
            ErrorType::AuthenticationError,
            ErrorCode::InvalidApiKey,
            "API key validation failed",
        );
        assert_eq!(
            err.user_message(),
            "Your API key is invalid. Please check your settings."
        );
    }
}
