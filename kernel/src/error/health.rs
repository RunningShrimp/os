//! Health Monitoring
//! 
//! This module provides health monitoring functionality for the kernel.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

/// Health level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HealthLevel {
    /// Healthy
    Healthy = 0,
    /// Degraded
    Degraded = 1,
    /// Critical
    Critical = 2,
    /// Unknown
    Unknown = 3,
}

/// Health severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HealthSeverity {
    /// Info
    Info = 0,
    /// Warning
    Warning = 1,
    /// Error
    Error = 2,
    /// Critical
    Critical = 3,
}

/// Health metric
#[derive(Debug, Clone)]
pub struct HealthMetric {
    /// Metric name
    pub name: String,
    /// Metric description
    pub description: String,
    /// Metric unit
    pub unit: String,
    /// Current value
    pub current_value: f64,
    /// Minimum value
    pub min_value: f64,
    /// Maximum value
    pub max_value: f64,
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Health threshold
#[derive(Debug, Clone)]
pub struct HealthThreshold {
    /// Threshold name
    pub name: String,
    /// Metric name
    pub metric_name: String,
    /// Minimum value
    pub min_value: f64,
    /// Maximum value
    pub max_value: f64,
    /// Severity
    pub severity: HealthSeverity,
}

/// Health status
#[derive(Debug, Clone, Default)]
pub struct HealthStatus {
    /// Overall health level
    pub overall_health: HealthLevel,
    /// Last checked timestamp
    pub last_checked: u64,
    /// Active alerts
    pub active_alerts: u32,
}

/// Health statistics
#[derive(Debug, Clone, Default)]
pub struct HealthStats {
    /// Total metrics checked
    pub total_metrics: u64,
    /// Total thresholds violated
    pub total_threshold_violations: u64,
    /// Last reset timestamp
    pub last_reset: u64,
}

/// Health monitor
#[derive(Default)]
pub struct HealthMonitor {
    /// Health metrics
    metrics: BTreeMap<String, HealthMetric>,
    /// Health thresholds
    thresholds: BTreeMap<String, HealthThreshold>,
    /// Health status
    status: Mutex<HealthStatus>,
    /// Health statistics
    stats: Mutex<HealthStats>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a health metric
    pub fn add_metric(&mut self, metric: HealthMetric) {
        self.metrics.insert(metric.name.clone(), metric);
    }

    /// Add a health threshold
    pub fn add_threshold(&mut self, threshold: HealthThreshold) {
        self.thresholds.insert(threshold.name.clone(), threshold);
    }

    /// Update a metric value
    pub fn update_metric(&mut self, name: &str, value: f64) -> crate::error::UnifiedResult<()> {
        let metric = self.metrics.get_mut(name)
            .ok_or_else(|| crate::error::create_error(
                crate::error::ErrorSeverity::Error,
                crate::error::ProcessError::NotFound,
                "Metric not found".to_string(),
            ))?;
        metric.current_value = value;
        metric.last_updated = crate::common::get_timestamp();
        Ok(())
    }

    /// Get current health status
    pub fn get_current_status(&self) -> HealthStatus {
        *self.status.lock()
    }

    /// Get health statistics
    pub fn get_stats(&self) -> HealthStats {
        *self.stats.lock()
    }
}

/// Global health monitor
static HEALTH_MONITOR: spin::Once<HealthMonitor> = spin::Once::new();

/// Initialize health monitor
pub fn init_health_monitor() -> crate::error::UnifiedResult<()> {
    HEALTH_MONITOR.call_once(|| HealthMonitor::new());
    crate::log_info!("Health monitor initialized");
    Ok(())
}

/// Get health monitor
pub fn get_health_monitor() -> &'static HealthMonitor {
    HEALTH_MONITOR.get().expect("Health monitor not initialized")
}
