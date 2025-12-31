//! Structured Logging Aggregation
//!
//! This module provides a comprehensive structured logging system with support for
//! multiple log levels, structured fields, and multiple output destinations including
//! remote transport.
//!
//! ## Key Features
//!
//! - **Structured Logging**: Log messages with key-value fields for machine parsing
//! - **Multiple Levels**: Trace, Debug, Info, Warn, Error levels with filtering
//! - **Multiple Outputs**: Console, file, and remote transport support
//! - **Buffering**: Efficient buffering for high-throughput scenarios
//! - **Thread-Safe**: All operations are thread-safe
//!
//! ## Log Levels
//!
//! - **Trace**: Extremely detailed debugging information
//! - **Debug**: Detailed debugging information
//! - **Info**: General informational messages
//! - **Warn**: Warning messages for potentially harmful situations
//! - **Error**: Error messages for error events
//!
//! ## Example
//!
//! ```rust
//! use kernel::monitoring::logging::{Logger, LogLevel};
//!
//! let logger = Logger::new(LogLevel::Info);
//! logger.info("system", "System started", &[("version", "1.0.0")]);
//! logger.error("network", "Connection failed", &[("addr", "10.0.0.1")]);
//! ```

extern crate alloc;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use crate::subsystems::sync::Mutex;

/// Maximum log entries in memory
const MAX_LOG_ENTRIES: usize = 10000;

/// Maximum log message length
const MAX_MESSAGE_LENGTH: usize = 4096;

/// Maximum number of fields per log entry
const MAX_FIELDS: usize = 32;

/// Maximum buffer size for remote transport
const REMOTE_BUFFER_SIZE: usize = 1000;

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl LogLevel {
    /// Get the log level name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }

    /// Check if this level should log for the given filter level
    pub fn should_log(&self, filter: LogLevel) -> bool {
        *self >= filter
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Log entry with structured fields
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Timestamp in nanoseconds
    pub timestamp: u64,
    /// Log level
    pub level: LogLevel,
    /// Target/component name
    pub target: String,
    /// Log message
    pub message: String,
    /// Structured fields
    pub fields: alloc::collections::BTreeMap<String, String>,
    /// Source file (optional)
    pub file: Option<String>,
    /// Source line (optional)
    pub line: Option<u32>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, target: String, message: String) -> Self {
        Self {
            timestamp: crate::subsystems::time::hrtime_nanos(),
            level,
            target,
            message,
            fields: alloc::collections::BTreeMap::new(),
            file: None,
            line: None,
        }
    }

    /// Add a field to the entry
    pub fn with_field(mut self, key: String, value: String) -> Self {
        self.fields.insert(key, value);
        self
    }

    /// Add multiple fields
    pub fn with_fields(mut self, fields: alloc::collections::BTreeMap<String, String>) -> Self {
        for (key, value) in fields {
            self.fields.insert(key, value);
        }
        self
    }

    /// Set source location
    pub fn with_location(mut self, file: String, line: u32) -> Self {
        self.file = Some(file);
        self.line = Some(line);
        self
    }

    /// Format as JSON
    pub fn to_json(&self) -> String {
        let mut s = format!(
            "{{\"timestamp\":{},\"level\":\"{}\",\"target\":\"{}\",\"message\":\"{}\"",
            self.timestamp,
            self.level.as_str(),
            escape_json(&self.target),
            escape_json(&self.message)
        );

        if !self.fields.is_empty() {
            s.push_str(",\"fields\":{");
            let mut first = true;
            for (key, value) in &self.fields {
                if !first {
                    s.push_str(",");
                }
                first = false;
                s.push_str(&format!("\"{}\":\"{}\"", escape_json(key), escape_json(value)));
            }
            s.push_str("}");
        }

        if let Some(file) = &self.file {
            s.push_str(&format!(",\"file\":\"{}\"", escape_json(file)));
        }
        if let Some(line) = self.line {
            s.push_str(&format!(",\"line\":{}", line));
        }

        s.push_str("}");
        s
    }

    /// Format as plain text
    pub fn to_text(&self) -> String {
        let ts_secs = self.timestamp / 1_000_000_000;
        let ts_nanos = self.timestamp % 1_000_000_000;

        let mut s = format!(
            "[{:5}.{:09}] {:5} {}: {}",
            ts_secs,
            ts_nanos,
            self.level.as_str(),
            self.target,
            self.message
        );

        if !self.fields.is_empty() {
            s.push_str(" {");
            let mut first = true;
            for (key, value) in &self.fields {
                if !first {
                    s.push_str(", ");
                }
                first = false;
                s.push_str(&format!("{}={}", key, value));
            }
            s.push_str("}");
        }

        s
    }
}

