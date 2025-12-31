//! Core logging infrastructure for the NOS kernel
//!
//! This module provides a comprehensive, thread-safe logging system with:
//! - Multiple log levels (Trace, Debug, Info, Warn, Error, Fatal)
//! - Multiple output targets (console, file, network, syslog)
//! - Per-module log level configuration
//! - Async logging backend for minimal overhead
//! - Structured logging support

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use spin::{Mutex, MutexGuard};

use super::async_logger::AsyncLogger;
use super::error::{LogError, Result};
use super::filter::{Filter, FilterStack};
use super::formatter::Formatter;
use super::metadata::Metadata;
use super::rotation::RotationPolicy;

/// Log level enumeration
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum LogLevel {
    /// Trace level - Extremely detailed debugging information
    Trace = 0,
    /// Debug level - Detailed debugging information
    Debug = 1,
    /// Info level - General informational messages
    Info = 2,
    /// Warn level - Warning messages
    Warn = 3,
    /// Error level - Error messages
    Error = 4,
    /// Fatal level - Critical errors that may cause system failure
    Fatal = 5,
    /// Off level - Logging disabled
    Off = 6,
}

impl LogLevel {
    /// Convert from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "trace" => Some(LogLevel::Trace),
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" | "warning" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            "fatal" | "critical" => Some(LogLevel::Fatal),
            "off" => Some(LogLevel::Off),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
            LogLevel::Off => "OFF",
        }
    }

    /// Get the ANSI color code for this log level
    #[cfg(feature = "ansi_colors")]
    pub fn color_code(self) -> &'static str {
        match self {
            LogLevel::Trace => "\x1b[90m",    // Bright black (gray)
            LogLevel::Debug => "\x1b[36m",    // Cyan
            LogLevel::Info => "\x1b[32m",     // Green
            LogLevel::Warn => "\x1b[33m",     // Yellow
            LogLevel::Error => "\x1b[31m",    // Red
            LogLevel::Fatal => "\x1b[35m",    // Magenta
            LogLevel::Off => "\x1b[0m",       // Reset
        }
    }

    /// Get the ANSI reset code
    #[cfg(feature = "ansi_colors")]
    pub fn reset_code() -> &'static str {
        "\x1b[0m"
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Log record containing all information about a log event
#[derive(Debug)]
pub struct LogRecord {
    /// Log level
    pub level: LogLevel,
    /// Module path
    pub module_path: Option<String>,
    /// File name
    pub file: Option<String>,
    /// Line number
    pub line: Option<u32>,
    /// Log message
    pub message: String,
    /// Timestamp (nanoseconds since boot)
    pub timestamp: u64,
    /// Thread ID
    pub thread_id: u64,
    /// Structured metadata
    pub metadata: Metadata,
    /// Request ID (if available)
    pub request_id: Option<String>,
    /// Trace ID (if available)
    pub trace_id: Option<String>,
}

impl LogRecord {
    /// Create a new log record
    pub fn new(
        level: LogLevel,
        module_path: Option<String>,
        file: Option<String>,
        line: Option<u32>,
        message: String,
    ) -> Self {
        Self {
            level,
            module_path,
            file,
            line,
            message,
            timestamp: Self::current_timestamp(),
            thread_id: Self::current_thread_id(),
            metadata: Metadata::new(),
            request_id: None,
            trace_id: None,
        }
    }

    /// Get current timestamp in nanoseconds
    fn current_timestamp() -> u64 {
        // In a real implementation, this would read from a hardware timer
        // For now, use a static counter
        static TIMESTAMP: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP.fetch_add(1, Ordering::Relaxed)
    }

    /// Get current thread ID
    fn current_thread_id() -> u64 {
        // In a real implementation, this would get the actual thread ID
        // For now, use a thread-local counter
        static THREAD_ID: AtomicU64 = AtomicU64::new(0);
        THREAD_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// Add metadata to this record
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set the request ID
    pub fn with_request_id(mut self, id: String) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Set the trace ID
    pub fn with_trace_id(mut self, id: String) -> Self {
        self.trace_id = Some(id);
        self
    }
}

/// Log target trait for output destinations
pub trait LogTarget: Send + Sync {
    /// Write a log record
    fn write(&self, record: &LogRecord) -> Result<()>;

    /// Flush any buffered output
    fn flush(&self) -> Result<()>;

