//! # Real-Time Performance Metrics
//!
//! This module provides comprehensive performance monitoring and measurement
//! for real-time systems, including latency tracking, jitter measurement, and
//! deadline analysis.
//!
//! ## Overview
//!
//! Real-time systems require continuous monitoring of timing characteristics
//! to ensure guarantees are being met. This module tracks:
//!
//! - **Wakeup latency**: Time from timer expiration to task execution
//! - **Scheduling latency**: Time from schedule() call to task running
//! - **Interrupt latency**: Time from IRQ assertion to handler execution
//! - **Context switch time**: Time to switch from one task to another
//! - **Jitter**: Variation in timing measurements
//! - **Deadline misses**: Tasks that miss their deadlines
//!
//! ## Metric Types
//!
//! ### Latency Metrics
//!
//! Measure timing delays in the system:
//!
//! ```text
//! Timer ──► Scheduler ──► Task Running
//!   │           │              │
//!   └───────────┴──────────────┘
//!    Wakeup Latency + Scheduling Latency = Total Latency
//! ```
//!
//! ### WCET Estimation
//!
//! Track worst-case execution time for tasks:
//!
//! ```text
//! Execution Times: [100, 105, 98, 110, 102, 120, ... μs]
//!                                         ↓
//!                                    WCET = 120 μs
//! ```
//!
//! ### Jitter Measurement
//!
//! Measure timing variability:
//!
//! ```text
//! Jitter = max(latency) - min(latency)
//!         or
//! Jitter = std(latency) * 3 (3σ rule)
//! ```
//!
//! ## Usage
//!
//! ### Measuring Latency
//!
//! ```no_run
//! use kernel::rtos::metrics::MetricsCollector;
//!
//! let collector = MetricsCollector::new();
//!
//! // Start measurement
//! let start = collector.measure_wakeup_latency(42)?;
//! // ... task runs ...
//! let latency = collector.end_measurement(start)?;
//!
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```
//!
//! ### Getting Statistics
//!
//! ```no_run
//! use kernel::rtos::metrics::MetricsCollector;
//!
//! let collector = MetricsCollector::new();
//! let stats = collector.get_task_stats(42)?;
//! println!("WCET: {} μs", stats.wcet_us);
//! println!("Jitter: {} μs", stats.jitter_us);
//!
//! # Ok::<(), kernel::rtos::RtError>(())
//! ```

use crate::rtos::RtError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering as AtomicOrdering};

/// Read Time-Stamp Counter (stub for ARM64)
/// On ARM64, use the system counter instead of x86 RDTSC
#[inline]
unsafe fn rdtsc() -> u64 {
    // ARM64 system counter (CNTVCT_EL0)
    // For now, return a monotonic value from arch-specific timer
    // GH-#1262: Implement proper ARM64 cycle counter reading
    // See: https://github.com/npos/kernel/issues/1262
    core::sync::atomic::AtomicU64::new(0).load(core::sync::atomic::Ordering::Relaxed)
}

/// Metrics collection token
///
/// Returned when starting a measurement, used to complete the measurement.
#[derive(Debug, Clone, Copy)]
pub struct MeasurementToken {
    /// Task ID
    pub task_id: u64,

    /// Start time (nanoseconds)
    pub start_ns: u64,

    /// Measurement type
    pub measurement_type: MeasurementType,
}

/// Types of measurements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementType {
    /// Wakeup latency
    Wakeup,

    /// Scheduling latency
    Scheduling,

    /// Interrupt latency
    Interrupt,

    /// Context switch time
    ContextSwitch,

    /// Task execution time
    Execution,
}

/// Task performance statistics
#[derive(Debug, Clone)]
pub struct TaskMetrics {
    /// Task ID
    pub task_id: u64,

    /// Worst-case execution time (microseconds)
    pub wcet_us: u64,

    /// Average execution time (microseconds)
    pub avg_execution_us: f64,

    /// Jitter (microseconds)
    pub jitter_us: u64,

    /// Deadline misses
    pub deadline_misses: u32,

    /// Total activations
    pub total_activations: u64,