/// Escape JSON string
fn escape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

/// Log output trait
pub trait LogOutput: Send + Sync {
    /// Write a log entry
    fn write(&self, entry: &LogEntry);

    /// Flush any buffered output
    fn flush(&self);
}

/// Console output
pub struct ConsoleOutput {
    /// Use colors
    use_colors: bool,
}

impl ConsoleOutput {
    /// Create a new console output
    pub fn new(use_colors: bool) -> Self {
        Self { use_colors }
    }
}

impl LogOutput for ConsoleOutput {
    fn write(&self, entry: &LogEntry) {
        let text = entry.to_text();
        crate::println!("{}", text);
    }

    fn flush(&self) {
        // Console output is unbuffered
    }
}

/// Memory buffer output
pub struct MemoryBufferOutput {
    /// Buffer of log entries
    buffer: Mutex<Vec<LogEntry>>,
    /// Maximum buffer size
    max_size: usize,
}

impl MemoryBufferOutput {
    /// Create a new memory buffer output
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    /// Get all buffered entries
    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.buffer.lock().clone()
    }

    /// Clear the buffer
    pub fn clear(&self) {
        self.buffer.lock().clear();
    }

    /// Get buffer size
    pub fn len(&self) -> usize {
        self.buffer.lock().len()
    }
}

impl LogOutput for MemoryBufferOutput {
    fn write(&self, entry: &LogEntry) {
        let mut buffer = self.buffer.lock();
        if buffer.len() < self.max_size {
            buffer.push(entry.clone());
        }
        // When full, drop oldest entries (circular buffer)
        else {
            buffer.remove(0);
            buffer.push(entry.clone());
        }
    }

    fn flush(&self) {
        // Nothing to flush for in-memory buffer
    }
}

/// Remote log transport
pub struct RemoteLogTransport {
    /// Endpoint URL
    pub endpoint: String,
    /// Buffer for batching
    buffer: Mutex<Vec<LogEntry>>,
    /// Maximum buffer size before flushing
    max_buffer_size: usize,
    /// Authentication token (optional)
    auth_token: Mutex<Option<String>>,
}

impl RemoteLogTransport {
    /// Create a new remote transport
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            buffer: Mutex::new(Vec::with_capacity(REMOTE_BUFFER_SIZE)),
            max_buffer_size: REMOTE_BUFFER_SIZE,
            auth_token: Mutex::new(None),
        }
    }

    /// Set authentication token
    pub fn set_auth_token(&self, token: String) {
        *self.auth_token.lock() = Some(token);
    }

    /// Flush buffered entries to remote endpoint
    pub fn flush(&self) {
        let mut buffer = self.buffer.lock();
        if buffer.is_empty() {
            return;
        }

        // Convert to JSON batch
        let entries: Vec<LogEntry> = buffer.drain(..).collect();
        drop(buffer);

        // Send to remote endpoint
        self.send_batch(entries);
    }

    /// Send a batch of log entries
    fn send_batch(&self, entries: Vec<LogEntry>) {
        // In a real implementation, this would use HTTP/HTTPS to send the logs
        // For now, we just log that we're sending
        let count = entries.len();
        let endpoint = &self.endpoint;
        crate::log_debug!("[remote_transport] Sending {} log entries to {}", count, endpoint);

        // TODO: Implement actual HTTP/HTTPS transport
        // This would require:
        // 1. TCP connection establishment
        // 2. HTTP request formatting
        // 3. Authentication handling
        // 4. Retry logic for failures
    }

    /// Add an entry to the buffer
    fn buffer_entry(&self, entry: &LogEntry) {
        let mut buffer = self.buffer.lock();
        buffer.push(entry.clone());

        if buffer.len() >= self.max_buffer_size {
            // Flush when buffer is full
            drop(buffer);
            self.flush();
        }
    }
}

