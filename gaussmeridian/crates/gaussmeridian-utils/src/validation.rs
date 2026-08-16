//! Validation utilities

use std::collections::HashSet;

/// Validate that a string is not empty
pub fn validate_not_empty(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{} cannot be empty", field_name))
    } else {
        Ok(())
    }
}

/// Validate that a string has a minimum length
pub fn validate_min_length(value: &str, min_length: usize, field_name: &str) -> Result<(), String> {
    if value.len() < min_length {
        Err(format!(
            "{} must be at least {} characters long",
            field_name, min_length
        ))
    } else {
        Ok(())
    }
}

/// Validate that a string has a maximum length
pub fn validate_max_length(value: &str, max_length: usize, field_name: &str) -> Result<(), String> {
    if value.len() > max_length {
        Err(format!(
            "{} must be at most {} characters long",
            field_name, max_length
        ))
    } else {
        Ok(())
    }
}

/// Validate that a value is within a range
pub fn validate_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: T,
    max: T,
    field_name: &str,
) -> Result<(), String> {
    if value < min || value > max {
        Err(format!(
            "{} must be between {} and {}",
            field_name, min, max
        ))
    } else {
        Ok(())
    }
}

/// Validate that a value is one of the allowed values
pub fn validate_enum<T: PartialEq + std::fmt::Display>(
    value: T,
    allowed_values: &[T],
    field_name: &str,
) -> Result<(), String> {
    if !allowed_values.contains(&value) {
        let allowed = allowed_values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(format!("{} must be one of: {}", field_name, allowed))
    } else {
        Ok(())
    }
}

/// Validate that a string contains only allowed characters
pub fn validate_chars(value: &str, allowed_chars: &str, field_name: &str) -> Result<(), String> {
    let allowed_set: HashSet<char> = allowed_chars.chars().collect();
    let invalid_chars: Vec<char> = value.chars().filter(|c| !allowed_set.contains(c)).collect();

    if !invalid_chars.is_empty() {
        Err(format!(
            "{} contains invalid characters: {}",
            field_name,
            invalid_chars.iter().collect::<String>()
        ))
    } else {
        Ok(())
    }
}
