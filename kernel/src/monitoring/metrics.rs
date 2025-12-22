//! System metrics collection
//!
//! Collects and tracks system metrics for production monitoring.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::subsystems::sync::Mutex;

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
        collector.register_counter("syscalls_success_total".to_string());
        collector.register_counter("syscalls_failed_total".to_string());
        collector.register_counter("syscalls_fast_path_total".to_string());
        collector.register_counter("syscalls_regular_total".to_string());
        collector.register_gauge("syscall_avg_latency_ns".to_string());
        collector.register_gauge("processes_running".to_string());
        collector.register_gauge("processes_total".to_string());
        collector.register_gauge("memory_used_bytes".to_string());
        collector.register_gauge("memory_free_bytes".to_string());
        collector.register_gauge("memory_total_bytes".to_string());
        collector.register_counter("context_switches_total".to_string());
        collector.register_counter("interrupts_total".to_string());
        collector.register_gauge("scheduler_runqueue_len_total".to_string());
        collector.register_gauge("scheduler_runqueue_len_max".to_string());
        collector.register_gauge("scheduler_runnable_total".to_string());
        collector.register_counter("locks_spin_acquire_total".to_string());
        collector.register_counter("locks_spin_contended_total".to_string());
        collector.register_counter("locks_mutex_acquire_total".to_string());
        collector.register_counter("locks_mutex_contended_total".to_string());
        
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
    
    /// Set counter to an absolute value
    pub fn set_counter(&self, name: &str, value: u64) {
        let mut metrics = self.metrics.lock();
        if let Some(metric) = metrics.get_mut(name) {
            if metric.metric_type == MetricType::Counter {
                metric.counter_value.store(value, Ordering::Relaxed);
            }
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
                    // Collect histogram summary (placeholder)
                    result.insert(format!("{}_histogram_count", name), 0);
                }
            }
        }
        
        result
    }
    
    /// Update system metrics (called periodically)
    pub fn update_system_metrics(&self) {
        // Update process count
        let proc_table = crate::subsystems::process::manager::PROC_TABLE.lock();
        let running_count = proc_table.iter()
            .filter(|p| p.state == crate::subsystems::process::manager::ProcState::Running)
            .count();
        let total_procs = proc_table.iter().count();
        drop(proc_table);
        self.set_gauge("processes_running", running_count as u64);
        self.set_gauge("processes_total", total_procs as u64);
        
        // Update memory metrics using physical stats
        let (free_pages, total_pages) = crate::subsystems::mm::phys::mem_stats();
        let page_size = 4096u64;
        let total_bytes = total_pages as u64 * page_size;
        let free_bytes = free_pages as u64 * page_size;
        let used_bytes = total_bytes.saturating_sub(free_bytes);
        self.set_gauge("memory_used_bytes", used_bytes);
        self.set_gauge("memory_free_bytes", free_bytes);
        self.set_gauge("memory_total_bytes", total_bytes);

        // Scheduler metrics (fast-path counters)
        if let Some(sched_stats) = crate::subsystems::scheduler::unified::get_scheduler_stats() {
            self.set_counter("context_switches_total", sched_stats.total_context_switches);
            self.set_gauge(
                "scheduler_runqueue_len_total",
                sched_stats.runqueue_len_total as u64,
            );
            self.set_gauge(
                "scheduler_runqueue_len_max",
                sched_stats.runqueue_len_max as u64,
            );
            self.set_gauge(
                "scheduler_runnable_total",
                sched_stats.runnable_threads as u64,
            );
        }

        // Lock analytics: sample global spinlock and selected mutex stats (best-effort)
        {
            // SpinLock analytics are per-lock; here we only expose aggregate of a representative lock if available.
            // For now we use the global scheduler spin locks via RawSpinLock interface (if any are exposed later
            // this can be extended).
            // As a placeholder, we read RawSpinLock::acquire_count/contended_count on a static instance if exposed.
        }
        
        // Update syscall metrics from unified dispatcher
        if let Some(dispatcher_mutex) = crate::subsystems::syscalls::dispatch::unified::get_unified_dispatcher() {
            let dispatcher = dispatcher_mutex.lock();
            if let Some(ref d) = *dispatcher {
                let stats = d.get_stats();
                let total = stats.total_dispatches.load(Ordering::Relaxed);
                let success = stats.successful_dispatches.load(Ordering::Relaxed);
                let failed = stats.failed_dispatches.load(Ordering::Relaxed);
                let fast = stats.fast_path_dispatches.load(Ordering::Relaxed);
                let regular = stats.regular_dispatches.load(Ordering::Relaxed);
                let total_time = stats.total_time_ns.load(Ordering::Relaxed);
                let avg_time = if total > 0 { total_time / total } else { 0 };
                
                self.set_counter("syscalls_total", total);
                self.set_counter("syscalls_success_total", success);
                self.set_counter("syscalls_failed_total", failed);
                self.set_counter("syscalls_fast_path_total", fast);
                self.set_counter("syscalls_regular_total", regular);
                self.set_gauge("syscall_avg_latency_ns", avg_time);
            }
        }
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
    static INIT_ONCE: crate::subsystems::sync::Once = crate::subsystems::sync::Once::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_metric_counter_and_gauge() {
        let counter = SystemMetric::new_counter("test_counter".to_string());
        assert_eq!(counter.get_counter(), 0);
        counter.increment(5);
        assert_eq!(counter.get_counter(), 5);

        let gauge = SystemMetric::new_gauge("test_gauge".to_string());
        assert_eq!(gauge.get_gauge(), 0);
        gauge.set_gauge(42);
        assert_eq!(gauge.get_gauge(), 42);
    }

    #[test]
    fn test_metrics_collector_register_and_collect() {
        let mut collector = MetricsCollector::new();
        collector.register_counter("test_counter".to_string());
        collector.register_gauge("test_gauge".to_string());

        collector.increment_counter("test_counter", 3);
        collector.set_gauge("test_gauge", 7);

        let metrics = collector.collect_metrics();
        assert_eq!(metrics.get("test_counter_counter").copied(), Some(3));
        assert_eq!(metrics.get("test_gauge_gauge").copied(), Some(7));
    }
}

