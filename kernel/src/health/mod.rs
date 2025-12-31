//! Health checking module for the NOS kernel.
//!
//! This module provides comprehensive health checking capabilities:
//! - Liveness, readiness, and startup probes
//! - Circuit breaker with half-open state
//! - Configurable thresholds and timeouts
//! - Health status aggregation
//! - Graceful degradation
//! - Kubernetes-compatible probe formats
//! - Multiple probe types (HTTP, TCP, exec)
//! - Component dependency tracking
//!
//! # Module Structure
//!
//! - [`check`]: Core health check implementation
//! - [`circuit`]: Circuit breaker pattern
//! - [`probe`]: Probe implementations (HTTP, TCP, exec)
//! - [`aggregate`]: Health aggregation strategies
//!
//! # Example
//!
//! ```rust
//! use kernel::health::{HealthCheck, CheckConfig, CheckType};
//! use alloc::sync::Arc;
//!
//! // Create a simple liveness check
//! let config = CheckConfig::liveness()
//!     .with_interval(Duration::from_secs(10))
//!     .with_timeout(Duration::from_secs(5));
//!
//! let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
//! let health_check = HealthCheck::new(config, check_fn);
//!
//! // Execute health check
//! let result = health_check.execute();
//! assert!(result.passed);
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::time::Duration;
use spin::{Mutex, RwLock};

use crate::subsystems::time::Timestamp;

pub mod check;
pub mod circuit;
pub mod probe;
pub mod aggregate;

// Re-exports for convenience
pub use check::{
    CheckConfig, CheckResult, CheckType, HealthCheck, HealthCheckBuilder, HealthCheckFn,
    HealthStatus,
};
pub use circuit::{
    CircuitBreaker, CircuitBreakerBuilder, CircuitConfig, CircuitState, CallResult,
};
pub use probe::{
    HealthProbe, HttpProbeConfig, ExecProbeConfig, TcpProbeConfig, ProbeConfig, ProbeResult,
    ProbeType, ProbeCache,
};
pub use aggregate::{
    HealthAggregator, AggregationStrategy, ComponentHealth, AggregationResult,
    DependencyGraph,
};

/// Health check manager - orchestrates all health checking
#[derive(Debug)]
pub struct HealthManager {
    /// Registered health checks
    health_checks: RwLock<BTreeMap<String, Arc<HealthCheck>>>,
    /// Registered circuit breakers
    circuit_breakers: RwLock<BTreeMap<String, Arc<CircuitBreaker>>>,
    /// Registered probes
    probes: RwLock<BTreeMap<String, Arc<HealthProbe>>>,
    /// Health aggregators
    aggregators: RwLock<BTreeMap<String, Arc<HealthAggregator>>>,
    /// Global health status
    global_status: Mutex<GlobalHealthStatus>,
    /// Metrics
    metrics: Mutex<HealthMetrics>,
}

/// Global health status
#[derive(Debug, Clone)]
pub struct GlobalHealthStatus {
    /// Overall system health
    pub overall_health: HealthStatus,
    /// Last update timestamp
    pub last_update: Timestamp,
    /// Component health summary
    pub component_summary: ComponentSummary,
}

/// Component health summary
#[derive(Debug, Clone)]
pub struct ComponentSummary {
    /// Total components
    pub total: usize,
    /// Healthy components
    pub healthy: usize,
    /// Unhealthy components
    pub unhealthy: usize,
    /// Unknown status components
    pub unknown: usize,
}

/// Health metrics
#[derive(Debug, Clone)]
pub struct HealthMetrics {
    /// Total health checks performed
    pub total_checks: u64,
    /// Total probe executions
    pub total_probes: u64,
    /// Total circuit breaker calls
    pub total_circuit_calls: u64,
    /// Total rejected calls (by circuit breakers)
    pub total_rejected: u64,
    /// Average check duration (nanoseconds)
    pub avg_check_duration_ns: u64,
}

impl Default for GlobalHealthStatus {
    fn default() -> Self {
        GlobalHealthStatus {
            overall_health: HealthStatus::Unknown,
            last_update: Timestamp::now(),
            component_summary: ComponentSummary {
                total: 0,
                healthy: 0,
                unhealthy: 0,
                unknown: 0,
            },
        }
    }
}

impl Default for HealthMetrics {
    fn default() -> Self {
        HealthMetrics {
            total_checks: 0,
            total_probes: 0,
            total_circuit_calls: 0,
            total_rejected: 0,
            avg_check_duration_ns: 0,
        }
    }
}

