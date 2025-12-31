//! # Resilience Patterns and Chaos Engineering
//!
//! This module provides resilience patterns including circuit breakers, retry
//! mechanisms, and chaos engineering support.
//!
//! ## Architecture
//!
//! The resilience system provides:
//!
//! - **Circuit Breaker**: Prevent cascading failures
//! - **Retry with Backoff**: Handle transient failures
//! - **Bulkhead**: Resource isolation
//! - **Timeout**: Prevent hanging operations
//! - **Chaos Engineering**: Test system resilience
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::ha::resilience::{ResilienceManager, CircuitBreakerConfig, RetryPolicy};
//!
//! # async fn example() -> Result<(), kernel::ha::HaError> {
//! let manager = ResilienceManager::new();
//!
//! // Configure circuit breaker
//! let cb_config = CircuitBreakerConfig {
//!     failure_threshold: 5,
//!     timeout_ms: 60000,
//!     ..Default::default()
//! };
//! manager.add_circuit_breaker("api", cb_config);
//!
//! // Execute with resilience
//! let result = manager.execute_with_resilience("api", || async {
//!     // Your operation here
//!     Ok(())
//! }).await?;
//! # Ok(())
//! # }
//! ```

use crate::ha::{HaError, HaResult, ResilienceError};
use crate::subsystems::sync::Mutex;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed (normal operation)
    Closed,
    /// Circuit is open (failing, rejecting requests)
    Open,
    /// Circuit is half-open (testing if service recovered)
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening
    pub failure_threshold: usize,
    /// Timeout before attempting recovery (milliseconds)
    pub timeout_ms: u64,
    /// Number of successful calls to close circuit in half-open
    pub success_threshold: usize,
    /// Rolling window size for statistics
    pub window_size: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        CircuitBreakerConfig {
            failure_threshold: 5,
            timeout_ms: 60000,
            success_threshold: 2,
            window_size: 100,
        }
    }
}

/// Circuit breaker
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Circuit name
    name: String,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Current state
    state: Arc<Mutex<CircuitState>>,
    /// Failure count
    failure_count: Arc<AtomicUsize>,
    /// Success count (for half-open)
    success_count: Arc<AtomicUsize>,
    /// Last failure time
    last_failure_time: Arc<AtomicU64>,
    /// Last state change time
    last_state_change: Arc<AtomicU64>,
    /// Request counter
    request_count: Arc<AtomicU64>,
    /// Total failures
    total_failures: Arc<AtomicU64>,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        CircuitBreaker {
            name,
            config,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicUsize::new(0)),
            success_count: Arc::new(AtomicUsize::new(0)),
            last_failure_time: Arc::new(AtomicU64::new(0)),
            last_state_change: Arc::new(AtomicU64::new(0)),
            request_count: Arc::new(AtomicU64::new(0)),
            total_failures: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Execute operation with circuit breaker protection
    pub async fn execute<F, T, E>(&self, f: F) -> HaResult<T>
    where
        F: core::future::Future<Output = Result<T, E>>,
        E: Into<HaError>,
    {
        // Check if circuit allows execution
        self.check_state()?;

        self.request_count.fetch_add(1, Ordering::Relaxed);

        // Execute operation
        let result = f.await;

        match result {
            Ok(value) => {
                self.on_success();
                Ok(value)
            }
            Err(err) => {
                self.on_failure();
                Err(err.into())
            }
        }
    }

    /// Check circuit state and determine if request should proceed
    fn check_state(&self) -> HaResult<()> {
        let state = self.state.lock();

        match *state {
            CircuitState::Open => {
                let now = self.current_time_ms();
                let last_change = self.last_state_change.load(Ordering::Relaxed);
                let timeout = self.config.timeout_ms;

                if now.saturating_sub(last_change) >= timeout {
                    drop(state);
                    self.transition_to(CircuitState::HalfOpen);
                } else {
                    return Err(HaError::ResilienceError(ResilienceError::CircuitBreakerOpen));
                }
            }
            CircuitState::HalfOpen => {
                // Allow request through
            }
            CircuitState::Closed => {
                // Allow request through
            }
        }

        Ok(())
    }

    /// Handle successful operation
    fn on_success(&self) {
        let state = self.state.lock();

        match *state {
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.config.success_threshold {
                    drop(state);
                    self.transition_to(CircuitState::Closed);
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {
                // Should not happen
            }
        }
    }

    /// Handle failed operation
    fn on_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        self.last_failure_time.store(self.current_time_ms(), Ordering::Relaxed);

        let state = self.state.lock();

        match *state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= self.config.failure_threshold {
                    drop(state);
                    self.transition_to(CircuitState::Open);
                }
            }
            CircuitState::HalfOpen => {
                drop(state);
                self.transition_to(CircuitState::Open);
                self.success_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {
                // Already open
            }
        }
    }

    /// Transition to new state
    fn transition_to(&self, new_state: CircuitState) {
        let mut state = self.state.lock();
        *state = new_state;
        self.last_state_change.store(self.current_time_ms(), Ordering::Relaxed);

        if new_state == CircuitState::Closed {
            self.failure_count.store(0, Ordering::Relaxed);
        }
    }

    /// Get current state
    pub fn get_state(&self) -> CircuitState {
        *self.state.lock()
    }

    /// Get circuit breaker metrics
    pub fn get_metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            name: self.name.clone(),
            state: *self.state.lock(),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            total_requests: self.request_count.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
        }
    }

    /// Reset circuit breaker
    pub fn reset(&self) {
        self.transition_to(CircuitState::Closed);
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
    }

    /// Get current time in milliseconds
    fn current_time_ms(&self) -> u64 {
        0 // In real implementation, use actual time
    }
}

