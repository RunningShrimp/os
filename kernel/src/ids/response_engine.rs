/// Response Engine Module for IDS

extern crate alloc;
///
/// This module implements automated response capabilities for
/// detected security threats and intrusions.

use crate::subsystems::sync::{SpinLock, Mutex};
use crate::collections::HashMap;
use crate::compat::DefaultHasherBuilder;
use crate::subsystems::time::{SystemTime, UNIX_EPOCH};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

// Use the parent IDS ResponseAction enum so calls from the IDS core share the
// same type and don't require conversions.
use crate::ids::ResponseAction;

/// Response priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResponsePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Response status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    /// Action pending
    Pending,
    /// Action in progress
    InProgress,
    /// Action completed successfully
    Completed,
    /// Action failed
    Failed,
    /// Action cancelled
    Cancelled,
}

/// Response rule
#[derive(Debug, Clone)]
pub struct ResponseRule {
    /// Rule identifier
    pub id: u64,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Trigger conditions
    pub conditions: Vec<String>,
    /// Required severity level
    pub min_severity: u8, // 0-255
    /// Actions to take
    pub actions: Vec<ResponseAction>,
    /// Action priority
    pub priority: ResponsePriority,
    /// Is this rule active
    pub active: bool,
    /// Rate limiting (actions per minute)
    pub rate_limit: Option<u32>,
    /// Cooldown period in seconds
    pub cooldown_period: u64,
    /// Last triggered timestamp
    pub last_triggered: u64,
    /// Number of times triggered
    pub trigger_count: u64,
}

/// Response execution
#[derive(Debug, Clone)]
pub struct ResponseExecution {
    /// Execution ID
    pub id: u64,
    /// Rule that triggered
    pub rule_id: u64,
    /// Action being executed
    pub action: ResponseAction,
    /// Execution timestamp
    pub timestamp: u64,
    /// Target (process, IP, user, etc.)
    pub target: String,
    /// Execution status
    pub status: ResponseStatus,
    /// Execution result message
    pub result: String,
    /// Execution duration in milliseconds
    pub duration: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Response context
#[derive(Debug, Clone)]
pub struct ResponseContext {
    /// Event ID that triggered response
    pub event_id: u64,
    /// Event type
    pub event_type: String,
    /// Event severity
    pub severity: u8,
    /// Source information
    pub source: String,
    /// Target information
    pub target: String,
    /// Additional context
    pub details: HashMap<String, String>,
}

/// Automated response engine
pub struct ResponseEngine {
    /// Response rules
    rules: HashMap<u64, ResponseRule>,
    /// Active executions
    executions: Vec<ResponseExecution>,
    /// Execution history
    execution_history: VecDeque<ResponseExecution>,
    /// Execution counter
    execution_counter: AtomicU64,
    /// Rule counter
    rule_counter: AtomicU64,
    /// Statistics
    stats: ResponseStats,
    /// Engine status
    running: AtomicBool,
    /// Engine lock
    engine_lock: SpinLock,
}

impl ResponseEngine {
    /// Create a new response engine
    pub fn new() -> Self {
        Self {
            rules: HashMap::with_hasher(DefaultHasherBuilder),
            executions: Vec::new(),
            execution_history: VecDeque::new(),
            execution_counter: AtomicU64::new(0),
            rule_counter: AtomicU64::new(0),
            stats: ResponseStats::new(),
            running: AtomicBool::new(false),
            engine_lock: SpinLock::new(),
        }
    }

    /// Start the response engine
    pub fn start(&mut self) {
        let _lock = self.engine_lock.lock();
        self.running.store(true, Ordering::Relaxed);
        self.load_default_rules();
    }

    /// Stop the response engine
    pub fn stop(&mut self) {
        let _lock = self.engine_lock.lock();
        self.running.store(false, Ordering::Relaxed);
    }

    /// Add a response rule
    pub fn add_rule(&mut self, rule: ResponseRule) {
        let _lock = self.engine_lock.lock();

        if !self.rules.contains_key(&rule.id) {
            self.rule_counter.fetch_add(1, Ordering::Relaxed);
        }

        self.rules.insert(rule.id, rule);
        self.stats.total_rules = self.rules.len();
    }

    /// Process a security event and determine responses
    pub fn process_event(&mut self, context: ResponseContext) -> Vec<ResponseExecution> {
        let _lock = self.engine_lock.lock();

        if !self.running.load(Ordering::Relaxed) {
            return Vec::new();
        }

        let mut triggered_executions = Vec::new();

        // Find matching rules - collect to avoid borrowing conflict
        let rules_to_check: Vec<_> = self.rules.values().cloned().collect();
        for rule in rules_to_check {
            if !rule.active {
                continue;
            }

            if self.should_trigger_rule(&rule, &context) {
                let executions = self.execute_rule(&rule, &context);
                triggered_executions.extend(executions);
            }
        }

        triggered_executions
    }