impl HealthManager {
    /// Create new health manager
    pub fn new() -> Self {
        HealthManager {
            health_checks: RwLock::new(BTreeMap::new()),
            circuit_breakers: RwLock::new(BTreeMap::new()),
            probes: RwLock::new(BTreeMap::new()),
            aggregators: RwLock::new(BTreeMap::new()),
            global_status: Mutex::new(GlobalHealthStatus::default()),
            metrics: Mutex::new(HealthMetrics::default()),
        }
    }

    /// Register a health check
    pub fn register_health_check(&self, name: String, check: Arc<HealthCheck>) {
        let mut checks = self.health_checks.write();
        checks.insert(name, check);
    }

    /// Unregister a health check
    pub fn unregister_health_check(&self, name: &str) -> bool {
        let mut checks = self.health_checks.write();
        checks.remove(name).is_some()
    }

    /// Get health check by name
    pub fn get_health_check(&self, name: &str) -> Option<Arc<HealthCheck>> {
        let checks = self.health_checks.read();
        checks.get(name).cloned()
    }

    /// Register a circuit breaker
    pub fn register_circuit_breaker(&self, name: String, breaker: Arc<CircuitBreaker>) {
        let mut breakers = self.circuit_breakers.write();
        breakers.insert(name, breaker);
    }

    /// Unregister a circuit breaker
    pub fn unregister_circuit_breaker(&self, name: &str) -> bool {
        let mut breakers = self.circuit_breakers.write();
        breakers.remove(name).is_some()
    }

    /// Get circuit breaker by name
    pub fn get_circuit_breaker(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        let breakers = self.circuit_breakers.read();
        breakers.get(name).cloned()
    }

    /// Register a probe
    pub fn register_probe(&self, name: String, probe: Arc<HealthProbe>) {
        let mut probes = self.probes.write();
        probes.insert(name, probe);
    }

    /// Unregister a probe
    pub fn unregister_probe(&self, name: &str) -> bool {
        let mut probes = self.probes.write();
        probes.remove(name).is_some()
    }

    /// Get probe by name
    pub fn get_probe(&self, name: &str) -> Option<Arc<HealthProbe>> {
        let probes = self.probes.read();
        probes.get(name).cloned()
    }

    /// Register a health aggregator
    pub fn register_aggregator(&self, name: String, aggregator: Arc<HealthAggregator>) {
        let mut aggregators = self.aggregators.write();
        aggregators.insert(name, aggregator);
    }

    /// Unregister an aggregator
    pub fn unregister_aggregator(&self, name: &str) -> bool {
        let mut aggregators = self.aggregators.write();
        aggregators.remove(name).is_some()
    }

    /// Get aggregator by name
    pub fn get_aggregator(&self, name: &str) -> Option<Arc<HealthAggregator>> {
        let aggregators = self.aggregators.read();
        aggregators.get(name).cloned()
    }

    /// Run all health checks
    pub fn run_all_checks(&self) -> Vec<(String, CheckResult)> {
        let checks = self.health_checks.read();
        let mut results = Vec::new();

        for (name, check) in checks.iter() {
            let result = check.execute();
            results.push((name.clone(), result));

            let mut metrics = self.metrics.lock();
            metrics.total_checks += 1;
        }

        self.update_global_status();
        results
    }

    /// Run all probes
    pub fn run_all_probes(&self) -> Vec<(String, ProbeResult)> {
        let probes = self.probes.read();
        let mut results = Vec::new();

        for (name, probe) in probes.iter() {
            let result = probe.execute();
            results.push((name.clone(), result));

            let mut metrics = self.metrics.lock();
            metrics.total_probes += 1;
        }

        results
    }

    /// Aggregate all health statuses
    pub fn aggregate_all(&self) -> BTreeMap<String, AggregationResult> {
        let aggregators = self.aggregators.read();
        let mut results = BTreeMap::new();

        for (name, aggregator) in aggregators.iter() {
            let result = aggregator.aggregate();
            results.insert(name.clone(), result);
        }

        results
    }

    /// Get global health status
    pub fn global_status(&self) -> GlobalHealthStatus {
        let status = self.global_status.lock();
        status.clone()
    }

    /// Update global health status
    fn update_global_status(&self) {
        let checks = self.health_checks.read();
        let mut healthy = 0;
        let mut unhealthy = 0;
        let mut unknown = 0;

        for (_name, check) in checks.iter() {
            match check.status() {
                HealthStatus::Healthy => healthy += 1,
                HealthStatus::Unhealthy => unhealthy += 1,
                HealthStatus::Unknown => unknown += 1,
            }
        }

        let total = healthy + unhealthy + unknown;
        let overall_health = if unhealthy > 0 {
            HealthStatus::Unhealthy
        } else if unknown > 0 {
            HealthStatus::Unknown
        } else {
            HealthStatus::Healthy
        };

        let mut status = self.global_status.lock();
        status.overall_health = overall_health;
        status.last_update = Timestamp::now();
        status.component_summary = ComponentSummary {
            total,
            healthy,
            unhealthy,
            unknown,
        };
    }