    /// Check if this target supports the specified feature
    fn supports_feature(&self, _feature: &str) -> bool {
        false
    }
}

/// Console target that writes to stdout/stderr
pub struct ConsoleTarget {
    /// Use stderr for errors and above
    use_stderr_for_errors: bool,
    /// Enable colors
    enable_colors: bool,
}

impl ConsoleTarget {
    /// Create a new console target
    pub fn new() -> Self {
        Self {
            use_stderr_for_errors: true,
            enable_colors: true,
        }
    }

    /// Configure whether to use stderr for errors
    pub fn with_stderr_for_errors(mut self, use_stderr: bool) -> Self {
        self.use_stderr_for_errors = use_stderr;
        self
    }

    /// Configure color support
    pub fn with_colors(mut self, enable: bool) -> Self {
        self.enable_colors = enable;
        self
    }
}

impl Default for ConsoleTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl LogTarget for ConsoleTarget {
    fn write(&self, record: &LogRecord) -> Result<()> {
        // In a real implementation, this would write to actual console
        // For now, we'll format and pretend
        let formatted = format!("[{}] {}\n", record.level, record.message);

        if self.use_stderr_for_errors && record.level >= LogLevel::Error {
            // Write to stderr
            let _ = formatted;
        } else {
            // Write to stdout
            let _ = formatted;
        }

        Ok(())
    }

    fn flush(&self) -> Result<()> {
        // Flush console output
        Ok(())
    }

