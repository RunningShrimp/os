//! Reliability module
//!
//! This module provides reliability-related types and functionality for the kernel.
//! It includes fault tolerance, error logging, and reliability metrics.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Fault type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    /// Memory fault
    Memory,
    /// Device fault
    Device,
    /// Software fault
    Software,
    /// Hardware fault
    Hardware,
    /// Network fault
    Network,
    /// File system fault
    FileSystem,
    /// User-space fault
    UserSpace,
}

/// Fault severity enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultSeverity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Checkpoint type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointType {
    /// Regular checkpoint
    Regular,
    /// Emergency checkpoint
    Emergency,
    /// Recovery checkpoint
    Recovery,
    /// Incremental checkpoint
    Incremental,
}

/// Error log manager
pub struct ErrorLogManager {
    logs: Vec<ErrorLogEntry>,
    max_entries: usize,
}

/// Error log entry
#[derive(Debug, Clone)]
pub struct ErrorLogEntry {
    pub timestamp: u64,
    pub fault_type: FaultType,
    pub severity: FaultSeverity,
    pub message: String,
    pub process_id: Option<u64>,
    pub stack_trace: Option<Vec<u64>>,
}

impl ErrorLogManager {
    pub fn new(max_entries: usize) -> Self {
        Self {
            logs: Vec::new(),
            max_entries,
        }
    }

    pub fn log_error(&mut self, entry: ErrorLogEntry) {
        self.logs.push(entry);
        if self.logs.len() > self.max_entries {
            self.logs.remove(0);
        }
    }

    pub fn get_logs(&self) -> &[ErrorLogEntry] {
        &self.logs
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }
}

/// Initialize error logging
pub fn init_error_logging(max_entries: usize) -> ErrorLogManager {
    ErrorLogManager::new(max_entries)
}

/// Get default error log manager
pub fn get_error_log_manager() -> &'static ErrorLogManager {
    static MANAGER: ErrorLogManager = ErrorLogManager {
        logs: Vec::new(),
        max_entries: 1000,
    };
    &MANAGER
}