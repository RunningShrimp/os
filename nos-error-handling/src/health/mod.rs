//! System health monitoring
//! 
//! This module provides system health monitoring functionality.
//! 
//! DEPRECATED: Implementation should be in kernel/src/error, not here

use crate::Error;
use crate::Result;
use spin::Mutex;
extern crate alloc;

use nos_api::collections::BTreeMap;
use alloc::string::ToString;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

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
        self.thresholds.insert(threshold.metric_name.clone(), threshold);
    }

    /// Update a metric value
    pub fn update_metric(&mut self, name: &str, value: f64) -> Result<()> {
        let metric = self.metrics.get_mut(name)
            .ok_or_else(|| Error::NotFound(format!("Metric {} not found", name)))?;
        
        metric.current_value = value;
        metric.last_updated = crate::common::get_timestamp();
        
        // Check thresholds
        self.check_thresholds(name, value)?;
        
        // Update health status
        self.update_health_status();
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_updates += 1;
        
        Ok(())
    }

    /// Get current health status
    pub fn get_current_status(&self) -> HealthStatus {
        self.status.lock().clone()
    }

    /// Get health metrics
    pub fn get_metrics(&self) -> Vec<HealthMetric> {
        self.metrics.values().cloned().collect()
    }

    /// Get health statistics
    pub fn get_stats(&self) -> HealthStats {
        self.stats.lock().clone()
    }

    /// Check thresholds
    fn check_thresholds(&self, name: &str, value: f64) -> Result<()> {
        let threshold = self.thresholds.get(name);
        
        if let Some(threshold) = threshold {
            if value < threshold.min_value || value > threshold.max_value {
                // Threshold violation
                let mut status = self.status.lock();
                status.overall_health = HealthLevel::Degraded;
                let alert_id = status.alerts.len() as u64;
                status.alerts.push(HealthAlert {
                    id: alert_id,
                    metric_name: name.to_string(),
                    threshold_name: threshold.name.clone(),
                    current_value: value,
                    threshold_value: if value < threshold.min_value { threshold.min_value } else { threshold.max_value },
                    severity: threshold.severity,
                    timestamp: crate::common::get_timestamp(),
                    message: format!("Metric {} value {} is outside threshold range [{}, {}]", 
                                   name, value, threshold.min_value, threshold.max_value),
                });
            }
        }
        
        Ok(())
    }

    /// Update health status
    fn update_health_status(&self) {
        let mut status = self.status.lock();
        
        // Calculate overall health based on metrics
        let mut _healthy_count = 0;
        let mut degraded_count = 0;
        let mut critical_count = 0;
        
        for (name, metric) in self.metrics.iter() {
            if let Some(threshold) = self.thresholds.get(name) {
                if metric.current_value < threshold.min_value || metric.current_value > threshold.max_value {
                    if threshold.severity == HealthSeverity::Critical {
                        critical_count += 1;
                    } else {
                        degraded_count += 1;
                    }
                } else {
                    _healthy_count += 1;
                }
            } else {
                _healthy_count += 1;
            }
        }
        
        // Update overall health
        status.overall_health = if critical_count > 0 {
            HealthLevel::Critical
        } else if degraded_count > 0 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Healthy
        };
        
        // Update component health
        // Note: clear() not available in no-alloc BTreeMap, so we'll rebuild it
        // For now, we'll just iterate through metrics and set values
        for (name, metric) in self.metrics.iter() {
            let health = if let Some(threshold) = self.thresholds.get(name) {
                if metric.current_value < threshold.min_value || metric.current_value > threshold.max_value {
                    if threshold.severity == HealthSeverity::Critical {
                        HealthLevel::Critical
                    } else {
                        HealthLevel::Degraded
                    }
                } else {
                    HealthLevel::Healthy
                }
            } else {
                HealthLevel::Healthy
            };
            
            status.component_health.insert(name.clone(), health);
        }
        
        status.last_updated = crate::common::get_timestamp();
    }

    /// Initialize monitor
    pub fn init(&mut self) -> Result<()> {
        // Add default metrics and thresholds
        self.add_default_metrics();
        self.add_default_thresholds();
        Ok(())
    }

    /// Shutdown monitor
    pub fn shutdown(&mut self) -> Result<()> {
        // TODO: Shutdown monitor
        Ok(())
    }

    /// Add default metrics
    fn add_default_metrics(&mut self) {
        // CPU usage metric
        self.add_metric(HealthMetric {
            name: "cpu_usage".to_string(),
            description: "CPU usage percentage".to_string(),
            unit: "%".to_string(),
            current_value: 0.0,
            min_value: 0.0,
            max_value: 100.0,
            last_updated: crate::common::get_timestamp(),
        });
        
        // Memory usage metric
        self.add_metric(HealthMetric {
            name: "memory_usage".to_string(),
            description: "Memory usage percentage".to_string(),
            unit: "%".to_string(),
            current_value: 0.0,
            min_value: 0.0,
            max_value: 100.0,
            last_updated: crate::common::get_timestamp(),
        });
        
        // Disk usage metric
        self.add_metric(HealthMetric {
            name: "disk_usage".to_string(),
            description: "Disk usage percentage".to_string(),
            unit: "%".to_string(),
            current_value: 0.0,
            min_value: 0.0,
            max_value: 100.0,
            last_updated: crate::common::get_timestamp(),
        });
        
        // Network latency metric
        self.add_metric(HealthMetric {
            name: "network_latency".to_string(),
            description: "Network latency in milliseconds".to_string(),
            unit: "ms".to_string(),
            current_value: 0.0,
            min_value: 0.0,
            max_value: 1000.0,
            last_updated: crate::common::get_timestamp(),
        });
    }

    /// Add default thresholds
    fn add_default_thresholds(&mut self) {
        // CPU usage threshold
        self.add_threshold(HealthThreshold {
            name: "cpu_usage_threshold".to_string(),
            metric_name: "cpu_usage".to_string(),
            min_value: 0.0,
            max_value: 80.0,
            severity: HealthSeverity::Warning,
        });
        
        // Memory usage threshold
        self.add_threshold(HealthThreshold {
            name: "memory_usage_threshold".to_string(),
            metric_name: "memory_usage".to_string(),
            min_value: 0.0,
            max_value: 90.0,
            severity: HealthSeverity::Warning,
        });
        
        // Disk usage threshold
        self.add_threshold(HealthThreshold {
            name: "disk_usage_threshold".to_string(),
            metric_name: "disk_usage".to_string(),
            min_value: 0.0,
            max_value: 85.0,
            severity: HealthSeverity::Warning,
        });
        
        // Network latency threshold
        self.add_threshold(HealthThreshold {
            name: "network_latency_threshold".to_string(),
            metric_name: "network_latency".to_string(),
            min_value: 0.0,
            max_value: 100.0,
            severity: HealthSeverity::Warning,
        });
    }
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
    /// Threshold severity
    pub severity: HealthSeverity,
}

