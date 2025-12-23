//! Performance Monitoring and Analysis Tools
//!
//! This module provides performance monitoring and analysis tools
//! for NOS operating system to improve maintainability and optimization.

use {
    alloc::{
        collections::BTreeMap,
        sync::Arc,
        vec::Vec,
        string::{String, ToString},
        boxed::Box,
        format,
    },
    spin::Mutex,
};
#[cfg(feature = "log")]
use log;
use nos_api::Result;
use crate::{SyscallHandler, SyscallDispatcher};
use core::sync::atomic::{AtomicU64, Ordering};
use libm::sqrt;

/// Performance metric types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Counter metric (incrementing value)
    Counter,
    /// Gauge metric (current value)
    Gauge,
    /// Histogram metric (distribution of values)
    Histogram,
    /// Timer metric (duration measurements)
    Timer,
}

/// Performance metric
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Current value
    pub value: f64,
    /// Unit of measurement
    pub unit: String,
    /// Description
    pub description: String,
    /// Timestamp of last update
    pub timestamp: u64,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl PerformanceMetric {
    /// Create a new performance metric
    pub fn new(
        name: String,
        metric_type: MetricType,
        value: f64,
        unit: String,
        description: String,
    ) -> Self {
        Self {
            name,
            metric_type,
            value,
            unit,
            description,
            timestamp: Self::get_time_us(),
            tags: Vec::new(),
        }
    }
    
    /// Add a tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
    
    /// Update metric value
    pub fn update(&mut self, value: f64) {
        self.value = value;
        self.timestamp = Self::get_time_us();
    }
    
    /// Increment counter metric
    pub fn increment(&mut self) {
        if self.metric_type == MetricType::Counter {
            self.value += 1.0;
            self.timestamp = Self::get_time_us();
        }
    }
    
    /// Add to gauge metric
    pub fn add(&mut self, delta: f64) {
        if self.metric_type == MetricType::Gauge {
            self.value += delta;
            self.timestamp = Self::get_time_us();
        }
    }
    
    /// Record timer measurement
    pub fn record_time(&mut self, duration_us: u64) {
        if self.metric_type == MetricType::Timer {
            // Update with exponential moving average
            let alpha = 0.1; // Smoothing factor
            self.value = self.value * (1.0 - alpha) + (duration_us as f64) * alpha;
            self.timestamp = Self::get_time_us();
        }
    }
    
    /// Get current time in microseconds
    fn get_time_us() -> u64 {
        // In a real implementation, this would use a high-precision timer
        static TIME_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIME_COUNTER.fetch_add(1, Ordering::SeqCst)
    }
}

/// Performance monitor
pub struct PerformanceMonitor {
    /// Registered metrics
    metrics: BTreeMap<String, PerformanceMetric>,
    /// Metric history for time series
    history: BTreeMap<String, Vec<f64>>,
    /// History size limit
    history_limit: usize,
    /// Monitor statistics
    stats: MonitorStats,
}

/// Monitor statistics
#[derive(Debug, Clone)]
pub struct MonitorStats {
    /// Total metrics registered
    pub total_metrics: usize,
    /// Total updates recorded
    pub total_updates: u64,
    /// Start time
    pub start_time: u64,
}

impl MonitorStats {
    /// Create new monitor statistics
    pub fn new() -> Self {
        Self {
            total_metrics: 0,
            total_updates: 0,
            start_time: PerformanceMonitor::get_time_us(),
        }
    }
    
    /// Record a metric update
    pub fn record_update(&mut self) {
        self.total_updates += 1;
    }
    
