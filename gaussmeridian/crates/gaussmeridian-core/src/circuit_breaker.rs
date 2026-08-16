//! Circuit breaker pattern implementation
//!
//! Provides fault tolerance for provider calls by tracking failures and
//! temporarily blocking requests to unhealthy providers.
//!
//! ## States
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Provider is unhealthy, requests are immediately rejected
//! - **Half-Open**: Testing if provider has recovered
//!
//! ## Configuration
//! - `failure_threshold`: Number of failures before opening the circuit
//! - `success_threshold`: Successes needed in half-open state to close
//! - `timeout`: Duration to wait before transitioning from open to half-open

use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Provider is unhealthy - requests are rejected
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

impl CircuitState {
    /// Returns the availability health factor S(t) for the legacy deterministic provider score.
    ///
    /// Per the GaussMeridian provider selection algorithm (docs/knowledge/architecture-decisions/):
    ///   S(t) = { closed=1.0, half_open=0.5, open=0.0 }
    pub fn scoring_weight(&self) -> f64 {
        match self {
            CircuitState::Closed   => 1.0,
            CircuitState::HalfOpen => 0.5,
            CircuitState::Open     => 0.0,
        }
    }
}

/// Statistics for a single provider's circuit breaker
#[derive(Debug)]
struct ProviderCircuitState {
    state: RwLock<CircuitState>,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    last_failure_time: RwLock<Option<Instant>>,
    last_state_change: RwLock<Instant>,
    total_failures: AtomicU64,
    total_successes: AtomicU64,
    total_rejections: AtomicU64,
    /// Guards the half-open probe: only one probe at a time via compare-and-swap.
    half_open_probe_in_flight: AtomicBool,
}

impl ProviderCircuitState {
    fn new() -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            last_failure_time: RwLock::new(None),
            last_state_change: RwLock::new(Instant::now()),
            total_failures: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            total_rejections: AtomicU64::new(0),
            half_open_probe_in_flight: AtomicBool::new(false),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u64,
    /// Number of successes in half-open state before closing
    pub success_threshold: u64,
    /// Duration to wait before transitioning from open to half-open
    pub open_timeout: Duration,
    /// Maximum number of requests allowed in half-open state
    pub half_open_max_requests: u64,
    /// Time window for counting failures (rolling window)
    pub failure_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            open_timeout: Duration::from_secs(30),
            half_open_max_requests: 3,
            failure_window: Duration::from_secs(60),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new configuration
    pub fn new(failure_threshold: u64, open_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            open_timeout,
            ..Default::default()
        }
    }

    /// Set success threshold
    pub fn with_success_threshold(mut self, threshold: u64) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Set half-open max requests
    pub fn with_half_open_max_requests(mut self, max: u64) -> Self {
        self.half_open_max_requests = max;
        self
    }

    /// Set failure window duration
    pub fn with_failure_window(mut self, duration: Duration) -> Self {
        self.failure_window = duration;
        self
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub provider: String,
    pub state: CircuitState,
    pub failure_count: u64,
    pub success_count: u64,
    pub total_failures: u64,
    pub total_successes: u64,
    pub total_rejections: u64,
    pub last_failure_time: Option<Instant>,
    pub time_in_current_state: Duration,
}

/// Result of checking if a request is allowed
#[derive(Debug, Clone)]
pub struct CircuitBreakerDecision {
    pub allowed: bool,
    pub state: CircuitState,
    pub reason: Option<String>,
}

