//! Distributed Logging
//!
//! This module implements distributed logging for cloud-native:
//! - Structured logging
//! - Log aggregation
//! - Log search
//!
//! Features:
//! - JSON-formatted logs
//! - Log levels
//! - Log correlation
//! - Log rotation

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

// ============================================================================
// Logging Constants
// ============================================================================

/// Maximum log entries
pub const MAX_LOG_ENTRIES: usize = 1 << 16;

/// Maximum log fields
pub const MAX_LOG_FIELDS: usize = 1 << 8;

// ============================================================================
// Log Levels
// ============================================================================

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

// ============================================================================
// Log Entry
// ============================================================================

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
    pub service: String,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub fields: BTreeMap<String, String>,
    pub correlation_id: Option<String>,
}

impl LogEntry {
    pub fn new(level: LogLevel, service: String, message: String) -> Self {
        Self {
            timestamp: crate::subsystems::time::timestamp_nanos(),
            level,
            service,
            message,
            trace_id: None,
            span_id: None,
            fields: BTreeMap::new(),
            correlation_id: None,
        }
    }

    pub fn with_trace(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    pub fn with_span(mut self, span_id: String) -> Self {
        self.span_id = Some(span_id);
        self
    }

    pub fn with_field(mut self, key: String, value: String) -> Self {
        self.fields.insert(key, value);
        self
    }

    pub fn to_json(&self) -> String {
        let mut json = String::from("{");
        json.push_str(alloc::string::String::from("\"));
        json.push_str(alloc::string::String::from("\"));
        json.push_str(alloc::string::String::from("\"));
        json.push_str(alloc::string::String::from("\"));
        
        if let Some(trace_id) = &self.trace_id {
            json.push_str(alloc::string::String::from(",\"));
        }
        
        if let Some(span_id) = &self.span_id {
            json.push_str(alloc::string::String::from(",\"));
        }
        
        if let Some(correlation_id) = &self.correlation_id {
            json.push_str(alloc::string::String::from(",\"));
        }
        
        if !self.fields.is_empty() {
            json.push_str(",\"fields\":{");
            let mut first = true;
            for (key, value) in &self.fields {
                if !first {
                    json.push_str(",");
                }
                json.push_str(alloc::string::String::from("\"));
                first = false;
            }
            json.push_str("}");
        }
        
        json.push_str("}");
        json
    }
}

// ============================================================================
// Logger
// ============================================================================

/// Distributed logger
pub struct DistributedLogger {
    pub entries: Mutex<Vec<LogEntry>>,
    pub next_entry_id: AtomicU64,
    pub max_entries: usize,
    pub stats: Mutex<LoggerStats>,
}

#[derive(Debug, Clone, Copy)]
pub struct LoggerStats {
    pub total_entries: u64,
    pub entries_by_level: [u64; 6],
    pub dropped_entries: u64,
}

impl Default for LoggerStats {
    fn default() -> Self {
        Self {
            total_entries: 0,
            entries_by_level: [0; 6],
            dropped_entries: 0,
        }
    }
}

impl DistributedLogger {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            next_entry_id: AtomicU64::new(1),
            max_entries,
            stats: Mutex::new(LoggerStats::default()),
        }
    }

    pub fn log(&self, level: LogLevel, service: String, message: String) {
        let entry = LogEntry::new(level, service, message);
        
        let mut entries = self.entries.lock();
        
        if entries.len() >= self.max_entries {
            entries.remove(0);
            self.stats.lock().dropped_entries.fetch_add(1, Ordering::Relaxed);
        }
        
        entries.push(entry);
        self.next_entry_id.fetch_add(1, Ordering::Relaxed);
        
        let mut stats = self.stats.lock();
        stats.total_entries.fetch_add(1, Ordering::Relaxed);
        stats.entries_by_level[level as usize].fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[distributed_log] [{:?}] {}: {}", level, service, message);
    }

    pub fn log_with_fields(&self, level: LogLevel, service: String, message: String, fields: BTreeMap<String, String>) {
        let mut entry = LogEntry::new(level, service, message);
        entry.fields = fields;
        
        let mut entries = self.entries.lock();
        
        if entries.len() >= self.max_entries {
            entries.remove(0);
            self.stats.lock().dropped_entries.fetch_add(1, Ordering::Relaxed);
        }
        
        entries.push(entry);
        self.next_entry_id.fetch_add(1, Ordering::Relaxed);
        
        let mut stats = self.stats.lock();
        stats.total_entries.fetch_add(1, Ordering::Relaxed);
        stats.entries_by_level[level as usize].fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[distributed_log] [{:?}] {}: {} with {} fields", level, service, message, fields.len());
    }

    pub fn search(&self, level_filter: Option<LogLevel>, service_filter: Option<String>, message_contains: Option<String>) -> Vec<LogEntry> {
        let entries = self.entries.lock();
        entries.iter().filter(|e| {
            if let Some(l) = level_filter {
                if e.level != l {
                    return false;
                }
            }
            if let Some(s) = &service_filter {
                if e.service != *s {
                    return false;
                }
            }
            if let Some(m) = &message_contains {
                if !e.message.contains(m) {
                    return false;
                }
            }
            true
        }).cloned().collect()
    }

    pub fn get_stats(&self) -> LoggerStats {
        *self.stats.lock()
    }
}