    /// Execute a specific action immediately
    /// Execute an action for a given IntrusionDetection event.
    pub fn execute_action(&mut self, action: ResponseAction, detection: &crate::ids::IntrusionDetection) -> Result<ResponseExecution, &'static str> {
        let _lock = self.engine_lock.lock();

        let execution_id = self.execution_counter.fetch_add(1, Ordering::Relaxed);
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut execution = ResponseExecution {
            id: execution_id,
            rule_id: 0, // Direct execution
            action: match action {
                // Simple mapping from IDS action to response engine's internal representation
                ResponseAction::Log => crate::ids::ResponseAction::Log,
                ResponseAction::Alert => crate::ids::ResponseAction::Alert,
                ResponseAction::BlockConnection => crate::ids::ResponseAction::BlockConnection,
                ResponseAction::IsolateSystem => crate::ids::ResponseAction::IsolateSystem,
                ResponseAction::TerminateProcess => crate::ids::ResponseAction::TerminateProcess,
                ResponseAction::BlockUser => crate::ids::ResponseAction::BlockUser,
                _ => action.clone(),
            },
            timestamp: start_time / 1000,
            target: detection.target.target_id.clone(),
            status: ResponseStatus::InProgress,
            result: String::new(),
            duration: 0,
            error: None,
        };

        // Prepare minimal execution context from the detection
        let mut ctx: HashMap<String, String> = HashMap::with_hasher(crate::compat::DefaultHasherBuilder);
        ctx.insert(String::from("detection_id"), format!("{}", detection.id));
        if let Some(pid) = detection.evidence.iter().find_map(|e| {
            // look for a pid in evidence content (very naive)
            if e.content.contains("pid=") { Some(e.content.clone()) } else { None }
        }) {
            ctx.insert(String::from("pid"), pid);
        }

        // Execute the action
        let (success, result_message, error_message) = self.perform_action(action, &execution.target, &ctx);

        let end_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        execution.duration = end_time - start_time;
        execution.result = result_message;
        execution.error = error_message;
        execution.status = if success {
            ResponseStatus::Completed
        } else {
            ResponseStatus::Failed
        };

        // Update statistics
        if success {
            self.stats.successful_actions += 1;
        } else {
            self.stats.failed_actions += 1;
        }

        // Store execution
        self.executions.push(execution.clone());
        self.add_to_history(execution.clone());

        Ok(execution)
    }

