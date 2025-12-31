//! # Model Monitoring and Data Drift Detection
//!
//! Comprehensive monitoring system for ML models in production.
//! Detects data drift, performance degradation, and triggers alerts.
//!
//! ## Features
//!
//! - **Data Drift Detection**: Statistical tests for distribution changes
//! - **Performance Monitoring**: Track model metrics over time
//! - **Resource Tracking**: Monitor CPU, memory, and latency
//! - **Alerting**: Trigger alerts on threshold breaches
//! - **Dashboard**: Visualize metrics and trends

use crate::mlops::{MonitoringError, MLOpsResult, MLOpsError};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    /// Info level
    Info,
    /// Warning level
    Warning,
    /// Critical level
    Critical,
}

impl fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Alert type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertType {
    /// Data drift detected
    DataDrift,
    /// Performance degradation
    PerformanceDegradation,
    /// Resource exhaustion
    ResourceExhaustion,
    /// High latency
    HighLatency,
    /// Error rate spike
    ErrorRateSpike,
    /// Custom alert
    Custom(String),
}

impl fmt::Display for AlertType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertType::DataDrift => write!(f, "DATA_DRIFT"),
            AlertType::PerformanceDegradation => write!(f, "PERFORMANCE_DEGRADATION"),
            AlertType::ResourceExhaustion => write!(f, "RESOURCE_EXHAUSTION"),
            AlertType::HighLatency => write!(f, "HIGH_LATENCY"),
            AlertType::ErrorRateSpike => write!(f, "ERROR_RATE_SPIKE"),
            AlertType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Alert notification
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert ID
    pub alert_id: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Severity level
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Model name
    pub model_name: String,
    /// Metric that triggered the alert
    pub metric: String,
    /// Current value
    pub current_value: f64,
    /// Threshold value
    pub threshold: f64,
    /// Timestamp
    pub timestamp: u64,
    /// Additional context
    pub context: BTreeMap<String, String>,
}

impl Alert {
    /// Create a new alert
    pub fn new(
        alert_type: AlertType,
        severity: AlertSeverity,
        model_name: String,
        metric: String,
        current_value: f64,
        threshold: f64,
    ) -> Self {
        Self {
            alert_id: generate_alert_id(),
            alert_type: alert_type.clone(),
            severity,
            message: format!(
                "{}: {} = {:.4} exceeds threshold {:.4}",
                alert_type, metric, current_value, threshold
            ),
            model_name,
            metric,
            current_value,
            threshold,
            timestamp: 0,
            context: BTreeMap::new(),
        }
    }

    /// Add context
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct MetricPoint {
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Timestamp
    pub timestamp: u64,
    /// Tags for filtering
    pub tags: BTreeMap<String, String>,
}

impl MetricPoint {
    /// Create a new metric point
    pub fn new(name: String, value: f64, timestamp: u64) -> Self {
        Self {
            name,
            value,
            timestamp,
            tags: BTreeMap::new(),
        }
    }

    /// Add a tag
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }
}

/// Drift detection result
#[derive(Debug, Clone)]
pub struct DriftResult {
    /// Feature name
    pub feature_name: String,
    /// Drift detected
    pub drift_detected: bool,
    /// P-value from statistical test
    pub p_value: f64,
    /// Test statistic
    pub statistic: f64,
    /// Drift magnitude
    pub magnitude: f64,
}

impl DriftResult {
    /// Create a new drift result
    pub fn new(feature_name: String, p_value: f64, statistic: f64) -> Self {
        let drift_detected = p_value < 0.05; // Common significance level

        Self {
            feature_name,
            drift_detected,
            p_value,
            statistic,
            magnitude: statistic.abs(),
        }
    }
}

/// Data distribution statistics
#[derive(Debug, Clone)]
pub struct DistributionStats {
    /// Mean
    pub mean: f64,
    /// Standard deviation
    pub std: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Median
    pub median: f64,
    /// 25th percentile
    pub p25: f64,
    /// 75th percentile
    pub p75: f64,
    /// Count
    pub count: usize,
}