    fn supports_feature(&self, feature: &str) -> bool {
        match feature {
            "colors" => self.enable_colors,
            "stderr" => true,
            _ => false,
        }
    }
}

/// File target that writes to log files
pub struct FileTarget {
    /// File path
    path: String,
    /// Formatter to use
    formatter: Box<dyn Formatter>,
    /// Rotation policy
    rotation_policy: Option<Box<dyn RotationPolicy>>,
    /// Current file size
    current_size: Arc<AtomicUsize>,
}

impl FileTarget {
    /// Create a new file target
    pub fn new(path: String, formatter: Box<dyn Formatter>) -> Self {
        Self {
            path,
            formatter,
            rotation_policy: None,
            current_size: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Set rotation policy
    pub fn with_rotation(mut self, policy: Box<dyn RotationPolicy>) -> Self {
        self.rotation_policy = Some(policy);
        self
    }
}

impl LogTarget for FileTarget {
    fn write(&self, record: &LogRecord) -> Result<()> {
        // Format the record
        let formatted = self.formatter.format(record)?;

        // In a real implementation, this would write to the file
        let _ = formatted;

        // Update file size
        self.current_size.fetch_add(formatted.len(), Ordering::Relaxed);

        // Check if rotation is needed
        if let Some(policy) = &self.rotation_policy {
            if policy.should_rotate(self.path.as_str(), self.current_size.load(Ordering::Relaxed)) {
                policy.rotate(self.path.as_str())?;
            }
        }

        Ok(())
    }

    fn flush(&self) -> Result<()> {
        // Flush file buffer
        Ok(())
    }
}

/// Core logger structure
pub struct Logger {
    /// Global log level
    global_level: Mutex<LogLevel>,
    /// Per-module log levels
    module_levels: Mutex<BTreeMap<String, LogLevel>>,
    /// Log targets
    targets: Mutex<Vec<Box<dyn LogTarget>>>,
    /// Filter stack
    filters: Mutex<FilterStack>,
    /// Async logger backend
    async_logger: Option<Arc<AsyncLogger>>,
    /// Logger name
    name: String,
    /// Minimum log level
    min_level: LogLevel,
}

impl Logger {
    /// Create a new logger
    pub fn new(name: String) -> Self {
        Self {
            global_level: Mutex::new(LogLevel::Info),
            module_levels: Mutex::new(BTreeMap::new()),
            targets: Mutex::new(Vec::new()),
            filters: Mutex::new(FilterStack::new()),
            async_logger: None,
            name,
            min_level: LogLevel::Trace,
        }
    }

    /// Set the global log level
    pub fn set_level(&self, level: LogLevel) {
        *self.global_level.lock() = level;
    }

    /// Get the global log level
    pub fn level(&self) -> LogLevel {
        *self.global_level.lock()
    }

    /// Set log level for a specific module
    pub fn set_module_level(&self, module: &str, level: LogLevel) {
        self.module_levels
            .lock()
            .insert(module.to_string(), level);
    }

    /// Get log level for a specific module
    pub fn module_level(&self, module: &str) -> Option<LogLevel> {
        self.module_levels.lock().get(module).copied()
    }

    /// Add a log target
    pub fn add_target(&self, target: Box<dyn LogTarget>) {
        self.targets.lock().push(target);
    }

    /// Add a filter
    pub fn add_filter(&self, filter: Box<dyn Filter>) {
        self.filters.lock().add(filter);
    }

    /// Enable async logging
    pub fn enable_async(&mut self, buffer_size: usize) -> Result<()> {
        let async_logger = AsyncLogger::new(buffer_size)?;
        self.async_logger = Some(Arc::new(async_logger));
        Ok(())
    }

    /// Check if a log record should be logged
    fn should_log(&self, record: &LogRecord) -> bool {
        // Check global level
        if record.level < *self.global_level.lock() {
            return false;
        }

        // Check module-level override
        if let Some(module) = &record.module_path {
            if let Some(&level) = self.module_levels.lock().get(module) {
                if record.level < level {
                    return false;
                }
            }
        }

        // Check filters
        !self.filters.lock().should_filter(record)
    }

    /// Log a record
    pub fn log(&self, record: LogRecord) -> Result<()> {
        // Check if we should log this record
        if !self.should_log(&record) {
            return Ok(());
        }

        // Use async logger if available
        if let Some(async_logger) = &self.async_logger {
            return async_logger.submit(record);
        }

        // Synchronous logging
        let targets: MutexGuard<Vec<Box<dyn LogTarget>>> = self.targets.lock();
        for target in targets.iter() {
            let target: &Box<dyn LogTarget> = target;
            target.write(&record)?;
        }

        Ok(())
    }

    /// Flush all targets
    pub fn flush(&self) -> Result<()> {
        if let Some(async_logger) = &self.async_logger {
            async_logger.flush()?;
        }

        let targets: MutexGuard<Vec<Box<dyn LogTarget>>> = self.targets.lock();
        for target in targets.iter() {
            let target: &Box<dyn LogTarget> = target;
            target.flush()?;
        }

        Ok(())
    }

    /// Log at a specific level
    #[inline]
    pub fn log_at_level(
        &self,
        level: LogLevel,
        module_path: Option<&str>,
        file: Option<&str>,
        line: Option<u32>,
        message: &str,
    ) -> Result<()> {
        let record = LogRecord::new(
            level,
            module_path.map(|s| s.to_string()),
            file.map(|s| s.to_string()),
            line,
            message.to_string(),
        );
        self.log(record)
    }

    /// Convenience methods for each log level
    pub fn trace(&self, message: &str) -> Result<()> {
        self.log_at_level(LogLevel::Trace, None, None, None, message)
    }

    pub fn debug(&self, message: &str) -> Result<()> {
        self.log_at_level(LogLevel::Debug, None, None, None, message)
    }

    pub fn info(&self, message: &str) -> Result<()> {
        self.log_at_level(LogLevel::Info, None, None, None, message)
    }

    pub fn warn(&self, message: &str) -> Result<()> {
        self.log_at_level(LogLevel::Warn, None, None, None, message)
    }

    pub fn error(&self, message: &str) -> Result<()> {
        self.log_at_level(LogLevel::Error, None, None, None, message)
    }

    pub fn fatal(&self, message: &str) -> Result<()> {
        self.log_at_level(LogLevel::Fatal, None, None, None, message)
    }
}

/// Global logger instance
static GLOBAL_LOGGER: Mutex<Option<Arc<Logger>>> = Mutex::new(None);

/// Initialize the global logger
pub fn init_logger(logger: Logger) -> Result<()> {
    let mut global = GLOBAL_LOGGER.lock();
    if global.is_some() {
        return Err(LogError::AlreadyInitialized);
    }
    *global = Some(Arc::new(logger));
    Ok(())
}

/// Get the global logger
pub fn logger() -> Option<Arc<Logger>> {
    GLOBAL_LOGGER.lock().as_ref().map(Arc::clone)
}

/// Log a message at the specified level
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        if let Some(logger) = $crate::logging::logger::logger() {
            let message = alloc::format!($($arg)*);
            let _ = logger.log_at_level(
                $level,
                Some(module_path!()),
                Some(file!()),
                Some(line!()),
                &message,
            );
        }
    };
}

/// Log at trace level
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::log!($crate::logging::logger::LogLevel::Trace, $($arg)*)
    };
}

