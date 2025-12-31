//! Performance Regression Detection System
//!
//! This module provides comprehensive performance regression detection for the NOS kernel,
//! including benchmarking, performance tracking, automatic regression detection,
//! and performance trend analysis.

use core::sync::atomic::{AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use crate::sync::Arc;
use crate::sync::Mutex;

/// Performance metric types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Latency metric (nanoseconds)
    Latency,
    /// Throughput metric (operations per second)
    Throughput,
    /// Memory usage (bytes)
    MemoryUsage,
    /// CPU utilization (percentage)
    CpuUtilization,
    /// Frame time (microseconds)
    FrameTime,
    /// Interrupt latency (microseconds)
    InterruptLatency,
    /// Context switch time (microseconds)
    ContextSwitchTime,
    /// Custom metric
    Custom,
}

/// Performance metric value
#[derive(Debug, Clone, Copy)]
pub enum MetricValue {
    /// 64-bit integer value
    Integer(i64),
    /// 64-bit floating point value
    Float(f64),
    /// Unsigned 64-bit value
    Unsigned(u64),
}

impl MetricValue {
    /// Convert to f64 for comparisons
    pub fn as_f64(&self) -> f64 {
        match self {
            MetricValue::Integer(v) => *v as f64,
            MetricValue::Float(v) => *v,
            MetricValue::Unsigned(v) => *v as f64,
        }
    }
}

/// Performance benchmark data point
#[derive(Debug, Clone)]
pub struct BenchmarkDataPoint {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Value
    pub value: MetricValue,
    /// Unit of measurement
    pub unit: String,
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}

/// Performance baseline
#[derive(Debug, Clone)]
pub struct PerformanceBaseline {
    /// Baseline name
    pub name: String,
    /// Commit hash
    pub commit: String,
    /// Branch name
    pub branch: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Benchmark data points
    pub data_points: Vec<BenchmarkDataPoint>,
}

/// Performance regression result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegressionSeverity {
    /// No regression (within threshold)
    None,
    /// Minor regression (1-5% degradation)
    Minor,
    /// Moderate regression (5-10% degradation)
    Moderate,
    /// Severe regression (10-20% degradation)
    Severe,
    /// Critical regression (>20% degradation)
    Critical,
}

/// Performance regression report
#[derive(Debug, Clone)]
pub struct RegressionReport {
    /// Benchmark name
    pub benchmark_name: String,
    /// Baseline value
    pub baseline_value: MetricValue,
    /// Current value
    pub current_value: MetricValue,
    /// Percentage change
    pub percent_change: f64,
    /// Regression severity
    pub severity: RegressionSeverity,
    /// Is this a regression?
    pub is_regression: bool,
    /// Is this an improvement?
    pub is_improvement: bool,
    /// Recommendation
    pub recommendation: String,
}

/// Performance trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    /// Improving (getting better)
    Improving,
    /// Stable (no significant change)
    Stable,
    /// Degrading (getting worse)
    Degrading,
}

/// Performance trend analysis
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    /// Benchmark name
    pub benchmark_name: String,
    /// Trend direction
    pub direction: TrendDirection,
    /// Average rate of change per commit
    pub rate_of_change: f64,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Number of data points analyzed
    pub data_points: usize,
    /// Predicted value for next commit
    pub prediction: MetricValue,
}

/// Performance regression detector
pub struct PerformanceRegressionDetector {
    /// Baseline storage
    baselines: Mutex<BTreeMap<String, PerformanceBaseline>>,
    /// Historical benchmark data
    history: Mutex<BTreeMap<String, Vec<BenchmarkDataPoint>>>,
    /// Regression threshold (percentage)
    regression_threshold: f64,
    /// Improvement threshold (percentage)
    improvement_threshold: f64,
    /// Minimum data points for trend analysis
    min_trend_data_points: usize,
    /// Maximum history size
    max_history_size: usize,
    /// Next baseline ID
    next_baseline_id: AtomicU64,
}

impl PerformanceRegressionDetector {
    /// Create a new performance regression detector
    pub fn new() -> Self {
        Self {
            baselines: Mutex::new(BTreeMap::new()),
            history: Mutex::new(BTreeMap::new()),
            regression_threshold: 5.0, // 5% threshold for regression
            improvement_threshold: -5.0, // -5% threshold for improvement
            min_trend_data_points: 10,
            max_history_size: 1000,
            next_baseline_id: AtomicU64::new(1),
        }
    }