impl DistributionStats {
    /// Calculate statistics from data
    pub fn calculate(data: &[f64]) -> Self {
        if data.is_empty() {
            return Self {
                mean: 0.0,
                std: 0.0,
                min: 0.0,
                max: 0.0,
                median: 0.0,
                p25: 0.0,
                p75: 0.0,
                count: 0,
            };
        }

        let count = data.len();
        let mean = data.iter().sum::<f64>() / count as f64;

        let variance = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / count as f64;
        let std = variance.sqrt();

        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = sorted[0];
        let max = sorted[count - 1];
        let median = if count % 2 == 0 {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        } else {
            sorted[count / 2]
        };

        let p25 = sorted[count / 4];
        let p75 = sorted[count * 3 / 4];

        Self {
            mean,
            std,
            min,
            max,
            median,
            p25,
            p75,
            count,
        }
    }
}

/// Threshold configuration
#[derive(Debug, Clone)]
pub struct Threshold {
    /// Threshold type
    pub threshold_type: ThresholdType,
    /// Warning threshold
    pub warning: f64,
    /// Critical threshold
    pub critical: f64,
}

/// Threshold type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdType {
    /// Upper threshold (alert if value > threshold)
    Upper,
    /// Lower threshold (alert if value < threshold)
    Lower,
    /// Both directions
    Both,
}

impl Threshold {
    /// Create new threshold
    pub fn new(threshold_type: ThresholdType, warning: f64, critical: f64) -> Self {
        Self {
            threshold_type,
            warning,
            critical,
        }
    }

    /// Check if value triggers warning
    pub fn check_warning(&self, value: f64) -> bool {
        match self.threshold_type {
            ThresholdType::Upper => value > self.warning,
            ThresholdType::Lower => value < self.warning,
            ThresholdType::Both => value > self.warning || value < -self.warning,
        }
    }

    /// Check if value triggers critical
    pub fn check_critical(&self, value: f64) -> bool {
        match self.threshold_type {
            ThresholdType::Upper => value > self.critical,
            ThresholdType::Lower => value < self.critical,
            ThresholdType::Both => value > self.critical || value < -self.critical,
        }
    }
}

/// Model performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Model name
    pub model_name: String,
    /// Accuracy
    pub accuracy: Option<f64>,
    /// Precision
    pub precision: Option<f64>,
    /// Recall
    pub recall: Option<f64>,
    /// F1 score
    pub f1_score: Option<f64>,
    /// AUC-ROC
    pub auc_roc: Option<f64>,
    /// Average prediction latency (ns)
    pub avg_latency_ns: Option<u64>,
    /// Error rate
    pub error_rate: Option<f64>,
    /// Throughput (predictions per second)
    pub throughput: Option<f64>,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new(model_name: String) -> Self {
        Self {
            model_name,
            accuracy: None,
            precision: None,
            recall: None,
            f1_score: None,
            auc_roc: None,
            avg_latency_ns: None,
            error_rate: None,
            throughput: None,
        }
    }

    /// Set accuracy
    pub fn with_accuracy(mut self, accuracy: f64) -> Self {
        self.accuracy = Some(accuracy);
        self
    }

    /// Set latency
    pub fn with_latency(mut self, latency_ns: u64) -> Self {
        self.avg_latency_ns = Some(latency_ns);
        self
    }

    /// Set error rate
    pub fn with_error_rate(mut self, error_rate: f64) -> Self {
        self.error_rate = Some(error_rate);
        self
    }
}

/// Resource usage metrics
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    /// CPU usage (0-100)
    pub cpu_usage_percent: f64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Memory usage percent
    pub memory_usage_percent: f64,
    /// GPU usage if available
    pub gpu_usage_percent: Option<f64>,
    /// Disk I/O
    pub disk_io_bytes: Option<u64>,
    /// Network I/O
    pub network_io_bytes: Option<u64>,
}

