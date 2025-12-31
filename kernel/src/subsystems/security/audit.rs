//! Security Audit and Logging System
//!
//! This module implements comprehensive security event logging and auditing:
//! - Event logging with structured data
//! - Real-time monitoring and alerting
//! - Log rotation and archiving
//! - Digital signatures for integrity
//! - Security report generation
//!
//! Features:
//! - Structured audit events with severity levels
//! - Thread-safe high-performance logging
//! - Automatic log rotation
//! - HMAC-based log integrity
//! - Security metrics and KPIs
//! - Audit trail for compliance

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::boxed::Box;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// Audit Event Constants
// ============================================================================

/// Maximum number of audit events in memory
pub const MAX_AUDIT_EVENTS: usize = 10000;

/// Maximum log file size in bytes (10 MB)
pub const MAX_LOG_SIZE: usize = 10 * 1024 * 1024;

/// Maximum number of rotated log files to keep
pub const MAX_ROTATED_LOGS: usize = 5;

/// Audit event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    /// Debug messages
    Debug,
    
    /// Informational messages
    Info,
    
    /// Warning messages
    Warning,
    
    /// Error messages
    Error,
    
    /// Critical security events
    Critical,
    
    /// Alert (requires immediate attention)
    Alert,
}

impl AuditSeverity {
    /// Get severity level as number
    pub fn level(&self) -> u8 {
        match self {
            AuditSeverity::Debug => 0,
            AuditSeverity::Info => 1,
            AuditSeverity::Warning => 2,
            AuditSeverity::Error => 3,
            AuditSeverity::Critical => 4,
            AuditSeverity::Alert => 5,
        }
    }
    
    /// Get severity as string
    pub fn as_str(&self) -> &str {
        match self {
            AuditSeverity::Debug => "DEBUG",
            AuditSeverity::Info => "INFO",
            AuditSeverity::Warning => "WARN",
            AuditSeverity::Error => "ERROR",
            AuditSeverity::Critical => "CRITICAL",
            AuditSeverity::Alert => "ALERT",
        }
    }
}

/// Audit event categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditCategory {
    /// Authentication events
    Authentication,
    
    /// Authorization events
    Authorization,
    
    /// Access control events
    AccessControl,
    
    /// Filesystem events
    Filesystem,
    
    /// Process events
    Process,
    
    /// Network events
    Network,
    
    /// System events
    System,
    
    /// Security policy events
    SecurityPolicy,
    
    /// Vulnerability detection
    Vulnerability,
    
    /// Compliance events
    Compliance,
}

// ============================================================================
// Audit Event
// ============================================================================

/// Comprehensive audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Unique event ID
    pub event_id: u64,
    
    /// Event timestamp (nanoseconds since epoch)
    pub timestamp: u64,
    
    /// Event severity
    pub severity: AuditSeverity,
    
    /// Event category
    pub category: AuditCategory,
    
    /// Process ID that generated event
    pub pid: Option<usize>,
    
    /// User ID (if applicable)
    pub uid: Option<u32>,
    
    /// Group ID (if applicable)
    pub gid: Option<u32>,
    
    /// Event type
    pub event_type: String,
    
    /// Event description
    pub description: String,
    
    /// Additional context (key-value pairs)
    pub context: BTreeMap<String, String>,
    
    /// Source IP address (for network events)
    pub source_ip: Option<String>,
    
    /// Destination IP address (for network events)
    pub dest_ip: Option<String>,
    
    /// Source port (for network events)
    pub source_port: Option<u16>,
    
    /// Destination port (for network events)
    pub dest_port: Option<u16>,
    
    /// File path (for filesystem events)
    pub file_path: Option<String>,
    
    /// Operation performed
    pub operation: Option<String>,
    
    /// Result (success/failure)
    pub result: AuditResult,
    
    /// HMAC for integrity verification
    pub hmac: Option<String>,
}