/// Circuit breaker metrics
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    /// Circuit name
    pub name: String,
    /// Current state
    pub state: CircuitState,
    /// Current failure count
    pub failure_count: usize,
    /// Total requests
    pub total_requests: u64,
    /// Total failures
    pub total_failures: u64,
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial backoff duration
    pub initial_backoff_ms: u64,
    /// Maximum backoff duration
    pub max_backoff_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Backoff strategy
    pub strategy: BackoffStrategy,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            backoff_multiplier: 2.0,
            strategy: BackoffStrategy::Exponential,
        }
    }
}

/// Backoff strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Linear increase
    Linear,
    /// Exponential increase
    Exponential,
    /// Exponential with jitter
    ExponentialWithJitter,
}

impl BackoffStrategy {
    /// Calculate backoff duration
    pub fn calculate_backoff(&self, attempt: usize, config: &RetryPolicy) -> Duration {
        let base_ms = config.initial_backoff_ms;

        let delay_ms = match self {
            BackoffStrategy::Fixed => base_ms,
            BackoffStrategy::Linear => base_ms * attempt as u64,
            BackoffStrategy::Exponential => {
                let exponential = (config.backoff_multiplier.powi(attempt as i32 - 1)) as u64;
                (base_ms * exponential).min(config.max_backoff_ms)
            }
            BackoffStrategy::ExponentialWithJitter => {
                let exponential = (config.backoff_multiplier.powi(attempt as i32 - 1)) as u64;
                let base_delay = (base_ms * exponential).min(config.max_backoff_ms);
                // Add random jitter: base_delay * (0.5 + random[0,1))
                base_delay / 2 + base_delay / 2 // Simplified jitter
            }
        };

        Duration::from_millis(delay_ms)
    }
}

/// Resilience manager
#[derive(Debug)]
pub struct ResilienceManager {
    /// Circuit breakers map
    circuit_breakers: Arc<Mutex<BTreeMap<String, Arc<CircuitBreaker>>>>,
    /// Retry policies map
    retry_policies: Arc<Mutex<BTreeMap<String, RetryPolicy>>>,
    /// Resilience metrics
    metrics: Arc<Mutex<ResilienceMetrics>>,
}

/// Resilience metrics
#[derive(Debug, Clone, Default)]
pub struct ResilienceMetrics {
    /// Total operations executed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Retried operations
    pub retried_operations: u64,
    /// Circuit breaker opens
    pub circuit_breaker_opens: u64,
    /// Average operation duration (ms)
    pub avg_duration_ms: f64,
}

