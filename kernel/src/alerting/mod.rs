//! Alerting and notification system for the NOS kernel
//!
//! This module provides a comprehensive alerting system with the following features:
//!
//! # Alert Rules
//! - Threshold-based alerts (static and dynamic thresholds)
//! - Rate-based alerts (rate of change detection)
//! - Anomaly detection alerts (statistical and ML-based)
//! - Composite rules (AND/OR/NOT logic)
//!
//! # Alert Evaluation
//! - Periodic rule evaluation at configurable intervals
//! - Streaming evaluation for real-time metrics
//! - Alert state machine (Pending → Firing → Resolved)
//! - Alert deduplication and grouping
//! - Hysteresis to prevent alert flapping
//!
//! # Notifications
//! - Multiple notification channels (Email, SMS, Webhook, Slack)
//! - Notification templates with variable substitution
//! - Retry logic with exponential backoff
//! - Rate limiting per channel
//! - Notification batching for efficiency
//!
//! # Silencing and Suppression
//! - Alert silencing rules (time-based, label-based)
//! - Alert inhibition (dependency-based suppression)
//! - Maintenance windows for scheduled downtime
//! - Silence expiration and management API
//!
//! # Integration
//! - Prometheus AlertManager compatibility
//! - Integration with kernel metrics system
//! - Integration with kernel logging system
//! - REST API for alert management
//!
//! # Example
//!
//! ```ignore
//! use kernel::alerting::{AlertManager, EvaluationContext};
//! use kernel::alerting::rule::{AlertRule, ComparisonOperator, Severity};
//! use core::time::Duration;
//!
//! // Create an alert manager
//! let mut alert_manager = AlertManager::new();
//!
//! // Add a threshold-based rule
//! let rule = AlertRule::new_threshold(
//!     "high-cpu".into(),
//!     "High CPU Usage".into(),
//!     "CPU usage exceeds 80%".into(),
//!     "cpu_usage".into(),
//!     ComparisonOperator::Gt,
//!     80.0,
//!     Severity::Warning,
//!     Duration::from_secs(60),
//! );
//!
//! alert_manager.add_rule(rule);
//!
//! // Evaluate metrics
//! let mut context = EvaluationContext::new();
//! context.add_metric("cpu_usage".into(), 85.0);
//!
//! let alerts = alert_manager.evaluate(&context);
//! for alert in alerts {
//!     println!("Alert fired: {}", alert.rule_id);
//! }
//! ```

#![no_std]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String,
    ToString},
    sync::Arc,
    vec::Vec,
};
use core::{
    fmt,
    time::Duration,
};

pub mod rule;
pub mod evaluator;
pub mod notifier;
pub mod silencing;

pub use rule::{
    AlertRule, ComparisonOperator, RuleConfig, RuleType, Severity,
    RuleBuilder, ThresholdConfig, RateConfig, AnomalyConfig,
    AnomalyAlgorithm, LogicalOperator, CompositeConfig,
    RuleValidationError,
};

pub use evaluator::{
    Alert, AlertState, EvaluationContext, AlertEvaluator,
    EvaluationError, EvaluatorConfig, EvaluatorStatistics,
    StreamingEvaluator,
};

pub use notifier::{
    NotificationChannel, Notification, DeliveryStatus,
    EmailConfig, SmsConfig, WebhookConfig, SlackConfig,
    NotificationTemplate, NotificationManager,
    NotificationError, NotifierConfig, RateLimiter,
    HttpMethod, PrometheusNotification,
};

pub use silencing::{
    SilenceRule, SilenceType, Matcher, MatchOp,
    InhibitionRule, MaintenanceWindow, MaintenanceType,
    SilenceManager, SilenceQueryBuilder, SilencingError,
};

use crate::monitoring::metrics::{MetricType, MetricValue};

/// Alert manager - main entry point for the alerting system
pub struct AlertManager {
    /// Rule evaluator
    evaluator: AlertEvaluator,
    /// Notification manager
    notifier: NotificationManager,
    /// Silence manager
    silence_manager: SilenceManager,
    /// Configuration
    config: AlertManagerConfig,
    /// Alert statistics
    stats: AlertStatistics,
}

