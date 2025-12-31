//! Alert evaluator for periodic and streaming rule evaluation
//!
//! This module provides the evaluation engine for alert rules, including:
//! - Periodic rule evaluation at configured intervals
//! - Streaming evaluation for real-time metrics
//! - Alert state machine (Pending → Firing → Resolved)
//! - Alert deduplication and grouping
//! - Hysteresis to prevent alert flapping

#![no_std]

extern crate alloc;

use alloc::{
    collections::{BTreeMap, BTreeSet},
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

use crate::alerting::rule::{AlertRule, EvaluationResult, Severity};

/// Alert state machine states
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertState {
    /// Alert is not firing
    Inactive,
    /// Alert condition is met but waiting for for_duration
    Pending,
    /// Alert is actively firing
    Firing,
    /// Alert was firing but condition is no longer met
    Resolved,
}

/// Alert instance represents a firing alert
#[derive(Clone, Debug)]
pub struct Alert {
    /// Unique alert identifier
    pub id: String,
    /// Rule that generated this alert
    pub rule_id: String,
    /// Alert state
    pub state: AlertState,
    /// Current severity
    pub severity: Severity,
    /// Labels for this alert instance
    pub labels: BTreeMap<String, String>,
    /// Annotations for this alert instance
    pub annotations: BTreeMap<String, String>,
    /// Current value that triggered the alert
    pub value: Option<f64>,
    /// Threshold that was exceeded
    pub threshold: Option<f64>,
    /// When the alert first started
    pub started_at: core::time::Instant,
    /// When the alert last fired
    pub fired_at: Option<core::time::Instant>,
    /// When the alert was resolved
    pub resolved_at: Option<core::time::Instant>,
    /// Number of times this alert has fired
    pub fire_count: usize,
    /// Reason for the current state
    pub reason: Option<String>,
}

/// Evaluation context containing metrics and metadata
#[derive(Clone, Debug)]
pub struct EvaluationContext {
    /// Metric values at evaluation time
    pub metrics: BTreeMap<String, f64>,
    /// Timestamp of evaluation
    pub timestamp: core::time::Instant,
    /// Additional labels to attach to alerts
    pub labels: BTreeMap<String, String>,
}

/// Alert evaluation configuration
#[derive(Clone, Debug)]
pub struct EvaluatorConfig {
    /// Maximum number of concurrent evaluations
    pub max_concurrent_evaluations: usize,
    /// Alert deduplication window
    pub deduplication_window: Duration,
    /// Alert grouping key configuration
    pub grouping_keys: Vec<String>,
    /// Hysteresis factor for preventing flapping (0.0 to 1.0)
    pub hysteresis_factor: f64,
    /// Minimum alert duration before notification
    pub min_alert_duration: Duration,
    /// Maximum alert history size
    pub max_history_size: usize,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_evaluations: 100,
            deduplication_window: Duration::from_secs(300),
            grouping_keys: vec![],
            hysteresis_factor: 0.1,
            min_alert_duration: Duration::from_secs(30),
            max_history_size: 1000,
        }
    }
}