impl ResourceMetrics {
    /// Create new resource metrics
    pub fn new(cpu: f64, memory_bytes: u64, memory_percent: f64) -> Self {
        Self {
            cpu_usage_percent: cpu,
            memory_usage_bytes: memory_bytes,
            memory_usage_percent: memory_percent,
            gpu_usage_percent: None,
            disk_io_bytes: None,
            network_io_bytes: None,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Metric thresholds
    pub thresholds: BTreeMap<String, Threshold>,
    /// Drift detection enabled
    pub drift_detection_enabled: bool,
    /// Performance monitoring enabled
    pub performance_monitoring_enabled: bool,
    /// Resource monitoring enabled
    pub resource_monitoring_enabled: bool,
    /// Alert cooldown period (ns)
    pub alert_cooldown_ns: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        let mut thresholds = BTreeMap::new();

        // Default thresholds
        thresholds.insert(
            String::from("accuracy"),
            Threshold::new(ThresholdType::Lower, 0.90, 0.85),
        );
        thresholds.insert(
            String::from("latency_ms"),
            Threshold::new(ThresholdType::Upper, 100.0, 200.0),
        );
        thresholds.insert(
            String::from("error_rate"),
            Threshold::new(ThresholdType::Upper, 0.05, 0.10),
        );
        thresholds.insert(
            String::from("cpu_usage"),
            Threshold::new(ThresholdType::Upper, 80.0, 95.0),
        );
        thresholds.insert(
            String::from("memory_usage"),
            Threshold::new(ThresholdType::Upper, 80.0, 95.0),
        );

        Self {
            thresholds,
            drift_detection_enabled: true,
            performance_monitoring_enabled: true,
            resource_monitoring_enabled: true,
            alert_cooldown_ns: 300_000_000_000, // 5 minutes
        }
    }
}

/// Model monitor
pub struct ModelMonitor {
    /// Monitoring configuration
    config: MonitoringConfig,
    /// Metric history
    metrics: Vec<MetricPoint>,
    /// Active alerts
    alerts: Vec<Alert>,
    /// Baseline distributions for drift detection
    baselines: BTreeMap<String, Vec<f64>>,
    /// Last alert timestamp per metric
    last_alert_time: BTreeMap<String, u64>,
}

impl ModelMonitor {
    /// Create a new model monitor
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            metrics: Vec::new(),
            alerts: Vec::new(),
            baselines: BTreeMap::new(),
            last_alert_time: BTreeMap::new(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(MonitoringConfig::default())
    }

    /// Record a metric
    pub fn record_metric(&mut self, metric: MetricPoint) -> MLOpsResult<()> {
        // Check thresholds
        if let Some(threshold) = self.config.thresholds.get(&metric.name) {
            let current_time = metric.timestamp;
            let metric_key = format!("{}_{}", metric.name, metric.tags.get("model").unwrap_or(&String::new()));

            // Check cooldown
            if let Some(&last_time) = self.last_alert_time.get(&metric_key) {
                if current_time.saturating_sub(last_time) < self.config.alert_cooldown_ns {
                    // Still in cooldown period
                    self.metrics.push(metric);
                    return Ok(());
                }
            }

            // Check critical threshold
            if threshold.check_critical(metric.value) {
                let alert = Alert::new(
                    AlertType::PerformanceDegradation,
                    AlertSeverity::Critical,
                    metric.tags.get("model").cloned().unwrap_or_default(),
                    metric.name.clone(),
                    metric.value,
                    threshold.critical,
                );
                self.trigger_alert(alert);
                self.last_alert_time.insert(metric_key, current_time);
            }
            // Check warning threshold
            else if threshold.check_warning(metric.value) {
                let alert = Alert::new(
                    AlertType::PerformanceDegradation,
                    AlertSeverity::Warning,
                    metric.tags.get("model").cloned().unwrap_or_default(),
                    metric.name.clone(),
                    metric.value,
                    threshold.warning,
                );
                self.trigger_alert(alert);
                self.last_alert_time.insert(metric_key, current_time);
            }
        }

        self.metrics.push(metric);
        Ok(())
    }

    /// Set baseline distribution for a feature
    pub fn set_baseline(&mut self, feature: String, data: Vec<f64>) {
        self.baselines.insert(feature, data);
    }

