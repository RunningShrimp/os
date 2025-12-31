//! Circuit breaker pattern implementation for the NOS kernel.
//!
//! This module provides a circuit breaker pattern to prevent cascading failures:
//! - States: Closed, Open, Half-Open
//! - Failure threshold configuration
//! - Recovery timeout
//! - Success threshold for state transitions
//! - Automatic recovery mechanisms
//! - Manual reset capabilities

#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::string::String;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use core::time::Duration;
use spin::Mutex;

use crate::subsystems::time::Timestamp;
use super::check::HealthStatus;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CircuitState {
    /// Circuit is closed - requests pass through normally
    Closed = 0,
    /// Circuit is open - requests are blocked
    Open = 1,
    /// Circuit is half-open - limited requests allowed to test recovery
    HalfOpen = 2,
}

impl CircuitState {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(CircuitState::Closed),
            1 => Some(CircuitState::Open),
            2 => Some(CircuitState::HalfOpen),
            _ => None,
        }
    }

    /// Check if circuit allows requests
    pub fn allows_requests(self) -> bool {
        matches!(self, CircuitState::Closed | CircuitState::HalfOpen)
    }

    /// Check if circuit is open (blocking requests)
    pub fn is_open(self) -> bool {
        self == CircuitState::Open
    }
}

/// Configuration for circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitConfig {
    /// Number of consecutive failures to open circuit
    pub failure_threshold: usize,
    /// Number of consecutive successes to close circuit from half-open
    pub success_threshold: usize,
    /// Timeout before attempting recovery (open -> half-open)
    pub recovery_timeout: Duration,
    /// Maximum number of requests to allow in half-open state
    pub half_open_max_calls: usize,
    /// Timeout for individual requests
    pub call_timeout: Duration,
    /// Window size for rolling failure count
    pub rolling_window_size: usize,
    /// Minimum number of calls before calculating error rate
    pub minimum_calls: usize,
    /// Error rate threshold (0.0 to 1.0) to open circuit
    pub error_rate_threshold: f64,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        CircuitConfig {
            failure_threshold: 5,
            success_threshold: 2,
            recovery_timeout: Duration::from_secs(60),
            half_open_max_calls: 3,
            call_timeout: Duration::from_secs(30),
            rolling_window_size: 100,
            minimum_calls: 10,
            error_rate_threshold: 0.5,
        }
    }
}

impl CircuitConfig {
    /// Create new circuit config with failure threshold
    pub fn new(failure_threshold: usize) -> Self {
        CircuitConfig {
            failure_threshold,
            ..Default::default()
        }
    }

    /// Set success threshold
    pub fn with_success_threshold(mut self, threshold: usize) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Set recovery timeout
    pub fn with_recovery_timeout(mut self, timeout: Duration) -> Self {
        self.recovery_timeout = timeout;
        self
    }

    /// Set half-open max calls
    pub fn with_half_open_max_calls(mut self, max: usize) -> Self {
        self.half_open_max_calls = max;
        self
    }

    /// Set call timeout
    pub fn with_call_timeout(mut self, timeout: Duration) -> Self {
        self.call_timeout = timeout;
        self
    }

    /// Set rolling window size
    pub fn with_rolling_window_size(mut self, size: usize) -> Self {
        self.rolling_window_size = size;
        self
    }

    /// Set minimum calls
    pub fn with_minimum_calls(mut self, min: usize) -> Self {
        self.minimum_calls = min;
        self
    }

    /// Set error rate threshold
    pub fn with_error_rate_threshold(mut self, threshold: f64) -> Self {
        self.error_rate_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}

/// Result of a circuit-protected call
#[derive(Debug, Clone)]
pub enum CallResult<T> {
    /// Call succeeded
    Success(T),
    /// Call failed
    Error(String),
    /// Call rejected by circuit breaker
    Rejected,
}

impl<T> CallResult<T> {
    /// Check if call succeeded
    pub fn is_success(&self) -> bool {
        matches!(self, CallResult::Success(_))
    }

