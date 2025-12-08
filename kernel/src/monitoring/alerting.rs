//! Alerting system
//!
//! Provides alerting capabilities for production monitoring.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use crate::sync::Mutex;

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    /// Alert name
    pub name: String,
    /// Severity
    pub severity: AlertSeverity,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: u64,
    /// Acknowledged flag
    pub acknowledged: bool,
}

/// Alert rule
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Metric name
    pub metric_name: String,
    /// Threshold
    pub threshold: u64,
    /// Comparison operator
    pub operator: AlertOperator,
    /// Severity
    pub severity: AlertSeverity,
}

/// Alert operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
}

/// Alert manager
pub struct AlertManager {
    /// Active alerts
    alerts: Mutex<BTreeMap<String, Alert>>,
    /// Alert rules
    rules: Mutex<Vec<AlertRule>>,
    /// Alert counter
    alert_counter: core::sync::atomic::AtomicU64,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new() -> Self {
        let mut manager = Self {
            alerts: Mutex::new(BTreeMap::new()),
            rules: Mutex::new(Vec::new()),
            alert_counter: core::sync::atomic::AtomicU64::new(1),
        };
        
        // Register default alert rules
        manager.register_rule(AlertRule {
            name: "high_memory_usage".to_string(),
            metric_name: "memory_used_bytes".to_string(),
            threshold: 1024 * 1024 * 1024, // 1GB
            operator: AlertOperator::GreaterThan,
            severity: AlertSeverity::Warning,
        });
        
        manager
    }
    
    /// Register an alert rule
    pub fn register_rule(&mut self, rule: AlertRule) {
        let mut rules = self.rules.lock();
        rules.push(rule);
    }
    
    /// Evaluate alert rules
    pub fn evaluate_rules(&self, metrics: &alloc::collections::BTreeMap<String, u64>) {
        let rules = self.rules.lock();
        for rule in rules.iter() {
            if let Some(&value) = metrics.get(&rule.metric_name) {
                let should_alert = match rule.operator {
                    AlertOperator::GreaterThan => value > rule.threshold,
                    AlertOperator::LessThan => value < rule.threshold,
                    AlertOperator::Equal => value == rule.threshold,
                    AlertOperator::NotEqual => value != rule.threshold,
                };
                
                if should_alert {
                    self.trigger_alert(&rule.name, rule.severity, &format!("{}: {} {} {}", 
                        rule.metric_name, value, 
                        match rule.operator {
                            AlertOperator::GreaterThan => ">",
                            AlertOperator::LessThan => "<",
                            AlertOperator::Equal => "==",
                            AlertOperator::NotEqual => "!=",
                        },
                        rule.threshold));
                }
            }
        }
    }
    
    /// Trigger an alert
    pub fn trigger_alert(&self, name: &str, severity: AlertSeverity, message: &str) {
        let id = format!("alert-{}", self.alert_counter.fetch_add(1, core::sync::atomic::Ordering::SeqCst));
        
        let alert = Alert {
            id: id.clone(),
            name: name.to_string(),
            severity,
            message: message.to_string(),
            timestamp: crate::time::hrtime_nanos(),
            acknowledged: false,
        };
        
        let mut alerts = self.alerts.lock();
        alerts.insert(id.clone(), alert);
        
        crate::println!("[alert] {}: {} - {}", 
            match severity {
                AlertSeverity::Info => "INFO",
                AlertSeverity::Warning => "WARNING",
                AlertSeverity::Critical => "CRITICAL",
            },
            name,
            message);
    }
    
    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.lock();
        alerts.values()
            .filter(|a| !a.acknowledged)
            .cloned()
            .collect()
    }
    
    /// Acknowledge alert
    pub fn acknowledge_alert(&self, alert_id: &str) -> Result<(), i32> {
        let mut alerts = self.alerts.lock();
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.acknowledged = true;
            Ok(())
        } else {
            Err(crate::reliability::errno::ENOENT)
        }
    }
}

/// Global alert manager instance
static ALERT_MANAGER: Mutex<Option<AlertManager>> = Mutex::new(None);

/// Initialize alert manager
pub fn init_alert_manager() -> Result<(), i32> {
    let mut manager = ALERT_MANAGER.lock();
    if manager.is_none() {
        *manager = Some(AlertManager::new());
        crate::println!("[monitoring] Alert manager initialized");
    }
    Ok(())
}

/// Get alert manager
pub fn get_alert_manager() -> &'static AlertManager {
    static INIT_ONCE: crate::sync::Once = crate::sync::Once::new();
    INIT_ONCE.call_once(|| {
        let mut manager = ALERT_MANAGER.lock();
        if manager.is_none() {
            *manager = Some(AlertManager::new());
        }
    });
    
    unsafe {
        &*(ALERT_MANAGER.lock().as_ref().unwrap() as *const AlertManager)
    }
}

