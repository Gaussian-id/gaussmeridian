//! Retry mechanism with exponential backoff for provider requests
//!
//! Provides configurable retry logic with exponential backoff to handle
//! transient failures gracefully.

use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub retryable_status_codes: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            retryable_status_codes: vec![429, 500, 502, 503, 504],
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new(max_retries: usize) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Check if a status code is retryable
    pub fn is_retryable(&self, status_code: u16) -> bool {
        self.retryable_status_codes.contains(&status_code)
    }
}

/// Execute a function with retry logic
pub async fn retry_with_backoff<F, T, E>(config: &RetryConfig, mut operation: F) -> Result<T, E>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>> + Send,
    E: std::fmt::Display + Send + Sync + 'static,
{
    let mut delay = config.initial_delay;
    let mut last_error = None;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("Operation succeeded after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);

                if attempt < config.max_retries {
                    warn!(
                        attempt = attempt + 1,
                        max_retries = config.max_retries,
                        delay_ms = delay.as_millis(),
                        "Retrying operation after error"
                    );

                    sleep(delay).await;

                    // Calculate next delay with exponential backoff
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * config.backoff_multiplier)
                            .min(config.max_delay.as_secs_f64()),
                    );
                } else {
                    warn!(
                        max_retries = config.max_retries,
                        "Operation failed after all retries"
                    );
                }
            }
        }
    }

    Err(last_error.expect("Should have at least one error"))
}

/// Execute a function with retry logic for HTTP requests
/// Returns the final `reqwest::Response` if successful.
pub async fn retry_http_request<F>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<reqwest::Response, reqwest::Error>
where
    F: FnMut() -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + Send>,
        > + Send,
{
    let mut delay = config.initial_delay;
    let mut last_error: Option<reqwest::Error> = None;

    for attempt in 0..=config.max_retries {
        match operation().await {
            Ok(response) => {
                let status = response.status();

                // Check if status code is retryable
                if config.is_retryable(status.as_u16()) && attempt < config.max_retries {
                    warn!(
                        attempt = attempt + 1,
                        status = %status,
                        delay_ms = delay.as_millis(),
                        "Retrying HTTP request due to retryable status code"
                    );

                    sleep(delay).await;
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * config.backoff_multiplier)
                            .min(config.max_delay.as_secs_f64()),
                    );
                    continue;
                }

                if attempt > 0 {
                    debug!("HTTP request succeeded after {} retries", attempt);
                }

                // Return the response wrapped in Ok
                return Ok(response);
            }
            Err(e) => {
                // Check if error is retryable (network errors, timeouts)
                let is_retryable = e.is_timeout() || e.is_connect() || e.is_request();

                if is_retryable && attempt < config.max_retries {
                    warn!(
                        attempt = attempt + 1,
                        error = %e,
                        delay_ms = delay.as_millis(),
                        "Retrying HTTP request after network error"
                    );

                    sleep(delay).await;
                    delay = Duration::from_secs_f64(
                        (delay.as_secs_f64() * config.backoff_multiplier)
                            .min(config.max_delay.as_secs_f64()),
                    );
                    last_error = Some(e);
                } else if attempt >= config.max_retries {
                    warn!(
                        max_retries = config.max_retries,
                        "HTTP request failed after all retries"
                    );
                    return Err(e);
                } else {
                    // Non-retryable error before max retries reached
                    last_error = Some(e);
                }
            }
        }
    }

    Err(last_error.expect("HTTP request failed after all retries with no error captured"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_config() {
        let config = RetryConfig::default();
        assert!(config.is_retryable(429));
        assert!(config.is_retryable(500));
        assert!(!config.is_retryable(400));
    }

    #[tokio::test]
    async fn test_retry_success() {
        let config = RetryConfig::new(3);
        let mut attempts = 0;

        let result = retry_with_backoff(&config, || {
            attempts += 1;
            Box::pin(async move {
                if attempts < 2 {
                    Err("Temporary error")
                } else {
                    Ok("Success")
                }
            })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(attempts, 2);
    }
}