    /// Get health metrics
    pub fn metrics(&self) -> HealthMetrics {
        let metrics = self.metrics.lock();
        metrics.clone()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        let mut metrics = self.metrics.lock();
        *metrics = HealthMetrics::default();
    }

    /// Get health status report
    pub fn health_report(&self) -> HealthReport {
        let global_status = self.global_status();
        let metrics = self.metrics();

        let checks = self.health_checks.read();
        let check_statuses: BTreeMap<String, HealthStatus> = checks
            .iter()
            .map(|(name, check)| (name.clone(), check.status()))
            .collect();

        let breakers = self.circuit_breakers.read();
        let breaker_states: BTreeMap<String, CircuitState> = breakers
            .iter()
            .map(|(name, breaker)| (name.clone(), breaker.state()))
            .collect();

        HealthReport {
            timestamp: Timestamp::now(),
            global_status,
            check_statuses,
            breaker_states,
            metrics,
        }
    }
}

/// Health status report
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// Report timestamp
    pub timestamp: Timestamp,
    /// Global health status
    pub global_status: GlobalHealthStatus,
    /// Individual health check statuses
    pub check_statuses: BTreeMap<String, HealthStatus>,
    /// Circuit breaker states
    pub breaker_states: BTreeMap<String, CircuitState>,
    /// Health metrics
    pub metrics: HealthMetrics,
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Benchmark utilities for health checking
pub struct BenchmarkUtils;