    /// Record a benchmark data point
    pub fn record_metric(&self, data_point: BenchmarkDataPoint) {
        let mut history = self.history.lock();
        let entry = history.entry(data_point.name.clone()).or_insert_with(Vec::new);
        entry.push(data_point.clone());
        
        // Trim history if necessary
        if entry.len() > self.max_history_size {
            entry.remove(0);
        }
    }

    /// Create a performance baseline
    pub fn create_baseline(&self, name: String, commit: String, branch: String) -> Result<String, String> {
        let baseline_id = format!("baseline_{}", self.next_baseline_id.fetch_add(1, Ordering::SeqCst));
        
        let history = self.history.lock();
        let mut data_points = Vec::new();
        
        // Collect all current data points
        for (_, metrics) in history.iter() {
            for metric in metrics {
                data_points.push(metric.clone());
            }
        }
        
        let baseline = PerformanceBaseline {
            name: baseline_id.clone(),
            commit,
            branch,
            created_at: self.get_current_time(),
            data_points,
        };
        
        let mut baselines = self.baselines.lock();
        baselines.insert(baseline_id.clone(), baseline);
        
        crate::println!("[perf] Created baseline: {}", baseline_id);
        Ok(baseline_id)
    }

    /// Detect performance regressions
    pub fn detect_regressions(&self, baseline_name: &str) -> Vec<RegressionReport> {
        let baselines = self.baselines.lock();
        let baseline = match baselines.get(baseline_name) {
            Some(b) => b,
            None => {
                crate::println!("[perf] Baseline not found: {}", baseline_name);
                return Vec::new();
            }
        };
        
        let history = self.history.lock();
        let mut reports = Vec::new();
        
        // Compare current metrics with baseline
        for baseline_metric in &baseline.data_points {
            if let Some(current_metrics) = history.get(&baseline_metric.name) {
                if let Some(latest) = current_metrics.last() {
                    let baseline_value = baseline_metric.value;
                    let current_value = latest.value;
                    
                    let percent_change = self.calculate_percent_change(baseline_value, current_value);
                    
                    let severity = if percent_change > 20.0 {
                        RegressionSeverity::Critical
                    } else if percent_change > 10.0 {
                        RegressionSeverity::Severe
                    } else if percent_change > 5.0 {
                        RegressionSeverity::Moderate
                    } else if percent_change > 1.0 {
                        RegressionSeverity::Minor
                    } else {
                        RegressionSeverity::None
                    };
                    
                    let is_regression = percent_change > self.regression_threshold;
                    let is_improvement = percent_change < self.improvement_threshold;
                    
                    let recommendation = if is_regression {
                        format!("PERFORMANCE REGRESSION DETECTED: {} degraded by {:.1}%. Consider investigating and reverting if necessary.", 
                                baseline_metric.name, percent_change)
                    } else if is_improvement {
                        format!("PERFORMANCE IMPROVEMENT: {} improved by {:.1}%. Great work!", 
                                baseline_metric.name, percent_change.abs())
                    } else {
                        format!("Performance stable for {} (change: {:.1}%)", 
                                baseline_metric.name, percent_change)
                    };
                    
                    reports.push(RegressionReport {
                        benchmark_name: baseline_metric.name.clone(),
                        baseline_value,
                        current_value,
                        percent_change,
                        severity,
                        is_regression,
                        is_improvement,
                        recommendation,
                    });
                }
            }
        }
        
        reports
    }

