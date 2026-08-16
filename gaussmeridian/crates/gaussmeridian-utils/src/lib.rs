//! Utility functions and helpers for GaussMeridian
//!
//! This crate provides common utility functions, macros, and helpers
//! used throughout the GaussMeridian ecosystem.

pub mod config;
pub mod error;
pub mod functions;
pub mod performance;
pub mod security;
pub mod validation;

pub use config::{get_env_bool, get_env_int, get_env_or_default};
pub use error::{chain_errors, error_to_debug_string, error_to_string};
pub use functions::{
    format_bytes, format_duration, generate_request_id, retry_with_backoff, with_timeout,
};
pub use performance::{measure_time, Timer};
pub use security::{
    AuditLogEntry, AuditLogger, AuditStatus, EncryptionUtils, InputValidator, ValidationResult,
};
pub use validation::{
    validate_chars, validate_enum, validate_max_length, validate_min_length, validate_not_empty,
    validate_range,
};

#[cfg(test)]
mod tests;