/// Alert evaluation error
#[derive(Clone, Debug)]
pub enum EvaluationError {
    /// Rule evaluation failed
    RuleEvaluation(String),
    /// Invalid alert state transition
    InvalidStateTransition {
        from: AlertState,
        to: AlertState,
    },
    /// Alert not found
    AlertNotFound(String),
    /// Evaluation queue full
    QueueFull,
    /// Concurrent evaluation limit exceeded
    ConcurrentLimitExceeded,
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuleEvaluation(msg) => write!(f, "Rule evaluation failed: {}", msg),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "Invalid state transition from {:?} to {:?}", from, to)
            }
            Self::AlertNotFound(id) => write!(f, "Alert not found: {}", id),
            Self::QueueFull => write!(f, "Evaluation queue is full"),
            Self::ConcurrentLimitExceeded => write!(f, "Concurrent evaluation limit exceeded"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EvaluationError {}

/// Alert evaluator
pub struct AlertEvaluator {
    /// Registered alert rules
    rules: BTreeMap<String, AlertRule>,
    /// Active alerts by ID
    alerts: BTreeMap<String, Alert>,
    /// Alert history
    alert_history: Vec<Alert>,
    /// Pending alerts waiting for for_duration
    pending_alerts: BTreeMap<String, (Alert, core::time::Instant)>,
    /// Configuration
    config: EvaluatorConfig,
    /// Deduplication cache
    dedup_cache: BTreeMap<String, core::time::Instant>,
}

impl AlertEvaluator {
    /// Create a new alert evaluator
    pub fn new(config: EvaluatorConfig) -> Self {
        Self {
            rules: BTreeMap::new(),
            alerts: BTreeMap::new(),
            alert_history: Vec::new(),
            pending_alerts: BTreeMap::new(),
            config,
            dedup_cache: BTreeMap::new(),
        }
    }

    /// Add a rule to the evaluator
    pub fn add_rule(&mut self, rule: AlertRule) -> Result<(), EvaluationError> {
        self.rules.insert(rule.id.clone(), rule);
        Ok(())
    }

    /// Remove a rule from the evaluator
    pub fn remove_rule(&mut self, rule_id: &str) -> Result<(), EvaluationError> {
        self.rules.remove(rule_id);
        // Also remove any alerts from this rule
        self.alerts.retain(|_, alert| alert.rule_id != rule_id);
        Ok(())
    }

    /// Get all registered rules
    pub fn rules(&self) -> &BTreeMap<String, AlertRule> {
        &self.rules
    }

    /// Get all active alerts
    pub fn alerts(&self) -> &BTreeMap<String, Alert> {
        &self.alerts
    }

    /// Get alert history
    pub fn alert_history(&self) -> &[Alert] {
        &self.alert_history
    }

    /// Evaluate all rules with the given context
    pub fn evaluate_all(
        &mut self,
        context: &EvaluationContext,
    ) -> Result<Vec<Alert>, EvaluationError> {
        let mut new_alerts = Vec::new();

        for (rule_id, rule) in &self.rules {
            if !rule.enabled {
                continue;
            }

            // Check if this evaluation would exceed concurrent limit
            if self.pending_alerts.len() + new_alerts.len() >= self.config.max_concurrent_evaluations {
                return Err(EvaluationError::ConcurrentLimitExceeded);
            }

            match self.evaluate_rule(rule, context) {
                Ok(Some(alert)) => {
                    new_alerts.push(alert);
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(EvaluationError::RuleEvaluation(format!(
                        "Rule {}: {}",
                        rule_id, e
                    )));
                }
            }
        }

        Ok(new_alerts)
    }

    /// Evaluate a single rule
    pub fn evaluate_rule(
        &mut self,
        rule: &AlertRule,
        context: &EvaluationContext,
    ) -> Result<Option<Alert>, EvaluationError> {
        // Convert context metrics to rule metric format
        let metric_values: BTreeMap<String, crate::monitoring::metrics::MetricValue> =
            context.metrics.iter().map(|(k, v)| {
                (k.clone(), crate::monitoring::metrics::MetricValue::Gauge(*v))
            }).collect();

        // Evaluate the rule
        let result = rule
            .evaluate(&metric_values)
            .map_err(|e| EvaluationError::RuleEvaluation(e.to_string()))?;

        // Get or create alert for this rule
        let alert_key = self.alert_key(rule, context);
        let existing_alert = self.alerts.get(&alert_key);

        match (existing_alert, result.fired) {
            // New alert firing
            (None, true) => {
                let alert = self.create_alert(rule, &result, context);
                self.pending_alerts.insert(
                    alert_key.clone(),
                    (alert.clone(), core::time::Instant::now()),
                );
                Ok(Some(alert))
            }

            // Existing alert continues firing
            (Some(alert), true) => {
                let updated_alert = self.update_firing_alert(alert, &result);
                self.alerts.insert(alert_key.clone(), updated_alert.clone());
                Ok(Some(updated_alert))
            }

            // Alert condition no longer met
            (Some(alert), false) => {
                let resolved_alert = self.resolve_alert(alert);
                self.alerts.insert(alert_key.clone(), resolved_alert.clone());
                self.add_to_history(resolved_alert.clone());
                Ok(Some(resolved_alert))
            }

            // No change
            _ => Ok(None),
        }
    }

    /// Check and transition pending alerts to firing
    pub fn check_pending_alerts(&mut self) -> Vec<Alert> {
        let mut firing_alerts = Vec::new();
        let now = core::time::Instant::now();

        let to_transition: Vec<String> = self
            .pending_alerts
            .iter()
            .filter(|(_, (alert, pending_since))| {
                // Check if for_duration has elapsed
                if let Some(for_duration) = self.get_rule_for_duration(&alert.rule_id) {
                    now.duration_since(*pending_since) >= for_duration
                } else {
                    true // No for_duration, fire immediately
                }
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in to_transition {
            if let Some((alert, _)) = self.pending_alerts.remove(&key) {
                let firing_alert = self.transition_to_firing(alert);
                self.alerts.insert(key.clone(), firing_alert.clone());
                firing_alerts.push(firing_alert);
            }
        }

        firing_alerts
    }

    /// Create a new alert from rule evaluation result
    fn create_alert(
        &self,
        rule: &AlertRule,
        result: &EvaluationResult,
        context: &EvaluationContext,
    ) -> Alert {
        let id = self.generate_alert_id(rule, context);
        let now = core::time::Instant::now();

        let mut labels = rule.labels.clone();
        labels.extend(context.labels.clone());

        let mut annotations = rule.annotations.clone();
        if let Some(reason) = &result.reason {
            annotations.insert("reason".into(), reason.clone());
        }

        Alert {
            id: id.clone(),
            rule_id: rule.id.clone(),
            state: AlertState::Pending,
            severity: rule.severity,
            labels,
            annotations,
            value: result.value,
            threshold: result.threshold,
            started_at: now,
            fired_at: None,
            resolved_at: None,
            fire_count: 0,
            reason: result.reason.clone(),
        }
    }

    /// Update an existing firing alert
    fn update_firing_alert(&self, alert: &Alert, result: &EvaluationResult) -> Alert {
        let mut updated = alert.clone();
        updated.value = result.value;
        updated.threshold = result.threshold;
        updated.reason = result.reason.clone();
        updated
    }

    /// Transition an alert to firing state
    fn transition_to_firing(&self, alert: Alert) -> Alert {
        let mut firing = alert;
        firing.state = AlertState::Firing;
        firing.fired_at = Some(core::time::Instant::now());
        firing.fire_count += 1;
        firing
    }

    /// Resolve an alert
    fn resolve_alert(&self, alert: &Alert) -> Alert {
        let mut resolved = alert.clone();
        resolved.state = AlertState::Resolved;
        resolved.resolved_at = Some(core::time::Instant::now());
        resolved
    }

    /// Add alert to history
    fn add_to_history(&mut self, alert: Alert) {
        self.alert_history.push(alert);
        // Trim history if needed
        if self.alert_history.len() > self.config.max_history_size {
            self.alert_history.remove(0);
        }
    }

    /// Generate a unique alert key for deduplication
    fn alert_key(&self, rule: &AlertRule, context: &EvaluationContext) -> String {
        // Group alerts by rule ID and grouping labels
        let mut key = format!("{}:", rule.id);
        for group_key in &self.config.grouping_keys {
            if let Some(value) = context.labels.get(group_key) {
                key.push_str(&format!("{}={},", group_key, value));
            }
        }
        key
    }

    /// Generate a unique alert ID
    fn generate_alert_id(&self, rule: &AlertRule, context: &EvaluationContext) -> String {
        let key = self.alert_key(rule, context);
        format!("{}:{}", key, core::time::Instant::now().as_secs())
    }

    /// Get the for_duration for a rule
    fn get_rule_for_duration(&self, rule_id: &str) -> Option<Duration> {
        self.rules.get(rule_id).and_then(|rule| match &rule.config {
            crate::alerting::rule::RuleConfig::Threshold(config) => config.for_duration,
            crate::alerting::rule::RuleConfig::Rate(config) => config.for_duration,
            _ => None,
        })
    }

    /// Check if an alert should be deduplicated
    pub fn should_deduplicate(&mut self, alert: &Alert) -> bool {
        let key = format!("{}:{}", alert.rule_id, alert.labels_to_string());
        if let Some(last_seen) = self.dedup_cache.get(&key) {
            let now = core::time::Instant::now();
            if now.duration_since(*last_seen) < self.config.deduplication_window {
                return true;
            }
        }
        self.dedup_cache.insert(key, core::time::Instant::now());
        false
    }

    /// Apply hysteresis to prevent alert flapping
    pub fn apply_hysteresis(&self, value: f64, threshold: f64) -> (f64, f64) {
        let margin = threshold * self.config.hysteresis_factor;
        let fire_threshold = threshold - margin;
        let resolve_threshold = threshold + margin;
        (fire_threshold, resolve_threshold)
    }

    /// Group alerts by common labels
    pub fn group_alerts(&self, alerts: &[Alert]) -> BTreeMap<String, Vec<Alert>> {
        let mut groups: BTreeMap<String, Vec<Alert>> = BTreeMap::new();

        for alert in alerts {
            let group_key = self.group_key(alert);
            groups.entry(group_key).or_default().push(alert.clone());
        }

        groups
    }

    /// Generate a grouping key for an alert
    fn group_key(&self, alert: &Alert) -> String {
        if self.config.grouping_keys.is_empty() {
            return "default".into();
        }

        let mut key = String::new();
        for group_label in &self.config.grouping_keys {
            if let Some(value) = alert.labels.get(group_label) {
                key.push_str(&format!("{}={},", group_label, value));
            }
        }
        if key.is_empty() {
            "default".into()
        } else {
            key
        }
    }

    /// Get statistics about alert evaluation
    pub fn statistics(&self) -> EvaluatorStatistics {
        EvaluatorStatistics {
            total_rules: self.rules.len(),
            active_alerts: self.alerts.len(),
            pending_alerts: self.pending_alerts.len(),
            history_size: self.alert_history.len(),
        }
    }
}

impl Alert {
    /// Convert labels to string for deduplication
    fn labels_to_string(&self) -> String {
        self.labels
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Get the age of the alert
    pub fn age(&self) -> Duration {
        core::time::Instant::now().duration_since(self.started_at)
    }

    /// Check if the alert is in a firing state
    pub fn is_firing(&self) -> bool {
        matches!(self.state, AlertState::Firing | AlertState::Pending)
    }

    /// Check if the alert is resolved
    pub fn is_resolved(&self) -> bool {
        self.state == AlertState::Resolved
    }
}

/// Evaluator statistics
#[derive(Clone, Debug)]
pub struct EvaluatorStatistics {
    /// Total number of registered rules
    pub total_rules: usize,
    /// Number of active alerts
    pub active_alerts: usize,
    /// Number of pending alerts
    pub pending_alerts: usize,
    /// Size of alert history
    pub history_size: usize,
}

/// Streaming evaluator for real-time metrics
pub struct StreamingEvaluator {
    base_evaluator: AlertEvaluator,
    buffer_size: usize,
    metric_buffer: Vec<(String, f64, core::time::Instant)>,
}

impl StreamingEvaluator {
    /// Create a new streaming evaluator
    pub fn new(config: EvaluatorConfig, buffer_size: usize) -> Self {
        Self {
            base_evaluator: AlertEvaluator::new(config),
            buffer_size,
            metric_buffer: Vec::with_capacity(buffer_size),
        }
    }

    /// Process a new metric value
    pub fn process_metric(
        &mut self,
        metric_name: String,
        value: f64,
    ) -> Result<Vec<Alert>, EvaluationError> {
        let timestamp = core::time::Instant::now();

        // Add to buffer
        self.metric_buffer.push((metric_name.clone(), value, timestamp));

        // Trim buffer if needed
        if self.metric_buffer.len() > self.buffer_size {
            self.metric_buffer.remove(0);
        }

        // Build evaluation context from buffered metrics
        let context = self.build_context();

        // Evaluate all rules
        self.base_evaluator.evaluate_all(&context)
    }

    /// Build evaluation context from buffered metrics
    fn build_context(&self) -> EvaluationContext {
        let metrics: BTreeMap<String, f64> = self
            .metric_buffer
            .iter()
            .map(|(name, value, _)| (name.clone(), *value))
            .collect();

        EvaluationContext {
            metrics,
            timestamp: core::time::Instant::now(),
            labels: BTreeMap::new(),
        }
    }

    /// Get the underlying evaluator
    pub fn evaluator(&self) -> &AlertEvaluator {
        &self.base_evaluator
    }

    /// Get the underlying evaluator mutably
    pub fn evaluator_mut(&mut self) -> &mut AlertEvaluator {
        &mut self.base_evaluator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerting::rule::ComparisonOperator;

    #[test]
    fn test_alert_state_transitions() {
        let rule = crate::alerting::rule::AlertRule::new_threshold(
            "test-rule".into(),
            "Test".into(),
            "Test".into(),
            "metric".into(),
            ComparisonOperator::Gt,
            80.0,
            Severity::Warning,
            Duration::from_secs(60),
        );

        let config = EvaluatorConfig::default();
        let evaluator = AlertEvaluator::new(config);
        evaluator.add_rule(rule).unwrap();

        let mut context = EvaluationContext {
            metrics: BTreeMap::new(),
            timestamp: core::time::Instant::now(),
            labels: BTreeMap::new(),
        };

        context.metrics.insert("metric".into(), 85.0);

        // This would need to be extended with a proper test framework
        // that handles time advancement
    }

    #[test]
    fn test_hysteresis_calculation() {
        let config = EvaluatorConfig {
            hysteresis_factor: 0.1,
            ..Default::default()
        };
        let evaluator = AlertEvaluator::new(config);

        let (fire_threshold, resolve_threshold) = evaluator.apply_hysteresis(85.0, 100.0);

        assert_eq!(fire_threshold, 90.0);
        assert_eq!(resolve_threshold, 110.0);
    }

    #[test]
    fn test_alert_grouping() {
        let config = EvaluatorConfig {
            grouping_keys: vec!["host".into(), "region".into()],
            ..Default::default()
        };
        let evaluator = AlertEvaluator::new(config);

        let alert1 = Alert {
            id: "1".into(),
            rule_id: "rule1".into(),
            state: AlertState::Firing,
            severity: Severity::Warning,
            labels: {
                let mut map = BTreeMap::new();
                map.insert("host".into(), "server1".into());
                map.insert("region".into(), "us-west".into());
                map
            },
            annotations: BTreeMap::new(),
            value: Some(85.0),
            threshold: Some(80.0),
            started_at: core::time::Instant::now(),
            fired_at: Some(core::time::Instant::now()),
            resolved_at: None,
            fire_count: 1,
            reason: None,
        };

        let alert2 = Alert {
            id: "2".into(),
            rule_id: "rule1".into(),
            state: AlertState::Firing,
            severity: Severity::Warning,
            labels: {
                let mut map = BTreeMap::new();
                map.insert("host".into(), "server1".into());
                map.insert("region".into(), "us-west".into());
                map
            },
            annotations: BTreeMap::new(),
            value: Some(90.0),
            threshold: Some(80.0),
            started_at: core::time::Instant::now(),
            fired_at: Some(core::time::Instant::now()),
            resolved_at: None,
            fire_count: 1,
            reason: None,
        };

        let groups = evaluator.group_alerts(&[alert1, alert2]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.values().next().unwrap().len(), 2);
    }

    #[test]
    fn test_evaluator_statistics() {
        let config = EvaluatorConfig::default();
        let evaluator = AlertEvaluator::new(config);

        let stats = evaluator.statistics();
        assert_eq!(stats.total_rules, 0);
        assert_eq!(stats.active_alerts, 0);
        assert_eq!(stats.pending_alerts, 0);
        assert_eq!(stats.history_size, 0);
    }
}
