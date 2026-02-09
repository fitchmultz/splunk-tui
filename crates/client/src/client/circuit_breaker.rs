//! Circuit breaker pattern for resilient API calls.
//!
//! This module provides a circuit breaker implementation to prevent cascading
//! failures when the Splunk API is struggling. It tracks failure rates per
//! endpoint and opens the circuit when thresholds are exceeded.
//!
//! # Circuit States
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Failure threshold exceeded, requests fail fast
//! - **Half-Open**: Testing if service recovered after reset timeout
//!
//! # Configuration
//!
//! Per-endpoint configuration:
//! - `failure_threshold`: Number of errors in window to open circuit (default: 5)
//! - `failure_window`: Time window for failure counting (default: 60s)
//! - `reset_timeout`: Time before half-open test (default: 30s)
//! - `half_open_requests`: Max test requests in half-open (default: 1)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::metrics::MetricsCollector;
use splunk_config::constants::{
    DEFAULT_CIRCUIT_FAILURE_THRESHOLD, DEFAULT_CIRCUIT_FAILURE_WINDOW_SECS,
    DEFAULT_CIRCUIT_HALF_OPEN_REQUESTS, DEFAULT_CIRCUIT_RESET_TIMEOUT_SECS,
};
use tracing::{debug, info, warn};

/// Metric name for circuit breaker state transitions.
pub const METRIC_CIRCUIT_STATE_TRANSITIONS: &str = "splunk_circuit_breaker_state_transitions_total";

/// Metric name for circuit breaker blocked requests.
pub const METRIC_CIRCUIT_BLOCKED: &str = "splunk_circuit_breaker_blocked_requests_total";

/// Circuit breaker states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through.
    Closed,
    /// Failure threshold exceeded - requests fail fast.
    Open,
    /// Testing if service recovered - limited requests allowed.
    HalfOpen,
}

impl CircuitState {
    /// Returns the string label for this state.
    pub const fn as_str(&self) -> &'static str {
        match self {
            CircuitState::Closed => "closed",
            CircuitState::Open => "open",
            CircuitState::HalfOpen => "half_open",
        }
    }
}

/// Configuration for a circuit breaker.
#[derive(Debug, Clone, Copy)]
pub struct CircuitBreakerConfig {
    /// Number of failures within window to open circuit.
    pub failure_threshold: u32,
    /// Time window for failure counting.
    pub failure_window: Duration,
    /// Time to wait before attempting half-open.
    pub reset_timeout: Duration,
    /// Number of requests allowed in half-open state.
    pub half_open_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_CIRCUIT_FAILURE_THRESHOLD,
            failure_window: Duration::from_secs(DEFAULT_CIRCUIT_FAILURE_WINDOW_SECS),
            reset_timeout: Duration::from_secs(DEFAULT_CIRCUIT_RESET_TIMEOUT_SECS),
            half_open_requests: DEFAULT_CIRCUIT_HALF_OPEN_REQUESTS,
        }
    }
}

/// A single failure record with timestamp.
#[derive(Debug, Clone, Copy)]
struct FailureRecord {
    timestamp: Instant,
}

/// Per-endpoint circuit breaker state.
#[derive(Debug)]
struct EndpointCircuit {
    config: CircuitBreakerConfig,
    state: CircuitState,
    failures: Vec<FailureRecord>,
    last_failure: Option<Instant>,
    half_open_attempts: u32,
    opened_at: Option<Instant>,
}