/// Alert manager configuration
#[derive(Clone, Debug)]
pub struct AlertManagerConfig {
    /// Evaluator configuration
    pub evaluator: EvaluatorConfig,
    /// Notifier configuration
    pub notifier: NotifierConfig,
    /// Enable automatic notifications
    pub auto_notify: bool,
    /// Enable alert history tracking
    pub enable_history: bool,
    /// Maximum history size
    pub max_history_size: usize,
}

impl Default for AlertManagerConfig {
    fn default() -> Self {
        Self {
            evaluator: EvaluatorConfig::default(),
            notifier: NotifierConfig::default(),
            auto_notify: true,
            enable_history: true,
            max_history_size: 10000,
        }
    }
}

/// Alert statistics
#[derive(Clone, Debug, Default)]
pub struct AlertStatistics {
    /// Total alerts fired
    pub total_fired: usize,
    /// Total alerts resolved
    pub total_resolved: usize,
    /// Total notifications sent
    pub total_notifications: usize,
    /// Total notifications failed
    pub failed_notifications: usize,
    /// Current active alerts
    pub active_alerts: usize,
    /// Silenced alerts
    pub silenced_alerts: usize,
    /// Inhibited alerts
    pub inhibited_alerts: usize,
}

/// Alert manager error
#[derive(Clone, Debug)]
pub enum AlertManagerError {
    /// Evaluator error
    Evaluator(String),
    /// Notifier error
    Notifier(String),
    /// Silencing error
    Silencing(String),
    /// Configuration error
    Config(String),
}

impl fmt::Display for AlertManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluator(msg) => write!(f, "Evaluator error: {}", msg),
            Self::Notifier(msg) => write!(f, "Notifier error: {}", msg),
            Self::Silencing(msg) => write!(f, "Silencing error: {}", msg),
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AlertManagerError {}

impl AlertManager {
    /// Create a new alert manager
    pub fn new() -> Self {
        Self::with_config(AlertManagerConfig::default())
    }

    /// Create an alert manager with custom configuration
    pub fn with_config(config: AlertManagerConfig) -> Self {
        Self {
            evaluator: AlertEvaluator::new(config.evaluator.clone()),
            notifier: NotificationManager::new(config.notifier.clone()),
            silence_manager: SilenceManager::new(),
            config,
            stats: AlertStatistics::default(),
        }
    }

    /// Add an alert rule
    pub fn add_rule(&mut self, rule: AlertRule) -> Result<(), AlertManagerError> {
        self.evaluator
            .add_rule(rule)
            .map_err(|e| AlertManagerError::Evaluator(e.to_string()))?;
        Ok(())
    }

    /// Remove an alert rule
    pub fn remove_rule(&mut self, rule_id: &str) -> Result<(), AlertManagerError> {
        self.evaluator
            .remove_rule(rule_id)
            .map_err(|e| AlertManagerError::Evaluator(e.to_string()))?;
        Ok(())
    }

    /// Add a notification channel
    pub fn add_notification_channel(&mut self, channel: NotificationChannel) {
        self.notifier.add_channel(channel);
    }

    /// Add a silence rule
    pub fn add_silence(&mut self, silence: SilenceRule) -> Result<(), AlertManagerError> {
        self.silence_manager
            .add_silence(silence)
            .map_err(|e| AlertManagerError::Silencing(e.to_string()))?;
        Ok(())
    }

    /// Add an inhibition rule
    pub fn add_inhibition(&mut self, inhibition: InhibitionRule) -> Result<(), AlertManagerError> {
        self.silence_manager
            .add_inhibition(inhibition)
            .map_err(|e| AlertManagerError::Silencing(e.to_string()))?;
        Ok(())
    }

    /// Create a maintenance window
    pub fn create_maintenance(
        &mut self,
        window: MaintenanceWindow,
    ) -> Result<(), AlertManagerError> {
        self.silence_manager
            .create_maintenance(window)
            .map_err(|e| AlertManagerError::Silencing(e.to_string()))?;
        Ok(())
    }

