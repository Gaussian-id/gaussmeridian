//! Tests for the utility functions

use std::time::Duration;

use crate::{
    config::{get_env_bool, get_env_int, get_env_or_default},
    error::{chain_errors, error_to_debug_string, error_to_string},
    functions::{
        format_bytes, format_duration, generate_request_id, retry_with_backoff, with_timeout,
    },
    validation::{
        validate_chars, validate_enum, validate_max_length, validate_min_length,
        validate_not_empty, validate_range,
    },
};

#[tokio::test]
async fn test_retry_with_backoff() {
    let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let attempts_clone = attempts.clone();
    let result = retry_with_backoff(
        move || {
            let attempts = attempts_clone.clone();
            async move {
                let current = attempts.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if current < 2 {
                    Err("temporary error")
                } else {
                    Ok("success")
                }
            }
        },
        5,
        Duration::from_millis(10),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(attempts.load(std::sync::atomic::Ordering::Relaxed), 3);
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
}

#[test]
fn test_format_duration() {
    assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
    assert_eq!(format_duration(Duration::from_secs(5)), "5s");
    assert_eq!(format_duration(Duration::from_millis(1500)), "1s 500ms");
}

#[test]
fn test_validation() {
    assert!(validate_not_empty("test", "field").is_ok());
    assert!(validate_not_empty("", "field").is_err());

    assert!(validate_min_length("test", 3, "field").is_ok());
    assert!(validate_min_length("ab", 3, "field").is_err());

    assert!(validate_max_length("test", 5, "field").is_ok());
    assert!(validate_max_length("testing", 5, "field").is_err());

    assert!(validate_range(5, 1, 10, "field").is_ok());
    assert!(validate_range(0, 1, 10, "field").is_err());
    assert!(validate_range(11, 1, 10, "field").is_err());
}