/// Multi-provider circuit breaker
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    providers: DashMap<String, ProviderCircuitState>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new(failure_threshold: usize, retry_timeout: Duration) -> Self {
        Self {
            config: CircuitBreakerConfig::new(failure_threshold as u64, retry_timeout),
            providers: DashMap::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            providers: DashMap::new(),
        }
    }

    /// Get or create provider state
    fn get_or_create_provider(&self, provider: &str) -> dashmap::mapref::one::Ref<'_, String, ProviderCircuitState> {
        if !self.providers.contains_key(provider) {
            self.providers.insert(provider.to_string(), ProviderCircuitState::new());
        }
        self.providers.get(provider).unwrap()
    }

    /// Check if a request to the provider is allowed
    pub async fn is_allowed(&self, provider: &str) -> CircuitBreakerDecision {
        let state = self.get_or_create_provider(provider);
        let current_state = *state.state.read().await;

        match current_state {
            CircuitState::Closed => {
                CircuitBreakerDecision {
                    allowed: true,
                    state: current_state,
                    reason: None,
                }
            }
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_change = *state.last_state_change.read().await;
                if last_change.elapsed() >= self.config.open_timeout {
                    // Transition to half-open; this request becomes the probe
                    drop(state);
                    self.transition_to_half_open(provider).await;
                    // Mark probe as in-flight so concurrent requests are rejected
                    if let Some(s) = self.providers.get(provider) {
                        s.half_open_probe_in_flight.store(true, Ordering::Release);
                    }

                    CircuitBreakerDecision {
                        allowed: true,
                        state: CircuitState::HalfOpen,
                        reason: Some("Half-open probe acquired".to_string()),
                    }
                } else {
                    state.total_rejections.fetch_add(1, Ordering::Relaxed);
                    let remaining = self.config.open_timeout - last_change.elapsed();
                    
                    CircuitBreakerDecision {
                        allowed: false,
                        state: current_state,
                        reason: Some(format!(
                            "Circuit is open. Retry in {:.1}s",
                            remaining.as_secs_f64()
                        )),
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Only one probe at a time — compare-and-swap from false → true
                let probe_acquired = state
                    .half_open_probe_in_flight
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok();

                if probe_acquired {
                    CircuitBreakerDecision {
                        allowed: true,
                        state: current_state,
                        reason: Some("Half-open probe acquired".to_string()),
                    }
                } else {
                    state.total_rejections.fetch_add(1, Ordering::Relaxed);
                    CircuitBreakerDecision {
                        allowed: false,
                        state: current_state,
                        reason: Some("Half-open probe already in flight — request rejected".to_string()),
                    }
                }
            }
        }
    }

    /// Legacy method for backward compatibility
    pub async fn is_open(&self, failure_count: usize) -> bool {
        failure_count >= self.config.failure_threshold as usize
    }

    /// Record a successful request
    pub async fn record_success(&self, provider: &str) {
        let state = self.get_or_create_provider(provider);
        let current_state = *state.state.read().await;

        state.total_successes.fetch_add(1, Ordering::Relaxed);

        match current_state {
            CircuitState::HalfOpen => {
                // Release the probe flag (whether we close or not)
                state.half_open_probe_in_flight.store(false, Ordering::Release);
                let success_count = state.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if success_count >= self.config.success_threshold {
                    drop(state);
                    self.transition_to_closed(provider).await;
                    info!("Circuit breaker for {} transitioning to Closed after {} successes",
                          provider, success_count);
                } else {
                    debug!("Circuit breaker for {} half-open: {}/{} successes",
                           provider, success_count, self.config.success_threshold);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                state.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                warn!("Success recorded while circuit is open for {}", provider);
            }
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self, provider: &str) {
        let state = self.get_or_create_provider(provider);
        let current_state = *state.state.read().await;

        state.total_failures.fetch_add(1, Ordering::Relaxed);
        *state.last_failure_time.write().await = Some(Instant::now());

        match current_state {
            CircuitState::Closed => {
                let failure_count = state.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if failure_count >= self.config.failure_threshold {
                    drop(state);
                    self.transition_to_open(provider).await;
                    warn!("Circuit breaker for {} opening after {} failures", 
                          provider, failure_count);
                } else {
                    debug!("Circuit breaker for {} recorded failure {}/{}", 
                           provider, failure_count, self.config.failure_threshold);
                }
            }
            CircuitState::HalfOpen => {
                // Release probe flag before reopening
                state.half_open_probe_in_flight.store(false, Ordering::Release);
                drop(state);
                self.transition_to_open(provider).await;
                warn!("Circuit breaker for {} reopening after failure in half-open state", provider);
            }
            CircuitState::Open => {
                // Already open, just update the failure time
                debug!("Additional failure recorded while circuit is open for {}", provider);
            }
        }
    }

    /// Transition to open state
    async fn transition_to_open(&self, provider: &str) {
        if let Some(state) = self.providers.get(provider) {
            let mut current_state = state.state.write().await;
            *current_state = CircuitState::Open;
            state.failure_count.store(0, Ordering::Relaxed);
            state.success_count.store(0, Ordering::Relaxed);
            state.half_open_probe_in_flight.store(false, Ordering::Release);
            *state.last_state_change.write().await = Instant::now();

            info!("Circuit breaker for {} transitioned to Open", provider);
        }
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self, provider: &str) {
        if let Some(state) = self.providers.get(provider) {
            let mut current_state = state.state.write().await;
            if *current_state == CircuitState::Open {
                *current_state = CircuitState::HalfOpen;
                state.success_count.store(0, Ordering::Relaxed);
                state.half_open_probe_in_flight.store(false, Ordering::Release);
                *state.last_state_change.write().await = Instant::now();

                info!("Circuit breaker for {} transitioned to HalfOpen", provider);
            }
        }
    }

    /// Transition to closed state
    async fn transition_to_closed(&self, provider: &str) {
        if let Some(state) = self.providers.get(provider) {
            let mut current_state = state.state.write().await;
            *current_state = CircuitState::Closed;
            state.failure_count.store(0, Ordering::Relaxed);
            state.success_count.store(0, Ordering::Relaxed);
            *state.last_state_change.write().await = Instant::now();
            
            info!("Circuit breaker for {} transitioned to Closed", provider);
        }
    }

    /// Force open the circuit for a provider
    pub async fn force_open(&self, provider: &str) {
        self.transition_to_open(provider).await;
    }

    /// Force close the circuit for a provider
    pub async fn force_close(&self, provider: &str) {
        self.transition_to_closed(provider).await;
    }

    /// Reset circuit breaker for a provider
    pub async fn reset(&self, provider: &str) {
        self.providers.remove(provider);
        info!("Circuit breaker for {} reset", provider);
    }

    /// Reset all circuit breakers
    pub async fn reset_all(&self) {
        self.providers.clear();
        info!("All circuit breakers reset");
    }

    /// Get current state for a provider
    pub async fn get_state(&self, provider: &str) -> CircuitState {
        if let Some(state) = self.providers.get(provider) {
            *state.state.read().await
        } else {
            CircuitState::Closed
        }
    }

    /// Get statistics for a provider
    pub async fn get_stats(&self, provider: &str) -> CircuitBreakerStats {
        if let Some(state) = self.providers.get(provider) {
            let current_state = *state.state.read().await;
            let last_change = *state.last_state_change.read().await;
            
            CircuitBreakerStats {
                provider: provider.to_string(),
                state: current_state,
                failure_count: state.failure_count.load(Ordering::Relaxed),
                success_count: state.success_count.load(Ordering::Relaxed),
                total_failures: state.total_failures.load(Ordering::Relaxed),
                total_successes: state.total_successes.load(Ordering::Relaxed),
                total_rejections: state.total_rejections.load(Ordering::Relaxed),
                last_failure_time: *state.last_failure_time.read().await,
                time_in_current_state: last_change.elapsed(),
            }
        } else {
            CircuitBreakerStats {
                provider: provider.to_string(),
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                total_failures: 0,
                total_successes: 0,
                total_rejections: 0,
                last_failure_time: None,
                time_in_current_state: Duration::ZERO,
            }
        }
    }

    /// Get statistics for all providers
    pub async fn get_all_stats(&self) -> Vec<CircuitBreakerStats> {
        let mut stats = Vec::new();
        for entry in self.providers.iter() {
            let provider = entry.key().clone();
            stats.push(self.get_stats(&provider).await);
        }
        stats
    }

    /// Get configuration
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Get number of tracked providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            open_timeout: Duration::from_millis(100),
            half_open_max_requests: 2,
            failure_window: Duration::from_secs(60),
        });

        let decision = cb.is_allowed("provider1").await;
        assert!(decision.allowed);
        assert_eq!(decision.state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            open_timeout: Duration::from_millis(100),
            half_open_max_requests: 2,
            failure_window: Duration::from_secs(60),
        });

        // Record failures
        cb.record_failure("provider1").await;
        cb.record_failure("provider1").await;
        
        // Should still be closed
        let decision = cb.is_allowed("provider1").await;
        assert!(decision.allowed);
        
        // Third failure opens the circuit
        cb.record_failure("provider1").await;
        
        let decision = cb.is_allowed("provider1").await;
        assert!(!decision.allowed);
        assert_eq!(decision.state, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_transition() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            open_timeout: Duration::from_millis(50),
            half_open_max_requests: 2,
            failure_window: Duration::from_secs(60),
        });

        // Open the circuit
        cb.record_failure("provider1").await;
        cb.record_failure("provider1").await;

        assert_eq!(cb.get_state("provider1").await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should transition to half-open
        let decision = cb.is_allowed("provider1").await;
        assert!(decision.allowed);
        assert_eq!(decision.state, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_after_successes() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            open_timeout: Duration::from_millis(50),
            half_open_max_requests: 5,
            failure_window: Duration::from_secs(60),
        });

        // Open the circuit
        cb.record_failure("provider1").await;
        cb.record_failure("provider1").await;

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Trigger half-open
        cb.is_allowed("provider1").await;
        assert_eq!(cb.get_state("provider1").await, CircuitState::HalfOpen);

        // Record successes
        cb.record_success("provider1").await;
        cb.record_success("provider1").await;

        // Should be closed now
        assert_eq!(cb.get_state("provider1").await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reopens_on_half_open_failure() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            open_timeout: Duration::from_millis(50),
            half_open_max_requests: 5,
            failure_window: Duration::from_secs(60),
        });

        // Open the circuit
        cb.record_failure("provider1").await;
        cb.record_failure("provider1").await;

        // Wait and transition to half-open
        tokio::time::sleep(Duration::from_millis(60)).await;
        cb.is_allowed("provider1").await;

        // Failure in half-open state
        cb.record_failure("provider1").await;

        // Should be back to open
        assert_eq!(cb.get_state("provider1").await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_stats() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig::default());

        cb.record_success("provider1").await;
        cb.record_success("provider1").await;
        cb.record_failure("provider1").await;

        let stats = cb.get_stats("provider1").await;
        assert_eq!(stats.total_successes, 2);
        assert_eq!(stats.total_failures, 1);
    }

    #[tokio::test]
    async fn test_multiple_providers() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        });

        // Provider 1 fails
        cb.record_failure("provider1").await;
        cb.record_failure("provider1").await;

        // Provider 2 is fine
        cb.record_success("provider2").await;

        assert_eq!(cb.get_state("provider1").await, CircuitState::Open);
        assert_eq!(cb.get_state("provider2").await, CircuitState::Closed);
    }

    fn make_cb(threshold: u64, timeout_secs: u64) -> CircuitBreaker {
        CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: threshold,
            success_threshold: 1,
            open_timeout: Duration::from_secs(timeout_secs),
            half_open_max_requests: 3,
            failure_window: Duration::from_secs(60),
        })
    }

    #[tokio::test]
    async fn test_closed_to_open_on_threshold_failures() {
        let cb = make_cb(3, 30);
        assert!(cb.is_allowed("p1").await.allowed);
        cb.record_failure("p1").await;
        cb.record_failure("p1").await;
        cb.record_failure("p1").await; // hits threshold
        let decision = cb.is_allowed("p1").await;
        assert!(!decision.allowed, "Circuit should be open after 3 failures");
        assert_eq!(decision.state, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_half_open_probe_success_closes_circuit() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            open_timeout: Duration::from_millis(1), // immediate for test
            half_open_max_requests: 3,
            failure_window: Duration::from_secs(60),
        });
        cb.record_failure("p2").await; // → Open
        tokio::time::sleep(Duration::from_millis(5)).await; // wait for timeout
        let d = cb.is_allowed("p2").await;
        assert!(d.allowed, "Should be allowed in half-open");
        cb.record_success("p2").await; // → Closed
        let d2 = cb.is_allowed("p2").await;
        assert!(d2.allowed);
        assert_eq!(d2.state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_probe_failure_reopens_circuit() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            open_timeout: Duration::from_millis(1),
            half_open_max_requests: 3,
            failure_window: Duration::from_secs(60),
        });
        cb.record_failure("p3").await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        let d = cb.is_allowed("p3").await;
        assert!(d.allowed);
        cb.record_failure("p3").await; // → back to Open
        let d2 = cb.is_allowed("p3").await;
        assert!(!d2.allowed);
        assert_eq!(d2.state, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_half_open_concurrent_probe_guard() {
        let cb = CircuitBreaker::with_config(CircuitBreakerConfig {
            failure_threshold: 1,
            success_threshold: 1,
            open_timeout: Duration::from_millis(1),
            half_open_max_requests: 3,
            failure_window: Duration::from_secs(60),
        });
        cb.record_failure("p4").await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        // First request triggers the Open→HalfOpen transition and acquires probe
        let d1 = cb.is_allowed("p4").await;
        assert!(d1.allowed, "First request should acquire the probe");
        // Second concurrent request should be rejected while probe is in flight
        let d2 = cb.is_allowed("p4").await;
        assert!(!d2.allowed, "Second concurrent request should be rejected (probe in flight)");
    }

    #[tokio::test]
    async fn test_scoring_weight_values() {
        assert_eq!(CircuitState::Closed.scoring_weight(),   1.0);
        assert_eq!(CircuitState::HalfOpen.scoring_weight(), 0.5);
        assert_eq!(CircuitState::Open.scoring_weight(),     0.0);
    }
}
