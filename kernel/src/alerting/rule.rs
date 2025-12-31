//! Alert rule definitions and evaluation logic
//!
//! This module provides a comprehensive rule-based alerting system with support for:
//! - Threshold-based alerts (static and dynamic thresholds)
//! - Pattern-based alerts (rate changes, trends)
//! - Anomaly detection alerts (statistical anomalies)
//! - Composite rules (AND/OR/NOT logic)
//! - Rule validation and testing

#![no_std]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt,
    time::Duration,
};

use crate::monitoring::metrics::{MetricValue, MetricType};

/// Alert rule definition
#[derive(Clone, Debug)]
pub struct AlertRule {
    /// Unique rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule type
    pub rule_type: RuleType,
    /// Severity level
    pub severity: Severity,
    /// Labels to attach to alerts
    pub labels: BTreeMap<String, String>,
    /// Annotations for alert metadata
    pub annotations: BTreeMap<String, String>,
    /// Evaluation interval
    pub interval: Duration,
    /// Rule configuration
    pub config: RuleConfig,
    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Rule evaluation strategy
#[derive(Clone, Debug, PartialEq)]
pub enum RuleType {
    /// Threshold-based rule
    Threshold,
    /// Rate-based rule
    Rate,
    /// Anomaly detection rule
    Anomaly,
    /// Composite rule combining multiple conditions
    Composite,
}

/// Severity levels for alerts
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational only
    Info,
    /// Warning condition
    Warning,
    /// Critical condition
    Critical,
    /// Emergency condition
    Emergency,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
            Self::Emergency => write!(f, "emergency"),
        }
    }
}

/// Rule configuration
#[derive(Clone, Debug)]
pub enum RuleConfig {
    /// Threshold rule configuration
    Threshold(ThresholdConfig),
    /// Rate rule configuration
    Rate(RateConfig),
    /// Anomaly detection configuration
    Anomaly(AnomalyConfig),
    /// Composite rule configuration
    Composite(CompositeConfig),
}

/// Threshold-based rule configuration
#[derive(Clone, Debug)]
pub struct ThresholdConfig {
    /// Metric name to query
    pub metric: String,
    /// Label selectors for the metric
    pub labels: BTreeMap<String, String>,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Threshold value
    pub threshold: f64,
    /// Duration the condition must be true
    pub for_duration: Option<Duration>,
}

/// Comparison operators
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComparisonOperator {
    /// Equal to
    Eq,
    /// Not equal to
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal to
    Gte,
    /// Less than
    Lt,
    /// Less than or equal to
    Lte,
}

/// Rate-based rule configuration
#[derive(Clone, Debug)]
pub struct RateConfig {
    /// Metric name to query
    pub metric: String,
    /// Label selectors for the metric
    pub labels: BTreeMap<String, String>,
    /// Time window for rate calculation
    pub window: Duration,
    /// Comparison operator
    pub operator: ComparisonOperator,
    /// Threshold value
    pub threshold: f64,
    /// Duration the condition must be true
    pub for_duration: Option<Duration>,
}

/// Anomaly detection configuration
#[derive(Clone, Debug)]
pub struct AnomalyConfig {
    /// Metric name to analyze
    pub metric: String,
    /// Label selectors for the metric
    pub labels: BTreeMap<String, String>,
    /// Anomaly detection algorithm
    pub algorithm: AnomalyAlgorithm,
    /// Sensitivity level (0.0 to 1.0)
    pub sensitivity: f64,
    /// Time window for analysis
    pub window: Duration,
    /// Minimum number of data points required
    pub min_data_points: usize,
}

/// Anomaly detection algorithms
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnomalyAlgorithm {
    /// Z-score based detection
    ZScore,
    /// Modified Z-score (median-based)
    ModifiedZScore,
    /// Interquartile range
    IQR,
    /// Exponential moving average
    EMA,
    /// Machine learning based
    ML,
}