    /// Evaluate metrics and generate alerts
    pub fn evaluate(&mut self, context: &EvaluationContext) -> Result<Vec<Alert>, AlertManagerError> {
        // Update silence manager (expire old silences)
        self.silence_manager.update();

        // Evaluate all rules
        let mut new_alerts = self
            .evaluator
            .evaluate_all(context)
            .map_err(|e| AlertManagerError::Evaluator(e.to_string()))?;

        // Check pending alerts that should fire
        let firing_alerts = self.evaluator.check_pending_alerts();
        new_alerts.extend(firing_alerts);

        // Filter alerts through silencing and inhibition
        let mut final_alerts = Vec::new();
        for alert in new_alerts {
            // Check if alert should be silenced
            if let Some(_silence) = self.silence_manager.should_silence(&alert) {
                self.stats.silenced_alerts += 1;
                continue;
            }

            // Check if alert is in maintenance window
            if let Some(_window) = self.silence_manager.in_maintenance(&alert) {
                self.stats.silenced_alerts += 1;
                continue;
            }

            // Check if alert should be inhibited
            let all_alerts = self.evaluator.alerts();
            let all_alert_vec: Vec<Alert> = all_alerts.values().cloned().collect();
            if self.silence_manager.should_inhibit(&alert, &all_alert_vec) {
                self.stats.inhibited_alerts += 1;
                continue;
            }

            final_alerts.push(alert);
        }

        // Send notifications for firing alerts
        if self.config.auto_notify {
            for alert in &final_alerts {
                if alert.is_firing() {
                    // Check for deduplication
                    if !self.evaluator.should_deduplicate(alert) {
                        self.send_notification(alert)?;
                    }
                }
            }
        }

        // Update statistics
        self.update_statistics(&final_alerts);

        Ok(final_alerts)
    }

    /// Send notification for an alert
    fn send_notification(&mut self, alert: &Alert) -> Result<(), AlertManagerError> {
        let template = NotificationTemplate::default_alert_template();

        match self.notifier.send_notification(alert, &template) {
            Ok(_) => {
                self.stats.total_notifications += 1;
                Ok(())
            }
            Err(e) => {
                self.stats.failed_notifications += 1;
                Err(AlertManagerError::Notifier(e.to_string()))
            }
        }
    }

    /// Update alert statistics
    fn update_statistics(&mut self, alerts: &[Alert]) {
        self.stats.active_alerts = self.evaluator.alerts().len();

        for alert in alerts {
            match alert.state {
                AlertState::Firing => self.stats.total_fired += 1,
                AlertState::Resolved => self.stats.total_resolved += 1,
                _ => {}
            }
        }
    }

    /// Process retry queue for notifications
    pub fn process_retries(&mut self) -> Result<(), AlertManagerError> {
        self.notifier
            .process_retries()
            .map_err(|e| AlertManagerError::Notifier(e.to_string()))?;
        Ok(())
    }

    /// Flush pending notifications
    pub fn flush_notifications(&mut self) -> Result<(), AlertManagerError> {
        self.notifier
            .flush_notifications()
            .map_err(|e| AlertManagerError::Notifier(e.to_string()))?;
        Ok(())
    }

    /// Get all registered rules
    pub fn rules(&self) -> &BTreeMap<String, AlertRule> {
        self.evaluator.rules()
    }

    /// Get all active alerts
    pub fn alerts(&self) -> &BTreeMap<String, Alert> {
        self.evaluator.alerts()
    }

    /// Get alert history
    pub fn alert_history(&self) -> &[Alert] {
        self.evaluator.alert_history()
    }

    /// Get silence rules
    pub fn silences(&self) -> &BTreeMap<String, SilenceRule> {
        self.silence_manager.silences()
    }

    /// Get inhibition rules
    pub fn inhibitions(&self) -> &[InhibitionRule] {
        self.silence_manager.inhibitions()
    }

    /// Get maintenance windows
    pub fn maintenance_windows(&self) -> &BTreeMap<String, MaintenanceWindow> {
        self.silence_manager.maintenance_windows()
    }