impl LogOutput for RemoteLogTransport {
    fn write(&self, entry: &LogEntry) {
        self.buffer_entry(entry);
    }

    fn flush(&self) {
        self.flush();
    }
}

/// Structured logger
pub struct Logger {
    /// Log entries
    entries: Mutex<Vec<LogEntry>>,
    /// Minimum log level
    level: Mutex<LogLevel>,
    /// Output destinations
    outputs: Mutex<Vec<Box<dyn LogOutput>>>,
    /// Total log count
    total_logs: Mutex<usize>,
    /// Dropped log count
    dropped_logs: Mutex<usize>,
}

impl Logger {
    /// Create a new logger with the given minimum level
    pub fn new(level: LogLevel) -> Self {
        Self {
            entries: Mutex::new(Vec::with_capacity(MAX_LOG_ENTRIES)),
            level: Mutex::new(level),
            outputs: Mutex::new(Vec::new()),
            total_logs: Mutex::new(0),
            dropped_logs: Mutex::new(0),
        }
    }

    /// Add an output destination
    pub fn add_output(&self, output: Box<dyn LogOutput>) {
        self.outputs.lock().push(output);
    }

    /// Set the minimum log level
    pub fn set_level(&self, level: LogLevel) {
        *self.level.lock() = level;
    }

    /// Get the current log level
    pub fn get_level(&self) -> LogLevel {
        *self.level.lock()
    }

    /// Log a message at the specified level
    pub fn log(&self, level: LogLevel, target: &str, message: &str) {
        self.log_fields(level, target, message, &[]);
    }

    /// Log a message with fields
    pub fn log_fields(&self, level: LogLevel, target: &str, message: &str, fields: &[(&str, &str)]) {
        // Check level filter
        let min_level = *self.level.lock();
        if !level.should_log(min_level) {
            return;
        }

        // Truncate message if too long
        let truncated_msg = if message.len() > MAX_MESSAGE_LENGTH {
            &message[..MAX_MESSAGE_LENGTH]
        } else {
            message
        };

        // Create log entry
        let mut entry = LogEntry::new(level, target.to_string(), truncated_msg.to_string());

        // Add fields
        let mut field_map = alloc::collections::BTreeMap::new();
        for (key, value) in fields.iter().take(MAX_FIELDS) {
            field_map.insert(key.to_string(), value.to_string());
        }
        entry = entry.with_fields(field_map);

        // Store entry
        {
            let mut entries = self.entries.lock();
            if entries.len() < MAX_LOG_ENTRIES {
                entries.push(entry.clone());
            } else {
                // Drop oldest entry
                entries.remove(0);
                entries.push(entry.clone());
                *self.dropped_logs.lock() += 1;
            }
            *self.total_logs.lock() += 1;
        }

        // Write to outputs
        let outputs = self.outputs.lock();
        for output in outputs.iter() {
            output.write(&entry);
        }
    }

    /// Log at TRACE level
    #[inline]
    pub fn trace(&self, target: &str, message: &str) {
        self.log(LogLevel::Trace, target, message);
    }

    /// Log at DEBUG level
    #[inline]
    pub fn debug(&self, target: &str, message: &str) {
        self.log(LogLevel::Debug, target, message);
    }

    /// Log at INFO level
    #[inline]
    pub fn info(&self, target: &str, message: &str) {
        self.log(LogLevel::Info, target, message);
    }

    /// Log at WARN level
    #[inline]
    pub fn warn(&self, target: &str, message: &str) {
        self.log(LogLevel::Warn, target, message);
    }

    /// Log at ERROR level
    #[inline]
    pub fn error(&self, target: &str, message: &str) {
        self.log(LogLevel::Error, target, message);
    }