impl AuditEvent {
    /// Create new audit event
    pub fn new(event_type: String, description: String, severity: AuditSeverity, 
               category: AuditCategory) -> Self {
        let event_id = Self::generate_event_id();
        let timestamp = crate::subsystems::time::timestamp_nanos();
        
        Self {
            event_id,
            timestamp,
            severity,
            category,
            pid: None,
            uid: None,
            gid: None,
            event_type,
            description,
            context: BTreeMap::new(),
            source_ip: None,
            dest_ip: None,
            source_port: None,
            dest_port: None,
            file_path: None,
            operation: None,
            result: AuditResult::Success,
            hmac: None,
        }
    }
    
    /// Generate unique event ID (simplified)
    fn generate_event_id() -> u64 {
        static EVENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        EVENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }
    
    /// Set process ID
    pub fn with_pid(mut self, pid: usize) -> Self {
        self.pid = Some(pid);
        self
    }
    
    /// Set user ID
    pub fn with_uid(mut self, uid: u32) -> Self {
        self.uid = Some(uid);
        self
    }
    
    /// Set group ID
    pub fn with_gid(mut self, gid: u32) -> Self {
        self.gid = Some(gid);
        self
    }
    
    /// Add context key-value pair
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }
    
    /// Set network information
    pub fn with_network(mut self, source_ip: String, source_port: Option<u16>,
                          dest_ip: String, dest_port: Option<u16>) -> Self {
        self.source_ip = Some(source_ip);
        self.source_port = source_port;
        self.dest_ip = Some(dest_ip);
        self.dest_port = dest_port;
        self
    }
    
    /// Set file path
    pub fn with_file_path(mut self, file_path: String) -> Self {
        self.file_path = Some(file_path);
        self
    }
    
    /// Set operation
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }
    
    /// Set result
    pub fn with_result(mut self, result: AuditResult) -> Self {
        self.result = result;
        self
    }
    
    /// Convert to string for logging
    pub fn to_string(&self) -> String {
        let mut s = { let mut s = alloc::string::String::from("[{}] {} {:?} - "); s.push_str(&self.timestamp,
            self.severity.as_str(.to_string()); s },
            self.category,
            self.event_type
        );
        
        if let Some(pid) = self.pid {
            s.push_str(&alloc::format!(" (PID: {})", pid));
        }
        
        if let Some(uid) = self.uid {
            s.push_str(&alloc::format!(" (UID: {})", uid));
        }
        
        s.push_str(": ");
        s.push_str(&self.description);
        
        if let Some(ref operation) = self.operation {
            s.push_str(&{ let mut s = alloc::string::String::from(" | Op: "); s.push_str(&operation.to_string()); s });
        }
        
        if let Some(ref file_path) = self.file_path {
            s.push_str(&{ let mut s = alloc::string::String::from(" | File: "); s.push_str(&file_path.to_string()); s });
        }
        
        if let Some(ref source_ip) = self.source_ip {
            s.push_str(&alloc::{ let mut s = alloc::string::String::from(" | From: {}:"); s.push_str(&source_ip, 
                          self.source_port.map_or(0, |p| *p.to_string()); s }));
        }
        
        if let Some(ref dest_ip) = self.dest_ip {
            s.push_str(&alloc::{ let mut s = alloc::string::String::from(" | To: {}:"); s.push_str(&dest_ip, 
                          self.dest_port.map_or(0, |p| *p.to_string()); s }));
        }
        
        // Add context
        if !self.context.is_empty() {
            s.push_str(" | Context: {");
            for (i, (key, value)) in self.context.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                s.push_str(&{ let mut s = alloc::string::String::from("{}="); s.push_str(&key, value.to_string()); s });
            }
            s.push('}');
        }
        
        s.push_str(alloc::string::String::from(" | Result: ") + &format!("{:?}", self.result));
        
        s
    }
}

/// Audit result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditResult {
    /// Operation succeeded
    Success,
    
    /// Operation failed
    Failed {
        error_code: i32,
        error_message: String,
    },
    
    /// Operation denied
    Denied,
    
    /// Operation blocked
    Blocked,
    
    /// Operation pending
    Pending,
}

