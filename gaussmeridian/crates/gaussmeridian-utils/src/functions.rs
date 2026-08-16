//! Utility functions for common operations

use std::time::Duration;
use tokio::time::timeout;
use tracing::warn;

/// Retry a function with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut f: F,
    max_retries: usize,
    initial_delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut delay = initial_delay;

    for attempt in 0..=max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt == max_retries {
                    return Err(e);
                }

                warn!(
                    "Attempt {} failed: {:?}, retrying in {:?}",
                    attempt + 1,
                    e,
                    delay
                );
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
    }

    unreachable!()
}

/// Execute a function with a timeout
pub async fn with_timeout<F, Fut, T>(
    duration: Duration,
    f: F,
) -> Result<T, tokio::time::error::Elapsed>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    timeout(duration, f()).await
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("req_{}_{}", timestamp, rand::random::<u32>())
}

/// Format bytes into human readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Format duration into human readable format
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if secs == 0 {
        format!("{}ms", millis)
    } else if millis == 0 {
        format!("{}s", secs)
    } else {
        format!("{}s {}ms", secs, millis)
    }
}
