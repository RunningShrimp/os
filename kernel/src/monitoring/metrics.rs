//! System metrics collection
//!
//! Collects and tracks system metrics for production monitoring.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::sync::Mutex;

/// System metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,    // Monotonically increasing counter
    Gauge,      // Value that can go up or down
    Histogram,  // Distribution of values
}

/// System metric
pub struct SystemMetric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Counter value (for Counter type)
    pub counter_value: AtomicU64,
    /// Gauge value (for Gauge type)
    pub gauge_value: AtomicU64,
    /// Histogram buckets (for Histogram type)
    pub histogram_buckets: Mutex<BTreeMap<u64, u64>>,
}

impl SystemMetric {
    /// Create a new counter metric
    pub fn new_counter(name: String) -> Self {
        Self {
            name,
            metric_type: MetricType::Counter,
            counter_value: AtomicU64::new(0),
            gauge_value: AtomicU64::new(0),
            histogram_buckets: Mutex::new(BTreeMap::new()),
        }
    }
    
    /// Create a new gauge metric
    pub fn new_gauge(name: String) -> Self {
        Self {
            name,
            metric_type: MetricType::Gauge,
            counter_value: AtomicU64::new(0),
            gauge_value: AtomicU64::new(0),
            histogram_buckets: Mutex::new(BTreeMap::new()),
        }
    }
    
    /// Increment counter
    pub fn increment(&self, value: u64) {
        if self.metric_type == MetricType::Counter {
            self.counter_value.fetch_add(value, Ordering::Relaxed);
        }
    }
    
    /// Set gauge value
    pub fn set_gauge(&self, value: u64) {
        if self.metric_type == MetricType::Gauge {
            self.gauge_value.store(value, Ordering::Relaxed);
        }
    }
    
    /// Record histogram value
    pub fn record_histogram(&self, value: u64) {
        if self.metric_type == MetricType::Histogram {
            let mut buckets = self.histogram_buckets.lock();
            *buckets.entry(value).or_insert(0) += 1;
        }
    }
    
    /// Get counter value
    pub fn get_counter(&self) -> u64 {
        self.counter_value.load(Ordering::Relaxed)
    }
    
    /// Get gauge value
    pub fn get_gauge(&self) -> u64 {
        self.gauge_value.load(Ordering::Relaxed)
    }
}

/// Metrics collector
pub struct MetricsCollector {
    /// Metrics by name
    metrics: Mutex<BTreeMap<String, SystemMetric>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let mut collector = Self {
            metrics: Mutex::new(BTreeMap::new()),
        };
        
        // Register default metrics
        collector.register_counter("syscalls_total".to_string());
        collector.register_counter("syscalls_errors_total".to_string());
        collector.register_gauge("processes_running".to_string());
        collector.register_gauge("memory_used_bytes".to_string());
        collector.register_gauge("memory_free_bytes".to_string());
        collector.register_counter("context_switches_total".to_string());
        collector.register_counter("interrupts_total".to_string());
        
        collector
    }
    
    /// Register a counter metric
    pub fn register_counter(&mut self, name: String) {
        let mut metrics = self.metrics.lock();
        metrics.insert(name.clone(), SystemMetric::new_counter(name));
    }
    
    /// Register a gauge metric
    pub fn register_gauge(&mut self, name: String) {
        let mut metrics = self.metrics.lock();
        metrics.insert(name.clone(), SystemMetric::new_gauge(name));
    }
    
    /// Get metric
    pub fn get_metric(&self, name: &str) -> Option<&SystemMetric> {
        let metrics = self.metrics.lock();
        // Return reference - in real implementation, would use Arc
        None // Placeholder
    }
    
    /// Increment counter
    pub fn increment_counter(&self, name: &str, value: u64) {
        let mut metrics = self.metrics.lock();
        if let Some(metric) = metrics.get_mut(name) {
            metric.increment(value);
        }
    }
    
    /// Set gauge
    pub fn set_gauge(&self, name: &str, value: u64) {
        let mut metrics = self.metrics.lock();
        if let Some(metric) = metrics.get_mut(name) {
            metric.set_gauge(value);
        }
    }
    
    /// Collect all metrics
    pub fn collect_metrics(&self) -> BTreeMap<String, u64> {
        let mut result = BTreeMap::new();
        let metrics = self.metrics.lock();
        
        for (name, metric) in metrics.iter() {
            match metric.metric_type {
                MetricType::Counter => {
                    result.insert(format!("{}_counter", name), metric.get_counter());
                }
                MetricType::Gauge => {
                    result.insert(format!("{}_gauge", name), metric.get_gauge());
                }
                MetricType::Histogram => {
                    // Collect histogram summary
                    result.insert(format!("{}_histogram_count", name), 0);
                }
            }
        }
        
        result
    }
    
    /// Update system metrics (called periodically)
    pub fn update_system_metrics(&self) {
        // Update process count
        let proc_table = crate::process::PROC_TABLE.lock();
        let running_count = proc_table.iter()
            .filter(|p| p.state == crate::process::ProcState::Running)
            .count();
        drop(proc_table);
        self.set_gauge("processes_running", running_count as u64);
        
        // Update memory metrics
        // In real implementation, would query memory manager
        self.set_gauge("memory_used_bytes", 0);
        self.set_gauge("memory_free_bytes", 0);
    }
}

/// Global metrics collector instance
static METRICS_COLLECTOR: Mutex<Option<MetricsCollector>> = Mutex::new(None);

/// Initialize metrics collector
pub fn init_metrics_collector() -> Result<(), i32> {
    let mut collector = METRICS_COLLECTOR.lock();
    if collector.is_none() {
        *collector = Some(MetricsCollector::new());
        crate::println!("[monitoring] Metrics collector initialized");
    }
    Ok(())
}

/// Get metrics collector
pub fn get_metrics_collector() -> &'static MetricsCollector {
    static INIT_ONCE: crate::sync::Once = crate::sync::Once::new();
    INIT_ONCE.call_once(|| {
        let mut collector = METRICS_COLLECTOR.lock();
        if collector.is_none() {
            *collector = Some(MetricsCollector::new());
        }
    });
    
    unsafe {
        &*(METRICS_COLLECTOR.lock().as_ref().unwrap() as *const MetricsCollector)
    }
}