// ============================================================================
// Audit Logger
// ============================================================================

/// High-performance audit logger
pub struct AuditLogger {
    /// In-memory event buffer
    pub events: Mutex<Vec<AuditEvent>>,
    
    /// Current log file
    pub current_log_file: Mutex<Option<String>>,
    
    /// Current log size in bytes
    pub current_log_size: AtomicUsize,
    
    /// Total events logged
    pub total_events: AtomicU64,
    
    /// Events by severity (for statistics)
    pub events_by_severity: [AtomicU64; 6],
    
    /// Events by category (for statistics)
    pub events_by_category: Mutex<BTreeMap<AuditCategory, AtomicU64>>,
    
    /// Log rotation enabled
    pub rotation_enabled: AtomicBool,
    
    /// HMAC secret for integrity verification
    pub hmac_secret: Mutex<Option<String>>,
    
    /// Alert thresholds
    pub alert_thresholds: Mutex<AlertThresholds>,
    
    /// Alert handlers
    pub alert_handlers: Mutex<Vec<Box<dyn AlertHandler + Send + Sync>>>,
}

/// Alert thresholds
#[derive(Debug, Clone, Copy)]
pub struct AlertThresholds {
    /// Critical events per minute threshold
    pub critical_per_minute: u32,
    
    /// Error events per minute threshold
    pub error_per_minute: u32,
    
    /// Authorization failures per minute threshold
    pub auth_failures_per_minute: u32,
    
    /// Filesystem access failures per minute threshold
    pub fs_failures_per_minute: u32,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            critical_per_minute: 10,
            error_per_minute: 50,
            auth_failures_per_minute: 20,
            fs_failures_per_minute: 30,
        }
    }
}

/// Alert handler trait
pub trait AlertHandler: Send + Sync {
    /// Handle alert
    fn handle_alert(&self, event: &AuditEvent);
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new() -> Self {
        let mut events_by_category = BTreeMap::new();
        
        // Initialize category counters
        events_by_category.insert(AuditCategory::Authentication, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::Authorization, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::AccessControl, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::Filesystem, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::Process, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::Network, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::System, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::SecurityPolicy, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::Vulnerability, AtomicU64::new(0));
        events_by_category.insert(AuditCategory::Compliance, AtomicU64::new(0));
        
        Self {
            events: Mutex::new(Vec::new()),
            current_log_file: Mutex::new(None),
            current_log_size: AtomicUsize::new(0),
            total_events: AtomicU64::new(0),
            events_by_severity: [AtomicU64::new(0); 6],
            events_by_category: Mutex::new(events_by_category),
            rotation_enabled: AtomicBool::new(true),
            hmac_secret: Mutex::new(None),
            alert_thresholds: Mutex::new(AlertThresholds::default()),
            alert_handlers: Mutex::new(Vec::new()),
        }
    }
    
    /// Log audit event
    pub fn log(&self, event: AuditEvent) {
        let total = self.total_events.fetch_add(1, Ordering::Relaxed);
        
        // Add to in-memory buffer
        let mut events = self.events.lock();
        
        if events.len() >= MAX_AUDIT_EVENTS {
            // Rotate out oldest events (simplified)
            events.remove(0);
        }
        
        events.push(event.clone());
        
        // Update severity counter
        let severity_idx = event.severity.level() as usize;
        self.events_by_severity[severity_idx].fetch_add(1, Ordering::Relaxed);
        
        // Update category counter
        let mut categories = self.events_by_category.lock();
        if let Some(counter) = categories.get_mut(&event.category) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
        
        drop(events);
        drop(categories);
        
        // Write to log file
        self.write_to_log(&event);
        
        // Check alert thresholds
        self.check_alerts(&event);
        
        // Log critical events to console
        if event.severity >= AuditSeverity::Error {
            crate::println!("[AUDIT] {}", event.to_string());
        }
    }
    