    /// Maximum wakeup latency (nanoseconds)
    pub max_wakeup_latency_ns: u64,

    /// Average wakeup latency (nanoseconds)
    pub avg_wakeup_latency_ns: f64,

    /// Maximum scheduling latency (nanoseconds)
    pub max_scheduling_latency_ns: u64,

    /// Context switches
    pub context_switches: u64,
}

/// System-wide performance statistics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    /// Total context switches
    pub total_context_switches: u64,

    /// Total interrupt count
    pub total_interrupts: u64,

    /// Maximum interrupt latency (nanoseconds)
    pub max_interrupt_latency_ns: u64,

    /// Average interrupt latency (nanoseconds)
    pub avg_interrupt_latency_ns: f64,

    /// CPU utilization (0.0 - 1.0)
    pub cpu_utilization: f64,

    /// Total deadline misses
    pub total_deadline_misses: u64,

    /// System uptime (nanoseconds)
    pub uptime_ns: u64,
}

/// Metrics collector for real-time performance
pub struct MetricsCollector {
    /// Per-task metrics
    task_metrics: spin::Mutex<BTreeMap<u64, TaskMetricsData>>,

    /// System metrics
    system_metrics: spin::Mutex<SystemMetricsData>,

    /// Collection enabled
    enabled: AtomicBool,

    /// Start time
    start_time_ns: AtomicU64,
}

/// Internal task metrics storage
#[derive(Debug)]
struct TaskMetricsData {
    task_id: u64,

    /// Execution times (microseconds)
    execution_times: Vec<u64>,

    /// WCET (microseconds)
    wcet_us: AtomicU64,

    /// Deadline misses
    deadline_misses: AtomicU32,

    /// Total activations
    total_activations: AtomicU64,

    /// Wakeup latencies (nanoseconds)
    wakeup_latencies: Vec<u64>,

    /// Maximum wakeup latency
    max_wakeup_latency_ns: AtomicU64,

    /// Scheduling latencies (nanoseconds)
    scheduling_latencies: Vec<u64>,

    /// Maximum scheduling latency
    max_scheduling_latency_ns: AtomicU64,

    /// Context switches
    context_switches: AtomicU64,
}

/// Internal system metrics storage
#[derive(Debug, Default)]
struct SystemMetricsData {
    /// Total context switches
    total_context_switches: AtomicU64,

    /// Total interrupts
    total_interrupts: AtomicU64,

    /// Interrupt latencies (nanoseconds)
    interrupt_latencies: Vec<u64>,

    /// Maximum interrupt latency
    max_interrupt_latency_ns: AtomicU64,