    /// Get all log entries
    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.lock().clone()
    }

    /// Get log entries by level
    pub fn get_entries_by_level(&self, level: LogLevel) -> Vec<LogEntry> {
        self.entries
            .lock()
            .iter()
            .filter(|e| e.level == level)
            .cloned()
            .collect()
    }

    /// Get log entries by target
    pub fn get_entries_by_target(&self, target: &str) -> Vec<LogEntry> {
        self.entries
            .lock()
            .iter()
            .filter(|e| e.target == target)
            .cloned()
            .collect()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.entries.lock().clear();
    }

    /// Get statistics
    pub fn get_stats(&self) -> LoggerStats {
        LoggerStats {
            total_logs: *self.total_logs.lock(),
            current_entries: self.entries.lock().len(),
            dropped_logs: *self.dropped_logs.lock(),
            level: *self.level.lock(),
            output_count: self.outputs.lock().len(),
        }
    }

    /// Flush all outputs
    pub fn flush_outputs(&self) {
        let outputs = self.outputs.lock();
        for output in outputs.iter() {
            output.flush();
        }
    }
}

/// Logger statistics
#[derive(Debug, Clone, Copy)]
pub struct LoggerStats {
    /// Total logs emitted
    pub total_logs: usize,
    /// Current entries in buffer
    pub current_entries: usize,
    /// Logs dropped due to capacity
    pub dropped_logs: usize,
    /// Current log level
    pub level: LogLevel,
    /// Number of output destinations
    pub output_count: usize,
}

/// Global logger instance
static GLOBAL_LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

/// Initialize the global logger
pub fn init_logger(level: LogLevel) -> Result<(), &'static str> {
    let mut logger_opt = GLOBAL_LOGGER.lock();
    if logger_opt.is_some() {
        return Err("Logger already initialized");
    }

    let logger = Logger::new(level);

    // Add default outputs
    logger.add_output(Box::new(ConsoleOutput::new(true)));
    logger.add_output(Box::new(MemoryBufferOutput::new(MAX_LOG_ENTRIES)));

    *logger_opt = Some(logger);
    crate::log_info!("[logging] Structured logging initialized at {} level", level);
    Ok(())
}

/// Get the global logger
pub fn get_logger() -> Option<&'static Logger> {
    unsafe {
        GLOBAL_LOGGER.lock().as_ref().map(|l| {
            &*(l as *const Logger)
        })
    }
}

/// Initialize logger with INFO level
pub fn init_logger_defaults() -> Result<(), &'static str> {
    init_logger(LogLevel::Info)
}

/// Convenience function to log at TRACE level
pub fn log_trace(target: &str, message: &str) {
    if let Some(logger) = get_logger() {
        logger.trace(target, message);
    }
}

/// Convenience function to log at DEBUG level
pub fn log_debug(target: &str, message: &str) {
    if let Some(logger) = get_logger() {
        logger.debug(target, message);
    }
}

/// Convenience function to log at INFO level
pub fn log_info(target: &str, message: &str) {
    if let Some(logger) = get_logger() {
        logger.info(target, message);
    }
}

/// Convenience function to log at WARN level
pub fn log_warn(target: &str, message: &str) {
    if let Some(logger) = get_logger() {
        logger.warn(target, message);
    }
}

/// Convenience function to log at ERROR level
pub fn log_error(target: &str, message: &str) {
    if let Some(logger) = get_logger() {
        logger.error(target, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level() {
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
        assert!(LogLevel::Info.should_log(LogLevel::Info));
        assert!(LogLevel::Error.should_log(LogLevel::Warn));
        assert!(!LogLevel::Debug.should_log(LogLevel::Info));
    }

    #[test]
    fn test_log_entry() {
        let entry = LogEntry::new(LogLevel::Info, "test".to_string(), "test message".to_string())
            .with_field("key".to_string(), "value".to_string());

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "test");
        assert_eq!(entry.fields.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_logger() {
        let logger = Logger::new(LogLevel::Info);

        // Should log
        logger.info("test", "info message");
        logger.warn("test", "warn message");

        // Should not log (below threshold)
        logger.debug("test", "debug message");

        let entries = logger.get_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[1].level, LogLevel::Warn);
    }

    #[test]
    fn test_memory_buffer() {
        let output = MemoryBufferOutput::new(10);
        let entry = LogEntry::new(LogLevel::Info, "test".to_string(), "message".to_string());

        output.write(&entry);
        output.write(&entry);

        assert_eq!(output.len(), 2);

        output.clear();
        assert_eq!(output.len(), 0);
    }
}