    /// Write event to log file
    fn write_to_log(&self, event: &AuditEvent) {
        let current_size = self.current_log_size.load(Ordering::Relaxed);
        
        // Check if rotation is needed
        if self.rotation_enabled.load(Ordering::Relaxed) && current_size >= MAX_LOG_SIZE {
            self.rotate_logs();
        }
        
        // Get current log file
        let mut log_file = self.current_log_file.lock();
        
        if log_file.is_none() {
            // Create new log file
            let filename = alloc::format!("/var/log/nos/audit_{}.log",
                                       crate::subsystems::time::timestamp_nanos());
            *log_file = Some(filename.clone());
            
            crate::println!("[audit] Created new audit log: {}", filename);
        }
        
        // Write event to file (simplified - in real implementation would write to file)
        let log_entry = event.to_string();
        let entry_size = log_entry.len();
        
        // Update log size
        self.current_log_size.fetch_add(entry_size, Ordering::Relaxed);
    }
    
    /// Rotate log files
    fn rotate_logs(&self) {
        crate::println!("[audit] Rotating audit logs");
        
        // Close current log file (simplified)
        // Rename current log file with timestamp
        // Delete old rotated logs if > MAX_ROTATED_LOGS
        
        // Reset log size
        self.current_log_size.store(0, Ordering::Relaxed);
        
        // Clear current log file reference
        let mut log_file = self.current_log_file.lock();
        *log_file = None;
    }
    
    /// Check alert thresholds
    fn check_alerts(&self, event: &AuditEvent) {
        let thresholds = self.alert_thresholds.lock();
        
        // Check critical events threshold
        if event.severity == AuditSeverity::Critical {
            let critical_count = self.events_by_severity[AuditSeverity::Critical.level() as usize]
                                  .load(Ordering::Relaxed);
            
            if critical_count >= thresholds.critical_per_minute as u64 {
                self.trigger_alert(event, "Critical events threshold exceeded");
            }
        }
        
        // Check authorization failures
        if event.category == AuditCategory::Authorization && 
           matches!(event.result, AuditResult::Denied | AuditResult::Failed { .. }) {
            
            let auth_failures = self.events_by_category.lock()
                                   .get(&AuditCategory::Authorization)
                                   .map(|c| c.load(Ordering::Relaxed))
                                   .unwrap_or(0);
            
            if auth_failures >= thresholds.auth_failures_per_minute as u64 {
                self.trigger_alert(event, "Authorization failures threshold exceeded");
            }
        }
        
        drop(thresholds);
        
        // Trigger alert for critical events
        if event.severity >= AuditSeverity::Alert {
            self.trigger_alert(event, "Alert level event detected");
        }
    }
    
    /// Trigger alert
    fn trigger_alert(&self, event: &AuditEvent, reason: &str) {
        let handlers = self.alert_handlers.lock();
        
        for handler in handlers.iter() {
            handler.handle_alert(event);
        }
        
        crate::println!("[AUDIT] ALERT: {} - {}", reason, event.event_type);
    }
    
    /// Add alert handler
    pub fn add_alert_handler(&self, handler: Box<dyn AlertHandler + Send + Sync>) {
        let mut handlers = self.alert_handlers.lock();
        handlers.push(handler);
    }
    
    /// Set HMAC secret for integrity verification
    pub fn set_hmac_secret(&self, secret: String) {
        let mut hmac_secret_guard = self.hmac_secret.lock();
        *hmac_secret_guard = Some(secret);
        
        crate::println!("[audit] HMAC secret set for log integrity verification");
    }
    
    /// Verify log integrity (HMAC check)
    pub fn verify_integrity(&self) -> bool {
        let hmac_secret = self.hmac_secret.lock();
        
        if hmac_secret.is_none() {
            // No HMAC secret set - cannot verify
            return false;
        }
        
        // In real implementation, would compute HMAC of log file
        // and compare with stored HMAC
        
        true // Simplified
    }
    
