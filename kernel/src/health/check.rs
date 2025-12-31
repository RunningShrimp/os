//! Health check implementation for the NOS kernel.
//!
//! This module provides comprehensive health checking capabilities including:
//! - Liveness probes (is the process alive?)
//! - Readiness probes (is it ready to serve traffic?)
//! - Startup probes (has it started successfully?)
//! - Configurable timeouts and intervals
//! - Success/failure thresholds
//! - Health status tracking

#![no_std]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use core::time::Duration;
use spin::Mutex;

use crate::subsystems::time::Timestamp;

/// Health status of a component or system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HealthStatus {
    /// Component is healthy and functioning normally
    Healthy = 0,
    /// Component is unhealthy or malfunctioning
    Unhealthy = 1,
    /// Component health status is unknown
    Unknown = 2,
}

impl HealthStatus {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(HealthStatus::Healthy),
            1 => Some(HealthStatus::Unhealthy),
            2 => Some(HealthStatus::Unknown),
            _ => None,
        }
    }

    /// Check if status is healthy
    pub fn is_healthy(self) -> bool {
        self == HealthStatus::Healthy
    }

    /// Check if status is unhealthy
    pub fn is_unhealthy(self) -> bool {
        self == HealthStatus::Unhealthy
    }

    /// Check if status is unknown
    pub fn is_unknown(self) -> bool {
        self == HealthStatus::Unknown
    }
}

/// Types of health checks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckType {
    /// Liveness probe - checks if the process is alive
    Liveness,
    /// Readiness probe - checks if ready to serve traffic
    Readiness,
    /// Startup probe - checks if startup has completed
    Startup,
}

impl CheckType {
    /// Get check type name
    pub fn name(&self) -> &str {
        match self {
            CheckType::Liveness => "liveness",
            CheckType::Readiness => "readiness",
            CheckType::Startup => "startup",
        }
    }
}

/// Configuration for a health check
#[derive(Debug, Clone)]
pub struct CheckConfig {
    /// Type of health check
    pub check_type: CheckType,
    /// Interval between checks
    pub interval: Duration,
    /// Timeout for each check
    pub timeout: Duration,
    /// Number of consecutive successes required to mark healthy
    pub success_threshold: usize,
    /// Number of consecutive failures required to mark unhealthy
    pub failure_threshold: usize,
    /// Initial delay before starting checks
    pub initial_delay: Duration,
}

impl Default for CheckConfig {
    fn default() -> Self {
        CheckConfig {
            check_type: CheckType::Liveness,
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
            success_threshold: 1,
            failure_threshold: 3,
            initial_delay: Duration::from_secs(0),
        }
    }
}

impl CheckConfig {
    /// Create new liveness check config
    pub fn liveness() -> Self {
        CheckConfig {
            check_type: CheckType::Liveness,
            interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
            success_threshold: 1,
            failure_threshold: 3,
            initial_delay: Duration::from_secs(0),
        }
    }

    /// Create new readiness check config
    pub fn readiness() -> Self {
        CheckConfig {
            check_type: CheckType::Readiness,
            interval: Duration::from_secs(5),
            timeout: Duration::from_secs(3),
            success_threshold: 1,
            failure_threshold: 3,
            initial_delay: Duration::from_secs(0),
        }
    }

    /// Create new startup check config
    pub fn startup() -> Self {
        CheckConfig {
            check_type: CheckType::Startup,
            interval: Duration::from_secs(5),
            timeout: Duration::from_secs(3),
            success_threshold: 1,
            failure_threshold: 30,
            initial_delay: Duration::from_secs(0),
        }
    }

    /// Set custom interval
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set success threshold
    pub fn with_success_threshold(mut self, threshold: usize) -> Self {
        self.success_threshold = threshold;
        self
    }

    /// Set failure threshold
    pub fn with_failure_threshold(mut self, threshold: usize) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }
}

/// Result of a single health check execution
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Whether the check passed
    pub passed: bool,
    /// Timestamp when check was performed
    pub timestamp: Timestamp,
    /// Duration of the check
    pub duration: Duration,
    /// Optional message or error details
    pub message: Option<String>,
}

impl CheckResult {
    /// Create successful check result
    pub fn success(duration: Duration) -> Self {
        CheckResult {
            passed: true,
            timestamp: Timestamp::now(),
            duration,
            message: None,
        }
    }

    /// Create successful check result with message
    pub fn success_with_message(duration: Duration, message: String) -> Self {
        CheckResult {
            passed: true,
            timestamp: Timestamp::now(),
            duration,
            message: Some(message),
        }
    }

    /// Create failed check result
    pub fn failure(duration: Duration, message: String) -> Self {
        CheckResult {
            passed: false,
            timestamp: Timestamp::now(),
            duration,
            message: Some(message),
        }
    }
}