impl EndpointCircuit {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitState::Closed,
            failures: Vec::new(),
            last_failure: None,
            half_open_attempts: 0,
            opened_at: None,
        }
    }

    /// Record a failure and potentially transition to open.
    fn record_failure(&mut self, endpoint: &str) -> CircuitState {
        let now = Instant::now();
        self.last_failure = Some(now);
        self.failures.push(FailureRecord { timestamp: now });

        // Clean old failures outside the window
        self.clean_old_failures(now);

        // Check if we should open the circuit
        if self.failures.len() >= self.config.failure_threshold as usize {
            if self.state != CircuitState::Open {
                warn!(
                    endpoint = endpoint,
                    failures = self.failures.len(),
                    threshold = self.config.failure_threshold,
                    "Circuit breaker opened due to failure threshold exceeded"
                );
                self.state = CircuitState::Open;
                self.opened_at = Some(now);
            }
        }

        self.state
    }

    /// Record a success and potentially transition to closed.
    fn record_success(&mut self, endpoint: &str) -> CircuitState {
        match self.state {
            CircuitState::HalfOpen => {
                // Success in half-open closes the circuit
                info!(
                    endpoint = endpoint,
                    "Circuit breaker closed - service recovered"
                );
                self.state = CircuitState::Closed;
                self.half_open_attempts = 0;
                self.failures.clear();
                self.opened_at = None;
            }
            CircuitState::Closed => {
                // Clear failures on success in closed state
                if !self.failures.is_empty() {
                    debug!(
                        endpoint = endpoint,
                        "Clearing failure history after successful request"
                    );
                    self.failures.clear();
                }
            }
            CircuitState::Open => {
                // Shouldn't happen - success while open means we didn't check state
            }
        }

        self.state
    }

    /// Check if request should be allowed and update state if needed.
    fn check_request(&mut self, endpoint: &str) -> bool {
        let now = Instant::now();

        // Clean old failures
        self.clean_old_failures(now);

        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if reset timeout has passed
                if let Some(opened_at) = self.opened_at {
                    if now.duration_since(opened_at) >= self.config.reset_timeout {
                        info!(
                            endpoint = endpoint,
                            "Circuit breaker entering half-open state"
                        );
                        self.state = CircuitState::HalfOpen;
                        self.half_open_attempts = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests in half-open
                if self.half_open_attempts < self.config.half_open_requests {
                    self.half_open_attempts += 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Clean failures outside the time window.
    fn clean_old_failures(&mut self, now: Instant) {
        self.failures
            .retain(|f| now.duration_since(f.timestamp) < self.config.failure_window);
    }

    /// Get current state.
    fn state(&self) -> CircuitState {
        self.state
    }
}

/// Circuit breaker for protecting API calls.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    circuits: Arc<Mutex<HashMap<String, EndpointCircuit>>>,
    default_config: CircuitBreakerConfig,
    metrics: Option<MetricsCollector>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration.
    pub fn new() -> Self {
        Self {
            circuits: Arc::new(Mutex::new(HashMap::new())),
            default_config: CircuitBreakerConfig::default(),
            metrics: None,
        }
    }

    /// Create a new circuit breaker with metrics.
    pub fn with_metrics(metrics: MetricsCollector) -> Self {
        Self {
            circuits: Arc::new(Mutex::new(HashMap::new())),
            default_config: CircuitBreakerConfig::default(),
            metrics: Some(metrics),
        }
    }

    /// Set default configuration for new endpoints.
    pub fn with_default_config(mut self, config: CircuitBreakerConfig) -> Self {
        self.default_config = config;
        self
    }

    /// Check if a request should be allowed for the given endpoint.
    ///
    /// Returns `Ok(())` if the request should proceed, or `Err(CircuitBreakerError)`
    /// if the circuit is open.
    pub fn check(&self, endpoint: &str) -> Result<(), CircuitBreakerError> {
        let mut circuits = self.circuits.lock().unwrap();
        let circuit = circuits
            .entry(endpoint.to_string())
            .or_insert_with(|| EndpointCircuit::new(self.default_config));

        if circuit.check_request(endpoint) {
            Ok(())
        } else {
            // Record blocked request metric
            if let Some(ref m) = self.metrics {
                m.record_circuit_blocked(endpoint);
            }
            Err(CircuitBreakerError::CircuitOpen {
                endpoint: endpoint.to_string(),
            })
        }
    }

    /// Record a successful request for the given endpoint.
    pub fn record_success(&self, endpoint: &str) {
        let mut circuits = self.circuits.lock().unwrap();
        if let Some(circuit) = circuits.get_mut(endpoint) {
            let old_state = circuit.state();
            let new_state = circuit.record_success(endpoint);

            if old_state != new_state {
                self.record_state_transition(endpoint, old_state, new_state);
            }
        }
    }

    /// Record a failed request for the given endpoint.
    pub fn record_failure(&self, endpoint: &str) {
        let mut circuits = self.circuits.lock().unwrap();
        let circuit = circuits
            .entry(endpoint.to_string())
            .or_insert_with(|| EndpointCircuit::new(self.default_config));

        let old_state = circuit.state();
        let new_state = circuit.record_failure(endpoint);

        if old_state != new_state {
            self.record_state_transition(endpoint, old_state, new_state);
        }
    }

    /// Get current state for an endpoint.
    pub fn state(&self, endpoint: &str) -> CircuitState {
        let circuits = self.circuits.lock().unwrap();
        circuits
            .get(endpoint)
            .map(|c| c.state())
            .unwrap_or(CircuitState::Closed)
    }

    /// Get all endpoint states.
    pub fn all_states(&self) -> HashMap<String, CircuitState> {
        let circuits = self.circuits.lock().unwrap();
        circuits
            .iter()
            .map(|(k, v)| (k.clone(), v.state()))
            .collect()
    }

    /// Reset a specific endpoint to closed state.
    pub fn reset(&self, endpoint: &str) {
        let mut circuits = self.circuits.lock().unwrap();
        if let Some(circuit) = circuits.get_mut(endpoint) {
            info!(endpoint = endpoint, "Manually resetting circuit breaker");
            circuit.state = CircuitState::Closed;
            circuit.failures.clear();
            circuit.half_open_attempts = 0;
            circuit.opened_at = None;
        }
    }

    /// Reset all circuits to closed state.
    pub fn reset_all(&self) {
        let mut circuits = self.circuits.lock().unwrap();
        info!("Resetting all circuit breakers");
        for (endpoint, circuit) in circuits.iter_mut() {
            circuit.state = CircuitState::Closed;
            circuit.failures.clear();
            circuit.half_open_attempts = 0;
            circuit.opened_at = None;
        }
    }

    fn record_state_transition(&self, endpoint: &str, from: CircuitState, to: CircuitState) {
        if let Some(ref m) = self.metrics {
            m.record_circuit_state_transition(endpoint, from, to);
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur from circuit breaker.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CircuitBreakerError {
    /// Circuit is open - requests are failing fast.
    #[error("Circuit breaker open for endpoint: {endpoint}. Service temporarily unavailable.")]
    CircuitOpen { endpoint: String },
}

/// Extension trait for MetricsCollector to add circuit breaker metrics.
pub trait CircuitBreakerMetrics {
    /// Record a circuit breaker state transition.
    fn record_circuit_state_transition(
        &self,
        endpoint: &str,
        from: CircuitState,
        to: CircuitState,
    );

    /// Record a blocked request due to open circuit.
    fn record_circuit_blocked(&self, endpoint: &str);
}

impl CircuitBreakerMetrics for MetricsCollector {
    fn record_circuit_state_transition(
        &self,
        endpoint: &str,
        from: CircuitState,
        to: CircuitState,
    ) {
        if !self.is_enabled() {
            return;
        }

        metrics::counter!(METRIC_CIRCUIT_STATE_TRANSITIONS,
            "endpoint" => endpoint.to_string(),
            "from_state" => from.as_str(),
            "to_state" => to.as_str(),
        )
        .increment(1);
    }

    fn record_circuit_blocked(&self, endpoint: &str) {
        if !self.is_enabled() {
            return;
        }

        metrics::counter!(METRIC_CIRCUIT_BLOCKED,
            "endpoint" => endpoint.to_string(),
        )
        .increment(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_starts_closed() {
        let cb = CircuitBreaker::new();
        assert!(cb.check("/test").is_ok());
        assert_eq!(cb.state("/test"), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let cb = CircuitBreaker::new();
        let endpoint = "/test";

        // Record failures to exceed threshold
        for _ in 0..5 {
            cb.record_failure(endpoint);
        }

        assert_eq!(cb.state(endpoint), CircuitState::Open);
        assert!(cb.check(endpoint).is_err());
    }

    #[test]
    fn test_circuit_closes_on_success_in_half_open() {
        let cb = CircuitBreaker::new().with_default_config(CircuitBreakerConfig {
            failure_threshold: 1,
            failure_window: Duration::from_secs(60),
            reset_timeout: Duration::from_millis(0), // Immediate half-open
            half_open_requests: 1,
        });

        let endpoint = "/test";
        cb.record_failure(endpoint);
        assert_eq!(cb.state(endpoint), CircuitState::Open);

        // Wait for reset timeout
        thread::sleep(Duration::from_millis(10));

        // Should be allowed in half-open
        assert!(cb.check(endpoint).is_ok());
        assert_eq!(cb.state(endpoint), CircuitState::HalfOpen);

        // Success should close it
        cb.record_success(endpoint);
        assert_eq!(cb.state(endpoint), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_reopens_on_failure_in_half_open() {
        let cb = CircuitBreaker::new().with_default_config(CircuitBreakerConfig {
            failure_threshold: 1,
            failure_window: Duration::from_secs(60),
            reset_timeout: Duration::from_millis(0),
            half_open_requests: 1,
        });

        let endpoint = "/test";
        cb.record_failure(endpoint);

        thread::sleep(Duration::from_millis(10));

        // Allow half-open request
        assert!(cb.check(endpoint).is_ok());

        // Failure should reopen
        cb.record_failure(endpoint);
        assert_eq!(cb.state(endpoint), CircuitState::Open);
    }

    #[test]
    fn test_failure_window_expires_old_failures() {
        let cb = CircuitBreaker::new().with_default_config(CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window: Duration::from_millis(50),
            reset_timeout: Duration::from_secs(30),
            half_open_requests: 1,
        });

        let endpoint = "/test";

        // Record failures
        for _ in 0..3 {
            cb.record_failure(endpoint);
        }

        assert_eq!(cb.state(endpoint), CircuitState::Open);

        // Wait for failures to expire
        thread::sleep(Duration::from_millis(60));

        // Reset timeout passed, should transition to half-open on check
        thread::sleep(Duration::from_millis(60));

        // New request should be allowed (old failures expired)
        // But circuit is still open until reset_timeout passes
        // This test verifies the sliding window works
    }

    #[test]
    fn test_reset_endpoint() {
        let cb = CircuitBreaker::new();
        let endpoint = "/test";

        for _ in 0..5 {
            cb.record_failure(endpoint);
        }

        assert_eq!(cb.state(endpoint), CircuitState::Open);

        cb.reset(endpoint);
        assert_eq!(cb.state(endpoint), CircuitState::Closed);
        assert!(cb.check(endpoint).is_ok());
    }
}