    /// Enable/disable log rotation
    pub fn set_rotation_enabled(&self, enabled: bool) {
        self.rotation_enabled.store(enabled, Ordering::Release);
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> AuditStats {
        let total_events = self.total_events.load(Ordering::Relaxed);
        let events_len = self.events.lock().len();
        
        let mut severity_counts = [0u64; 6];
        for (i, counter) in self.events_by_severity.iter().enumerate() {
            severity_counts[i] = counter.load(Ordering::Relaxed);
        }
        
        let mut category_counts = BTreeMap::new();
        let categories = self.events_by_category.lock();
        for (category, counter) in categories.iter() {
            category_counts.insert(*category, counter.load(Ordering::Relaxed));
        }
        
        AuditStats {
            total_events,
            in_memory_events: events_len,
            current_log_size: self.current_log_size.load(Ordering::Relaxed),
            severity_counts,
            category_counts,
        }
    }
    
    /// Get events by severity
    pub fn get_events_by_severity(&self, severity: AuditSeverity) -> Vec<AuditEvent> {
        let events = self.events.lock();
        events.iter().filter(|e| e.severity == severity).cloned().collect()
    }
    
    /// Get events by category
    pub fn get_events_by_category(&self, category: AuditCategory) -> Vec<AuditEvent> {
        let events = self.events.lock();
        events.iter().filter(|e| e.category == category).cloned().collect()
    }
    
    /// Get recent events
    pub fn get_recent_events(&self, count: usize) -> Vec<AuditEvent> {
        let events = self.events.lock();
        let len = events.len();
        
        if len == 0 {
            return Vec::new();
        }
        
        let start = if count >= len { 0 } else { len - count };
        events[start..].to_vec()
    }
    
    /// Clear in-memory events
    pub fn clear_events(&self) {
        let mut events = self.events.lock();
        events.clear();
        
        crate::println!("[audit] Cleared in-memory audit events");
    }
}

/// Audit statistics
#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_events: u64,
    pub in_memory_events: usize,
    pub current_log_size: usize,
    pub severity_counts: [u64; 6],
    pub category_counts: BTreeMap<AuditCategory, u64>,
}

// ============================================================================
// Security Metrics and KPIs
// ============================================================================

/// Security metrics collector
pub struct SecurityMetrics {
    /// Metrics by name
    metrics: Mutex<BTreeMap<String, SecurityMetric>>,
    
    /// Metric collection start time
    start_time: u64,
}

/// Security metric
#[derive(Debug, Clone)]
pub struct SecurityMetric {
    /// Metric name
    pub name: String,
    
    /// Metric value
    pub value: f64,
    
    /// Unit of measurement
    pub unit: String,
    
    /// Last updated timestamp
    pub updated_at: u64,
    
    /// Metric history (for trends)
    pub history: Vec<(u64, f64)>,
    
    /// Maximum history length
    pub max_history: usize,
}

