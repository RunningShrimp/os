//! Structured boot logging
//!
//! This module provides structured, timestamped logging for the boot process
//! to aid in debugging and monitoring system startup.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

/// Boot log entry severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BootLogLevel {
    /// Debug information
    Debug = 0,
    /// Informational message
    Info = 1,
    /// Warning
    Warning = 2,
    /// Error
    Error = 3,
    /// Critical error
    Critical = 4,
}

impl BootLogLevel {
    /// Get log level name
    pub fn name(&self) -> &'static str {
        match self {
            BootLogLevel::Debug => "DEBUG",
            BootLogLevel::Info => "INFO",
            BootLogLevel::Warning => "WARN",
            BootLogLevel::Error => "ERROR",
            BootLogLevel::Critical => "CRITICAL",
        }
    }

    /// Get log level color code (for VGA output)
    pub fn color_code(&self) -> u8 {
        match self {
            BootLogLevel::Debug => 8,   // Dark gray
            BootLogLevel::Info => 7,    // White
            BootLogLevel::Warning => 14, // Yellow
            BootLogLevel::Error => 12,  // Red
            BootLogLevel::Critical => 4, // Bright red (or use 128+ for background)
        }
    }
}

/// Boot phase tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    /// Early initialization (arch-specific)
    EarlyInit,
    /// Boot parameter validation
    BootValidation,
    /// Memory initialization
    MemoryInit,
    /// Interrupt/trap initialization
    InterruptInit,
    /// Device initialization
    DeviceInit,
    /// AP (Application Processor) startup
    ApStartup,
    /// Scheduler initialization
    SchedulerInit,
    /// Filesystem initialization
    FilesystemInit,
    /// Network initialization (if enabled)
    NetworkInit,
    /// Services initialization
    ServicesInit,
    /// Boot complete
    BootComplete,
    /// Unknown phase
    Unknown,
}

impl BootPhase {
    /// Get phase name
    pub fn name(&self) -> &'static str {
        match self {
            BootPhase::EarlyInit => "EarlyInit",
            BootPhase::BootValidation => "BootValidation",
            BootPhase::MemoryInit => "MemoryInit",
            BootPhase::InterruptInit => "InterruptInit",
            BootPhase::DeviceInit => "DeviceInit",
            BootPhase::ApStartup => "ApStartup",
            BootPhase::SchedulerInit => "SchedulerInit",
            BootPhase::FilesystemInit => "FilesystemInit",
            BootPhase::NetworkInit => "NetworkInit",
            BootPhase::ServicesInit => "ServicesInit",
            BootPhase::BootComplete => "BootComplete",
            BootPhase::Unknown => "Unknown",
        }
    }

    /// Get phase order number (for sorting)
    pub fn order(&self) -> u32 {
        match self {
            BootPhase::EarlyInit => 0,
            BootPhase::BootValidation => 1,
            BootPhase::MemoryInit => 2,
            BootPhase::InterruptInit => 3,
            BootPhase::DeviceInit => 4,
            BootPhase::ApStartup => 5,
            BootPhase::SchedulerInit => 6,
            BootPhase::FilesystemInit => 7,
            BootPhase::NetworkInit => 8,
            BootPhase::ServicesInit => 9,
            BootPhase::BootComplete => 10,
            BootPhase::Unknown => 999,
        }
    }
}

/// Single boot log entry
#[derive(Debug, Clone)]
pub struct BootLogEntry {
    /// Timestamp in nanoseconds since boot start
    pub timestamp_ns: u64,
    /// Boot phase
    pub phase: BootPhase,
    /// Log level
    pub level: BootLogLevel,
    /// Log message
    pub message: String,
    /// Optional context information
    pub context: Option<String>,
}

impl BootLogEntry {
    /// Create a new boot log entry
    pub fn new(
        timestamp_ns: u64,
        phase: BootPhase,
        level: BootLogLevel,
        message: String,
    ) -> Self {
        Self {
            timestamp_ns,
            phase,
            level,
            message,
            context: None,
        }
    }

    /// Add context to the entry
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Format entry for display
    pub fn format(&self) -> String {
        let time_us = self.timestamp_ns / 1000;
        let phase = self.phase.name();
        let level = self.level.name();

        if let Some(ctx) = &self.context {
            alloc::format!(
                "[{:10}.{:06} us] [{:14}] [{:8}] {} - {}",
                time_us / 1_000_000,
                time_us % 1_000_000,
                phase,
                level,
                self.message,
                ctx
            )
        } else {
            alloc::format!(
                "[{:10}.{:06} us] [{:14}] [{:8}] {}",
                time_us / 1_000_000,
                time_us % 1_000_000,
                phase,
                level,
                self.message
            )
        }
    }
}