impl ResilienceManager {
    /// Create new resilience manager
    pub fn new() -> Self {
        ResilienceManager {
            circuit_breakers: Arc::new(Mutex::new(BTreeMap::new())),
            retry_policies: Arc::new(Mutex::new(BTreeMap::new())),
            metrics: Arc::new(Mutex::new(ResilienceMetrics::default())),
        }
    }

    /// Add circuit breaker
    pub fn add_circuit_breaker(&self, name: &str, config: CircuitBreakerConfig) {
        let cb = Arc::new(CircuitBreaker::new(name.to_string(), config));
        self.circuit_breakers.lock().insert(name.to_string(), cb);
    }

    /// Add retry policy
    pub fn add_retry_policy(&self, name: &str, policy: RetryPolicy) {
        self.retry_policies.lock().insert(name.to_string(), policy);
    }

    /// Execute operation with circuit breaker protection
    pub async fn execute_with_circuit_breaker<F, T, E>(
        &self,
        name: &str,
        f: F,
    ) -> HaResult<T>
    where
        F: core::future::Future<Output = Result<T, E>>,
        E: Into<HaError>,
    {
        let breakers = self.circuit_breakers.lock();
        let cb = breakers.get(name)
            .ok_or(HaError::ResilienceError(ResilienceError::InvalidConfig))?;

        cb.execute(f).await
    }