    /// Check if call failed
    pub fn is_error(&self) -> bool {
        matches!(self, CallResult::Error(_))
    }

    /// Check if call was rejected
    pub fn is_rejected(&self) -> bool {
        matches!(self, CallResult::Rejected)
    }
}

/// Record of a call attempt
#[derive(Debug, Clone)]
struct CallRecord {
    /// Timestamp of the call
    timestamp: Timestamp,
    /// Whether the call succeeded
    success: bool,
    /// Duration of the call
    duration: Duration,
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Circuit configuration
    config: CircuitConfig,
    /// Current circuit state
    state: AtomicU8,
    /// Timestamp when state changed
    state_changed: Mutex<Option<Timestamp>>,
    /// Consecutive failures in current state
    consecutive_failures: AtomicUsize,
    /// Consecutive successes in current state
    consecutive_successes: AtomicUsize,
    /// Call history for rolling window
    call_history: Mutex<VecDeque<CallRecord>>,
    /// Number of calls made in half-open state
    half_open_calls: AtomicUsize,
    /// Total number of rejected calls
    rejected_calls: AtomicUsize,
    /// Total number of successful calls
    total_successes: AtomicUsize,
    /// Total number of failed calls
    total_failures: AtomicUsize,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(config: CircuitConfig) -> Self {
        CircuitBreaker {
            config,
            state: AtomicU8::new(CircuitState::Closed as u8),
            state_changed: Mutex::new(Some(Timestamp::now())),
            consecutive_failures: AtomicUsize::new(0),
            consecutive_successes: AtomicUsize::new(0),
            call_history: Mutex::new(VecDeque::with_capacity(100)),
            half_open_calls: AtomicUsize::new(0),
            rejected_calls: AtomicUsize::new(0),
            total_successes: AtomicUsize::new(0),
            total_failures: AtomicUsize::new(0),
        }
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        CircuitState::from_u8(self.state.load(Ordering::Relaxed))
            .unwrap_or(CircuitState::Closed)
    }

    /// Get circuit configuration
    pub fn config(&self) -> &CircuitConfig {
        &self.config
    }

    /// Check if circuit allows requests
    pub fn allows_requests(&self) -> bool {
        self.state().allows_requests()
    }

    /// Check if we should attempt recovery
    fn should_attempt_recovery(&self) -> bool {
        if let Some(timestamp) = *self.state_changed.lock() {
            let elapsed = Timestamp::now().duration_since(timestamp);
            elapsed >= self.config.recovery_timeout
        } else {
            false
        }
    }

    /// Transition to new state
    fn transition_to(&self, new_state: CircuitState) {
        let current = self.state();
        if current != new_state {
            self.state.store(new_state as u8, Ordering::Relaxed);
            *self.state_changed.lock() = Some(Timestamp::now());
            self.consecutive_failures.store(0, Ordering::Relaxed);
            self.consecutive_successes.store(0, Ordering::Relaxed);

            if new_state == CircuitState::HalfOpen {
                self.half_open_calls.store(0, Ordering::Relaxed);
            }
        }
    }