    /// Get uptime in microseconds
    pub fn uptime_us(&self) -> u64 {
        PerformanceMonitor::get_time_us() - self.start_time
    }
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self::with_history_limit(1000)
    }
    
    /// Create a new performance monitor with history limit
    pub fn with_history_limit(limit: usize) -> Self {
        Self {
            metrics: BTreeMap::new(),
            history: BTreeMap::new(),
            history_limit: limit,
            stats: MonitorStats::new(),
        }
    }
    
    /// Register a new metric
    pub fn register_metric(&mut self, metric: PerformanceMetric) -> Result<()> {
        let name = metric.name.clone();
        self.metrics.insert(name.clone(), metric);
        self.history.insert(name, Vec::new());
        self.stats.total_metrics += 1;
        Ok(())
    }
    
    /// Update a metric value
    pub fn update_metric(&mut self, name: &str, value: f64) -> Result<()> {
        if let Some(metric) = self.metrics.get_mut(name) {
            metric.update(value);
            self.add_to_history(name, value);
            self.stats.record_update();
            Ok(())
        } else {
            Err(nos_api::Error::NotFound(
                format!("Metric '{}' not found", name)
            ))
        }
    }
    
    /// Increment a counter metric
    pub fn increment_metric(&mut self, name: &str) -> Result<()> {
        if self.metrics.contains_key(name) {
            // Get the value after incrementing
            let new_value = {
                let metric = self.metrics.get_mut(name).unwrap();
                metric.increment();
                metric.value
            };
            self.add_to_history(name, new_value);
            self.stats.record_update();
            Ok(())
        } else {
            Err(nos_api::Error::NotFound(
                if cfg!(feature = "alloc") {
                    format!("Metric '{}' not found", name)
                } else {
                    "Metric not found".to_string()
                }
            ))
        }
    }
    
    /// Add to a gauge metric
    pub fn add_to_metric(&mut self, name: &str, delta: f64) -> Result<()> {
        if self.metrics.contains_key(name) {
            // Get the value after adding
            let new_value = {
                let metric = self.metrics.get_mut(name).unwrap();
                metric.add(delta);
                metric.value
            };
            self.add_to_history(name, new_value);
            self.stats.record_update();
            Ok(())
        } else {
            Err(nos_api::Error::NotFound(
                if cfg!(feature = "alloc") {
                    format!("Metric '{}' not found", name)
                } else {
                    "Metric not found".to_string()
                }
            ))
        }
    }
    
    /// Record a timer measurement
    pub fn record_timer(&mut self, name: &str, duration_us: u64) -> Result<()> {
        if self.metrics.contains_key(name) {
            // Get the value after recording time
            let new_value = {
                let metric = self.metrics.get_mut(name).unwrap();
                metric.record_time(duration_us);
                metric.value
            };
            self.add_to_history(name, new_value);
            self.stats.record_update();
            Ok(())
        } else {
            Err(nos_api::Error::NotFound(
                if cfg!(feature = "alloc") {
                    format!("Metric '{}' not found", name)
                } else {
                    "Metric not found".to_string()
                }
            ))
        }
    }
    
    /// Add value to history
    fn add_to_history(&mut self, name: &str, value: f64) {
        if let Some(history) = self.history.get_mut(name) {
            history.push(value);
            
            // Limit history size
            if history.len() > self.history_limit {
                history.remove(0);
            }
        }
    }
    
    /// Get metric value
    pub fn get_metric(&self, name: &str) -> Option<&PerformanceMetric> {
        self.metrics.get(name)
    }
    
    /// Get metric history
    pub fn get_metric_history(&self, name: &str) -> Option<&Vec<f64>> {
        self.history.get(name)
    }
    
    /// Get all metrics
    pub fn get_all_metrics(&self) -> Vec<&PerformanceMetric> {
        self.metrics.values().collect()
    }
    
    /// Get monitor statistics
    pub fn get_stats(&self) -> &MonitorStats {
        &self.stats
    }
    
    /// Generate performance report
    pub fn generate_report(&self) -> String {
        let mut report = String::from("=== Performance Monitor Report ===\n");
        
        report.push_str(&format!("Total metrics: {}\n", self.stats.total_metrics));
        report.push_str(&format!("Total updates: {}\n", self.stats.total_updates));
        report.push_str(&format!("Uptime: {}μs\n", self.stats.uptime_us()));
        
        report.push_str("\nMetrics:\n");
        for metric in self.metrics.values() {
            report.push_str(&format!(
                "  {}: {:.2} {} ({})\n",
                metric.name, metric.value, metric.unit, metric.description
            ));
            
            // Add history statistics if available
            if let Some(history) = self.history.get(&metric.name) {
                if !history.is_empty() {
                    let min = history.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                    let max = history.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                    let avg = history.iter().sum::<f64>() / history.len() as f64;
                    
                    report.push_str(&format!(
                        "    History: min={:.2}, max={:.2}, avg={:.2}\n",
                        min, max, avg
                    ));
                }
            }
        }
        
        report
    }
    
    /// Get current time in microseconds
    fn get_time_us() -> u64 {
        // In a real implementation, this would use a high-precision timer
        static TIME_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIME_COUNTER.fetch_add(1, Ordering::SeqCst)
    }
}

/// Performance analyzer
pub struct PerformanceAnalyzer {
    /// Performance monitor
    monitor: Arc<Mutex<PerformanceMonitor>>,
    /// Analysis results
    results: BTreeMap<String, AnalysisResult>,
}

/// Analysis result
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// Metric name
    pub metric_name: String,
    /// Analysis type
    pub analysis_type: AnalysisType,
    /// Analysis result
    pub result: AnalysisValue,
    /// Confidence level (0-1)
    pub confidence: f32,
    /// Timestamp
    pub timestamp: u64,
}

