//! Performance Profiling - Boot Time Analysis and Profiling
//!
//! Provides performance profiling capabilities:
//! - Timing measurements
//! - Bottleneck detection
//! - Performance reports
//! - Optimization suggestions

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

/// Performance metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Duration,
    Throughput,
    Latency,
    Utilization,
}

impl fmt::Display for MetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetricType::Duration => write!(f, "Duration"),
            MetricType::Throughput => write!(f, "Throughput"),
            MetricType::Latency => write!(f, "Latency"),
            MetricType::Utilization => write!(f, "Utilization"),
        }
    }
}

/// Performance metric
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    pub name: String,
    pub metric_type: MetricType,
    pub value: u64,
    pub unit: String,
    pub timestamp: u64,
}

impl PerformanceMetric {
    /// Create new metric
    pub fn new(name: &str, metric_type: MetricType, unit: &str) -> Self {
        PerformanceMetric {
            name: String::from(name),
            metric_type,
            value: 0,
            unit: String::from(unit),
            timestamp: 0,
        }
    }

    /// Set value
    pub fn set_value(&mut self, value: u64) {
        self.value = value;
    }

    /// Set timestamp
    pub fn set_timestamp(&mut self, ts: u64) {
        self.timestamp = ts;
    }
}

impl fmt::Display for PerformanceMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {}", self.name, self.value, self.unit
        )
    }
}

/// Bottleneck analysis
#[derive(Debug, Clone)]
pub struct Bottleneck {
    pub name: String,
    pub duration: u64,
    pub percentage: u32,
    pub severity: u8,
}

impl Bottleneck {
    /// Create new bottleneck
    pub fn new(name: &str, duration: u64) -> Self {
        Bottleneck {
            name: String::from(name),
            duration,
            percentage: 0,
            severity: 0,
        }
    }

    /// Set percentage
    pub fn set_percentage(&mut self, pct: u32) {
        self.percentage = pct;
        self.severity = if pct > 50 { 3 } else if pct > 30 { 2 } else { 1 };
    }
}

impl fmt::Display for Bottleneck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}ms ({}%, severity: {})",
            self.name, self.duration, self.percentage, self.severity
        )
    }
}

/// Performance sample
#[derive(Debug, Clone)]
pub struct PerformanceSample {
    pub component: String,
    pub start_time: u64,
    pub end_time: u64,
    pub cpu_usage: u32,
    pub memory_peak: u32,
}

impl PerformanceSample {
    /// Create new sample
    pub fn new(component: &str, start: u64) -> Self {
        PerformanceSample {
            component: String::from(component),
            start_time: start,
            end_time: 0,
            cpu_usage: 0,
            memory_peak: 0,
        }
    }

    /// End sample
    pub fn end(&mut self, end: u64) {
        self.end_time = end;
    }

    /// Get duration
    pub fn duration(&self) -> u64 {
        if self.end_time > self.start_time {
            self.end_time - self.start_time
        } else {
            0
        }
    }

    /// Set CPU usage
    pub fn set_cpu_usage(&mut self, usage: u32) {
        self.cpu_usage = usage;
    }

    /// Set memory peak
    pub fn set_memory_peak(&mut self, peak: u32) {
        self.memory_peak = peak;
    }
}

impl fmt::Display for PerformanceSample {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}ms (CPU: {}%, MEM: {}MB)",
            self.component, self.duration(), self.cpu_usage, self.memory_peak
        )
    }
}

/// Boot Profiler
pub struct BootProfiler {
    metrics: Vec<PerformanceMetric>,
    samples: Vec<PerformanceSample>,
    bottlenecks: Vec<Bottleneck>,
    total_boot_time: u64,
    profiling_enabled: bool,
}

impl BootProfiler {
    /// Create new profiler
    pub fn new() -> Self {
        BootProfiler {
            metrics: Vec::new(),
            samples: Vec::new(),
            bottlenecks: Vec::new(),
            total_boot_time: 0,
            profiling_enabled: false,
        }
    }

    /// Enable profiling
    pub fn enable_profiling(&mut self) -> bool {
        self.profiling_enabled = true;
        true
    }

    /// Record metric
    pub fn record_metric(&mut self, metric: PerformanceMetric) -> bool {
        self.metrics.push(metric);
        true
    }

    /// Record sample
    pub fn record_sample(&mut self, sample: PerformanceSample) -> bool {
        self.samples.push(sample);
        true
    }

    /// Analyze bottlenecks
    pub fn analyze_bottlenecks(&mut self) -> u32 {
        if self.total_boot_time == 0 {
            return 0;
        }

        for sample in &self.samples {
            let pct = ((sample.duration() as u64 * 100) / self.total_boot_time) as u32;
            if pct > 10 {
                let mut bn = Bottleneck::new(&sample.component, sample.duration());
                bn.set_percentage(pct);
                self.bottlenecks.push(bn);
            }
        }

        self.bottlenecks.len() as u32
    }