/// Health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall health level
    pub overall_health: HealthLevel,
    /// Component health
    pub component_health: BTreeMap<String, HealthLevel>,
    /// Health alerts
    pub alerts: Vec<HealthAlert>,
    /// Last updated timestamp
    pub last_updated: u64,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            overall_health: HealthLevel::Healthy,
            component_health: BTreeMap::new(),
            alerts: Vec::new(),
            last_updated: crate::common::get_timestamp(),
        }
    }
}

/// Health level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(Default)]
pub enum HealthLevel {
    /// Healthy
    #[default]
    Healthy = 0,
    /// Degraded
    Degraded = 1,
    /// Critical
    Critical = 2,
    /// Unknown
    Unknown = 3,
}

/// Health severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Critical
    Critical,
}

/// Health alert
#[derive(Debug, Clone)]
pub struct HealthAlert {
    /// Alert ID
    pub id: u64,
    /// Metric name
    pub metric_name: String,
    /// Threshold name
    pub threshold_name: String,
    /// Current value
    pub current_value: f64,
    /// Threshold value
    pub threshold_value: f64,
    /// Alert severity
    pub severity: HealthSeverity,
    /// Alert timestamp
    pub timestamp: u64,
    /// Alert message
    pub message: String,
}

/// Health statistics
#[derive(Debug, Clone, Default)]
pub struct HealthStats {
    /// Total updates
    pub total_updates: u64,
    /// Total alerts
    pub total_alerts: u64,
    /// Alerts by severity
    pub alerts_by_severity: BTreeMap<HealthSeverity, u64>,
    /// Average update time (microseconds)
    pub avg_update_time: u64,
}

