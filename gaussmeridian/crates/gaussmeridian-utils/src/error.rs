//! Error handling utilities

use std::fmt;

/// Convert any error type to a string
pub fn error_to_string<E: fmt::Display>(error: E) -> String {
    error.to_string()
}

/// Convert any error type to a debug string
pub fn error_to_debug_string<E: fmt::Debug>(error: E) -> String {
    format!("{:?}", error)
}

/// Chain multiple errors together
pub fn chain_errors<E1, E2>(error1: E1, error2: E2) -> String
where
    E1: fmt::Display,
    E2: fmt::Display,
{
    format!("{}: {}", error1, error2)
}