/// Log at debug level
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::log!($crate::logging::logger::LogLevel::Debug, $($arg)*)
    };
}

/// Log at info level
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log!($crate::logging::logger::LogLevel::Info, $($arg)*)
    };
}

/// Log at warn level
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log!($crate::logging::logger::LogLevel::Warn, $($arg)*)
    };
}

/// Log at error level
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log!($crate::logging::logger::LogLevel::Error, $($arg)*)
    };
}

/// Log at fatal level
#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => {
        $crate::log!($crate::logging::logger::LogLevel::Fatal, $($arg)*)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("trace"), Some(LogLevel::Trace));
        assert_eq!(LogLevel::from_str("DEBUG"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("Info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("WARN"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("warning"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("Error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::from_str("FATAL"), Some(LogLevel::Fatal));
        assert_eq!(LogLevel::from_str("off"), Some(LogLevel::Off));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
        assert!(LogLevel::Fatal < LogLevel::Off);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn test_log_record_creation() {
        let record = LogRecord::new(
            LogLevel::Info,
            Some("test_module".to_string()),
            Some("test.rs".to_string()),
            Some(42),
            "Test message".to_string(),
        );

        assert_eq!(record.level, LogLevel::Info);
        assert_eq!(record.module_path, Some("test_module".to_string()));
        assert_eq!(record.file, Some("test.rs".to_string()));
        assert_eq!(record.line, Some(42));
        assert_eq!(record.message, "Test message");
    }

    #[test]
    fn test_log_record_with_metadata() {
        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        )
        .with_metadata("key".to_string(), "value".to_string());

        assert!(record.metadata.contains("key"));
        assert_eq!(record.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_logger_creation() {
        let logger = Logger::new("test".to_string());
        assert_eq!(logger.level(), LogLevel::Info);
    }

    #[test]
    fn test_logger_set_level() {
        let logger = Logger::new("test".to_string());
        logger.set_level(LogLevel::Debug);
        assert_eq!(logger.level(), LogLevel::Debug);
    }

    #[test]
    fn test_logger_module_level() {
        let logger = Logger::new("test".to_string());
        logger.set_module_level("test_module", LogLevel::Trace);
        assert_eq!(
            logger.module_level("test_module"),
            Some(LogLevel::Trace)
        );
        assert_eq!(logger.module_level("other"), None);
    }

    #[test]
    fn test_logger_should_log() {
        let logger = Logger::new("test".to_string());

        // Default level is Info
        let debug_record = LogRecord::new(
            LogLevel::Debug,
            None,
            None,
            None,
            "Test".to_string(),
        );
        assert!(!logger.should_log(&debug_record));

        let info_record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );
        assert!(logger.should_log(&info_record));
    }

    #[test]
    fn test_logger_module_override() {
        let logger = Logger::new("test".to_string());
        logger.set_module_level("verbose_module", LogLevel::Trace);

        let record = LogRecord::new(
            LogLevel::Trace,
            Some("verbose_module".to_string()),
            None,
            None,
            "Test".to_string(),
        );

        // Module-level override should allow trace
        assert!(logger.should_log(&record));
    }

    #[test]
    fn test_console_target() {
        let target = ConsoleTarget::new();
        assert!(target.supports_feature("stderr"));
        assert!(!target.supports_feature("invalid"));

        let colored = ConsoleTarget::new().with_colors(true);
        assert!(colored.enable_colors);
    }

    #[test]
    fn test_global_logger() {
        let logger = Logger::new("global".to_string());
        assert!(init_logger(logger).is_ok());

        let retrieved = logger();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "global");

        // Reset for other tests
        *GLOBAL_LOGGER.lock() = None;
    }

    #[test]
    fn test_global_logger_already_initialized() {
        let logger1 = Logger::new("first".to_string());
        assert!(init_logger(logger1).is_ok());

        let logger2 = Logger::new("second".to_string());
        assert!(init_logger(logger2).is_err());

        // Reset
        *GLOBAL_LOGGER.lock() = None;
    }
}