    /// Record a call result
    fn record_call(&self, success: bool, duration: Duration) {
        let mut history = self.call_history.lock();

        // Add new record
        history.push_back(CallRecord {
            timestamp: Timestamp::now(),
            success,
            duration,
        });

        // Maintain rolling window size
        while history.len() > self.config.rolling_window_size {
            history.pop_front();
        }

        // Update counters
        if success {
            self.total_successes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.total_failures.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Calculate error rate in rolling window
    fn calculate_error_rate(&self) -> f64 {
        let history = self.call_history.lock();

        if history.len() < self.config.minimum_calls {
            return 0.0;
        }

        let failures = history.iter().filter(|r| !r.success).count();
        (failures as f64) / (history.len() as f64)
    }

    /// Execute a circuit-protected call
    pub fn call<T, F>(&self, f: F) -> CallResult<T>
    where
        F: FnOnce() -> Result<T, String>,
    {
        let state = self.state();

        // Check if we should transition from open to half-open
        if state == CircuitState::Open && self.should_attempt_recovery() {
            self.transition_to(CircuitState::HalfOpen);
        }

        let state = self.state();

        // Reject if circuit is open
        if state == CircuitState::Open {
            self.rejected_calls.fetch_add(1, Ordering::Relaxed);
            return CallResult::Rejected;
        }

        // Check half-open call limit
        if state == CircuitState::HalfOpen {
            let calls = self.half_open_calls.fetch_add(1, Ordering::Relaxed);
            if calls >= self.config.half_open_max_calls {
                self.half_open_calls.fetch_sub(1, Ordering::Relaxed);
                self.rejected_calls.fetch_add(1, Ordering::Relaxed);
                return CallResult::Rejected;
            }
        }

        // Execute the call
        let start = Timestamp::now();
        let result = f();
        let duration = Timestamp::now().duration_since(start);

        match result {
            Ok(value) => {
                self.record_call(true, duration);
                self.handle_success();
                CallResult::Success(value)
            }
            Err(error) => {
                self.record_call(false, duration);
                self.handle_failure();
                CallResult::Error(error)
            }
        }
    }

    /// Handle successful call
    fn handle_success(&self) {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                let _successes = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
                self.consecutive_failures.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let successes = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
                self.consecutive_failures.store(0, Ordering::Relaxed);

                if successes >= self.config.success_threshold {
                    self.transition_to(CircuitState::Closed);
                }
            }
            CircuitState::Open => {
                // Should not happen
            }
        }
    }

