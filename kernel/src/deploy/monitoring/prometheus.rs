//! Prometheus Integration
//!
//! This module implements Prometheus metrics and alerting:
//! - Metrics collection
//! - Alert rules
//! - Dashboard integration
//!
//! Features:
//! - Counter/Gauge/Histogram metrics
//! - Alert rules with thresholds
//! - Prometheus exposition format
//! - Grafana dashboard templates

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;

// ============================================================================
// Metrics Constants
// ============================================================================

/// Maximum metrics
pub const MAX_METRICS: usize = 1 << 12;

/// Maximum alert rules
pub const MAX_ALERT_RULES: usize = 1 << 8;

// ============================================================================
// Metric Types
// ============================================================================

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrometheusMetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Metric
#[derive(Debug, Clone)]
pub struct PrometheusMetric {
    pub metric_name: String,
    pub metric_type: PrometheusMetricType,
    pub labels: BTreeMap<String, String>,
    pub value: AtomicU64,
    pub help_text: String,
}

impl PrometheusMetric {
    pub fn new(name: String, metric_type: PrometheusMetricType, help_text: String) -> Self {
        Self {
            metric_name: name,
            metric_type,
            labels: BTreeMap::new(),
            value: AtomicU64::new(0),
            help_text,
        }
    }

    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn set(&self, value: u64) {
        self.value.store(value, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn add_label(&mut self, key: String, value: String) {
        self.labels.insert(key, value);
    }

    pub fn to_prometheus_format(&self) -> String {
        let mut output = String::from("# HELP ");
        output.push_str(&self.metric_name);
        output.push_str("\n");
        output.push_str("# TYPE ");
        output.push_str(match self.metric_type {
            PrometheusMetricType::Counter => "counter",
            PrometheusMetricType::Gauge => "gauge",
            PrometheusMetricType::Histogram => "histogram",
            PrometheusMetricType::Summary => "summary",
        });
        output.push_str("\n");

        output.push_str(alloc::string::String::from("# ") + &self.metric_name.to_string() + alloc::string::String::from(" ") + &self.help_text.to_string() + alloc::string::String::from("\n"));

        if !self.labels.is_empty() {
            output.push_str(&{ let mut s = alloc::string::String::from("{}_"); s.push_str(&self.metric_name,
                self.labels.iter(.to_string()); s }
                    .map(|(k, v)| &{}\"".to_string() + alloc::string::String::from("=\"))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        } else {
            output.push_str(&self.metric_name);
        }

        let value = self.get();
        output.push_str(alloc::string::String::from(" ") + &value.to_string() + alloc::string::String::from("\n"));

        output
    }
}

// ============================================================================
// Alert Rules
// ============================================================================

/// Comparison operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equals,
    NotEquals,
}

/// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertState {
    /// Alert is inactive
    Inactive,
    
    /// Alert is firing
    Firing,
    
    /// Alert is resolved
    Resolved,
    
    /// Alert is pending
    Pending,
}

/// Alert rule
#[derive(Debug, Clone)]
pub struct AlertRule {
    pub alert_id: String,
    pub alert_name: String,
    pub metric_name: String,
    pub operator: ComparisonOperator,
    pub threshold: f64,
    pub duration_seconds: u64,
    pub labels: BTreeMap<String, String>,
    pub state: Mutex<AlertState>,
    pub last_triggered: AtomicU64,
    pub trigger_count: AtomicU64,
}

impl AlertRule {
    pub fn new(alert_id: String, alert_name: String, metric_name: String,
                 operator: ComparisonOperator, threshold: f64, duration_seconds: u64) -> Self {
        Self {
            alert_id,
            alert_name,
            metric_name,
            operator,
            threshold,
            duration_seconds,
            labels: BTreeMap::new(),
            state: Mutex::new(AlertState::Inactive),
            last_triggered: AtomicU64::new(0),
            trigger_count: AtomicU64::new(0),
        }
    }

    pub fn add_label(&mut self, key: String, value: String) {
        self.labels.insert(key, value);
    }

    pub fn evaluate(&self, current_value: f64) -> AlertState {
        let threshold = self.threshold;
        let should_fire = match self.operator {
            ComparisonOperator::GreaterThan => current_value > threshold,
            ComparisonOperator::LessThan => current_value < threshold,
            ComparisonOperator::Equals => (current_value - threshold).abs() < 0.0001,
            ComparisonOperator::NotEquals => (current_value - threshold).abs() >= 0.0001,
        };

        let mut state = *self.state.lock();

        state = if should_fire {
            match state {
                AlertState::Inactive | AlertState::Resolved => {
                    AlertState::Firing
                }
                AlertState::Firing => {
                    AlertState::Firing
                }
                _ => AlertState::Pending
            }
        } else {
            match state {
                AlertState::Firing | AlertState::Pending => {
                    AlertState::Resolved
                }
                _ => {
                    AlertState::Inactive
                }
            }
        };

        if state == AlertState::Firing {
            self.last_triggered.store(crate::subsystems::time::timestamp_nanos(), Ordering::Relaxed);
            self.trigger_count.fetch_add(1, Ordering::Relaxed);
        }

        *self.state.lock() = state;
        state
    }
}

// ============================================================================
// Prometheus Manager
// ============================================================================

/// Prometheus manager
pub struct PrometheusManager {
    pub metrics: Mutex<BTreeMap<String, Arc<PrometheusMetric>>>>,
    pub alert_rules: Mutex<BTreeMap<String, Arc<AlertRule>>>>,
    pub next_metric_id: AtomicU64,
    pub next_alert_id: AtomicU64,
    pub stats: Mutex<PrometheusManagerStats>,
}

/// Manager statistics
#[derive(Debug, Clone, Copy)]
pub struct PrometheusManagerStats {
    pub total_metrics: usize,
    pub total_alert_rules: usize,
    pub active_alerts: usize,
    pub fired_alerts: usize,
    pub metric_expositions: u64,
}

impl Default for PrometheusManagerStats {
    fn default() -> Self {
        Self {
            total_metrics: 0,
            total_alert_rules: 0,
            active_alerts: 0,
            fired_alerts: 0,
            metric_expositions: 0,
        }
    }
}

impl PrometheusManager {
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(BTreeMap::new()),
            alert_rules: Mutex::new(BTreeMap::new()),
            next_metric_id: AtomicU64::new(1),
            next_alert_id: AtomicU64::new(1),
            stats: Mutex::new(PrometheusManagerStats::default()),
        }
    }

    pub fn register_metric(&self, metric: Arc<PrometheusMetric>) -> Result<(), String> {
        let mut metrics = self.metrics.lock();
        let name = metric.metric_name.clone();

        if metrics.contains_key(&name) {
            return Err(alloc::string::String::from("Metric ") + &name.to_string() + alloc::string::String::from(" already exists"));
        }

        metrics.insert(name, metric);
        crate::println!("[prometheus] Registered metric: {}", name);

        let mut stats = self.stats.lock();
        stats.total_metrics = metrics.len();

        Ok(())
    }

    pub fn get_metric(&self, name: String) -> Option<Arc<PrometheusMetric>> {
        let metrics = self.metrics.lock();
        metrics.get(&name).cloned()
    }

    pub fn register_alert_rule(&self, rule: Arc<AlertRule>) -> Result<(), String> {
        let mut alert_rules = self.alert_rules.lock();
        let alert_id = rule.alert_id.clone();

        if alert_rules.len() >= MAX_ALERT_RULES {
            return Err("Maximum alert rules reached".to_string());
        }

        alert_rules.insert(alert_id, rule);
        crate::println!("[prometheus] Registered alert rule: {}", rule.alert_name);

        let mut stats = self.stats.lock();
        stats.total_alert_rules = alert_rules.len();

        Ok(())
    }

    pub fn evaluate_alert_rules(&self) {
        let alert_rules = self.alert_rules.lock();
        let mut stats = self.stats.lock();

        stats.active_alerts = 0;
        stats.fired_alerts = 0;

        for rule in alert_rules.values() {
            // Get metric value
            if let Some(metric) = self.get_metric(rule.metric_name.clone()) {
                let current_value = metric.get() as f64;
                let state = rule.evaluate(current_value);

                if state == AlertState::Firing {
                    stats.fired_alerts += 1;
                } else if state == AlertState::Pending {
                    stats.active_alerts += 1;
                }
            }
        }
    }

    pub fn get_metrics_exposition(&self) -> String {
        let metrics = self.metrics.lock();
        let mut exposition = String::new();

        for metric in metrics.values() {
            exposition.push_str(&metric.to_prometheus_format());
        }

        self.stats.lock().metric_expositions.fetch_add(1, Ordering::Relaxed);
        exposition
    }

    pub fn get_alerts_status(&self) -> Vec<(String, AlertState)> {
        let alert_rules = self.alert_rules.lock();
        alert_rules.values()
            .map(|rule| (rule.alert_name.clone(), *rule.state.lock()))
            .collect()
    }

    pub fn get_stats(&self) -> PrometheusManagerStats {
        let mut stats = self.stats.lock();
        stats.total_metrics = self.metrics.lock().len();
        stats.total_alert_rules = self.alert_rules.lock().len();
        *stats
    }
}