    /// Deadline misses
    total_deadline_misses: AtomicU64,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            task_metrics: spin::Mutex::new(BTreeMap::new()),
            system_metrics: spin::Mutex::new(SystemMetricsData::default()),
            enabled: AtomicBool::new(true),
            start_time_ns: AtomicU64::new(Self::now_ns()),
        }
    }

    /// Register a task for metrics collection
    pub fn register_task(&self, task_id: u64) -> Result<(), RtError> {
        let mut metrics = self.task_metrics.lock();

        if metrics.contains_key(&task_id) {
            return Err(RtError::AlreadyExists {
                resource_type: "task metrics",
                id: task_id,
            });
        }

        metrics.insert(task_id, TaskMetricsData {
            task_id,
            execution_times: Vec::new(),
            wcet_us: AtomicU64::new(0),
            deadline_misses: AtomicU32::new(0),
            total_activations: AtomicU64::new(0),
            wakeup_latencies: Vec::new(),
            max_wakeup_latency_ns: AtomicU64::new(0),
            scheduling_latencies: Vec::new(),
            max_scheduling_latency_ns: AtomicU64::new(0),
            context_switches: AtomicU64::new(0),
        });

        Ok(())
    }

    /// Unregister a task
    pub fn unregister_task(&self, task_id: u64) -> Result<(), RtError> {
        let mut metrics = self.task_metrics.lock();

        metrics.remove(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task metrics",
                id: task_id,
            })?;

        Ok(())
    }

    /// Start measuring wakeup latency
    pub fn measure_wakeup_latency(&self, task_id: u64) -> Result<MeasurementToken, RtError> {
        if !self.enabled.load(AtomicOrdering::Acquire) {
            return Err(RtError::InvalidState {
                state: "disabled",
                expected: "enabled",
            });
        }

        Ok(MeasurementToken {
            task_id,
            start_ns: Self::now_ns(),
            measurement_type: MeasurementType::Wakeup,
        })
    }

    /// Record task activation
    pub fn record_activation(&self, task_id: u64) -> Result<(), RtError> {
        let metrics = self.task_metrics.lock();

        let data = metrics.get(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task metrics",
                id: task_id,
            })?;

        data.total_activations.fetch_add(1, AtomicOrdering::Relaxed);

        Ok(())
    }

    /// End measurement and record latency
    pub fn end_measurement(&self, token: MeasurementToken) -> Result<u64, RtError> {
        if !self.enabled.load(AtomicOrdering::Acquire) {
            return Err(RtError::InvalidState {
                state: "disabled",
                expected: "enabled",
            });
        }

        let end_ns = Self::now_ns();
        let latency_ns = end_ns.saturating_sub(token.start_ns);

        let mut metrics = self.task_metrics.lock();

        let data = metrics.get_mut(&token.task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task metrics",
                id: token.task_id,
            })?;

        match token.measurement_type {
            MeasurementType::Wakeup => {
                data.wakeup_latencies.push(latency_ns);
                Self::update_max(&data.max_wakeup_latency_ns, latency_ns);
            }
            MeasurementType::Scheduling => {
                data.scheduling_latencies.push(latency_ns);
                Self::update_max(&data.max_scheduling_latency_ns, latency_ns);
            }
            MeasurementType::Execution => {
                let execution_us = latency_ns / 1000;
                data.execution_times.push(execution_us);
                Self::update_max(&data.wcet_us, execution_us);
            }
            _ => {}
        }

        Ok(latency_ns)
    }

    /// Record task execution time
    pub fn record_execution(&self, task_id: u64, duration_us: u64) -> Result<(), RtError> {
        let mut metrics = self.task_metrics.lock();

        let data = metrics.get_mut(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task metrics",
                id: task_id,
            })?;

        data.execution_times.push(duration_us);
        Self::update_max(&data.wcet_us, duration_us);

        Ok(())
    }

    /// Record a deadline miss
    pub fn record_deadline_miss(&self, task_id: u64) -> Result<(), RtError> {
        let metrics = self.task_metrics.lock();

        let data = metrics.get(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task metrics",
                id: task_id,
            })?;

        data.deadline_misses.fetch_add(1, AtomicOrdering::Relaxed);

        // Also update system counter
        let system = self.system_metrics.lock();
        system.total_deadline_misses.fetch_add(1, AtomicOrdering::Relaxed);

        Ok(())
    }

    /// Record context switch
    pub fn record_context_switch(&self, from_task: u64, to_task: u64) -> Result<(), RtError> {
        let metrics = self.task_metrics.lock();

        if let Some(data) = metrics.get(&from_task) {
            data.context_switches.fetch_add(1, AtomicOrdering::Relaxed);
        }

        if let Some(data) = metrics.get(&to_task) {
            data.context_switches.fetch_add(1, AtomicOrdering::Relaxed);
        }

        let system = self.system_metrics.lock();
        system.total_context_switches.fetch_add(1, AtomicOrdering::Relaxed);

        Ok(())
    }

    /// Record interrupt latency
    pub fn record_interrupt_latency(&self, latency_ns: u64) -> Result<(), RtError> {
        let mut system = self.system_metrics.lock();

        system.interrupt_latencies.push(latency_ns);
        system.total_interrupts.fetch_add(1, AtomicOrdering::Relaxed);
        Self::update_max(&system.max_interrupt_latency_ns, latency_ns);

        Ok(())
    }

    /// Get task statistics
    pub fn get_task_stats(&self, task_id: u64) -> Result<TaskMetrics, RtError> {
        let metrics = self.task_metrics.lock();

        let data = metrics.get(&task_id)
            .ok_or_else(|| RtError::NotFound {
                resource_type: "task metrics",
                id: task_id,
            })?;

        let wcet_us = data.wcet_us.load(AtomicOrdering::Relaxed);

        let avg_execution_us = if data.execution_times.is_empty() {
            0.0
        } else {
            let sum: u64 = data.execution_times.iter().sum();
            sum as f64 / data.execution_times.len() as f64
        };

        let jitter_us = if data.execution_times.len() < 2 {
            0
        } else {
            let max = *data.execution_times.iter().max().unwrap();
            let min = *data.execution_times.iter().min().unwrap();
            max - min
        };

        let avg_wakeup_latency_ns = if data.wakeup_latencies.is_empty() {
            0.0
        } else {
            let sum: u64 = data.wakeup_latencies.iter().sum();
            sum as f64 / data.wakeup_latencies.len() as f64
        };

        Ok(TaskMetrics {
            task_id: data.task_id,
            wcet_us,
            avg_execution_us,
            jitter_us,
            deadline_misses: data.deadline_misses.load(AtomicOrdering::Relaxed),
            total_activations: data.total_activations.load(AtomicOrdering::Relaxed),
            max_wakeup_latency_ns: data.max_wakeup_latency_ns.load(AtomicOrdering::Relaxed),
            avg_wakeup_latency_ns,
            max_scheduling_latency_ns: data.max_scheduling_latency_ns.load(AtomicOrdering::Relaxed),
            context_switches: data.context_switches.load(AtomicOrdering::Relaxed),
        })
    }

    /// Get system statistics
    pub fn get_system_stats(&self) -> SystemMetrics {
        let system = self.system_metrics.lock();

        let total_interrupts = system.total_interrupts.load(AtomicOrdering::Relaxed);
        let total_context_switches = system.total_context_switches.load(AtomicOrdering::Relaxed);
        let total_deadline_misses = system.total_deadline_misses.load(AtomicOrdering::Relaxed);

        let avg_interrupt_latency_ns = if system.interrupt_latencies.is_empty() {
            0.0
        } else {
            let sum: u64 = system.interrupt_latencies.iter().sum();
            sum as f64 / system.interrupt_latencies.len() as f64
        };

        let uptime_ns = Self::now_ns().saturating_sub(self.start_time_ns.load(AtomicOrdering::Relaxed));

        SystemMetrics {
            total_context_switches,
            total_interrupts,
            max_interrupt_latency_ns: system.max_interrupt_latency_ns.load(AtomicOrdering::Relaxed),
            avg_interrupt_latency_ns,
            cpu_utilization: 0.0, // Would be calculated from scheduling data
            total_deadline_misses,
            uptime_ns,
        }
    }

    /// Enable metrics collection
    pub fn enable(&self) {
        self.enabled.store(true, AtomicOrdering::Release);
    }

    /// Disable metrics collection
    pub fn disable(&self) {
        self.enabled.store(false, AtomicOrdering::Release);
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.task_metrics.lock().clear();
        self.start_time_ns.store(Self::now_ns(), AtomicOrdering::Release);
    }

    /// Get current time in nanoseconds
    fn now_ns() -> u64 {
        unsafe { rdtsc() / 3 } // Approximate
    }

    /// Update atomic maximum
    fn update_max(atomic: &AtomicU64, value: u64) {
        let mut current = atomic.load(AtomicOrdering::Relaxed);
        loop {
            if value <= current {
                break;
            }
            match atomic.compare_exchange_weak(
                current,
                value,
                AtomicOrdering::Release,
                AtomicOrdering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance counter reader
///
/// Reads CPU performance counters for detailed metrics.
#[derive(Debug)]
pub struct PerformanceCounters {
    /// Counter for cycles
    cycles: AtomicU64,

    /// Counter for instructions
    instructions: AtomicU64,

    /// Counter for cache misses
    cache_misses: AtomicU64,

    /// Counter for branch mispredictions
    branch_mispredicts: AtomicU64,
}

impl PerformanceCounters {
    /// Create new performance counter reader
    pub fn new() -> Self {
        Self {
            cycles: AtomicU64::new(0),
            instructions: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            branch_mispredicts: AtomicU64::new(0),
        }
    }

    /// Read cycle counter
    pub fn read_cycles(&self) -> u64 {
        unsafe { rdtsc() }
    }

    /// Read instruction counter
    pub fn read_instructions(&self) -> u64 {
        // In real implementation, use CPUID or perf counters
        0
    }

    /// Calculate IPC (instructions per cycle)
    pub fn calculate_ipc(&self) -> f64 {
        let cycles = self.read_cycles();
        let instructions = self.read_instructions();

        if cycles == 0 {
            0.0
        } else {
            instructions as f64 / cycles as f64
        }
    }

    /// Calculate cache miss rate
    pub fn cache_miss_rate(&self) -> f64 {
        let misses = self.cache_misses.load(AtomicOrdering::Relaxed);
        let accesses = self.read_cycles(); // Approximate

        if accesses == 0 {
            0.0
        } else {
            misses as f64 / accesses as f64
        }
    }
}

impl Default for PerformanceCounters {
    fn default() -> Self {
        Self::new()
    }
}

/// Jitter tracker
///
/// Tracks timing variability for specific operations.
#[derive(Debug)]
pub struct JitterTracker {
    /// Measurement history
    measurements: spin::Mutex<Vec<u64>>,

    /// Maximum history size
    max_history: usize,

    /// Current jitter
    jitter_ns: AtomicU64,
}

impl JitterTracker {
    /// Create new jitter tracker
    pub fn new(max_history: usize) -> Self {
        Self {
            measurements: spin::Mutex::new(Vec::with_capacity(max_history)),
            max_history,
            jitter_ns: AtomicU64::new(0),
        }
    }

    /// Add a measurement
    pub fn add_measurement(&self, value_ns: u64) {
        let mut measurements = self.measurements.lock();

        measurements.push(value_ns);

        if measurements.len() > self.max_history {
            measurements.remove(0);
        }

        // Recalculate jitter
        if measurements.len() >= 2 {
            let min = *measurements.iter().min().unwrap();
            let max = *measurements.iter().max().unwrap();
            self.jitter_ns.store(max - min, AtomicOrdering::Release);
        }
    }

    /// Get current jitter
    pub fn jitter(&self) -> u64 {
        self.jitter_ns.load(AtomicOrdering::Acquire)
    }

    /// Reset tracker
    pub fn reset(&self) {
        self.measurements.lock().clear();
        self.jitter_ns.store(0, AtomicOrdering::Release);
    }
}

/// Latency histogram
///
/// Tracks distribution of latency values.
#[derive(Debug)]
pub struct LatencyHistogram {
    /// Buckets (ns boundary → count)
    buckets: spin::Mutex<BTreeMap<u64, u64>>,

    /// Total samples
    total_samples: AtomicU64,

    /// Bucket boundaries
    boundaries: Vec<u64>,
}

impl LatencyHistogram {
    /// Create new histogram
    pub fn new(boundaries: Vec<u64>) -> Self {
        let mut buckets = BTreeMap::new();
        for &boundary in &boundaries {
            buckets.insert(boundary, 0);
        }

        Self {
            buckets: spin::Mutex::new(buckets),
            total_samples: AtomicU64::new(0),
            boundaries,
        }
    }

    /// Record a latency value
    pub fn record(&self, latency_ns: u64) {
        let mut buckets = self.buckets.lock();

        for &boundary in self.boundaries.iter().rev() {
            if latency_ns >= boundary {
                *buckets.entry(boundary).or_insert(0) += 1;
                break;
            }
        }

        self.total_samples.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Get percentile
    pub fn percentile(&self, percentile: f64) -> Option<u64> {
        let buckets = self.buckets.lock();
        let total = self.total_samples.load(AtomicOrdering::Relaxed) as f64;

        if total == 0.0 {
            return None;
        }

        let target = total * percentile / 100.0;
        let mut count = 0.0;

        for (&boundary, &bucket_count) in buckets.iter() {
            count += bucket_count as f64;
            if count >= target {
                return Some(boundary);
            }
        }

        None
    }

    /// Get total samples
    pub fn total_samples(&self) -> u64 {
        self.total_samples.load(AtomicOrdering::Relaxed)
    }
}

/// Global metrics collector
pub static METRICS_COLLECTOR: spin::Once<MetricsCollector> = spin::Once::new();

/// Initialize the global metrics collector
pub fn init_metrics_collector() {
    METRICS_COLLECTOR.call_once(|| MetricsCollector::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(collector.enabled.load(AtomicOrdering::Acquire));
    }

    #[test]
    fn test_task_registration() {
        let collector = MetricsCollector::new();
        collector.register_task(1).unwrap();
        let stats = collector.get_task_stats(1).unwrap();
        assert_eq!(stats.task_id, 1);
    }

    #[test]
    fn test_wakeup_latency_measurement() {
        let collector = MetricsCollector::new();
        collector.register_task(1).unwrap();

        let start = collector.measure_wakeup_latency(1).unwrap();
        // Simulate some delay
        let _latency = collector.end_measurement(start).unwrap();
    }

    #[test]
    fn test_execution_recording() {
        let collector = MetricsCollector::new();
        collector.register_task(1).unwrap();

        collector.record_execution(1, 100).unwrap();
        collector.record_execution(1, 150).unwrap();
        collector.record_execution(1, 120).unwrap();

        let stats = collector.get_task_stats(1).unwrap();
        assert_eq!(stats.wcet_us, 150);
    }

    #[test]
    fn test_deadline_miss_recording() {
        let collector = MetricsCollector::new();
        collector.register_task(1).unwrap();

        collector.record_deadline_miss(1).unwrap();
        collector.record_deadline_miss(1).unwrap();

        let stats = collector.get_task_stats(1).unwrap();
        assert_eq!(stats.deadline_misses, 2);
    }

    #[test]
    fn test_context_switch_recording() {
        let collector = MetricsCollector::new();
        collector.register_task(1).unwrap();
        collector.register_task(2).unwrap();

        collector.record_context_switch(1, 2).unwrap();

        let system_stats = collector.get_system_stats();
        assert_eq!(system_stats.total_context_switches, 1);
    }

    #[test]
    fn test_interrupt_latency_recording() {
        let collector = MetricsCollector::new();

        collector.record_interrupt_latency(500).unwrap();
        collector.record_interrupt_latency(750).unwrap();
        collector.record_interrupt_latency(600).unwrap();

        let system_stats = collector.get_system_stats();
        assert_eq!(system_stats.total_interrupts, 3);
        assert_eq!(system_stats.max_interrupt_latency_ns, 750);
    }

    #[test]
    fn test_jitter_tracker() {
        let tracker = JitterTracker::new(100);

        tracker.add_measurement(1000);
        tracker.add_measurement(1050);
        tracker.add_measurement(980);

        let jitter = tracker.jitter();
        assert!(jitter > 0);
    }

    #[test]
    fn test_latency_histogram() {
        let histogram = LatencyHistogram::new(vec![100, 500, 1000, 5000]);

        histogram.record(250);
        histogram.record(750);
        histogram.record(2000);

        assert_eq!(histogram.total_samples(), 3);
    }

    #[test]
    fn test_performance_counters() {
        let counters = PerformanceCounters::new();
        let cycles = counters.read_cycles();
        assert!(cycles > 0);
    }

    #[test]
    fn test_metrics_disable_enable() {
        let collector = MetricsCollector::new();

        collector.disable();
        assert!(!collector.enabled.load(AtomicOrdering::Acquire));

        collector.enable();
        assert!(collector.enabled.load(AtomicOrdering::Acquire));
    }

    #[test]
    fn test_metrics_reset() {
        let collector = MetricsCollector::new();
        collector.register_task(1).unwrap();

        collector.record_execution(1, 100).unwrap();
        collector.reset();

        // After reset, task should be cleared
        let result = collector.get_task_stats(1);
        assert!(result.is_err());
    }
}