    /// Get notification history
    pub fn notifications(&self) -> &[Notification] {
        self.notifier.notifications()
    }

    /// Get alert statistics
    pub fn statistics(&self) -> &AlertStatistics {
        &self.stats
    }

    /// Get evaluator statistics
    pub fn evaluator_stats(&self) -> EvaluatorStatistics {
        self.evaluator.statistics()
    }

    /// Export alerts in Prometheus format
    pub fn export_prometheus(&self) -> Vec<PrometheusNotification> {
        self.evaluator
            .alerts()
            .values()
            .map(PrometheusNotification::from_alert)
            .collect()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
            timestamp: core::time::Instant::now(),
            labels: BTreeMap::new(),
        }
    }

    /// Add a metric to the context
    pub fn add_metric(&mut self, name: String, value: f64) {
        self.metrics.insert(name, value);
    }

    /// Add a label to the context
    pub fn add_label(&mut self, key: String, value: String) {
        self.labels.insert(key, value);
    }

    /// Get a metric value
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        self.metrics.get(name).copied()
    }

    /// Get all metrics
    pub fn metrics(&self) -> &BTreeMap<String, f64> {
        &self.metrics
    }

    /// Get all labels
    pub fn labels(&self) -> &BTreeMap<String, String> {
        &self.labels
    }
}

impl Default for EvaluationContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Integration helper with metrics system
pub struct MetricsIntegration {
    alert_manager: AlertManager,
}

impl MetricsIntegration {
    /// Create a new metrics integration
    pub fn new(alert_manager: AlertManager) -> Self {
        Self { alert_manager }
    }

    /// Process metric values and generate alerts
    pub fn process_metrics(
        &mut self,
        metrics: &BTreeMap<String, MetricValue>,
    ) -> Result<Vec<Alert>, AlertManagerError> {
        let mut context = EvaluationContext::new();

        for (name, value) in metrics {
            if let Some(float_val) = value.as_float() {
                context.add_metric(name.clone(), *float_val);
            }
        }

        self.alert_manager.evaluate(&context)
    }

    /// Get the underlying alert manager
    pub fn alert_manager(&self) -> &AlertManager {
        &self.alert_manager
    }

    /// Get the underlying alert manager mutably
    pub fn alert_manager_mut(&mut self) -> &mut AlertManager {
        &mut self.alert_manager
    }
}

/// Benchmark utilities for testing alerting performance
pub struct BenchmarkUtils;

impl BenchmarkUtils {
    /// Generate a set of test rules
    pub fn generate_test_rules(count: usize) -> Vec<AlertRule> {
        let mut rules = Vec::new();

        for i in 0..count {
            let rule = AlertRule::new_threshold(
                format!("benchmark-rule-{}", i),
                format!("Benchmark Rule {}", i),
                format!("Test rule for benchmarking"),
                format!("metric_{}", i % 100),
                ComparisonOperator::Gt,
                (i as f64) * 1.5,
                if i % 3 == 0 { Severity::Critical } else { Severity::Warning },
                Duration::from_secs(60),
            );
            rules.push(rule);
        }

        rules
    }

    /// Generate test metrics
    pub fn generate_test_metrics(count: usize) -> BTreeMap<String, MetricValue> {
        let mut metrics = BTreeMap::new();

        for i in 0..count {
            let value = (i as f64) * 1.1 + 50.0;
            metrics.insert(format!("metric_{}", i % 100), MetricValue::Gauge(value));
        }

        metrics
    }