/// Analysis types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisType {
    /// Trend analysis
    Trend,
    /// Anomaly detection
    Anomaly,
    /// Correlation analysis
    Correlation,
    /// Performance prediction
    Prediction,
}

/// Analysis values
#[derive(Debug, Clone)]
pub enum AnalysisValue {
    /// Trend direction
    Trend(TrendDirection),
    /// Anomaly score
    Anomaly(f32),
    /// Correlation coefficient
    Correlation(f32),
    /// Predicted value
    Prediction(f64),
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    /// Increasing trend
    Increasing,
    /// Decreasing trend
    Decreasing,
    /// Stable trend
    Stable,
}

impl PerformanceAnalyzer {
    /// Create a new performance analyzer
    pub fn new(monitor: Arc<Mutex<PerformanceMonitor>>) -> Self {
        Self {
            monitor,
            results: BTreeMap::new(),
        }
    }
    
    /// Analyze all metrics
    pub fn analyze_all(&mut self) -> Result<()> {
        // Collect metric names first to avoid borrowing conflicts
        let metric_names: Vec<String> = {
            let monitor = self.monitor.lock();
            monitor.metrics.keys()
                .map(|s: &String| s.clone())
                .collect()
        };
        
        for metric_name in &metric_names {
            self.analyze_metric(metric_name)?;
        }
        Ok(())
    }
    
    /// Analyze a specific metric
    pub fn analyze_metric(&mut self, metric_name: &str) -> Result<()> {
        let history: Option<Vec<f64>> = {
            let monitor = self.monitor.lock();
            if let Some(hist) = monitor.get_metric_history(metric_name) {
                let cloned_hist: Vec<f64> = (*hist).clone();
                Some(cloned_hist)
            } else {
                None
            }
        };
        
        if let Some(history) = history {
            if history.len() < 10 {
                return Err(nos_api::Error::InvalidArgument(
                    "Insufficient data for analysis".to_string()
                ));
            }
            
            // Perform trend analysis
            let trend = self.analyze_trend(&history);
            
            // Perform anomaly detection
            let anomaly = self.detect_anomalies(&history);
            
            // Store results
            let timestamp = PerformanceMonitor::get_time_us();
            
            self.results.insert(
                metric_name.to_string(),
                AnalysisResult {
                    metric_name: metric_name.to_string(),
                    analysis_type: AnalysisType::Trend,
                    result: AnalysisValue::Trend(trend),
                    confidence: 0.8,
                    timestamp,
                },
            );
            
            self.results.insert(
                format!("{}_anomaly", metric_name),
                AnalysisResult {
                    metric_name: metric_name.to_string(),
                    analysis_type: AnalysisType::Anomaly,
                    result: AnalysisValue::Anomaly(anomaly),
                    confidence: 0.7,
                    timestamp,
                },
            );
            
            Ok(())
        } else {
            Err(nos_api::Error::NotFound(
                if cfg!(feature = "alloc") {
                    format!("Metric '{}' not found", metric_name)
                } else {
                    "Metric not found".to_string()
                }
            ))
        }
    }
    
    /// Analyze trend
    fn analyze_trend(&self, history: &[f64]) -> TrendDirection {
        if history.len() < 2 {
            return TrendDirection::Stable;
        }
        
        // Simple linear regression to determine trend
        let n = history.len() as f64;
        let sum_x: f64 = (0..history.len()).map(|i| i as f64).sum();
        let sum_y: f64 = history.iter().sum();
        let sum_xy: f64 = history.iter().enumerate()
            .map(|(i, &y)| i as f64 * y)
            .sum();
        let sum_x2: f64 = (0..history.len()).map(|i| (i as f64) * (i as f64)).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        
        if slope > 0.1 {
            TrendDirection::Increasing
        } else if slope < -0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }
    
    /// Detect anomalies
    fn detect_anomalies(&self, history: &[f64]) -> f32 {
        if history.len() < 3 {
            return 0.0;
        }
        
        // Simple anomaly detection using standard deviation
        let mean = history.iter().sum::<f64>() / history.len() as f64;
        let variance = history.iter()
            .map(|x| (x - mean) * (x - mean))
            .sum::<f64>() / history.len() as f64;
        let std_dev = sqrt(variance);
        
        // Check if last value is an outlier
        let last_value = history[history.len() - 1];
        let z_score = (last_value - mean) / std_dev;
        
        // Convert z-score to anomaly score (0-1)
        if z_score.abs() > 3.0 {
            1.0 // High anomaly
        } else if z_score.abs() > 2.0 {
            0.7 // Medium anomaly
        } else if z_score.abs() > 1.0 {
            0.4 // Low anomaly
        } else {
            0.0 // No anomaly
        }
    }
    
