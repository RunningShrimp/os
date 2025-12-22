// Security Audit Subsystem
//
// This module provides comprehensive security auditing capabilities to track
// security-relevant events and maintain audit trails for compliance and forensics.

extern crate alloc;

use alloc::format;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditEventType {
    /// System call execution
    Syscall,
    /// File access
    FileAccess,
    /// Process creation/termination
    Process,
    /// Network activity
    Network,
    /// Security policy violation
    SecurityViolation,
    /// Authentication event
    Authentication,
    /// Permission change
    PermissionChange,
    /// Configuration change
    Configuration,
    /// System startup/shutdown
    SystemEvent,
    /// Resource allocation
    ResourceAllocation,
    /// Kernel event
    KernelEvent,
    /// Custom event
    Custom,
}

/// Audit event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    /// Informational
    Info = 0,
    /// Warning
    Warning = 1,
    /// Error
    Error = 2,
    /// Critical
    Critical = 3,
    /// Emergency
    Emergency = 4,
}

/// Audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Event ID
    pub id: u64,
    /// Event type
    pub event_type: AuditEventType,
    /// Timestamp (nanoseconds since epoch)
    pub timestamp: u64,
    /// Process ID that generated the event
    pub pid: u64,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Event severity
    pub severity: AuditSeverity,
    /// Event message
    pub message: String,
    /// Additional event data
    pub data: BTreeMap<String, String>,
    /// Source file and line
    pub source_location: Option<(String, u32)>,
    /// Thread ID
    pub tid: u64,
    /// System call number (if applicable)
    pub syscall: Option<u32>,
}

/// Audit configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Whether audit is enabled
    pub enabled: bool,
    /// Minimum severity level to log
    pub min_severity: AuditSeverity,
    /// Maximum number of events to keep in memory
    pub max_events: usize,
    /// Whether to write to persistent storage
    pub persistent_storage: bool,
    /// Storage path for audit logs
    pub storage_path: Option<String>,
    /// Event types to audit
    pub audited_events: Vec<AuditEventType>,
    /// Whether to compress old logs
    pub compress_logs: bool,
    /// Maximum log file size in bytes
    pub max_log_size: u64,
    /// Number of backup logs to keep
    pub backup_count: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_severity: AuditSeverity::Info,
            max_events: 10000,
            persistent_storage: false,
            storage_path: None,
            audited_events: vec![
                AuditEventType::SecurityViolation,
                AuditEventType::Authentication,
                AuditEventType::PermissionChange,
                AuditEventType::Configuration,
            ],
            compress_logs: true,
            max_log_size: 10 * 1024 * 1024, // 10MB
            backup_count: 5,
        }
    }
}

/// Audit statistics
#[derive(Debug, Default, Clone)]
pub struct AuditStats {
    pub events_processed: u64,
    /// Total events audited
    pub total_events: u64,
    /// Events by type
    pub events_by_type: BTreeMap<AuditEventType, u64>,
    /// Events by severity
    pub events_by_severity: BTreeMap<AuditSeverity, u64>,
    /// Events by process
    pub events_by_process: BTreeMap<u64, u64>,
    /// Dropped events (buffer full)
    pub dropped_events: u64,
    /// Events written to storage
    pub stored_events: u64,
    /// Storage write errors
    pub storage_errors: u64,
    /// Average event processing time (nanoseconds)
    pub avg_processing_time_ns: u64,
}

/// Audit filter criteria
#[derive(Debug, Clone)]
pub struct AuditFilter {
    /// Event type filter
    pub event_type: Option<AuditEventType>,
    /// Severity filter
    pub severity: Option<AuditSeverity>,
    /// PID filter
    pub pid: Option<u64>,
    /// UID filter
    pub uid: Option<u32>,
    /// Time range filter (start timestamp)
    pub time_start: Option<u64>,
    /// Time range filter (end timestamp)
    pub time_end: Option<u64>,
    /// Message pattern filter
    pub message_pattern: Option<String>,
    /// Data key-value filter
    pub data_filter: Option<(String, String)>,
}

/// Audit subsystem
pub struct AuditSubsystem {
    /// Configuration
    config: AuditConfig,
    /// Audit events (circular buffer)
    events: Vec<AuditEvent>,
    /// Next event ID
    next_event_id: AtomicU64,
    /// Current write position
    write_pos: usize,
    /// Event count
    event_count: usize,
    /// Statistics
    stats: Arc<Mutex<AuditStats>>,
}

impl AuditSubsystem {
    /// Create new audit subsystem
    pub fn new(config: AuditConfig) -> Self {
        Self {
            events: Vec::with_capacity(config.max_events),
            next_event_id: AtomicU64::new(1),
            write_pos: 0,
            event_count: 0,
            stats: Arc::new(Mutex::new(AuditStats::default())),
            config,
        }
    }

    /// Log an audit event
    pub fn log_event(&mut self, event: AuditEvent) -> Result<(), &'static str> {
        if !self.config.enabled {
            return Ok(());
        }