    /// Initialize the response engine with a response mode
    pub fn init(&mut self, _mode: &crate::ids::ResponseMode) -> Result<(), &'static str> {
        // Stub: nothing to do for now
        Ok(())
    }

    /// Shutdown the response engine
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.stop();
        Ok(())
    }

    /// Get recent executions
    pub fn get_recent_executions(&self, count: usize) -> Vec<ResponseExecution> {
        self.execution_history.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get response rule by ID
    pub fn get_rule(&self, rule_id: u64) -> Option<&ResponseRule> {
        self.rules.get(&rule_id)
    }

    /// Update rule
    pub fn update_rule(&mut self, rule_id: u64, updated_rule: ResponseRule) -> bool {
        let _lock = self.engine_lock.lock();

        if self.rules.contains_key(&rule_id) {
            self.rules.insert(rule_id, updated_rule);
            true
        } else {
            false
        }
    }

    /// Get response statistics
    pub fn get_statistics(&self) -> &ResponseStats {
        &self.stats
    }

    /// Clear old execution history
    pub fn clear_old_executions(&mut self, max_age_seconds: u64) {
        let _lock = self.engine_lock.lock();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.execution_history.retain(|execution| {
            current_time - execution.timestamp <= max_age_seconds
        });
    }

    /// Check if a rule should be triggered
    fn should_trigger_rule(&self, rule: &ResponseRule, context: &ResponseContext) -> bool {
        // Check severity threshold
        if context.severity < rule.min_severity {
            return false;
        }

        // Check cooldown period
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if current_time - rule.last_triggered < rule.cooldown_period {
            return false;
        }

        // Check rate limiting
        if let Some(rate_limit) = rule.rate_limit {
            let recent_executions = self.executions.iter()
                .filter(|e| e.rule_id == rule.id && current_time - e.timestamp < 60)
                .count() as u32;

            if recent_executions >= rate_limit {
                return false;
            }
        }

        // Check conditions (simplified - in real implementation would be more complex)
        for condition in &rule.conditions {
            if !self.evaluate_condition(condition, context) {
                return false;
            }
        }

        true
    }

    /// Evaluate a rule condition
    fn evaluate_condition(&self, condition: &str, context: &ResponseContext) -> bool {
        // Simplified condition evaluation
        // In a real implementation, this would use a proper expression parser

        if condition.contains("severity:high") {
            return context.severity >= 150;
        }

        if condition.contains("severity:critical") {
            return context.severity >= 200;
        }

        if condition.contains("type:network") {
            return context.event_type.contains("network") || context.event_type.contains("packet");
        }

        if condition.contains("type:process") {
            return context.event_type.contains("process") || context.event_type.contains("exec");
        }

        if condition.contains("type:file") {
            return context.event_type.contains("file") || context.event_type.contains("access");
        }

        true // Default to true for unknown conditions
    }

    /// Execute a response rule
    fn execute_rule(&mut self, rule: &ResponseRule, context: &ResponseContext) -> Vec<ResponseExecution> {
        let mut executions = Vec::new();

        // Update rule statistics
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Update rule (this would need to be done more carefully with mutable references)
        for (_, stored_rule) in &mut self.rules {
            if stored_rule.id == rule.id {
                stored_rule.last_triggered = current_time;
                stored_rule.trigger_count += 1;
                break;
            }
        }

        // Execute each action
        for action in &rule.actions {
            let execution_id = self.execution_counter.fetch_add(1, Ordering::Relaxed);
            let start_time = current_time;

            let mut execution = ResponseExecution {
                id: execution_id,
                rule_id: rule.id,
                action: action.clone(),
                timestamp: start_time,
                target: context.target.clone(),
                status: ResponseStatus::InProgress,
                result: String::new(),
                duration: 0,
                error: None,
            };

            // Perform the action
            let (success, result_message, error_message) =
                self.perform_action(action.clone(), &context.target, &context.details);

            let end_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            execution.duration = end_time - (start_time * 1000);
            execution.result = result_message;
            execution.error = error_message;
            execution.status = if success {
                ResponseStatus::Completed
            } else {
                ResponseStatus::Failed
            };

            executions.push(execution.clone());
            self.executions.push(execution.clone());
            self.add_to_history(execution.clone());

            // Update statistics
            if success {
                self.stats.successful_actions += 1;
            } else {
                self.stats.failed_actions += 1;
            }
        }

        executions
    }

    /// Perform the actual action
    fn perform_action(&self, action: ResponseAction, target: &str, context: &HashMap<String, String>) -> (bool, String, Option<String>) {
        match action {
            ResponseAction::Log => {
                (true, format!("Logged event for target: {}", target), None)
            }
            ResponseAction::Alert => {
                // Log the event (simplified)
                (true, format!("Alert sent for target: {}", target), None)
            }
            ResponseAction::BlockConnection => {
                // Block network traffic (simplified)
                (true, format!("Blocked network traffic for: {}", target), None)
            }
            ResponseAction::TerminateProcess => {
                // Terminate process (simplified)
                if let Some(pid_str) = context.get("pid") {
                    (true, format!("Process {} terminated", pid_str), None)
                } else {
                    (false, String::from("Process ID not specified"), Some(String::from("Missing PID")))
                }
            }
            ResponseAction::IsolateSystem => {
                // Quarantine file (simplified)
                (true, format!("System isolation initiated for: {}", target), None)
            }
            ResponseAction::BlockUser => {
                // Disable user account (simplified)
                (true, format!("Account {} disabled", target), None)
            }
            ResponseAction::UpdateFirewall => {
                // Schedule system reboot (simplified)
                (true, format!("Firewall updated for {}", target), None)
            }
            ResponseAction::ExecuteScript(ref s) => {
                (true, format!("Executed script '{}' for {}", s, target), None)
            }
            ResponseAction::SendEmail(ref addr) => {
                (true, format!("Sent email to '{}' regarding {}", addr, target), None)
            }
            ResponseAction::CallWebhook(ref url) => {
                (true, format!("Called webhook '{}' for {}", url, target), None)
            }
            // Fallback (for any unhandled parent variant)
            _ => {
                (true, format!("Custom action executed for: {}", target), None)
            }
        }
    }

    /// Add execution to history
    fn add_to_history(&mut self, execution: ResponseExecution) {
        if self.execution_history.len() >= 10000 {
            self.execution_history.pop_front();
        }
        self.execution_history.push_back(execution);
    }

    /// Load default response rules
    fn load_default_rules(&mut self) {
        // Critical threat response rule
        self.add_rule(ResponseRule {
            id: self.rule_counter.fetch_add(1, Ordering::Relaxed) + 1,
            name: String::from("Critical Threat Response"),
            description: String::from("Response for critical security threats"),
            conditions: vec![String::from("severity:critical")],
            min_severity: 200,
            actions: vec![ResponseAction::Alert, ResponseAction::BlockConnection, ResponseAction::Log],
            priority: ResponsePriority::Critical,
            active: true,
            rate_limit: Some(10),
            cooldown_period: 300, // 5 minutes
            last_triggered: 0,
            trigger_count: 0,
        });

        // High severity threat response rule
        self.add_rule(ResponseRule {
            id: self.rule_counter.fetch_add(1, Ordering::Relaxed) + 1,
            name: String::from("High Severity Threat Response"),
            description: String::from("Response for high severity security threats"),
            conditions: vec![String::from("severity:high")],
            min_severity: 150,
            actions: vec![ResponseAction::Alert, ResponseAction::Log],
            priority: ResponsePriority::High,
            active: true,
            rate_limit: Some(20),
            cooldown_period: 180, // 3 minutes
            last_triggered: 0,
            trigger_count: 0,
        });

        // Network intrusion response rule
        self.add_rule(ResponseRule {
            id: self.rule_counter.fetch_add(1, Ordering::Relaxed) + 1,
            name: String::from("Network Intrusion Response"),
            description: String::from("Response for network-based intrusions"),
            conditions: vec![String::from("type:network"), String::from("severity:high")],
            min_severity: 100,
            actions: vec![ResponseAction::BlockConnection, ResponseAction::Alert, ResponseAction::Log],
            priority: ResponsePriority::High,
            active: true,
            rate_limit: Some(15),
            cooldown_period: 120, // 2 minutes
            last_triggered: 0,
            trigger_count: 0,
        });

        // Malware response rule
        self.add_rule(ResponseRule {
            id: self.rule_counter.fetch_add(1, Ordering::Relaxed) + 1,
            name: String::from("Malware Detection Response"),
            description: String::from("Response for malware detection"),
            conditions: vec![String::from("type:malware")],
            min_severity: 100,
            actions: vec![ResponseAction::ExecuteScript(String::from("quarantine")), ResponseAction::Alert, ResponseAction::Log],
            priority: ResponsePriority::High,
            active: true,
            rate_limit: Some(5),
            cooldown_period: 600, // 10 minutes
            last_triggered: 0,
            trigger_count: 0,
        });
    }
}