/// Structured boot logger
pub struct BootLogger {
    /// Start timestamp (when logger was created)
    start_timestamp: u64,
    /// Current boot phase
    current_phase: BootPhase,
    /// Log entries
    entries: Vec<BootLogEntry>,
    /// Maximum number of entries to store
    max_entries: usize,
    /// Whether to output logs immediately
    immediate_output: bool,
    /// Minimum log level to record
    min_level: BootLogLevel,
}

impl BootLogger {
    /// Create a new boot logger
    pub fn new() -> Self {
        Self {
            start_timestamp: 0,
            current_phase: BootPhase::EarlyInit,
            entries: Vec::new(),
            max_entries: 1000,
            immediate_output: true,
            min_level: BootLogLevel::Info,
        }
    }

    /// Initialize the logger (must be called first)
    pub fn init(&mut self) {
        self.start_timestamp = self.get_timestamp();
        self.log(BootPhase::EarlyInit, BootLogLevel::Info, "Boot logger initialized");
    }

    /// Log a message
    pub fn log(&mut self, phase: BootPhase, level: BootLogLevel, message: &str) {
        // Check if we should record this message
        if level < self.min_level {
            return;
        }

        let timestamp = self.get_timestamp().saturating_sub(self.start_timestamp);
        let entry = BootLogEntry::new(timestamp, phase, level, message.to_string());

        // Output immediately if enabled
        if self.immediate_output {
            crate::println!("{}", entry.format());
        }

        // Store entry
        if self.entries.len() < self.max_entries {
            self.entries.push(entry);
        } else {
            // Log buffer full - evict oldest entry
            crate::println!("[boot_log] WARNING: Log buffer full, dropping oldest entry");
            self.entries.remove(0);
            self.entries.push(entry);
        }
    }

    /// Log a message with context
    pub fn log_with_context(&mut self, phase: BootPhase, level: BootLogLevel, message: &str, context: &str) {
        // Check if we should record this message
        if level < self.min_level {
            return;
        }

        let timestamp = self.get_timestamp().saturating_sub(self.start_timestamp);
        let entry = BootLogEntry::new(timestamp, phase, level, message.to_string())
            .with_context(context.to_string());

        // Output immediately if enabled
        if self.immediate_output {
            crate::println!("{}", entry.format());
        }

        // Store entry
        if self.entries.len() < self.max_entries {
            self.entries.push(entry);
        }
    }

    /// Set current boot phase
    pub fn set_phase(&mut self, phase: BootPhase) {
        if self.current_phase != phase {
            self.log(
                self.current_phase,
                BootLogLevel::Info,
                &alloc::format!("Transitioning from {} to {}", self.current_phase.name(), phase.name()),
            );
            self.current_phase = phase;
        }
    }

    /// Get current boot phase
    pub fn current_phase(&self) -> BootPhase {
        self.current_phase
    }

    /// Set minimum log level
    pub fn set_min_level(&mut self, level: BootLogLevel) {
        self.min_level = level;
    }

    /// Enable/disable immediate output
    pub fn set_immediate_output(&mut self, enabled: bool) {
        self.immediate_output = enabled;
    }

    /// Get all log entries
    pub fn entries(&self) -> &[BootLogEntry] {
        &self.entries
    }

    /// Get entries filtered by level
    pub fn entries_by_level(&self, level: BootLogLevel) -> Vec<&BootLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.level == level)
            .collect()
    }

    /// Get entries filtered by phase
    pub fn entries_by_phase(&self, phase: BootPhase) -> Vec<&BootLogEntry> {
        self.entries
            .iter()
            .filter(|e| e.phase == phase)
            .collect()
    }

    /// Get error and critical entries
    pub fn errors(&self) -> Vec<&BootLogEntry] {
        self.entries
            .iter()
            .filter(|e| e.level >= BootLogLevel::Error)
            .collect()
    }

    /// Print a summary of the boot log
    pub fn print_summary(&self) {
        crate::println!();
        crate::println!("=== Boot Log Summary ===");
        crate::println!("Total entries: {}", self.entries.len());
        crate::println!(
            "Boot duration: {} ms",
            self.get_timestamp().saturating_sub(self.start_timestamp) / 1_000_000
        );

        let errors = self.errors();
        if !errors.is_empty() {
            crate::println!("Errors/Critical: {}", errors.len());
        }

        crate::println!();

        // Count entries per phase
        let mut phase_counts = alloc::collections::BTreeMap::new();
        for entry in &self.entries {
            *phase_counts.entry(entry.phase).or_insert(0) += 1;
        }

        crate::println!("Entries by phase:");
        for (phase, count) in phase_counts {
            crate::println!("  {}: {}", phase.name(), count);
        }
        crate::println!();
    }

    /// Get current timestamp in nanoseconds
    ///
    /// Note: This is a simplified implementation. In a real system,
    /// this would read from a proper timer.
    fn get_timestamp(&self) -> u64 {
        // GH-#1291: Use actual timer (TSC, generic timer, etc.)
        // See: https://github.com/npos/kernel/issues/1291
        // For now, return a simple counter or read from time module
        0 // Placeholder
    }
}