    /// Detect data drift
    pub fn detect_drift(&mut self, feature: String, current_data: &[f64]) -> MLOpsResult<DriftResult> {
        if !self.config.drift_detection_enabled {
            return Err(MonitoringError::DriftDetectionFailed(String::from(
                "Drift detection disabled",
            ))
            .into());
        }

        let baseline = self
            .baselines
            .get(&feature)
            .ok_or_else(|| {
                MLOpsError::MonitoringError(MonitoringError::DriftDetectionFailed(format!(
                    "No baseline for feature '{}'",
                    feature
                )))
            })?;

        // Perform Kolmogorov-Smirnov test (simplified)
        let result = self.ks_test(baseline, current_data);

        // Trigger alert if drift detected
        if result.drift_detected {
            let alert = Alert::new(
                AlertType::DataDrift,
                AlertSeverity::Warning,
                feature.clone(),
                String::from("drift_magnitude"),
                result.magnitude,
                0.5,
            );
            self.trigger_alert(alert);
        }

        Ok(result)
    }

    /// Kolmogorov-Smirnov test (simplified implementation)
    fn ks_test(&self, baseline: &[f64], current: &[f64]) -> DriftResult {
        // Calculate empirical CDFs and find maximum distance
        let mut sorted_baseline = baseline.to_vec();
        sorted_baseline.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut sorted_current = current.to_vec();
        sorted_current.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut max_distance = 0.0f64;
        let mut i = 0;
        let mut j = 0;
        let n = sorted_baseline.len();
        let m = sorted_current.len();

        while i < n && j < m {
            if sorted_baseline[i] < sorted_current[j] {
                i += 1;
            } else {
                j += 1;
            }

            let cdf_baseline = i as f64 / n as f64;
            let cdf_current = j as f64 / m as f64;
            let distance = (cdf_baseline - cdf_current).abs();

            if distance > max_distance {
                max_distance = distance;
            }
        }

        // Simplified p-value calculation
        let statistic = max_distance;
        let p_value = if statistic > 0.3 {
            0.01 // Significant drift
        } else if statistic > 0.2 {
            0.05 // Moderate drift
        } else {
            0.5 // No significant drift
        };

        DriftResult::new(String::from("feature"), p_value, statistic)
    }

    /// Update performance metrics
    pub fn update_performance(&mut self, metrics: PerformanceMetrics) -> MLOpsResult<()> {
        if !self.config.performance_monitoring_enabled {
            return Ok(());
        }

        let timestamp = get_current_time_ns();

        if let Some(accuracy) = metrics.accuracy {
            let mut metric = MetricPoint::new(String::from("accuracy"), accuracy, timestamp);
            metric.tags.insert(String::from("model"), metrics.model_name.clone());
            self.record_metric(metric)?;
        }

        if let Some(latency_ns) = metrics.avg_latency_ns {
            let latency_ms = latency_ns as f64 / 1_000_000.0;
            let mut metric = MetricPoint::new(String::from("latency_ms"), latency_ms, timestamp);
            metric.tags.insert(String::from("model"), metrics.model_name.clone());
            self.record_metric(metric)?;
        }

        if let Some(error_rate) = metrics.error_rate {
            let mut metric = MetricPoint::new(String::from("error_rate"), error_rate, timestamp);
            metric.tags.insert(String::from("model"), metrics.model_name.clone());
            self.record_metric(metric)?;
        }

        Ok(())
    }

    /// Update resource metrics
    pub fn update_resources(&mut self, metrics: ResourceMetrics) -> MLOpsResult<()> {
        if !self.config.resource_monitoring_enabled {
            return Ok(());
        }

        let timestamp = get_current_time_ns();

        // CPU usage
        let cpu_metric = MetricPoint::new(String::from("cpu_usage"), metrics.cpu_usage_percent, timestamp);
        self.record_metric(cpu_metric)?;

        // Memory usage
        let mem_metric = MetricPoint::new(
            String::from("memory_usage"),
            metrics.memory_usage_percent,
            timestamp,
        );
        self.record_metric(mem_metric)?;

        Ok(())
    }

    /// Trigger an alert
    fn trigger_alert(&mut self, alert: Alert) {
        self.alerts.push(alert);

        // Keep only last 1000 alerts
        if self.alerts.len() > 1000 {
            self.alerts.remove(0);
        }
    }

    /// Get active alerts
    pub fn get_alerts(&self) -> &[Alert] {
        &self.alerts
    }

    /// Get alerts by severity
    pub fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Vec<&Alert> {
        self.alerts
            .iter()
            .filter(|a| a.severity == severity)
            .collect()
    }