    /// Get slowest component
    pub fn get_slowest_component(&self) -> Option<&PerformanceSample> {
        self.samples.iter().max_by_key(|s| s.duration())
    }

    /// Get average duration
    pub fn get_average_duration(&self) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }

        let total: u64 = self.samples.iter().map(|s| s.duration()).sum();
        total / (self.samples.len() as u64)
    }

    /// Get profiling report
    pub fn profiling_report(&self) -> String {
        let mut report = String::from("=== Performance Profiling Report ===\n");

        report.push_str(&format!("Profiling Enabled: {}\n", self.profiling_enabled));
        report.push_str(&format!("Total Boot Time: {} ms\n", self.total_boot_time));
        report.push_str(&format!("Average Component Time: {} ms\n\n", self.get_average_duration()));

        report.push_str("--- Samples ---\n");
        for sample in &self.samples {
            report.push_str(&format!("{}\n", sample));
        }

        report.push_str("\n--- Bottlenecks ---\n");
        for bn in &self.bottlenecks {
            report.push_str(&format!("{}\n", bn));
        }

        if let Some(slowest) = self.get_slowest_component() {
            report.push_str(&format!("\nSlowest: {} ({}ms)\n", 
                slowest.component, slowest.duration()));
        }

        report
    }
}

impl fmt::Display for BootProfiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BootProfiler {{ samples: {}, bottlenecks: {}, time: {}ms }}",
            self.samples.len(),
            self.bottlenecks.len(),
            self.total_boot_time
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metric() {
        let metric = PerformanceMetric::new("Test", MetricType::Duration, "ms");
        assert_eq!(metric.metric_type, MetricType::Duration);
    }

    #[test]
    fn test_performance_metric_value() {
        let mut metric = PerformanceMetric::new("Test", MetricType::Duration, "ms");
        metric.set_value(100);
        assert_eq!(metric.value, 100);
    }

    #[test]
    fn test_bottleneck() {
        let bn = Bottleneck::new("Stage", 500);
        assert_eq!(bn.duration, 500);
    }

    #[test]
    fn test_bottleneck_severity() {
        let mut bn = Bottleneck::new("Stage", 500);
        bn.set_percentage(60);
        assert_eq!(bn.severity, 3);
    }

    #[test]
    fn test_performance_sample() {
        let sample = PerformanceSample::new("Component", 100);
        assert_eq!(sample.start_time, 100);
    }

    #[test]
    fn test_performance_sample_duration() {
        let mut sample = PerformanceSample::new("Component", 100);
        sample.end(200);
        assert_eq!(sample.duration(), 100);
    }

    #[test]
    fn test_performance_sample_resources() {
        let mut sample = PerformanceSample::new("Component", 100);
        sample.set_cpu_usage(50);
        sample.set_memory_peak(256);
        assert_eq!(sample.cpu_usage, 50);
        assert_eq!(sample.memory_peak, 256);
    }

    #[test]
    fn test_boot_profiler() {
        let profiler = BootProfiler::new();
        assert!(!profiler.profiling_enabled);
    }

    #[test]
    fn test_boot_profiler_enable() {
        let mut profiler = BootProfiler::new();
        assert!(profiler.enable_profiling());
        assert!(profiler.profiling_enabled);
    }

    #[test]
    fn test_boot_profiler_record_metric() {
        let mut profiler = BootProfiler::new();
        let metric = PerformanceMetric::new("Test", MetricType::Duration, "ms");
        assert!(profiler.record_metric(metric));
    }

    #[test]
    fn test_boot_profiler_record_sample() {
        let mut profiler = BootProfiler::new();
        let sample = PerformanceSample::new("Component", 100);
        assert!(profiler.record_sample(sample));
    }

    #[test]
    fn test_boot_profiler_average() {
        let mut profiler = BootProfiler::new();
        let mut s1 = PerformanceSample::new("C1", 0);
        s1.end(100);
        let mut s2 = PerformanceSample::new("C2", 100);
        s2.end(300);
        profiler.record_sample(s1);
        profiler.record_sample(s2);
        assert_eq!(profiler.get_average_duration(), 150);
    }

    #[test]
    fn test_boot_profiler_slowest() {
        let mut profiler = BootProfiler::new();
        let mut s1 = PerformanceSample::new("C1", 0);
        s1.end(100);
        let mut s2 = PerformanceSample::new("C2", 100);
        s2.end(300);
        profiler.record_sample(s1);
        profiler.record_sample(s2);
        assert!(profiler.get_slowest_component().is_some());
    }

    #[test]
    fn test_boot_profiler_report() {
        let profiler = BootProfiler::new();
        let report = profiler.profiling_report();
        assert!(report.contains("Performance Profiling Report"));
    }
}