impl BenchmarkUtils {
    /// Create a fast health check for benchmarking
    pub fn fast_check() -> HealthCheck {
        let config = CheckConfig::liveness()
            .with_interval(Duration::from_millis(10))
            .with_timeout(Duration::from_millis(5));

        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_nanos(100)));
        HealthCheck::new(config, check_fn)
    }

    /// Create a slow health check for testing timeouts
    pub fn slow_check(duration: Duration) -> HealthCheck {
        let config = CheckConfig::liveness()
            .with_interval(Duration::from_secs(10))
            .with_timeout(Duration::from_millis(100));

        let check_fn: Arc<HealthCheckFn> = Arc::new(move || {
            // Simulate slow check
            Ok(duration)
        });
        HealthCheck::new(config, check_fn)
    }

    /// Create a failing health check for testing
    pub fn failing_check() -> HealthCheck {
        let config = CheckConfig::liveness();
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| {
            Err("Simulated failure".to_string())
        });
        HealthCheck::new(config, check_fn)
    }

    /// Benchmark health check execution
    pub fn benchmark_checks(check: &HealthCheck, iterations: usize) -> Duration {
        let start = Timestamp::now();

        for _ in 0..iterations {
            check.execute();
        }

        Timestamp::now().duration_since(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_manager_creation() {
        let manager = HealthManager::new();
        assert_eq!(manager.health_checks.read().len(), 0);
        assert_eq!(manager.circuit_breakers.read().len(), 0);
        assert_eq!(manager.probes.read().len(), 0);
    }

    #[test]
    fn test_health_manager_register_checks() {
        let manager = HealthManager::new();

        let config = CheckConfig::liveness();
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let check = Arc::new(HealthCheck::new(config, check_fn));

        manager.register_health_check("test_check".to_string(), check.clone());

        assert!(manager.get_health_check("test_check").is_some());
        assert!(manager.get_health_check("nonexistent").is_none());
    }

    #[test]
    fn test_health_manager_unregister_checks() {
        let manager = HealthManager::new();

        let config = CheckConfig::liveness();
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let check = Arc::new(HealthCheck::new(config, check_fn));

        manager.register_health_check("test_check".to_string(), check);
        assert!(manager.unregister_health_check("test_check"));
        assert!(!manager.unregister_health_check("test_check"));
    }

    #[test]
    fn test_health_manager_run_all_checks() {
        let manager = HealthManager::new();

        // Register a few checks
        for i in 0..3 {
            let name = format!("check_{}", i);
            let config = CheckConfig::liveness();
            let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
            let check = Arc::new(HealthCheck::new(config, check_fn));
            manager.register_health_check(name, check);
        }

        let results = manager.run_all_checks();
        assert_eq!(results.len(), 3);

        let metrics = manager.metrics();
        assert_eq!(metrics.total_checks, 3);
    }

    #[test]
    fn test_health_manager_global_status() {
        let manager = HealthManager::new();

        let status = manager.global_status();
        assert_eq!(status.overall_health, HealthStatus::Unknown);

        // Add healthy check
        let config = CheckConfig::liveness();
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let check = Arc::new(HealthCheck::new(config, check_fn));
        manager.register_health_check("healthy".to_string(), check);

        manager.run_all_checks();

        let status = manager.global_status();
        assert_eq!(status.overall_health, HealthStatus::Healthy);
        assert_eq!(status.component_summary.healthy, 1);
    }

    #[test]
    fn test_health_manager_circuit_breakers() {
        let manager = HealthManager::new();

        let config = CircuitConfig::new(5);
        let breaker = Arc::new(CircuitBreaker::new(config));

        manager.register_circuit_breaker("test_breaker".to_string(), breaker);

        assert!(manager.get_circuit_breaker("test_breaker").is_some());
        assert!(manager.unregister_circuit_breaker("test_breaker"));
    }

    #[test]
    fn test_health_manager_probes() {
        let manager = HealthManager::new();

        let http_config = HttpProbeConfig::new("http://localhost/health".to_string());
        let probe_config = ProbeConfig::Http(http_config);
        let check_config = CheckConfig::default();
        let probe = Arc::new(HealthProbe::new(probe_config, check_config));

        manager.register_probe("test_probe".to_string(), probe);

        assert!(manager.get_probe("test_probe").is_some());
        assert!(manager.unregister_probe("test_probe"));
    }

    #[test]
    fn test_health_manager_aggregators() {
        let manager = HealthManager::new();

        let aggregator = Arc::new(HealthAggregator::new(AggregationStrategy::All));
        manager.register_aggregator("test_aggregator".to_string(), aggregator);

        assert!(manager.get_aggregator("test_aggregator").is_some());
        assert!(manager.unregister_aggregator("test_aggregator"));
    }

    #[test]
    fn test_health_manager_aggregate_all() {
        let manager = HealthManager::new();

        let aggregator = Arc::new(HealthAggregator::new(AggregationStrategy::All));
        aggregator.set_component(
            ComponentHealth::new("c1".to_string()).with_status(HealthStatus::Healthy),
        );

        manager.register_aggregator("test".to_string(), aggregator);

        let results = manager.aggregate_all();
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("test"));
    }

    #[test]
    fn test_health_manager_health_report() {
        let manager = HealthManager::new();

        let config = CheckConfig::liveness();
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let check = Arc::new(HealthCheck::new(config, check_fn));
        manager.register_health_check("test".to_string(), check);

        let report = manager.health_report();

        assert_eq!(report.check_statuses.len(), 1);
        assert!(report.check_statuses.contains_key("test"));
    }

    #[test]
    fn test_health_manager_metrics() {
        let manager = HealthManager::new();

        let config = CheckConfig::liveness();
        let check_fn: Arc<HealthCheckFn> = Arc::new(|| Ok(Duration::from_millis(100)));
        let check = Arc::new(HealthCheck::new(config, check_fn));
        manager.register_health_check("test".to_string(), check);

        manager.run_all_checks();

        let metrics = manager.metrics();
        assert_eq!(metrics.total_checks, 1);

        manager.reset_metrics();

        let metrics = manager.metrics();
        assert_eq!(metrics.total_checks, 0);
    }

    #[test]
    fn test_benchmark_utils_fast_check() {
        let check = BenchmarkUtils::fast_check();
        let result = check.execute();

        assert!(result.passed);
    }

    #[test]
    fn test_benchmark_utils_failing_check() {
        let check = BenchmarkUtils::failing_check();
        let result = check.execute();

        assert!(!result.passed);
    }

    #[test]
    fn test_benchmark_utils_benchmark() {
        let check = BenchmarkUtils::fast_check();
        let duration = BenchmarkUtils::benchmark_checks(&check, 100);

        // Should complete in reasonable time
        assert!(duration.as_secs() < 10);
    }

    #[test]
    fn test_health_report_structure() {
        let manager = HealthManager::new();
        let report = manager.health_report();

        // Report should have timestamp
        assert!(report.timestamp.duration_since(Timestamp::now()).as_secs() < 1);

        // Should have global status
        assert_eq!(report.global_status.overall_health, HealthStatus::Unknown);

        // Should have metrics
        assert_eq!(report.metrics.total_checks, 0);
    }

    #[test]
    fn test_component_summary() {
        let summary = ComponentSummary {
            total: 10,
            healthy: 7,
            unhealthy: 2,
            unknown: 1,
        };

        assert_eq!(summary.total, 10);
        assert_eq!(summary.healthy, 7);
        assert_eq!(summary.unhealthy, 2);
        assert_eq!(summary.unknown, 1);
    }
}