/// History of health check results
#[derive(Debug)]
pub struct CheckHistory {
    /// Recent check results (max size determined by capacity)
    results: VecDeque<CheckResult>,
    /// Maximum number of results to store
    capacity: usize,
}

impl CheckHistory {
    /// Create new check history
    pub fn new(capacity: usize) -> Self {
        CheckHistory {
            results: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Add a check result
    pub fn add(&mut self, result: CheckResult) {
        if self.results.len() == self.capacity {
            self.results.pop_front();
        }
        self.results.push_back(result);
    }

    /// Get all results
    pub fn results(&self) -> &VecDeque<CheckResult> {
        &self.results
    }

    /// Get recent results
    pub fn recent(&self, count: usize) -> Vec<&CheckResult> {
        self.results.iter().rev().take(count).collect()
    }

    /// Count consecutive passes
    pub fn consecutive_passes(&self) -> usize {
        let mut count = 0;
        for result in self.results.iter().rev() {
            if result.passed {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Count consecutive failures
    pub fn consecutive_failures(&self) -> usize {
        let mut count = 0;
        for result in self.results.iter().rev() {
            if !result.passed {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let passed = self.results.iter().filter(|r| r.passed).count();
        (passed as f64) / (self.results.len() as f64)
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.results.clear();
    }

    /// Get last result if any
    pub fn last(&self) -> Option<&CheckResult> {
        self.results.back()
    }
}

/// Health check function signature
pub type HealthCheckFn = dyn Fn() -> Result<Duration, String> + Send + Sync;

/// Health check executor
pub struct HealthCheck {
    /// Check configuration
    config: CheckConfig,
    /// Check function
    check_fn: Arc<HealthCheckFn>,
    /// Current health status
    status: AtomicU8,
    /// Consecutive successes
    consecutive_successes: AtomicUsize,
    /// Consecutive failures
    consecutive_failures: AtomicUsize,
    /// Check history
    history: Mutex<CheckHistory>,
    /// Last check timestamp
    last_check: Mutex<Option<Timestamp>>,
    /// Total checks performed
    total_checks: AtomicUsize,
}

impl HealthCheck {
    /// Create new health check
    pub fn new(config: CheckConfig, check_fn: Arc<HealthCheckFn>) -> Self {
        HealthCheck {
            config,
            check_fn,
            status: AtomicU8::new(HealthStatus::Unknown as u8),
            consecutive_successes: AtomicUsize::new(0),
            consecutive_failures: AtomicUsize::new(0),
            history: Mutex::new(CheckHistory::new(100)),
            last_check: Mutex::new(None),
            total_checks: AtomicUsize::new(0),
        }
    }

    /// Get current health status
    pub fn status(&self) -> HealthStatus {
        HealthStatus::from_u8(self.status.load(Ordering::Relaxed))
            .unwrap_or(HealthStatus::Unknown)
    }

    /// Get check configuration
    pub fn config(&self) -> &CheckConfig {
        &self.config
    }

    /// Get check type
    pub fn check_type(&self) -> CheckType {
        self.config.check_type
    }

    /// Execute the health check
    pub fn execute(&self) -> CheckResult {
        let start = Timestamp::now();
        let result = (self.check_fn)();
        let end = Timestamp::now();
        let duration = end.duration_since(start);

        let check_result = match result {
            Ok(d) => CheckResult::success(d),
            Err(e) => CheckResult::failure(duration, e),
        };

        self.process_result(check_result.clone());
        check_result
    }

    /// Process check result and update state
    fn process_result(&self, result: CheckResult) {
        // Update history
        let mut history = self.history.lock();
        history.add(result.clone());

        // Update counters
        self.total_checks.fetch_add(1, Ordering::Relaxed);

        // Update consecutive counters
        if result.passed {
            self.consecutive_successes.fetch_add(1, Ordering::Relaxed);
            self.consecutive_failures.store(0, Ordering::Relaxed);

            // Check if we've reached success threshold
            let successes = self.consecutive_successes.load(Ordering::Relaxed);
            if successes >= self.config.success_threshold {
                self.status.store(HealthStatus::Healthy as u8, Ordering::Relaxed);
            }
        } else {
            self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
            self.consecutive_successes.store(0, Ordering::Relaxed);

            // Check if we've reached failure threshold
            let failures = self.consecutive_failures.load(Ordering::Relaxed);
            if failures >= self.config.failure_threshold {
                self.status.store(HealthStatus::Unhealthy as u8, Ordering::Relaxed);
            }
        }

        // Update last check time
        let mut last_check = self.last_check.lock();
        *last_check = Some(Timestamp::now());
    }

    /// Get check history
    pub fn history(&self) -> Vec<CheckResult> {
        let history = self.history.lock();
        history.results().iter().cloned().collect()
    }

    /// Get recent check results
    pub fn recent_results(&self, count: usize) -> Vec<CheckResult> {
        let history = self.history.lock();
        history.recent(count).into_iter().cloned().collect()
    }

    /// Get consecutive successes
    pub fn consecutive_successes(&self) -> usize {
        self.consecutive_successes.load(Ordering::Relaxed)
    }

    /// Get consecutive failures
    pub fn consecutive_failures(&self) -> usize {
        self.consecutive_failures.load(Ordering::Relaxed)
    }

    /// Get total checks performed
    pub fn total_checks(&self) -> usize {
        self.total_checks.load(Ordering::Relaxed)
    }

    /// Get last check timestamp
    pub fn last_check(&self) -> Option<Timestamp> {
        *self.last_check.lock()
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let history = self.history.lock();
        history.success_rate()
    }

    /// Reset health check state
    pub fn reset(&self) {
        self.status.store(HealthStatus::Unknown as u8, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.total_checks.store(0, Ordering::Relaxed);
        let mut history = self.history.lock();
        history.clear();
        let mut last_check = self.last_check.lock();
        *last_check = None;
    }

    /// Manually set health status
    pub fn set_status(&self, status: HealthStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }

    /// Check if enough time has passed since last check
    pub fn should_check(&self) -> bool {
        if let Some(last) = *self.last_check.lock() {
            let elapsed = Timestamp::now().duration_since(last);
            elapsed >= self.config.interval
        } else {
            true
        }
    }
}

impl core::fmt::Debug for HealthCheck {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HealthCheck")
            .field("config", &self.config)
            .field("check_fn", &"<health check function>")
            .field("status", &self.status())
            .field("consecutive_successes", &self.consecutive_successes.load(Ordering::Relaxed))
            .field("consecutive_failures", &self.consecutive_failures.load(Ordering::Relaxed))
            .field("history", &self.history)
            .field("last_check", &self.last_check)
            .field("total_checks", &self.total_checks.load(Ordering::Relaxed))
            .finish()
    }
}

/// Builder for creating health checks
pub struct HealthCheckBuilder {
    config: CheckConfig,
}

impl HealthCheckBuilder {
    /// Create new builder
    pub fn new(check_type: CheckType) -> Self {
        let config = match check_type {
            CheckType::Liveness => CheckConfig::liveness(),
            CheckType::Readiness => CheckConfig::readiness(),
            CheckType::Startup => CheckConfig::startup(),
        };

        HealthCheckBuilder { config }
    }

    /// Set check interval
    pub fn interval(mut self, interval: Duration) -> Self {
        self.config.interval = interval;
        self
    }

    /// Set check timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Set success threshold
    pub fn success_threshold(mut self, threshold: usize) -> Self {
        self.config.success_threshold = threshold;
        self
    }

    /// Set failure threshold
    pub fn failure_threshold(mut self, threshold: usize) -> Self {
        self.config.failure_threshold = threshold;
        self
    }

    /// Set initial delay
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.config.initial_delay = delay;
        self
    }

    /// Build the health check
    pub fn build(self, check_fn: Arc<HealthCheckFn>) -> HealthCheck {
        HealthCheck::new(self.config, check_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;

    #[test]
    fn test_health_status_conversions() {
        assert_eq!(HealthStatus::from_u8(0), Some(HealthStatus::Healthy));
        assert_eq!(HealthStatus::from_u8(1), Some(HealthStatus::Unhealthy));
        assert_eq!(HealthStatus::from_u8(2), Some(HealthStatus::Unknown));
        assert_eq!(HealthStatus::from_u8(3), None);

        assert!(HealthStatus::Healthy.is_healthy());
        assert!(HealthStatus::Unhealthy.is_unhealthy());
        assert!(HealthStatus::Unknown.is_unknown());
    }

    #[test]
    fn test_check_type_names() {
        assert_eq!(CheckType::Liveness.name(), "liveness");
        assert_eq!(CheckType::Readiness.name(), "readiness");
        assert_eq!(CheckType::Startup.name(), "startup");
    }

    #[test]
    fn test_check_config_defaults() {
        let config = CheckConfig::default();
        assert_eq!(config.check_type, CheckType::Liveness);
        assert_eq!(config.interval, Duration::from_secs(10));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.success_threshold, 1);
        assert_eq!(config.failure_threshold, 3);
    }

    #[test]
    fn test_check_config_builders() {
        let liveness = CheckConfig::liveness();
        assert_eq!(liveness.check_type, CheckType::Liveness);

        let readiness = CheckConfig::readiness();
        assert_eq!(readiness.check_type, CheckType::Readiness);

        let startup = CheckConfig::startup();
        assert_eq!(startup.check_type, CheckType::Startup);
    }

    #[test]
    fn test_check_config_chaining() {
        let config = CheckConfig::liveness()
            .with_interval(Duration::from_secs(5))
            .with_timeout(Duration::from_secs(2))
            .with_success_threshold(2)
            .with_failure_threshold(5);

        assert_eq!(config.interval, Duration::from_secs(5));
        assert_eq!(config.timeout, Duration::from_secs(2));
        assert_eq!(config.success_threshold, 2);
        assert_eq!(config.failure_threshold, 5);
    }

    #[test]
    fn test_check_result() {
        let success = CheckResult::success(Duration::from_millis(100));
        assert!(success.passed);
        assert!(success.message.is_none());

        let success_msg = CheckResult::success_with_message(
            Duration::from_millis(50),
            "OK".to_string(),
        );
        assert!(success_msg.passed);
        assert_eq!(success_msg.message, Some("OK".to_string()));

        let failure = CheckResult::failure(Duration::from_millis(200), "Error".to_string());
        assert!(!failure.passed);
        assert_eq!(failure.message, Some("Error".to_string()));
    }

    #[test]
    fn test_check_history() {
        let mut history = CheckHistory::new(3);

        assert_eq!(history.consecutive_passes(), 0);
        assert_eq!(history.consecutive_failures(), 0);

        history.add(CheckResult::success(Duration::from_millis(100)));
        history.add(CheckResult::success(Duration::from_millis(100)));
        history.add(CheckResult::failure(Duration::from_millis(100)));

        assert_eq!(history.consecutive_passes(), 0);
        assert_eq!(history.consecutive_failures(), 1);
        assert_eq!(history.results().len(), 3);

        // Test capacity limit
        history.add(CheckResult::success(Duration::from_millis(100)));
        assert_eq!(history.results().len(), 3);
    }

    #[test]
    fn test_check_history_success_rate() {
        let mut history = CheckHistory::new(10);

        // Empty history
        assert_eq!(history.success_rate(), 0.0);

        // Add some results
        for _ in 0..7 {
            history.add(CheckResult::success(Duration::from_millis(100)));
        }
        for _ in 0..3 {
            history.add(CheckResult::failure(Duration::from_millis(100), "err".to_string()));
        }

        assert_eq!(history.success_rate(), 0.7);
    }

    #[test]
    fn test_health_check_execution() {
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let config = CheckConfig::liveness()
            .with_success_threshold(2)
            .with_failure_threshold(2);

        let check = HealthCheck::new(config, check_fn);

        // Initial status should be unknown
        assert_eq!(check.status(), HealthStatus::Unknown);

        // First successful check
        let result = check.execute();
        assert!(result.passed);
        assert_eq!(check.consecutive_successes(), 1);
        assert_eq!(check.consecutive_failures(), 0);
        // Still unknown because threshold is 2
        assert_eq!(check.status(), HealthStatus::Unknown);

        // Second successful check
        check.execute();
        assert_eq!(check.consecutive_successes(), 2);
        // Now healthy
        assert_eq!(check.status(), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_check_failures() {
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| {
            Err("Failed".to_string())
        });
        let config = CheckConfig::liveness()
            .with_failure_threshold(2);

        let check = HealthCheck::new(config, check_fn);

        // Two failures
        check.execute();
        check.execute();

        assert_eq!(check.consecutive_failures(), 2);
        assert_eq!(check.status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_check_reset() {
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let config = CheckConfig::liveness();
        let check = HealthCheck::new(config, check_fn);

        check.execute();
        assert_eq!(check.total_checks(), 1);

        check.reset();
        assert_eq!(check.total_checks(), 0);
        assert_eq!(check.status(), HealthStatus::Unknown);
        assert_eq!(check.consecutive_successes(), 0);
        assert!(check.last_check().is_none());
    }

    #[test]
    fn test_health_check_manually_set_status() {
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let config = CheckConfig::liveness();
        let check = HealthCheck::new(config, check_fn);

        check.set_status(HealthStatus::Healthy);
        assert_eq!(check.status(), HealthStatus::Healthy);

        check.set_status(HealthStatus::Unhealthy);
        assert_eq!(check.status(), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_health_check_history() {
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let config = CheckConfig::liveness();
        let check = HealthCheck::new(config, check_fn);

        check.execute();
        check.execute();

        let history = check.history();
        assert_eq!(history.len(), 2);

        let recent = check.recent_results(1);
        assert_eq!(recent.len(), 1);
    }
}