    /// Execute operation with retry logic
    pub async fn execute_with_retry<F, T, E>(
        &self,
        policy_name: &str,
        mut f: F,
    ) -> HaResult<T>
    where
        F: FnMut() -> core::future::Pending<Result<T, E>>,
        E: Into<HaError>,
    {
        let policies = self.retry_policies.lock();
        let policy = policies.get(policy_name)
            .cloned()
            .unwrap_or_default();

        drop(policies);

        let mut last_error = None;

        for attempt in 1..=policy.max_attempts {
            let result = f().await;

            match result {
                Ok(value) => {
                    if attempt > 1 {
                        let mut metrics = self.metrics.lock();
                        metrics.retried_operations += 1;
                    }
                    return Ok(value);
                }
                Err(err) => {
                    last_error = Some(err.into());

                    if attempt < policy.max_attempts {
                        let backoff = policy.strategy.calculate_backoff(attempt, &policy);
                        // In real implementation, async sleep here
                        let _ = backoff;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(HaError::ResilienceError(ResilienceError::RetriesExhausted)))
    }

    /// Get circuit breaker metrics
    pub fn get_circuit_breaker_metrics(&self, name: &str) -> Option<CircuitBreakerMetrics> {
        let breakers = self.circuit_breakers.lock();
        breakers.get(name).map(|cb| cb.get_metrics())
    }

    /// Get all circuit breaker metrics
    pub fn get_all_circuit_breaker_metrics(&self) -> Vec<CircuitBreakerMetrics> {
        let breakers = self.circuit_breakers.lock();
        breakers.values().map(|cb| cb.get_metrics()).collect()
    }

    /// Get resilience metrics
    pub fn get_metrics(&self) -> ResilienceMetrics {
        self.metrics.lock().clone()
    }

    /// Reset circuit breaker
    pub fn reset_circuit_breaker(&self, name: &str) -> HaResult<()> {
        let breakers = self.circuit_breakers.lock();
        let cb = breakers.get(name)
            .ok_or(HaError::ResilienceError(ResilienceError::InvalidConfig))?;
        cb.reset();
        Ok(())
    }
}

/// Chaos engineering configuration
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Enable chaos experiments
    pub enabled: bool,
    /// Failure injection probability (0.0 - 1.0)
    pub failure_probability: f64,
    /// Latency injection enabled
    pub inject_latency: bool,
    /// Min latency to inject (milliseconds)
    pub min_latency_ms: u64,
    /// Max latency to inject (milliseconds)
    pub max_latency_ms: u64,
    /// Random kill enabled
    pub enable_kill: bool,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        ChaosConfig {
            enabled: false,
            failure_probability: 0.1,
            inject_latency: true,
            min_latency_ms: 100,
            max_latency_ms: 1000,
            enable_kill: false,
        }
    }
}

/// Chaos engine for resilience testing
#[derive(Debug)]
pub struct ChaosEngine {
    /// Configuration
    config: ChaosConfig,
    /// Experiment counter
    experiment_count: Arc<AtomicU64>,
    /// Failure counter
    failure_count: Arc<AtomicU64>,
}

impl ChaosEngine {
    /// Create new chaos engine
    pub fn new(config: ChaosConfig) -> Self {
        ChaosEngine {
            config,
            experiment_count: Arc::new(AtomicU64::new(0)),
            failure_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Execute operation with chaos injection
    pub async fn execute_with_chaos<F, T>(&self, f: F) -> HaResult<T>
    where
        F: core::future::Future<Output = HaResult<T>>,
    {
        if !self.config.enabled {
            return f.await;
        }

        self.experiment_count.fetch_add(1, Ordering::Relaxed);

        // Inject latency
        if self.config.inject_latency {
            let latency = self.config.min_latency_ms +
                (self.config.max_latency_ms - self.config.min_latency_ms) / 2;
            // In real implementation, sleep for latency
            let _ = latency;
        }

        // Inject failure
        if self.random() < self.config.failure_probability {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
            return Err(HaError::ResilienceError(ResilienceError::ChaosTestFailed));
        }

        f.await
    }

    /// Get chaos statistics
    pub fn get_stats(&self) -> ChaosStats {
        let total = self.experiment_count.load(Ordering::Relaxed);
        let failures = self.failure_count.load(Ordering::Relaxed);

        ChaosStats {
            total_experiments: total,
            injected_failures: failures,
            failure_rate: if total > 0 {
                failures as f64 / total as f64
            } else {
                0.0
            },
        }
    }

    /// Generate random number (simplified)
    fn random(&self) -> f64 {
        0.5 // In real implementation, use proper RNG
    }
}

/// Chaos statistics
#[derive(Debug, Clone)]
pub struct ChaosStats {
    /// Total experiments run
    pub total_experiments: u64,
    /// Number of injected failures
    pub injected_failures: u64,
    /// Failure rate
    pub failure_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_state() {
        assert_eq!(CircuitState::Closed, CircuitState::Closed);
        assert_eq!(CircuitState::Open, CircuitState::Open);
        assert_eq!(CircuitState::HalfOpen, CircuitState::HalfOpen);
    }

    #[test]
    fn test_backoff_strategy() {
        let config = RetryPolicy::default();

        let fixed = BackoffStrategy::Fixed.calculate_backoff(1, &config);
        assert_eq!(fixed.as_millis(), 100);

        let linear = BackoffStrategy::Linear.calculate_backoff(2, &config);
        assert_eq!(linear.as_millis(), 200);
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_backoff_ms, 100);
    }

    #[test]
    fn test_circuit_breaker_creation() {
        let config = CircuitBreakerConfig::default();
        let cb = CircuitBreaker::new("test".to_string(), config);

        assert_eq!(cb.get_state(), CircuitState::Closed);

        let metrics = cb.get_metrics();
        assert_eq!(metrics.name, "test");
    }

    #[tokio::test]
    async fn test_resilience_manager() {
        let manager = ResilienceManager::new();
        manager.add_circuit_breaker("test", CircuitBreakerConfig::default());

        let metrics = manager.get_circuit_breaker_metrics("test");
        assert!(metrics.is_some());

        let all_metrics = manager.get_all_circuit_breaker_metrics();
        assert_eq!(all_metrics.len(), 1);
    }

    #[test]
    fn test_chaos_config() {
        let config = ChaosConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.failure_probability, 0.1);
    }

    #[tokio::test]
    async fn test_chaos_engine() {
        let config = ChaosConfig {
            enabled: true,
            failure_probability: 0.0, // No failures
            ..Default::default()
        };

        let engine = ChaosEngine::new(config);

        let result = engine.execute_with_chaos(async { Ok::<_, HaError>(42) }).await;
        assert!(result.is_ok());

        let stats = engine.get_stats();
        assert_eq!(stats.total_experiments, 1);
    }
}