    /// Get analysis results
    pub fn get_results(&self) -> &BTreeMap<String, AnalysisResult> {
        &self.results
    }
    
    /// Generate analysis report
    pub fn generate_report(&self) -> String {
        let mut report = String::from("=== Performance Analysis Report ===\n");
        
        for (name, result) in &self.results {
            report.push_str(&format!(
                "  {}: {:?} = {:?} (confidence: {:.1}%)\n",
                name, result.analysis_type, result.result, result.confidence * 100.0
            ));
        }
        
        report
    }
}

/// System call performance monitor
pub struct SyscallMonitor {
    monitor: Arc<Mutex<PerformanceMonitor>>,
}

impl SyscallMonitor {
    /// Create a new syscall monitor
    pub fn new() -> Self {
        Self {
            monitor: Arc::new(Mutex::new(PerformanceMonitor::new())),
        }
    }
    
    pub fn new_with_monitor(monitor: Arc<Mutex<PerformanceMonitor>>) -> Self {
        Self { monitor }
    }
}

impl SyscallHandler for SyscallMonitor {
    fn id(&self) -> u32 {
        crate::types::SYS_PERF_MONITOR
    }
    
    fn name(&self) -> &str {
        "perf_monitor"
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        let monitor = &self.monitor;
        
        // Parse arguments
        if args.len() < 2 {
            return Err(nos_api::Error::InvalidArgument(
                "Insufficient arguments for performance monitor".to_string()
            ));
        }
        
        let operation = args[0];
        let metric_name_ptr = args[1];
        
        // In a real implementation, this would read the metric name from memory
        let metric_name = format!("metric_{}", metric_name_ptr);
        
        match operation {
            0 => {
                // Get metric value
                let metric_value = {
                    let guard = monitor.lock();
                    let metric = guard.get_metric(&metric_name);
                    metric.map(|m| m.value)
                };
                if let Some(value) = metric_value {
                    Ok(value as isize)
                } else {
                    Err(nos_api::Error::NotFound(
                        format!("Metric '{}' not found", metric_name)
                    ))
                }
            },
            1 => {
                // Increment metric
                {
                    monitor.lock().increment_metric(&metric_name)?;
                }
                Ok(0)
            },
            2 => {
                // Generate report
                let report = {
                    monitor.lock().generate_report()
                };
                #[cfg(feature = "log")]
                log::info!("{}", report);
                #[cfg(not(feature = "log"))]
                let _ = report; // Suppress unused variable warning
                Ok(0)
            },
            _ => {
                Err(nos_api::Error::InvalidArgument(
                    format!("Invalid operation: {}", operation)
                ))
            }
        }
    }
}

/// Register performance monitoring system call handlers
pub fn register_handlers(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // Create performance monitor
    let monitor = Arc::new(Mutex::new(PerformanceMonitor::new()));
    
    // Register standard metrics
    monitor.lock().register_metric(PerformanceMetric::new(
        "syscall_count".to_string(),
        MetricType::Counter,
        0.0,
        "count".to_string(),
        "Total number of system calls".to_string(),
    ))?;
    
    monitor.lock().register_metric(PerformanceMetric::new(
        "cpu_usage".to_string(),
        MetricType::Gauge,
        0.0,
        "percent".to_string(),
        "CPU usage percentage".to_string(),
    ))?;
    
    monitor.lock().register_metric(PerformanceMetric::new(
        "memory_usage".to_string(),
        MetricType::Gauge,
        0.0,
        "bytes".to_string(),
        "Memory usage in bytes".to_string(),
    ))?;
    
    monitor.lock().register_metric(PerformanceMetric::new(
        "syscall_latency".to_string(),
        MetricType::Timer,
        0.0,
        "microseconds".to_string(),
        "Average system call latency".to_string(),
    ))?;
    
    // Register performance monitor system call
    let monitor_handler = SyscallMonitor::new_with_monitor(monitor.clone());
    dispatcher.register_handler(crate::types::SYS_PERF_MONITOR, Box::new(monitor_handler));
    
    // Create analyzer and run analysis
    let mut analyzer = PerformanceAnalyzer::new(monitor);
    analyzer.analyze_all()?;
    
    // Print analysis report
    let report = analyzer.generate_report();
    #[cfg(feature = "log")]
    log::info!("{}", report);
    #[cfg(not(feature = "log"))]
    let _ = report; // Suppress unused variable warning
    
    Ok(())
}