    /// Get metric history
    pub fn get_metrics(&self, metric_name: &str) -> Vec<&MetricPoint> {
        self.metrics
            .iter()
            .filter(|m| m.name == metric_name)
            .collect()
    }

    /// Clear old metrics
    pub fn clear_old_metrics(&mut self, older_than_ns: u64) {
        let current_time = get_current_time_ns();
        self.metrics
            .retain(|m| current_time.saturating_sub(m.timestamp) < older_than_ns);
    }

    /// Get monitoring summary
    pub fn get_summary(&self) -> MonitoringSummary {
        let total_alerts = self.alerts.len();
        let critical_alerts = self.alerts.iter().filter(|a| a.severity == AlertSeverity::Critical).count();
        let warning_alerts = self.alerts.iter().filter(|a| a.severity == AlertSeverity::Warning).count();

        MonitoringSummary {
            total_metrics: self.metrics.len(),
            total_alerts,
            critical_alerts,
            warning_alerts,
        }
    }
}

/// Monitoring summary
#[derive(Debug, Clone)]
pub struct MonitoringSummary {
    /// Total number of metrics recorded
    pub total_metrics: usize,
    /// Total number of alerts
    pub total_alerts: usize,
    /// Number of critical alerts
    pub critical_alerts: usize,
    /// Number of warning alerts
    pub warning_alerts: usize,
}

/// Generate alert ID
fn generate_alert_id() -> String {
    use core::fmt::Write;
    let mut id = String::with_capacity(16);
    write!(&mut id, "alert_{}", generate_counter()).unwrap();
    id
}

/// Simple counter
static mut COUNTER: u64 = 0;

fn generate_counter() -> u64 {
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

/// Get current time in nanoseconds
fn get_current_time_ns() -> u64 {
    unsafe {
        COUNTER += 1;
        COUNTER * 1_000_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_point() {
        let metric = MetricPoint::new(String::from("accuracy"), 0.95, 1000);
        assert_eq!(metric.name, "accuracy");
        assert_eq!(metric.value, 0.95);
    }

    #[test]
    fn test_threshold_check() {
        let threshold = Threshold::new(ThresholdType::Lower, 0.9, 0.85);

        assert!(!threshold.check_warning(0.95));
        assert!(threshold.check_warning(0.88));
        assert!(threshold.check_critical(0.80));
    }

    #[test]
    fn test_distribution_stats() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = DistributionStats::calculate(&data);

        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
    }

    #[test]
    fn test_alert_creation() {
        let alert = Alert::new(
            AlertType::DataDrift,
            AlertSeverity::Critical,
            String::from("model"),
            String::from("drift_score"),
            0.8,
            0.5,
        );

        assert_eq!(alert.severity, AlertSeverity::Critical);
        assert!(alert.message.contains("drift_score"));
    }

    #[test]
    fn test_monitoring_config() {
        let config = MonitoringConfig::default();

        assert!(config.drift_detection_enabled);
        assert!(config.performance_monitoring_enabled);
        assert!(config.resource_monitoring_enabled);
    }

    #[test]
    fn test_model_monitor() {
        let monitor = ModelMonitor::with_defaults();

        // Set baseline
        monitor.baselines.insert(String::from("feature"), vec![1.0, 2.0, 3.0]);

        // Detect drift
        let current_data = vec![1.0, 2.0, 3.0];
        let result = monitor.detect_drift(String::from("feature"), &current_data);

        assert!(result.is_ok());
        let drift = result.unwrap();
        assert!(!drift.drift_detected); // No drift with similar data
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new(String::from("model"))
            .with_accuracy(0.95)
            .with_latency(100_000)
            .with_error_rate(0.01);

        assert_eq!(metrics.accuracy, Some(0.95));
        assert_eq!(metrics.avg_latency_ns, Some(100_000));
    }

    #[test]
    fn test_metric_thresholds() {
        let mut monitor = ModelMonitor::with_defaults();

        let mut metric = MetricPoint::new(String::from("accuracy"), 0.80, 1000);
        metric.tags.insert(String::from("model"), String::from("test"));

        monitor.record_metric(metric).unwrap();

        // Should trigger an alert (below threshold 0.85)
        assert!(!monitor.get_alerts().is_empty());
    }
}