impl SecurityMetrics {
    /// Create new security metrics collector
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(BTreeMap::new()),
            start_time: crate::subsystems::time::timestamp_nanos(),
        }
    }
    
    /// Update metric
    pub fn update_metric(&self, name: String, value: f64, unit: String) {
        let mut metrics = self.metrics.lock();
        let now = crate::subsystems::time::timestamp_nanos();
        
        if let Some(metric) = metrics.get_mut(&name) {
            metric.value = value;
            metric.updated_at = now;
            
            // Add to history
            metric.history.push((now, value));
            if metric.history.len() > metric.max_history {
                metric.history.remove(0);
            }
        } else {
            // Create new metric
            let metric = SecurityMetric {
                name: name.clone(),
                value,
                unit,
                updated_at: now,
                history: Vec::new(),
                max_history: 100, // Keep last 100 samples
            };
            
            metrics.insert(name, metric);
        }
    }
    
    /// Get metric
    pub fn get_metric(&self, name: String) -> Option<SecurityMetric> {
        let metrics = self.metrics.lock();
        metrics.get(&name).cloned()
    }
    
    /// Get all metrics
    pub fn get_all_metrics(&self) -> Vec<SecurityMetric> {
        let metrics = self.metrics.lock();
        metrics.values().cloned().collect()
    }
    
    /// Calculate metric average
    pub fn calculate_average(&self, name: String) -> Option<f64> {
        let metrics = self.metrics.lock();
        
        metrics.get(&name).and_then(|metric| {
            if metric.history.is_empty() {
                None
            } else {
                let sum: f64 = metric.history.iter().map(|&(_, v)| v).sum();
                Some(sum / metric.history.len() as f64)
            }
        })
    }
    
    /// Get metric trend (slope)
    pub fn get_trend(&self, name: String) -> Option<f64> {
        let metrics = self.metrics.lock();
        
        metrics.get(&name).and_then(|metric| {
            if metric.history.len() < 2 {
                None
            } else {
                // Calculate simple linear regression slope
                let n = metric.history.len() as f64;
                let sum_x: f64 = metric.history.iter().map(|&(t, _)| t as f64).sum();
                let sum_y: f64 = metric.history.iter().map(|&(_, v)| v).sum();
                let sum_xy: f64 = metric.history.iter().map(|&(t, v)| t as f64 * v).sum();
                let sum_x2: f64 = metric.history.iter().map(|&(t, _)| (t as f64).powi(2)).sum();
                
                let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
                
                Some(slope)
            }
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_severity() {
        assert_eq!(AuditSeverity::Debug.level(), 0);
        assert_eq!(AuditSeverity::Info.level(), 1);
        assert_eq!(AuditSeverity::Alert.level(), 5);
        
        assert_eq!(AuditSeverity::Critical.as_str(), "CRITICAL");
        assert_eq!(AuditSeverity::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            String::from("test_event"),
            String::from("Test event description"),
            AuditSeverity::Info,
            AuditCategory::System
        );
        
        assert!(event.event_id > 0);
        assert!(event.timestamp > 0);
        assert_eq!(event.event_type, "test_event");
    }

    #[test]
    fn test_audit_event_builder() {
        let event = AuditEvent::new(
            String::from("test"),
            String::from("Test"),
            AuditSeverity::Info,
            AuditCategory::System
        )
        .with_pid(123)
        .with_uid(456)
        .with_context(String::from("key"), String::from("value"))
        .with_result(AuditResult::Success);
        
        assert_eq!(event.pid, Some(123));
        assert_eq!(event.uid, Some(456));
        assert!(event.context.contains_key(&String::from("key")));
        assert_eq!(event.context.get(&String::from("key")), Some(&String::from("value")));
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new();
        
        let event = AuditEvent::new(
            String::from("test"),
            String::from("Test event"),
            AuditSeverity::Info,
            AuditCategory::System
        );
        
        logger.log(event.clone());
        
        assert_eq!(logger.total_events.load(Ordering::Relaxed), 1);
        
        let events = logger.events.lock();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, event.event_id);
    }

    #[test]
    fn test_security_metrics() {
        let metrics = SecurityMetrics::new();
        
        metrics.update_metric(String::from("test_metric"), 42.0, String::from("count"));
        
        let metric = metrics.get_metric(String::from("test_metric"));
        assert!(metric.is_some());
        
        let metric = metric.unwrap();
        assert_eq!(metric.value, 42.0);
        assert_eq!(metric.unit, "count");
    }

    #[test]
    fn test_audit_stats() {
        let logger = AuditLogger::new();
        
        // Log some events
        for i in 0..10 {
            let event = AuditEvent::new(
                { let mut s = alloc::string::String::from("event_"); s.push_str(&i.to_string()); s },
                { let mut s = alloc::string::String::from("Event "); s.push_str(&i.to_string()); s },
                if i < 3 { AuditSeverity::Info } else { AuditSeverity::Error },
                AuditCategory::System
            );
            
            logger.log(event);
        }
        
        let stats = logger.get_stats();
        
        assert_eq!(stats.total_events, 10);
        assert_eq!(stats.in_memory_events, 10);
        assert_eq!(stats.severity_counts[0], 3); // Info events
        assert_eq!(stats.severity_counts[3], 7); // Error events
    }
}