/// Global health monitor
static GLOBAL_MONITOR: spin::Once<Mutex<HealthMonitor>> = spin::Once::new();

/// Initialize global health monitor
pub fn init_monitor() -> Result<()> {
    GLOBAL_MONITOR.call_once(|| {
        Mutex::new(HealthMonitor::new())
    });
    
    // Initialize monitor
    GLOBAL_MONITOR.get().unwrap().lock().init()
}

/// Get the global health monitor
pub fn get_monitor() -> &'static Mutex<HealthMonitor> {
    GLOBAL_MONITOR.get().expect("Health monitor not initialized")
}

/// Internal function to get the global health monitor
fn get_monitor_internal() -> &'static Mutex<HealthMonitor> {
    get_monitor()
}

/// Shutdown the global health monitor
pub fn shutdown_monitor() -> Result<()> {
    // Note: spin::Once doesn't provide a way to reset, so we just return Ok(())
    // In a real implementation, you might want to provide a different approach
    Ok(())
}

/// Update a metric value
pub fn update_metric(name: &str, value: f64) -> Result<()> {
    let mut monitor = get_monitor_internal().lock();
    monitor.update_metric(name, value)
}

/// Get current health status
pub fn get_current_status() -> HealthStatus {
    get_monitor_internal().lock().get_current_status()
}

/// Get health statistics
pub fn health_get_stats() -> HealthStats {
    get_monitor_internal().lock().get_stats()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_health_monitor() {
        let mut monitor = HealthMonitor::new();
        
        // Add a test metric
        let metric = HealthMetric {
            name: "test_metric".to_string(),
            description: "Test metric".to_string(),
            unit: "test".to_string(),
            current_value: 0.0,
            min_value: 0.0,
            max_value: 100.0,
            last_updated: crate::common::get_timestamp(),
        };
        monitor.add_metric(metric);
        
        // Add a test threshold
        let threshold = HealthThreshold {
            name: "test_threshold".to_string(),
            metric_name: "test_metric".to_string(),
            min_value: 0.0,
            max_value: 80.0,
            severity: HealthSeverity::Warning,
        };
        monitor.add_threshold(threshold);
        
        // Update metric
        assert!(monitor.update_metric("test_metric", 50.0).is_ok());
        
        // Check health status
        let status = monitor.get_current_status();
        assert_eq!(status.overall_health, HealthLevel::Healthy);
    }

    #[test]
    fn test_health_level() {
        assert!(HealthLevel::Healthy < HealthLevel::Degraded);
        assert!(HealthLevel::Degraded < HealthLevel::Critical);
        assert!(HealthLevel::Critical < HealthLevel::Unknown);
        
        assert_eq!(HealthLevel::default(), HealthLevel::Healthy);
    }

    #[test]
    fn test_health_stats() {
        let stats = HealthStats::default();
        assert_eq!(stats.total_updates, 0);
        assert_eq!(stats.total_alerts, 0);
        assert_eq!(stats.avg_update_time, 0);
        assert!(stats.alerts_by_severity.is_empty());
    }
}