    /// Analyze performance trends
    pub fn analyze_trends(&self) -> Vec<TrendAnalysis> {
        let history = self.history.lock();
        let mut analyses = Vec::new();
        
        for (metric_name, data_points) in history.iter() {
            if data_points.len() < self.min_trend_data_points {
                continue;
            }
            
            // Calculate linear regression
            let n = data_points.len() as f64;
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            let mut sum_xy = 0.0;
            let mut sum_x2 = 0.0;
            
            for (i, point) in data_points.iter().enumerate() {
                let x = i as f64;
                let y = point.value.as_f64();
                sum_x += x;
                sum_y += y;
                sum_xy += x * y;
                sum_x2 += x * x;
            }
            
            let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
            let intercept = (sum_y - slope * sum_x) / n;
            
            // Determine trend direction
            let direction = if slope > 0.1 {
                // For latency, higher is worse
                if matches!(data_points[0].metric_type, MetricType::Latency | MetricType::InterruptLatency) {
                    TrendDirection::Degrading
                } else {
                    TrendDirection::Improving
                }
            } else if slope < -0.1 {
                if matches!(data_points[0].metric_type, MetricType::Latency | MetricType::InterruptLatency) {
                    TrendDirection::Improving
                } else {
                    TrendDirection::Degrading
                }
            } else {
                TrendDirection::Stable
            };
            
            // Calculate confidence (R-squared)
            let mean_y = sum_y / n;
            let mut ss_tot = 0.0;
            let mut ss_res = 0.0;
            
            for point in data_points.iter() {
                let x = data_points.iter().position(|p| p.name == point.name).unwrap() as f64;
                let y = point.value.as_f64();
                let y_pred = slope * x + intercept;
                ss_tot += (y - mean_y).powi(2);
                ss_res += (y - y_pred).powi(2);
            }
            
            let confidence = if ss_tot > 0.0 {
                1.0 - (ss_res / ss_tot)
            } else {
                0.0
            };
            
            // Predict next value
            let next_x = data_points.len() as f64;
            let predicted_value = slope * next_x + intercept;
            
            analyses.push(TrendAnalysis {
                benchmark_name: metric_name.clone(),
                direction,
                rate_of_change: slope,
                confidence,
                data_points: data_points.len(),
                prediction: MetricValue::Float(predicted_value),
            });
        }
        
        analyses
    }

    /// Get performance summary
    pub fn get_summary(&self) -> PerformanceSummary {
        let history = self.history.lock();
        let baselines = self.baselines.lock();
        
        let total_metrics = history.len();
        let total_data_points = history.values().map(|v| v.len()).sum();
        let total_baselines = baselines.len();
        
        PerformanceSummary {
            total_metrics,
            total_data_points,
            total_baselines,
            regression_threshold: self.regression_threshold,
            improvement_threshold: self.improvement_threshold,
        }
    }

    // Private helper methods

    fn calculate_percent_change(&self, baseline: MetricValue, current: MetricValue) -> f64 {
        let baseline_f = baseline.as_f64();
        let current_f = current.as_f64();
        
        if baseline_f == 0.0 {
            return 0.0;
        }
        
        ((current_f - baseline_f) / baseline_f) * 100.0
    }

    fn get_current_time(&self) -> u64 {
        // In a real implementation, this would get the actual time
        // For now, return a placeholder
        0
    }
}

impl Default for PerformanceRegressionDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance summary
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    /// Total number of metrics tracked
    pub total_metrics: usize,
    /// Total number of data points recorded
    pub total_data_points: usize,
    /// Total number of baselines
    pub total_baselines: usize,
    /// Regression threshold percentage
    pub regression_threshold: f64,
    /// Improvement threshold percentage
    pub improvement_threshold: f64,
}

/// Global performance regression detector instance
static mut PERF_DETECTOR: Option<PerformanceRegressionDetector> = None;

/// Initialize performance regression detector
pub fn init_performance_detector() {
    unsafe {
        if PERF_DETECTOR.is_none() {
            PERF_DETECTOR = Some(PerformanceRegressionDetector::new());
            crate::println!("[perf] Performance regression detector initialized");
        }
    }
}

/// Get performance regression detector instance
pub fn get_performance_detector() -> Option<&'static PerformanceRegressionDetector> {
    unsafe { PERF_DETECTOR.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = PerformanceRegressionDetector::new();
        assert_eq!(detector.regression_threshold, 5.0);
    }

    #[test]
    fn test_metric_recording() {
        let detector = PerformanceRegressionDetector::new();
        let data_point = BenchmarkDataPoint {
            name: "test_metric".to_string(),
            metric_type: MetricType::Latency,
            value: MetricValue::Integer(100),
            unit: "ns".to_string(),
            timestamp: 0,
            metadata: BTreeMap::new(),
        };
        
        detector.record_metric(data_point);
        
        let history = detector.history.lock();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_baseline_creation() {
        let detector = PerformanceRegressionDetector::new();
        
        let result = detector.create_baseline(
            "test_baseline".to_string(),
            "abc123".to_string(),
            "main".to_string(),
        );
        
        assert!(result.is_ok());
    }
}