/// Composite rule configuration
#[derive(Clone, Debug)]
pub struct CompositeConfig {
    /// Sub-rules to combine
    pub rules: Vec<AlertRule>,
    /// Logical operator for combining rules
    pub operator: LogicalOperator,
}

/// Logical operators for composite rules
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogicalOperator {
    /// All conditions must be true
    And,
    /// At least one condition must be true
    Or,
    /// Condition must be false
    Not,
}

/// Rule evaluation result
#[derive(Clone, Debug)]
pub struct EvaluationResult {
    /// Whether the rule fired
    pub fired: bool,
    /// Current metric value
    pub value: Option<f64>,
    /// Calculated threshold (for dynamic thresholds)
    pub threshold: Option<f64>,
    /// Evaluation time
    pub evaluated_at: core::time::Instant,
    /// Reason for firing (if fired)
    pub reason: Option<String>,
}

/// Rule validation error
#[derive(Clone, Debug)]
pub enum RuleValidationError {
    /// Invalid rule ID
    InvalidId(String),
    /// Invalid threshold value
    InvalidThreshold(String),
    /// Invalid time window
    InvalidWindow(String),
    /// Invalid metric name
    InvalidMetric(String),
    /// Missing required field
    MissingField(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// Empty composite rule
    EmptyComposite,
}

impl fmt::Display for RuleValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "Invalid rule ID: {}", id),
            Self::InvalidThreshold(msg) => write!(f, "Invalid threshold: {}", msg),
            Self::InvalidWindow(msg) => write!(f, "Invalid time window: {}", msg),
            Self::InvalidMetric(msg) => write!(f, "Invalid metric: {}", msg),
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::EmptyComposite => write!(f, "Composite rule must have at least one sub-rule"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RuleValidationError {}

/// Validate a comparison with the configured operator
impl ComparisonOperator {
    /// Evaluate the comparison
    pub fn evaluate(&self, value: f64, threshold: f64) -> bool {
        match self {
            Self::Eq => (value - threshold).abs() < f64::EPSILON,
            Self::Ne => (value - threshold).abs() >= f64::EPSILON,
            Self::Gt => value > threshold,
            Self::Gte => value >= threshold,
            Self::Lt => value < threshold,
            Self::Lte => value <= threshold,
        }
    }
}

impl AlertRule {
    /// Create a new threshold-based alert rule
    pub fn new_threshold(
        id: String,
        name: String,
        description: String,
        metric: String,
        operator: ComparisonOperator,
        threshold: f64,
        severity: Severity,
        interval: Duration,
    ) -> Self {
        let labels = BTreeMap::new();
        let annotations = BTreeMap::new();

        Self {
            id,
            name,
            description,
            rule_type: RuleType::Threshold,
            severity,
            labels,
            annotations,
            interval,
            config: RuleConfig::Threshold(ThresholdConfig {
                metric,
                labels: BTreeMap::new(),
                operator,
                threshold,
                for_duration: None,
            }),
            enabled: true,
        }
    }

    /// Create a new rate-based alert rule
    pub fn new_rate(
        id: String,
        name: String,
        description: String,
        metric: String,
        window: Duration,
        operator: ComparisonOperator,
        threshold: f64,
        severity: Severity,
        interval: Duration,
    ) -> Self {
        Self {
            id,
            name,
            description,
            rule_type: RuleType::Rate,
            severity,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            interval,
            config: RuleConfig::Rate(RateConfig {
                metric,
                labels: BTreeMap::new(),
                window,
                operator,
                threshold,
                for_duration: None,
            }),
            enabled: true,
        }
    }

    /// Create a new anomaly detection rule
    pub fn new_anomaly(
        id: String,
        name: String,
        description: String,
        metric: String,
        algorithm: AnomalyAlgorithm,
        sensitivity: f64,
        window: Duration,
        severity: Severity,
        interval: Duration,
    ) -> Self {
        Self {
            id,
            name,
            description,
            rule_type: RuleType::Anomaly,
            severity,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            interval,
            config: RuleConfig::Anomaly(AnomalyConfig {
                metric,
                labels: BTreeMap::new(),
                algorithm,
                sensitivity,
                window,
                min_data_points: 10,
            }),
            enabled: true,
        }
    }

    /// Add a label to the rule
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// Add an annotation to the rule
    pub fn with_annotation(mut self, key: String, value: String) -> Self {
        self.annotations.insert(key, value);
        self
    }

    /// Set the "for" duration for threshold and rate rules
    pub fn with_for_duration(mut self, duration: Duration) -> Self {
        match &mut self.config {
            RuleConfig::Threshold(config) => config.for_duration = Some(duration),
            RuleConfig::Rate(config) => config.for_duration = Some(duration),
            _ => {}
        }
        self
    }

    /// Validate the rule configuration
    pub fn validate(&self) -> Result<(), RuleValidationError> {
        // Validate ID
        if self.id.is_empty() {
            return Err(RuleValidationError::InvalidId("ID cannot be empty".into()));
        }

        // Validate name
        if self.name.is_empty() {
            return Err(RuleValidationError::MissingField("name".into()));
        }

        // Validate interval
        if self.interval.as_secs() == 0 {
            return Err(RuleValidationError::InvalidWindow(
                "Interval must be greater than 0".into(),
            ));
        }

        // Validate configuration
        match &self.config {
            RuleConfig::Threshold(config) => {
                if config.metric.is_empty() {
                    return Err(RuleValidationError::InvalidMetric(
                        "Metric name cannot be empty".into(),
                    ));
                }
                if !config.threshold.is_finite() {
                    return Err(RuleValidationError::InvalidThreshold(
                        "Threshold must be finite".into(),
                    ));
                }
            }
            RuleConfig::Rate(config) => {
                if config.metric.is_empty() {
                    return Err(RuleValidationError::InvalidMetric(
                        "Metric name cannot be empty".into(),
                    ));
                }
                if config.window.as_secs() == 0 {
                    return Err(RuleValidationError::InvalidWindow(
                        "Rate window must be greater than 0".into(),
                    ));
                }
                if !config.threshold.is_finite() {
                    return Err(RuleValidationError::InvalidThreshold(
                        "Threshold must be finite".into(),
                    ));
                }
            }
            RuleConfig::Anomaly(config) => {
                if config.metric.is_empty() {
                    return Err(RuleValidationError::InvalidMetric(
                        "Metric name cannot be empty".into(),
                    ));
                }
                if !(0.0..=1.0).contains(&config.sensitivity) {
                    return Err(RuleValidationError::InvalidConfig(
                        "Sensitivity must be between 0.0 and 1.0".into(),
                    ));
                }
                if config.window.as_secs() == 0 {
                    return Err(RuleValidationError::InvalidWindow(
                        "Analysis window must be greater than 0".into(),
                    ));
                }
                if config.min_data_points < 2 {
                    return Err(RuleValidationError::InvalidConfig(
                        "Min data points must be at least 2".into(),
                    ));
                }
            }
            RuleConfig::Composite(config) => {
                if config.rules.is_empty() {
                    return Err(RuleValidationError::EmptyComposite);
                }
            }
        }

        Ok(())
    }

    /// Evaluate the rule with the given metric values
    pub fn evaluate(
        &self,
        metric_values: &BTreeMap<String, MetricValue>,
    ) -> Result<EvaluationResult, RuleValidationError> {
        if !self.enabled {
            return Ok(EvaluationResult {
                fired: false,
                value: None,
                threshold: None,
                evaluated_at: core::time::Instant::now(),
                reason: Some("Rule is disabled".into()),
            });
        }

        match &self.config {
            RuleConfig::Threshold(config) => {
                self.evaluate_threshold(config, metric_values)
            }
            RuleConfig::Rate(config) => self.evaluate_rate(config, metric_values),
            RuleConfig::Anomaly(config) => self.evaluate_anomaly(config, metric_values),
            RuleConfig::Composite(config) => self.evaluate_composite(config, metric_values),
        }
    }

    /// Evaluate a threshold-based rule
    fn evaluate_threshold(
        &self,
        config: &ThresholdConfig,
        metric_values: &BTreeMap<String, MetricValue>,
    ) -> Result<EvaluationResult, RuleValidationError> {
        let value = metric_values
            .get(&config.metric)
            .and_then(|v| v.as_float())
            .ok_or_else(|| {
                RuleValidationError::InvalidMetric(format!("Metric {} not found or not a number", config.metric))
            })?;

        let fired = config.operator.evaluate(*value, config.threshold);

        Ok(EvaluationResult {
            fired,
            value: Some(*value),
            threshold: Some(config.threshold),
            evaluated_at: core::time::Instant::now(),
            reason: if fired {
                Some(format!(
                    "{} {} {}",
                    config.metric, config.operator, config.threshold
                ))
            } else {
                None
            },
        })
    }

    /// Evaluate a rate-based rule
    fn evaluate_rate(
        &self,
        config: &RateConfig,
        _metric_values: &BTreeMap<String, MetricValue>,
    ) -> Result<EvaluationResult, RuleValidationError> {
        // In a real implementation, this would calculate the rate from historical data
        // For now, we return a placeholder result
        Ok(EvaluationResult {
            fired: false,
            value: None,
            threshold: Some(config.threshold),
            evaluated_at: core::time::Instant::now(),
            reason: Some("Rate calculation requires historical data".into()),
        })
    }

    /// Evaluate an anomaly detection rule
    fn evaluate_anomaly(
        &self,
        config: &AnomalyConfig,
        _metric_values: &BTreeMap<String, MetricValue>,
    ) -> Result<EvaluationResult, RuleValidationError> {
        // In a real implementation, this would analyze historical data
        // For now, we return a placeholder result
        Ok(EvaluationResult {
            fired: false,
            value: None,
            threshold: None,
            evaluated_at: core::time::Instant::now(),
            reason: Some("Anomaly detection requires historical data".into()),
        })
    }

    /// Evaluate a composite rule
    fn evaluate_composite(
        &self,
        config: &CompositeConfig,
        metric_values: &BTreeMap<String, MetricValue>,
    ) -> Result<EvaluationResult, RuleValidationError> {
        let mut results = Vec::new();
        for rule in &config.rules {
            results.push(rule.evaluate(metric_values)?);
        }

        let fired = match config.operator {
            LogicalOperator::And => results.iter().all(|r| r.fired),
            LogicalOperator::Or => results.iter().any(|r| r.fired),
            LogicalOperator::Not => !results.iter().all(|r| r.fired),
        };

        Ok(EvaluationResult {
            fired,
            value: None,
            threshold: None,
            evaluated_at: core::time::Instant::now(),
            reason: Some(format!("Composite rule: {} sub-rules evaluated", results.len())),
        })
    }

    /// Generate a summary of the rule
    pub fn summary(&self) -> String {
        format!(
            "Rule '{}' ({}): {} - {}",
            self.name,
            self.id,
            self.rule_type_str(),
            self.severity
        )
    }

    /// Get a string representation of the rule type
    fn rule_type_str(&self) -> &str {
        match self.rule_type {
            RuleType::Threshold => "Threshold",
            RuleType::Rate => "Rate",
            RuleType::Anomaly => "Anomaly",
            RuleType::Composite => "Composite",
        }
    }
}

/// Rule builder for constructing rules with a fluent API
pub struct RuleBuilder {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    severity: Severity,
    interval: Duration,
    labels: BTreeMap<String, String>,
    annotations: BTreeMap<String, String>,
}

impl RuleBuilder {
    /// Create a new rule builder
    pub fn new() -> Self {
        Self {
            id: None,
            name: None,
            description: None,
            severity: Severity::Warning,
            interval: Duration::from_secs(60),
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
        }
    }

    /// Set the rule ID
    pub fn id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the rule name
    pub fn name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the rule description
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the severity level
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the evaluation interval
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Add a label
    pub fn label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// Add an annotation
    pub fn annotation(mut self, key: String, value: String) -> Self {
        self.annotations.insert(key, value);
        self
    }

    /// Build a threshold rule
    pub fn build_threshold(
        self,
        metric: String,
        operator: ComparisonOperator,
        threshold: f64,
    ) -> Result<AlertRule, RuleValidationError> {
        let id = self.id.ok_or_else(|| RuleValidationError::MissingField("id".into()))?;
        let name = self
            .name
            .ok_or_else(|| RuleValidationError::MissingField("name".into()))?;
        let description = self.description.unwrap_or_default();

        let mut rule = AlertRule::new_threshold(
            id,
            name,
            description,
            metric,
            operator,
            threshold,
            self.severity,
            self.interval,
        );

        rule.labels = self.labels;
        rule.annotations = self.annotations;
        rule.validate()?;

        Ok(rule)
    }

    /// Build a rate rule
    pub fn build_rate(
        self,
        metric: String,
        window: Duration,
        operator: ComparisonOperator,
        threshold: f64,
    ) -> Result<AlertRule, RuleValidationError> {
        let id = self.id.ok_or_else(|| RuleValidationError::MissingField("id".into()))?;
        let name = self
            .name
            .ok_or_else(|| RuleValidationError::MissingField("name".into()))?;
        let description = self.description.unwrap_or_default();

        let mut rule = AlertRule::new_rate(
            id,
            name,
            description,
            metric,
            window,
            operator,
            threshold,
            self.severity,
            self.interval,
        );

        rule.labels = self.labels;
        rule.annotations = self.annotations;
        rule.validate()?;

        Ok(rule)
    }

    /// Build an anomaly detection rule
    pub fn build_anomaly(
        self,
        metric: String,
        algorithm: AnomalyAlgorithm,
        sensitivity: f64,
        window: Duration,
    ) -> Result<AlertRule, RuleValidationError> {
        let id = self.id.ok_or_else(|| RuleValidationError::MissingField("id".into()))?;
        let name = self
            .name
            .ok_or_else(|| RuleValidationError::MissingField("name".into()))?;
        let description = self.description.unwrap_or_default();

        let mut rule = AlertRule::new_anomaly(
            id,
            name,
            description,
            metric,
            algorithm,
            sensitivity,
            window,
            self.severity,
            self.interval,
        );

        rule.labels = self.labels;
        rule.annotations = self.annotations;
        rule.validate()?;

        Ok(rule)
    }
}

impl Default for RuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::metrics::MetricValue;

    #[test]
    fn test_threshold_rule_greater_than() {
        let rule = AlertRule::new_threshold(
            "test-1".into(),
            "CPU High".into(),
            "CPU usage is high".into(),
            "cpu_usage".into(),
            ComparisonOperator::Gt,
            80.0,
            Severity::Warning,
            Duration::from_secs(60),
        );

        let mut metrics = BTreeMap::new();
        metrics.insert("cpu_usage".into(), MetricValue::Gauge(85.0));

        let result = rule.evaluate(&metrics).unwrap();
        assert!(result.fired);
        assert_eq!(result.value, Some(85.0));
    }

    #[test]
    fn test_threshold_rule_not_fired() {
        let rule = AlertRule::new_threshold(
            "test-2".into(),
            "CPU High".into(),
            "CPU usage is high".into(),
            "cpu_usage".into(),
            ComparisonOperator::Gt,
            80.0,
            Severity::Warning,
            Duration::from_secs(60),
        );

        let mut metrics = BTreeMap::new();
        metrics.insert("cpu_usage".into(), MetricValue::Gauge(75.0));

        let result = rule.evaluate(&metrics).unwrap();
        assert!(!result.fired);
    }

    #[test]
    fn test_comparison_operators() {
        assert!(ComparisonOperator::Gt.evaluate(10.0, 5.0));
        assert!(ComparisonOperator::Gte.evaluate(5.0, 5.0));
        assert!(ComparisonOperator::Lt.evaluate(3.0, 5.0));
        assert!(ComparisonOperator::Lte.evaluate(5.0, 5.0));
        assert!(ComparisonOperator::Eq.evaluate(5.0, 5.0));
        assert!(ComparisonOperator::Ne.evaluate(5.0, 6.0));
    }

    #[test]
    fn test_rule_validation() {
        let rule = AlertRule::new_threshold(
            "test-3".into(),
            "Test Rule".into(),
            "Test description".into(),
            "test_metric".into(),
            ComparisonOperator::Gt,
            50.0,
            Severity::Info,
            Duration::from_secs(10),
        );

        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_rule_builder() {
        let rule = RuleBuilder::new()
            .id("builder-test".into())
            .name("Builder Test".into())
            .description("Testing the builder".into())
            .severity(Severity::Critical)
            .label("environment".into(), "production".into())
            .annotation("runbook".into(), "https://example.com".into())
            .build_threshold("memory_usage".into(), ComparisonOperator::Gt, 90.0)
            .unwrap();

        assert_eq!(rule.id, "builder-test");
        assert_eq!(rule.severity, Severity::Critical);
        assert_eq!(rule.labels.get("environment"), Some(&"production".into()));
    }

    #[test]
    fn test_composite_rule() {
        let rule1 = AlertRule::new_threshold(
            "sub-1".into(),
            "CPU High".into(),
            "CPU is high".into(),
            "cpu".into(),
            ComparisonOperator::Gt,
            80.0,
            Severity::Warning,
            Duration::from_secs(60),
        );

        let rule2 = AlertRule::new_threshold(
            "sub-2".into(),
            "Memory High".into(),
            "Memory is high".into(),
            "memory".into(),
            ComparisonOperator::Gt,
            80.0,
            Severity::Warning,
            Duration::from_secs(60),
        );

        let composite_rule = AlertRule {
            id: "composite-1".into(),
            name: "System Overload".into(),
            description: "Both CPU and memory are high".into(),
            rule_type: RuleType::Composite,
            severity: Severity::Critical,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            interval: Duration::from_secs(60),
            config: RuleConfig::Composite(CompositeConfig {
                rules: vec![rule1, rule2],
                operator: LogicalOperator::And,
            }),
            enabled: true,
        };

        assert!(composite_rule.validate().is_ok());
    }

    #[test]
    fn test_invalid_threshold() {
        let rule = AlertRule {
            id: "invalid".into(),
            name: "Invalid".into(),
            description: "Invalid threshold".into(),
            rule_type: RuleType::Threshold,
            severity: Severity::Warning,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
            interval: Duration::from_secs(60),
            config: RuleConfig::Threshold(ThresholdConfig {
                metric: "test".into(),
                labels: BTreeMap::new(),
                operator: ComparisonOperator::Gt,
                threshold: f64::NAN,
                for_duration: None,
            }),
            enabled: true,
        };

        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Emergency > Severity::Critical);
    }

    #[test]
    fn test_rule_summary() {
        let rule = AlertRule::new_threshold(
            "summary-test".into(),
            "Test Summary".into(),
            "Testing summary".into(),
            "metric".into(),
            ComparisonOperator::Gt,
            100.0,
            Severity::Info,
            Duration::from_secs(30),
        );

        let summary = rule.summary();
        assert!(summary.contains("Test Summary"));
        assert!(summary.contains("summary-test"));
        assert!(summary.contains("Threshold"));
    }
}