    /// Handle failed call
    fn handle_failure(&self) {
        let state = self.state();
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
        self.consecutive_successes.store(0, Ordering::Relaxed);

        match state {
            CircuitState::Closed => {
                // Check if we should open the circuit
                if failures >= self.config.failure_threshold {
                    self.transition_to(CircuitState::Open);
                }

                // Also check error rate
                let error_rate = self.calculate_error_rate();
                if error_rate >= self.config.error_rate_threshold {
                    self.transition_to(CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                // Immediately go back to open on any failure
                self.transition_to(CircuitState::Open);
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Manually reset circuit to closed state
    pub fn reset(&self) {
        self.transition_to(CircuitState::Closed);
        self.half_open_calls.store(0, Ordering::Relaxed);
        let mut history = self.call_history.lock();
        history.clear();
    }

    /// Manually open circuit
    pub fn open(&self) {
        self.transition_to(CircuitState::Open);
    }

    /// Get consecutive failures
    pub fn consecutive_failures(&self) -> usize {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    /// Get consecutive successes
    pub fn consecutive_successes(&self) -> usize {
        self.consecutive_successes.load(Ordering::Relaxed)
    }

    /// Get total rejected calls
    pub fn rejected_calls(&self) -> usize {
        self.rejected_calls.load(Ordering::Relaxed)
    }

    /// Get total successful calls
    pub fn total_successes(&self) -> usize {
        self.total_successes.load(Ordering::Relaxed)
    }

    /// Get total failed calls
    pub fn total_failures(&self) -> usize {
        self.total_failures.load(Ordering::Relaxed)
    }

    /// Get total calls (successes + failures)
    pub fn total_calls(&self) -> usize {
        self.total_successes() + self.total_failures()
    }

    /// Get current error rate
    pub fn error_rate(&self) -> f64 {
        self.calculate_error_rate()
    }

    /// Get time since state change
    pub fn time_since_state_change(&self) -> Option<Duration> {
        self.state_changed.lock().map(|ts| Timestamp::now().duration_since(ts))
    }

    /// Get call history size
    pub fn history_size(&self) -> usize {
        self.call_history.lock().len()
    }

    /// Convert circuit state to health status
    pub fn health_status(&self) -> HealthStatus {
        match self.state() {
            CircuitState::Closed => HealthStatus::Healthy,
            CircuitState::HalfOpen => HealthStatus::Unknown,
            CircuitState::Open => HealthStatus::Unhealthy,
        }
    }
}

/// Builder for creating circuit breakers
pub struct CircuitBreakerBuilder {
    config: CircuitConfig,
}

impl CircuitBreakerBuilder {
    /// Create new builder
    pub fn new() -> Self {
        CircuitBreakerBuilder {
            config: CircuitConfig::default(),
        }
    }

    /// Set failure threshold
    pub fn failure_threshold(mut self, threshold: usize) -> Self {
        self.config.failure_threshold = threshold;
        self
    }

    /// Set success threshold
    pub fn success_threshold(mut self, threshold: usize) -> Self {
        self.config.success_threshold = threshold;
        self
    }

    /// Set recovery timeout
    pub fn recovery_timeout(mut self, timeout: Duration) -> Self {
        self.config.recovery_timeout = timeout;
        self
    }

    /// Set half-open max calls
    pub fn half_open_max_calls(mut self, max: usize) -> Self {
        self.config.half_open_max_calls = max;
        self
    }

    /// Set call timeout
    pub fn call_timeout(mut self, timeout: Duration) -> Self {
        self.config.call_timeout = timeout;
        self
    }

    /// Set error rate threshold
    pub fn error_rate_threshold(mut self, threshold: f64) -> Self {
        self.config.error_rate_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Build the circuit breaker
    pub fn build(self) -> CircuitBreaker {
        CircuitBreaker::new(self.config)
    }
}

impl Default for CircuitBreakerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_state_conversions() {
        assert_eq!(CircuitState::from_u8(0), Some(CircuitState::Closed));
        assert_eq!(CircuitState::from_u8(1), Some(CircuitState::Open));
        assert_eq!(CircuitState::from_u8(2), Some(CircuitState::HalfOpen));
        assert_eq!(CircuitState::from_u8(3), None);

        assert!(CircuitState::Closed.allows_requests());
        assert!(CircuitState::HalfOpen.allows_requests());
        assert!(!CircuitState::Open.allows_requests());

        assert!(CircuitState::Open.is_open());
        assert!(!CircuitState::Closed.is_open());
        assert!(!CircuitState::HalfOpen.is_open());
    }

    #[test]
    fn test_circuit_config_default() {
        let config = CircuitConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.recovery_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_circuit_config_builder() {
        let config = CircuitConfig::new(10)
            .with_success_threshold(3)
            .with_recovery_timeout(Duration::from_secs(30))
            .with_error_rate_threshold(0.7);

        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.recovery_timeout, Duration::from_secs(30));
        assert_eq!(config.error_rate_threshold, 0.7);
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let breaker = CircuitBreaker::new(CircuitConfig::default());
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.allows_requests());
        assert_eq!(breaker.consecutive_failures(), 0);
        assert_eq!(breaker.consecutive_successes(), 0);
    }

    #[test]
    fn test_circuit_breaker_success() {
        let breaker = CircuitBreaker::new(CircuitConfig::default());
        let result = breaker.call(|| Ok("success".to_string()));

        assert!(result.is_success());
        assert_eq!(breaker.total_successes(), 1);
        assert_eq!(breaker.total_failures(), 0);
    }

    #[test]
    fn test_circuit_breaker_failure() {
        let config = CircuitConfig::new(3);
        let breaker = CircuitBreaker::new(config);

        // Fail three times
        for _ in 0..3 {
            breaker.call::<(), _>(|| Err("error".to_string()));
        }

        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.allows_requests());
        assert_eq!(breaker.consecutive_failures(), 3);
    }

    #[test]
    fn test_circuit_breaker_rejects_when_open() {
        let config = CircuitConfig::new(2);
        let breaker = CircuitBreaker::new(config);

        // Fail twice to open circuit
        for _ in 0..2 {
            breaker.call::<(), _>(|| Err("error".to_string()));
        }

        assert_eq!(breaker.state(), CircuitState::Open);

        // Next call should be rejected
        let result = breaker.call(|| Ok("success"));
        assert!(result.is_rejected());
        assert_eq!(breaker.rejected_calls(), 1);
    }

    #[test]
    fn test_circuit_breaker_manual_reset() {
        let config = CircuitConfig::new(2);
        let breaker = CircuitBreaker::new(config);

        // Open circuit
        for _ in 0..2 {
            breaker.call::<(), _>(|| Err("error".to_string()));
        }

        assert_eq!(breaker.state(), CircuitState::Open);

        // Reset
        breaker.reset();

        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.allows_requests());
    }

    #[test]
    fn test_circuit_breaker_manual_open() {
        let breaker = CircuitBreaker::new(CircuitConfig::default());
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.open();
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_call_result() {
        let success: CallResult<String> = CallResult::Success("value".to_string());
        assert!(success.is_success());
        assert!(!success.is_error());
        assert!(!success.is_rejected());

        let error: CallResult<String> = CallResult::Error("err".to_string());
        assert!(!error.is_success());
        assert!(error.is_error());
        assert!(!error.is_rejected());

        let rejected: CallResult<String> = CallResult::Rejected;
        assert!(!rejected.is_success());
        assert!(!rejected.is_error());
        assert!(rejected.is_rejected());
    }

    #[test]
    fn test_circuit_breaker_error_rate() {
        let config = CircuitConfig::new(100)
            .with_minimum_calls(5)
            .with_error_rate_threshold(0.5);

        let breaker = CircuitBreaker::new(config);

        // 4 failures out of 6 calls = 66% error rate
        for _ in 0..4 {
            breaker.call::<(), _>(|| Err("error".to_string()));
        }
        for _ in 0..2 {
            breaker.call(|| Ok(()));
        }

        let error_rate = breaker.error_rate();
        assert!(error_rate > 0.6);
        assert!(error_rate < 0.7);
    }

    #[test]
    fn test_circuit_breaker_builder() {
        let breaker = CircuitBreakerBuilder::new()
            .failure_threshold(10)
            .success_threshold(3)
            .recovery_timeout(Duration::from_secs(120))
            .error_rate_threshold(0.8)
            .build();

        assert_eq!(breaker.config().failure_threshold, 10);
        assert_eq!(breaker.config().success_threshold, 3);
        assert_eq!(breaker.config().recovery_timeout, Duration::from_secs(120));
        assert_eq!(breaker.config().error_rate_threshold, 0.8);
    }

    #[test]
    fn test_circuit_breaker_health_status() {
        let breaker = CircuitBreaker::new(CircuitConfig::default());

        assert_eq!(breaker.health_status(), HealthStatus::Healthy);

        breaker.open();
        assert_eq!(breaker.health_status(), HealthStatus::Unhealthy);

        breaker.reset();
        // Manually transition to half-open (normally happens automatically)
        breaker.state.store(CircuitState::HalfOpen as u8, Ordering::Relaxed);
        assert_eq!(breaker.health_status(), HealthStatus::Unknown);
    }

    #[test]
    fn test_circuit_breaker_stats() {
        let breaker = CircuitBreaker::new(CircuitConfig::default());

        breaker.call(|| Ok::<_, String>("success"));
        breaker.call::<_, String>(|| Err("error".to_string()));

        assert_eq!(breaker.total_successes(), 1);
        assert_eq!(breaker.total_failures(), 1);
        assert_eq!(breaker.total_calls(), 2);
        assert_eq!(breaker.history_size(), 2);
    }
}