/// Global boot logger instance
static mut BOOT_LOGGER: Option<BootLogger> = None;

/// Get the global boot logger
pub fn get_boot_logger() -> Option<&'static mut BootLogger> {
    unsafe { BOOT_LOGGER.as_mut() }
}

/// Initialize the global boot logger
pub fn init_boot_logger() {
    unsafe {
        if BOOT_LOGGER.is_none() {
            let mut logger = BootLogger::new();
            logger.init();
            BOOT_LOGGER = Some(logger);
        }
    }
}

/// Log a boot message
pub fn log_boot(phase: BootPhase, level: BootLogLevel, message: &str) {
    if let Some(logger) = get_boot_logger() {
        logger.log(phase, level, message);
    }
}

/// Log a boot message with context
pub fn log_boot_with_context(phase: BootPhase, level: BootLogLevel, message: &str, context: &str) {
    if let Some(logger) = get_boot_logger() {
        logger.log_with_context(phase, level, message, context);
    }
}

/// Set current boot phase
pub fn set_boot_phase(phase: BootPhase) {
    if let Some(logger) = get_boot_logger() {
        logger.set_phase(phase);
    }
}

/// Print boot log summary
pub fn print_boot_summary() {
    if let Some(logger) = get_boot_logger() {
        logger.print_summary();
    }
}

/// Convenience macros for logging
#[macro_export]
macro_rules! boot_debug {
    ($msg:expr) => {
        $crate::debug::boot_log::log_boot(
            $crate::debug::boot_log::BootPhase::Unknown,
            $crate::debug::boot_log::BootLogLevel::Debug,
            $msg,
        )
    };
    ($msg:expr, $($arg:tt)*) => {
        $crate::debug::boot_log::log_boot(
            $crate::debug::boot_log::BootPhase::Unknown,
            $crate::debug::boot_log::BootLogLevel::Debug,
            &alloc::format!($msg, $($arg)*),
        )
    };
}

#[macro_export]
macro_rules! boot_info {
    ($phase:expr, $msg:expr) => {
        $crate::debug::boot_log::log_boot(
            $phase,
            $crate::debug::boot_log::BootLogLevel::Info,
            $msg,
        )
    };
    ($phase:expr, $msg:expr, $($arg:tt)*) => {
        $crate::debug::boot_log::log_boot(
            $phase,
            $crate::debug::boot_log::BootLogLevel::Info,
            &alloc::format!($msg, $($arg)*),
        )
    };
}

#[macro_export]
macro_rules! boot_warn {
    ($phase:expr, $msg:expr) => {
        $crate::debug::boot_log::log_boot(
            $phase,
            $crate::debug::boot_log::BootLogLevel::Warning,
            $msg,
        )
    };
}

#[macro_export]
macro_rules! boot_error {
    ($phase:expr, $msg:expr) => {
        $crate::debug::boot_log::log_boot(
            $phase,
            $crate::debug::boot_log::BootLogLevel::Error,
            $msg,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_logger_creation() {
        let logger = BootLogger::new();
        assert_eq!(logger.current_phase(), BootPhase::EarlyInit);
    }

    #[test]
    fn test_log_level_filtering() {
        let mut logger = BootLogger::new();
        logger.set_min_level(BootLogLevel::Warning);
        logger.set_immediate_output(false);

        logger.log(BootPhase::MemoryInit, BootLogLevel::Debug, "Debug msg");
        logger.log(BootPhase::MemoryInit, BootLogLevel::Warning, "Warning msg");

        assert_eq!(logger.entries().len(), 1);
        assert_eq!(logger.entries()[0].level, BootLogLevel::Warning);
    }

    #[test]
    fn test_entry_formatting() {
        let entry = BootLogEntry::new(
            1_500_000, // 1.5 ms
            BootPhase::MemoryInit,
            BootLogLevel::Info,
            "Test message".to_string(),
        );

        let formatted = entry.format();
        assert!(formatted.contains("MemoryInit"));
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("Test message"));
    }
}