        // Check if event type is being audited
        if !self.config.audited_events.contains(&event.event_type) {
            return Ok(());
        }

        // Check severity threshold
        if event.severity < self.config.min_severity {
            return Ok(());
        }

        let start_time = crate::subsystems::time::get_timestamp_nanos();

        // Add event to buffer
        if self.event_count < self.events.capacity() {
            self.events.push(event.clone());
            self.event_count += 1;
        } else {
            // Circular buffer - overwrite oldest event
            self.events[self.write_pos] = event.clone();
            self.write_pos = (self.write_pos + 1) % self.events.capacity();
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_events += 1;
            *stats.events_by_type.entry(event.event_type).or_insert(0) += 1;
            *stats.events_by_severity.entry(event.severity).or_insert(0) += 1;
            *stats.events_by_process.entry(event.pid).or_insert(0) += 1;

            let elapsed = crate::subsystems::time::get_timestamp_nanos() - start_time;
            stats.avg_processing_time_ns = (stats.avg_processing_time_ns + elapsed) / 2;
        }

        // Write to persistent storage if configured
        if self.config.persistent_storage {
            if let Err(_) = self.write_to_storage(&event) {
                let mut stats = self.stats.lock();
                stats.storage_errors += 1;
            }
        }

        // Print critical events to console
        if event.severity >= AuditSeverity::Error {
            self.print_event(&event);
        }

        Ok(())
    }

    /// Get audit events matching filter
    pub fn get_events(&self, filter: &AuditFilter) -> Vec<&AuditEvent> {
        let mut results = Vec::new();

        for event in &self.events {
            if self.event_matches_filter(event, filter) {
                results.push(event);
            }
        }

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        results
    }

    /// Check if event matches filter
    fn event_matches_filter(&self, event: &AuditEvent, filter: &AuditFilter) -> bool {
        // Check event type
        if let Some(event_type) = filter.event_type {
            if event.event_type != event_type {
                return false;
            }
        }

        // Check severity
        if let Some(severity) = filter.severity {
            if event.severity != severity {
                return false;
            }
        }

        // Check PID
        if let Some(pid) = filter.pid {
            if event.pid != pid {
                return false;
            }
        }

        // Check UID
        if let Some(uid) = filter.uid {
            if event.uid != uid {
                return false;
            }
        }

        // Check time range
        if let Some(start) = filter.time_start {
            if event.timestamp < start {
                return false;
            }
        }

        if let Some(end) = filter.time_end {
            if event.timestamp > end {
                return false;
            }
        }

        // Check message pattern
        if let Some(pattern) = &filter.message_pattern {
            if !event.message.contains(pattern) {
                return false;
            }
        }

        // Check data filter
        if let Some((key, value)) = &filter.data_filter {
            if event.data.get(key) != Some(value) {
                return false;
            }
        }

        true
    }

    /// Write event to persistent storage
    fn write_to_storage(&self, event: &AuditEvent) -> Result<(), &'static str> {
        // This would integrate with the VFS to write audit logs
        // For now, we'll just update statistics
        {
            let mut stats = self.stats.lock();
            stats.stored_events += 1;
        }
        Ok(())
    }

    /// Print event to console
    fn print_event(&self, event: &AuditEvent) {
        let severity_str = match event.severity {
            AuditSeverity::Info => "INFO",
            AuditSeverity::Warning => "WARN",
            AuditSeverity::Error => "ERROR",
            AuditSeverity::Critical => "CRIT",
            AuditSeverity::Emergency => "EMERG",
        };

        crate::println!(
            "[AUDIT] {} [{}] pid:{} uid:{} {}",
            severity_str,
            event.timestamp,
            event.pid,
            event.uid,
            event.message
        );

        if !event.data.is_empty() {
            crate::println!("[AUDIT] Data:");
            for (key, value) in &event.data {
                crate::println!("  {}: {}", key, value);
            }
        }
    }

    /// Get configuration
    pub fn config(&self) -> &AuditConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: AuditConfig) {
        self.config = config;
    }

    /// Get statistics
    pub fn get_stats(&self) -> AuditStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = AuditStats::default();
    }

    /// Clear all events
    pub fn clear_events(&mut self) {
        self.events.clear();
        self.write_pos = 0;
        self.event_count = 0;
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.event_count
    }
}

/// High-level audit interface functions

/// Create audit event
pub fn create_audit_event(
    event_type: AuditEventType,
    severity: AuditSeverity,
    pid: u64,
    uid: u32,
    message: String,
) -> AuditEvent {
    let event_id = {
        let guard = crate::security::AUDIT.lock();
        if let Some(ref s) = *guard {
            s.next_event_id.load(Ordering::SeqCst)
        } else {
            0
        }
    };
    {
        let guard = crate::security::AUDIT.lock();
        if let Some(ref s) = *guard {
            s.next_event_id.fetch_add(1, Ordering::SeqCst);
        }
    }

    AuditEvent {
        id: event_id,
        event_type,
        timestamp: crate::subsystems::time::get_timestamp_nanos(),
        pid,
        uid,
        gid: 0, // Would be populated from process
        severity,
        message,
        data: BTreeMap::new(),
        source_location: None,
        tid: pid, // Simplified
        syscall: None,
    }
}