    /// Generate test evaluation context
    pub fn generate_test_context(metric_count: usize) -> EvaluationContext {
        let mut context = EvaluationContext::new();
        let metrics = Self::generate_test_metrics(metric_count);

        for (name, value) in metrics {
            if let Some(float_val) = value.as_float() {
                context.add_metric(name, float_val);
            }
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerting::rule::ComparisonOperator;

    #[test]
    fn test_alert_manager_creation() {
        let manager = AlertManager::new();
        assert_eq!(manager.rules().len(), 0);
        assert_eq!(manager.alerts().len(), 0);
    }

    #[test]
    fn test_add_rule() {
        let mut manager = AlertManager::new();

        let rule = AlertRule::new_threshold(
            "test-rule".into(),
            "Test".into(),
            "Test description".into(),
            "cpu_usage".into(),
            ComparisonOperator::Gt,
            80.0,
            Severity::Warning,
            Duration::from_secs(60),
        );

        manager.add_rule(rule).unwrap();
        assert_eq!(manager.rules().len(), 1);
    }

    #[test]
    fn test_evaluate_context() {
        let mut context = EvaluationContext::new();
        context.add_metric("cpu".into(), 85.0);
        context.add_label("host".into(), "server1".into());

        assert_eq!(context.get_metric("cpu"), Some(85.0));
        assert_eq!(context.labels().get("host"), Some(&"server1".into()));
    }

    #[test]
    fn test_benchmark_utils() {
        let rules = BenchmarkUtils::generate_test_rules(10);
        assert_eq!(rules.len(), 10);

        let context = BenchmarkUtils::generate_test_context(50);
        assert_eq!(context.metrics.len(), 50);
    }

    #[test]
    fn test_alert_statistics() {
        let config = AlertManagerConfig::default();
        let manager = AlertManager::with_config(config);

        let stats = manager.statistics();
        assert_eq!(stats.total_fired, 0);
        assert_eq!(stats.total_resolved, 0);
        assert_eq!(stats.active_alerts, 0);
    }

    #[test]
    fn test_silence_integration() {
        let mut manager = AlertManager::new();

        let silence = SilenceRule::with_duration(
            "silence-1".into(),
            "Test silence".into(),
            vec![Matcher::equal("host".into(), "test-host".into())],
            SilenceType::Manual,
            Duration::from_secs(3600),
            "admin".into(),
            "Testing".into(),
        );

        manager.add_silence(silence).unwrap();
        assert_eq!(manager.silences().len(), 1);
    }

    #[test]
    fn test_inhibition_integration() {
        let mut manager = AlertManager::new();

        let inhibition = InhibitionRule::new(
            "inhibit-1".into(),
            vec![Matcher::equal("severity".into(), "critical".into())],
            vec![Matcher::equal("severity".into(), "warning".into())],
            {
                let mut set = alloc::collections::BTreeSet::new();
                set.insert("host".into());
                set
            },
        );

        manager.add_inhibition(inhibition).unwrap();
        assert_eq!(manager.inhibitions().len(), 1);
    }

    #[test]
    fn test_maintenance_window_integration() {
        let mut manager = AlertManager::new();

        let window = MaintenanceWindow::with_duration(
            "maint-1".into(),
            "Test maintenance".into(),
            vec![Matcher::equal("host".into(), "server1".into())],
            Duration::from_secs(7200),
            MaintenanceType::Planned,
            "Test maintenance".into(),
        );

        manager.create_maintenance(window).unwrap();
        assert_eq!(manager.maintenance_windows().len(), 1);
    }

    #[test]
    fn test_notification_channel_integration() {
        let mut manager = AlertManager::new();

        // Add a webhook channel (doesn't require actual SMTP/SMS setup)
        let channel = NotificationChannel::Webhook(WebhookConfig {
            url: "https://example.com/webhook".into(),
            method: HttpMethod::POST,
            headers: BTreeMap::new(),
            body_template: "Alert: {{alert_id}}".into(),
            content_type: "application/json".into(),
        });

        manager.add_notification_channel(channel);
        // Channel is added, we can't easily test it without network access
    }

    #[test]
    fn test_prometheus_export() {
        let mut manager = AlertManager::new();

        let rule = AlertRule::new_threshold(
            "prometheus-test".into(),
            "Prometheus Test".into(),
            "Test Prometheus export".into(),
            "metric".into(),
            ComparisonOperator::Gt,
            100.0,
            Severity::Critical,
            Duration::from_secs(60),
        );

        manager.add_rule(rule).unwrap();

        let prometheus = manager.export_prometheus();
        // Initially no alerts firing, so export should be empty
        assert_eq!(prometheus.len(), 0);
    }
}