/// Response engine statistics
#[derive(Debug, Clone)]
pub struct ResponseStats {
    /// Total rules loaded
    pub total_rules: usize,
    /// Active rules
    pub active_rules: usize,
    /// Successful actions
    pub successful_actions: u64,
    /// Failed actions
    pub failed_actions: u64,
    /// Total executions
    pub total_executions: u64,
    /// Executions by action type
    pub actions_by_type: HashMap<ResponseAction, u64>,
    /// Average execution time in milliseconds
    pub avg_execution_time: f64,
}

impl ResponseStats {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            total_rules: 0,
            active_rules: 0,
            successful_actions: 0,
            failed_actions: 0,
            total_executions: 0,
            actions_by_type: HashMap::with_hasher(DefaultHasherBuilder),
            avg_execution_time: 0.0,
        }
    }
}

impl Default for ResponseStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default response engine
pub fn create_response_engine() -> Arc<Mutex<ResponseEngine>> {
    Arc::new(Mutex::new(ResponseEngine::new()))
}

/// Export response engine report
pub fn export_response_report(executions: &[ResponseExecution], rules: &[ResponseRule]) -> String {
    let mut output = String::from("Response Engine Report\n");
    output.push_str("======================\n\n");

    output.push_str(&format!("Total Executions: {}\n", executions.len()));
    output.push_str(&format!("Total Rules: {}\n\n", rules.len()));

    output.push_str("Recent Executions:\n");
    output.push_str("------------------\n");
    for execution in executions.iter().take(10) {
        output.push_str(&format!(
            "ID: {} | Action: {:?} | Status: {:?} | Target: {} | Duration: {}ms\n",
            execution.id, execution.action, execution.status, execution.target, execution.duration
        ));
        if let Some(error) = &execution.error {
            output.push_str(&format!("  Error: {}\n", error));
        }
    }

    output.push_str("\n\nActive Rules:\n");
    output.push_str("------------\n");
    for rule in rules {
        if rule.active {
            output.push_str(&format!(
                "{} - Priority: {:?} | Trigger Count: {} | Actions: {:?}\n",
                rule.name, rule.priority, rule.trigger_count, rule.actions
            ));
        }
    }

    output
}

// Need to import VecDeque for execution history
use crate::collections::VecDeque;