/// Log audit event
pub fn log_audit_event(event: AuditEvent) -> Result<(), &'static str> {
    let mut guard = crate::security::AUDIT.lock();
    if let Some(ref mut s) = *guard {
        s.log_event(event)
    } else {
        Ok(())
    }
}

/// Log security violation
pub fn log_security_violation(pid: u64, uid: u32, message: String) -> Result<(), &'static str> {
    let event = create_audit_event(
        AuditEventType::SecurityViolation,
        AuditSeverity::Critical,
        pid,
        uid,
        message,
    );
    log_audit_event(event)
}

/// Log authentication event
pub fn log_authentication_event(pid: u64, uid: u32, success: bool, username: &str) -> Result<(), &'static str> {
    let message = if success {
        format!("Authentication successful for user {}", username)
    } else {
        format!("Authentication failed for user {}", username)
    };

    let severity = if success { AuditSeverity::Info } else { AuditSeverity::Warning };

    let event = create_audit_event(
        AuditEventType::Authentication,
        severity,
        pid,
        uid,
        message,
    );

    let mut enhanced_event = event;
    enhanced_event.data.insert("username".to_string(), username.to_string());
    enhanced_event.data.insert("success".to_string(), success.to_string());

    log_audit_event(enhanced_event)
}

/// Get audit events matching filter
pub fn get_audit_events(filter: AuditFilter) -> Vec<AuditEvent> {
    let guard = crate::security::AUDIT.lock();
    if let Some(ref s) = *guard {
        s.get_events(&filter).into_iter().cloned().collect()
    } else {
        Vec::new()
    }
}

/// Get audit statistics
pub fn get_audit_statistics() -> AuditStats {
    let guard = crate::security::AUDIT.lock();
    guard.as_ref().map(|s| s.get_stats()).unwrap_or_default()
}

/// Update audit configuration
pub fn update_audit_config(config: AuditConfig) -> Result<(), &'static str> {
    *crate::security::AUDIT.lock() = Some(AuditSubsystem::new(config));
    Ok(())
}

// Global instance moved to crate::security::AUDIT (Option)

/// Initialize audit subsystem
pub fn initialize_audit() -> Result<(), i32> {
    // Audit is already initialized via global static instance
    Ok(())
}

/// Cleanup audit subsystem
pub fn cleanup_audit() {
    // Placeholder: In a real implementation, this would clean up audit resources
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = create_audit_event(
            AuditEventType::SecurityViolation,
            AuditSeverity::Critical,
            1234,
            1000,
            "Test violation".to_string(),
        );

        assert_eq!(event.event_type, AuditEventType::SecurityViolation);
        assert_eq!(event.severity, AuditSeverity::Critical);
        assert_eq!(event.pid, 1234);
        assert_eq!(event.uid, 1000);
        assert_eq!(event.message, "Test violation");
    }

    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert_eq!(config.min_severity, AuditSeverity::Info);
        assert_eq!(config.max_events, 10000);
        assert!(!config.persistent_storage);
    }

    #[test]
    fn test_audit_subsystem() {
        let config = AuditConfig::default();
        let mut subsystem = AuditSubsystem::new(config);

        let event = create_audit_event(
            AuditEventType::SecurityViolation,
            AuditSeverity::Critical,
            1234,
            1000,
            "Test violation".to_string(),
        );

        let result = subsystem.log_event(event);
        assert!(result.is_ok());

        assert_eq!(subsystem.event_count(), 1);

        let stats = subsystem.get_stats();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.events_by_type.get(&AuditEventType::SecurityViolation), Some(&1));
        assert_eq!(stats.events_by_severity.get(&AuditSeverity::Critical), Some(&1));
        assert_eq!(stats.events_by_process.get(&1234), Some(&1));
    }

    #[test]
    fn test_audit_filter() {
        let filter = AuditFilter {
            event_type: Some(AuditEventType::SecurityViolation),
            severity: Some(AuditSeverity::Critical),
            pid: Some(1234),
            uid: Some(1000),
            time_start: None,
            time_end: None,
            message_pattern: Some("Test".to_string()),
            data_filter: None,
        };

        assert_eq!(filter.event_type, Some(AuditEventType::SecurityViolation));
        assert_eq!(filter.severity, Some(AuditSeverity::Critical));
        assert_eq!(filter.pid, Some(1234));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(AuditSeverity::Info < AuditSeverity::Warning);
        assert!(AuditSeverity::Warning < AuditSeverity::Error);
        assert!(AuditSeverity::Error < AuditSeverity::Critical);
        assert!(AuditSeverity::Critical < AuditSeverity::Emergency);
    }
}
