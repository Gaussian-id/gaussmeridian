//! Performance measurement utilities

use std::time::{Duration, Instant};
use tracing::debug;

/// Measure execution time of a function
pub async fn measure_time<F, Fut, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    (result, duration)
}

/// Performance timer for measuring code execution
pub struct Timer {
    start: Instant,
    name: String,
}

impl Timer {
    /// Create a new timer
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }

    /// Stop the timer and log the duration
    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();
        debug!("{} took {:?}", self.name, duration);
        duration
    }

    /// Stop the timer and return the duration without logging
    pub fn stop_quiet(self) -> Duration {
        self.start.elapsed()
    }
}
