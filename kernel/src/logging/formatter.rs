//! Log formatters for different output formats
//!
//! This module provides various formatters for log records:
//! - Text formatter: Human-readable plain text output
//! - JSON formatter: Structured JSON output
//! - Compact formatter: Minimal output for high-volume logging
//! - Custom formatter: User-defined formatting

#![no_std]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write;

use crate::logging::error::Result;
use crate::logging::logger::LogRecord;
use crate::logging::metadata::Metadata;

/// Timestamp format options
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimestampFormat {
    /// Unix timestamp (seconds since epoch)
    Unix,
    /// Unix timestamp with milliseconds
    UnixMillis,
    /// Unix timestamp with microseconds
    UnixMicros,
    /// Unix timestamp with nanoseconds
    UnixNanos,
    /// ISO 8601 format (if time functions available)
    Iso8601,
    /// RFC 2822 format (if time functions available)
    Rfc2822,
    /// Simple HH:MM:SS format
    Simple,
    /// No timestamp
    None,
}

/// Color scheme for console output
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme {
    /// No colors
    None,
    /// ANSI colors (terminal)
    Ansi,
    /// 256-color palette
    Ansi256,
    /// RGB colors
    Rgb,
}

/// Text formatter for human-readable output
pub struct TextFormatter {
    /// Include timestamp
    include_timestamp: bool,
    /// Timestamp format
    timestamp_format: TimestampFormat,
    /// Include module path
    include_module: bool,
    /// Include file location
    include_file: bool,
    /// Include thread ID
    include_thread: bool,
    /// Color scheme
    colors: ColorScheme,
    /// Custom prefix
    prefix: Option<String>,
    /// Custom suffix
    suffix: Option<String>,
    /// Field separator
    separator: String,
}

impl TextFormatter {
    /// Create a new text formatter with default settings
    pub fn new() -> Self {
        Self {
            include_timestamp: true,
            timestamp_format: TimestampFormat::Simple,
            include_module: true,
            include_file: false,
            include_thread: false,
            colors: ColorScheme::Ansi,
            prefix: None,
            suffix: None,
            separator: " | ".to_string(),
        }
    }

    /// Set timestamp inclusion
    pub fn with_timestamp(mut self, include: bool) -> Self {
        self.include_timestamp = include;
        self
    }

    /// Set timestamp format
    pub fn with_timestamp_format(mut self, format: TimestampFormat) -> Self {
        self.timestamp_format = format;
        self
    }

    /// Set module inclusion
    pub fn with_module(mut self, include: bool) -> Self {
        self.include_module = include;
        self
    }

    /// Set file location inclusion
    pub fn with_file(mut self, include: bool) -> Self {
        self.include_file = include;
        self
    }

    /// Set thread ID inclusion
    pub fn with_thread(mut self, include: bool) -> Self {
        self.include_thread = include;
        self
    }

    /// Set color scheme
    pub fn with_colors(mut self, colors: ColorScheme) -> Self {
        self.colors = colors;
        self
    }

    /// Set custom prefix
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Set custom suffix
    pub fn with_suffix(mut self, suffix: String) -> Self {
        self.suffix = Some(suffix);
        self
    }

    /// Set field separator
    pub fn with_separator(mut self, separator: String) -> Self {
        self.separator = separator;
        self
    }

    /// Format timestamp
    fn format_timestamp(&self, timestamp: u64) -> String {
        match self.timestamp_format {
            TimestampFormat::Unix => format!("{}", timestamp / 1_000_000_000),
            TimestampFormat::UnixMillis => format!("{}", timestamp / 1_000_000),
            TimestampFormat::UnixMicros => format!("{}", timestamp / 1_000),
            TimestampFormat::UnixNanos => format!("{}", timestamp),
            TimestampFormat::Iso8601 => {
                // In a real implementation, convert to ISO 8601
                format!("{:.9}", timestamp)
            }
            TimestampFormat::Rfc2822 => {
                // In a real implementation, convert to RFC 2822
                format!("{:.9}", timestamp)
            }
            TimestampFormat::Simple => {
                // Format as HH:MM:SS.mmm
                let total_secs = (timestamp / 1_000_000_000) % 86_400;
                let hours = total_secs / 3600;
                let mins = (total_secs % 3600) / 60;
                let secs = total_secs % 60;
                let millis = (timestamp % 1_000_000_000) / 1_000_000;
                format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
            }
            TimestampFormat::None => String::new(),
        }
    }

    /// Get color codes for log level
    fn get_level_colors(&self, level: &crate::logging::logger::LogLevel) -> (Option<&'static str>, Option<&'static str>) {
        if self.colors == ColorScheme::None {
            return (None, None);
        }

        match level {
            crate::logging::logger::LogLevel::Trace => (Some("\x1b[90m"), Some("\x1b[0m")),  // Gray
            crate::logging::logger::LogLevel::Debug => (Some("\x1b[36m"), Some("\x1b[0m")),  // Cyan
            crate::logging::logger::LogLevel::Info => (Some("\x1b[32m"), Some("\x1b[0m")),   // Green
            crate::logging::logger::LogLevel::Warn => (Some("\x1b[33m"), Some("\x1b[0m")),   // Yellow
            crate::logging::logger::LogLevel::Error => (Some("\x1b[31m"), Some("\x1b[0m")),  // Red
            crate::logging::logger::LogLevel::Fatal => (Some("\x1b[35m"), Some("\x1b[0m")),  // Magenta
            crate::logging::logger::LogLevel::Off => (Some("\x1b[0m"), Some("\x1b[0m")),     // Reset
        }
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Formatter trait for custom formatters
pub trait Formatter: Send + Sync {
    /// Format a log record into a string
    fn format(&self, record: &LogRecord) -> Result<String>;

    /// Format multiple records (batch formatting)
    fn format_batch(&self, records: &[LogRecord]) -> Result<Vec<String>> {
        records.iter().map(|r| self.format(r)).collect()
    }

    /// Get the content type this formatter produces
    fn content_type(&self) -> &str {
        "text/plain"
    }
}

impl Formatter for TextFormatter {
    fn format(&self, record: &LogRecord) -> Result<String> {
        let mut output = String::new();

        // Add prefix
        if let Some(prefix) = &self.prefix {
            write!(output, "{}", prefix)?;
        }

        // Add timestamp
        if self.include_timestamp && self.timestamp_format != TimestampFormat::None {
            let ts = self.format_timestamp(record.timestamp);
            write!(output, "[{}]", ts)?;
        }

        // Add log level with colors
        let (color_open, color_close) = self.get_level_colors(&record.level);
        if let Some(open) = color_open {
            write!(output, "{}", open)?;
        }
        write!(output, "[{:5}]", record.level)?;
        if let Some(close) = color_close {
            write!(output, "{}", close)?;
        }

        // Add thread ID
        if self.include_thread {
            write!(output, "[T{:03}]", record.thread_id % 1000)?;
        }

        // Add separator
        if !output.is_empty() {
            write!(output, "{}", self.separator)?;
        }

        // Add module path
        if self.include_module {
            if let Some(module) = &record.module_path {
                // Shorten module path if too long
                let short_module = if module.len() > 30 {
                    format!("...{}", &module[module.len() - 27..])
                } else {
                    module.clone()
                };
                write!(output, "{}:", short_module)?;
            }
        }

        // Add file location
        if self.include_file {
            if let Some(file) = &record.file {
                write!(output, "{}:", file)?;
                if let Some(line) = record.line {
                    write!(output, "{}:", line)?;
                }
            }
        }

        // Add message
        writeln!(output, "{}", record.message)?;

        // Add metadata if present
        if !record.metadata.is_empty() {
            write!(output, "  Metadata: ")?;
            let mut first = true;
            for (key, value) in record.metadata.iter() {
                if !first {
                    write!(output, ", ")?;
                }
                write!(output, "{}={}", key, value)?;
                first = false;
            }
            writeln!(output)?;
        }

        // Add request/trace IDs if present
        if let Some(rid) = &record.request_id {
            writeln!(output, "  Request-ID: {}", rid)?;
        }
        if let Some(tid) = &record.trace_id {
            writeln!(output, "  Trace-ID: {}", tid)?;
        }

        // Add suffix
        if let Some(suffix) = &self.suffix {
            write!(output, "{}", suffix)?;
        }

        Ok(output)
    }
}

/// JSON formatter for structured logging
pub struct JsonFormatter {
    /// Pretty print JSON
    pretty: bool,
    /// Include timestamp
    include_timestamp: bool,
    /// Include all metadata
    include_metadata: bool,
    /// Include source location
    include_location: bool,
    /// Custom fields to add to every record
    custom_fields: Metadata,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new() -> Self {
        Self {
            pretty: false,
            include_timestamp: true,
            include_metadata: true,
            include_location: true,
            custom_fields: Metadata::new(),
        }
    }

    /// Enable pretty printing
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Include timestamp
    pub fn with_timestamp(mut self, include: bool) -> Self {
        self.include_timestamp = include;
        self
    }

    /// Include metadata
    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    /// Include source location
    pub fn with_location(mut self, include: bool) -> Self {
        self.include_location = include;
        self
    }

    /// Add a custom field to all records
    pub fn with_custom_field(mut self, key: String, value: String) -> Self {
        self.custom_fields.insert(key, value);
        self
    }

    /// Escape JSON string
    fn escape_string(s: &str) -> String {
        let mut escaped = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => escaped.push_str("\\\""),
                '\\' => escaped.push_str("\\\\"),
                '\n' => escaped.push_str("\\n"),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                c if c.is_control() => escaped.push_str(&format!("\\u{:04x}", c as u32)),
                c => escaped.push(c),
            }
        }
        escaped
    }

    /// Format metadata as JSON object
    fn format_metadata(&self, metadata: &Metadata) -> String {
        if metadata.is_empty() {
            return "{}".to_string();
        }

        let mut parts = Vec::new();
        for (key, value) in metadata.iter() {
            parts.push(format!(
                "\"{}\":\"{}\"",
                Self::escape_string(key),
                Self::escape_string(value)
            ));
        }

        format!("{{{}}}", parts.join(","))
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for JsonFormatter {
    fn format(&self, record: &LogRecord) -> Result<String> {
        let mut json = String::new();

        if self.pretty {
            json.push('{');

            // Timestamp
            if self.include_timestamp {
                json.push_str(&format!("\n  \"timestamp\": {},", record.timestamp));
            }

            // Level
            json.push_str(&format!("\n  \"level\": \"{}\",", record.level));

            // Message
            json.push_str(&format!(
                "\n  \"message\": \"{}\",",
                Self::escape_string(&record.message)
            ));

            // Module
            if let Some(module) = &record.module_path {
                json.push_str(&format!(
                    "\n  \"module\": \"{}\",",
                    Self::escape_string(module)
                ));
            }

            // Location
            if self.include_location {
                if let Some(file) = &record.file {
                    json.push_str(&format!(
                        "\n  \"file\": \"{}\",",
                        Self::escape_string(file)
                    ));
                    if let Some(line) = record.line {
                        json.push_str(&format!("\n  \"line\": {},", line));
                    }
                }
            }

            // Thread ID
            json.push_str(&format!("\n  \"thread_id\": {},", record.thread_id));

            // Request/Trace IDs
            if let Some(rid) = &record.request_id {
                json.push_str(&format!(
                    "\n  \"request_id\": \"{}\",",
                    Self::escape_string(rid)
                ));
            }
            if let Some(tid) = &record.trace_id {
                json.push_str(&format!(
                    "\n  \"trace_id\": \"{}\",",
                    Self::escape_string(tid)
                ));
            }

            // Metadata
            if self.include_metadata && !record.metadata.is_empty() {
                json.push_str(&format!(
                    "\n  \"metadata\": {},",
                    self.format_metadata(&record.metadata)
                ));
            }

            // Custom fields
            if !self.custom_fields.is_empty() {
                for (key, value) in self.custom_fields.iter() {
                    json.push_str(&format!(
                        "\n  \"{}\": \"{}\",",
                        Self::escape_string(key),
                        Self::escape_string(value)
                    ));
                }
            }

            // Remove trailing comma
            if json.ends_with(',') {
                json.pop();
                json.push('\n');
            }

            json.push_str("\n}");
        } else {
            json.push('{');

            // Timestamp
            if self.include_timestamp {
                json.push_str(&format!("\"timestamp\":{},", record.timestamp));
            }

            // Level
            json.push_str(&format!("\"level\":\"{}\",", record.level));

            // Message
            json.push_str(&format!(
                "\"message\":\"{}\"",
                Self::escape_string(&record.message)
            ));

            // Module
            if let Some(module) = &record.module_path {
                json.push_str(&format!(
                    ",\"module\":\"{}\"",
                    Self::escape_string(module)
                ));
            }

            // Location
            if self.include_location {
                if let Some(file) = &record.file {
                    json.push_str(&format!(
                        ",\"file\":\"{}\"",
                        Self::escape_string(file)
                    ));
                    if let Some(line) = record.line {
                        json.push_str(&format!(",\"line\":{}", line));
                    }
                }
            }

            // Thread ID
            json.push_str(&format!(",\"thread_id\":{}", record.thread_id));

            // Request/Trace IDs
            if let Some(rid) = &record.request_id {
                json.push_str(&format!(
                    ",\"request_id\":\"{}\"",
                    Self::escape_string(rid)
                ));
            }
            if let Some(tid) = &record.trace_id {
                json.push_str(&format!(
                    ",\"trace_id\":\"{}\"",
                    Self::escape_string(tid)
                ));
            }

            // Metadata
            if self.include_metadata && !record.metadata.is_empty() {
                json.push_str(&format!(
                    ",\"metadata\":{}",
                    self.format_metadata(&record.metadata)
                ));
            }

            // Custom fields
            if !self.custom_fields.is_empty() {
                for (key, value) in self.custom_fields.iter() {
                    json.push_str(&format!(
                        ",\"{}\":\"{}\"",
                        Self::escape_string(key),
                        Self::escape_string(value)
                    ));
                }
            }

            json.push('}');
        }

        json.push('\n');
        Ok(json)
    }

    fn content_type(&self) -> &str {
        "application/json"
    }
}

/// Compact formatter for high-volume logging
pub struct CompactFormatter {
    /// Include timestamp
    include_timestamp: bool,
    /// Include level
    include_level: bool,
    /// Include module
    include_module: bool,
}

impl CompactFormatter {
    /// Create a new compact formatter
    pub fn new() -> Self {
        Self {
            include_timestamp: true,
            include_level: true,
            include_module: false,
        }
    }

    /// Set timestamp inclusion
    pub fn with_timestamp(mut self, include: bool) -> Self {
        self.include_timestamp = include;
        self
    }

    /// Set level inclusion
    pub fn with_level(mut self, include: bool) -> Self {
        self.include_level = include;
        self
    }

    /// Set module inclusion
    pub fn with_module(mut self, include: bool) -> Self {
        self.include_module = include;
        self
    }
}

impl Default for CompactFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for CompactFormatter {
    fn format(&self, record: &LogRecord) -> Result<String> {
        let mut output = String::new();

        if self.include_timestamp {
            write!(output, "{} ", record.timestamp)?;
        }

        if self.include_level {
            write!(output, "{} ", record.level.as_str())?;
        }

        if self.include_module {
            if let Some(module) = &record.module_path {
                write!(output, "{} ", module)?;
            }
        }

        writeln!(output, "{}", record.message)?;

        Ok(output)
    }

    fn content_type(&self) -> &str {
        "text/plain"
    }
}

/// Custom formatter with user-defined formatting function
pub struct CustomFormatter<F>
where
    F: Fn(&LogRecord) -> String + Send + Sync,
{
    format_fn: F,
    content_type: String,
}

impl<F> CustomFormatter<F>
where
    F: Fn(&LogRecord) -> String + Send + Sync,
{
    /// Create a new custom formatter
    pub fn new(format_fn: F) -> Self {
        Self {
            format_fn,
            content_type: "text/plain".to_string(),
        }
    }

    /// Set content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = content_type;
        self
    }
}

impl<F> Formatter for CustomFormatter<F>
where
    F: Fn(&LogRecord) -> String + Send + Sync,
{
    fn format(&self, record: &LogRecord) -> Result<String> {
        Ok((self.format_fn)(record))
    }

    fn content_type(&self) -> &str {
        &self.content_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::logger::{LogLevel, LogRecord};

    #[test]
    fn test_text_formatter_basic() {
        let formatter = TextFormatter::new();
        let record = LogRecord::new(
            LogLevel::Info,
            Some("test_module".to_string()),
            Some("test.rs".to_string()),
            Some(42),
            "Test message".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("INFO"));
        assert!(output.contains("Test message"));
        assert!(output.contains("test_module"));
    }

    #[test]
    fn test_text_formatter_no_timestamp() {
        let formatter = TextFormatter::new().with_timestamp(false);
        let record = LogRecord::new(
            LogLevel::Debug,
            None,
            None,
            None,
            "No timestamp".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("DEBUG"));
        assert!(output.contains("No timestamp"));
    }

    #[test]
    fn test_text_formatter_with_file() {
        let formatter = TextFormatter::new().with_file(true);
        let record = LogRecord::new(
            LogLevel::Warn,
            None,
            Some("file.rs".to_string()),
            Some(123),
            "Warning".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("file.rs"));
        assert!(output.contains("123"));
    }

    #[test]
    fn test_json_formatter_basic() {
        let formatter = JsonFormatter::new();
        let record = LogRecord::new(
            LogLevel::Info,
            Some("test".to_string()),
            None,
            None,
            "JSON test".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("\"level\""));
        assert!(output.contains("\"message\""));
        assert!(output.contains("JSON test"));
        assert!(output.contains("\"module\""));
    }

    #[test]
    fn test_json_formatter_pretty() {
        let formatter = JsonFormatter::new().with_pretty(true);
        let record = LogRecord::new(
            LogLevel::Error,
            None,
            None,
            None,
            "Error message".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains('\n'));
        assert!(output.contains("\"level\":"));
        assert!(output.contains("\"message\":"));
    }

    #[test]
    fn test_json_formatter_escape() {
        let formatter = JsonFormatter::new();
        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Message with \"quotes\" and\nnewlines".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("\\\"quotes\\\""));
        assert!(output.contains("\\n"));
    }

    #[test]
    fn test_json_formatter_custom_field() {
        let formatter = JsonFormatter::new()
            .with_custom_field("app".to_string(), "test_app".to_string())
            .with_custom_field("env".to_string(), "dev".to_string());

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("\"app\":\"test_app\""));
        assert!(output.contains("\"env\":\"dev\""));
    }

    #[test]
    fn test_compact_formatter() {
        let formatter = CompactFormatter::new();
        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Compact log".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("Compact log"));
        // Should be more compact than text formatter
        assert!(output.len() < 100);
    }

    #[test]
    fn test_compact_formatter_minimal() {
        let formatter = CompactFormatter::new()
            .with_timestamp(false)
            .with_level(false);

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Just message".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("Just message"));
        // Should be very compact
        assert!(output.len() < 30);
    }

    #[test]
    fn test_custom_formatter() {
        let formatter = CustomFormatter::new(|record| {
            format!("CUSTOM: {} - {}", record.level, record.message)
        });

        let record = LogRecord::new(
            LogLevel::Debug,
            None,
            None,
            None,
            "Custom test".to_string(),
        );

        let output = formatter.format(&record).unwrap();
        assert_eq!(output, "CUSTOM: DEBUG - Custom test\n");
    }

    #[test]
    fn test_timestamp_format() {
        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );

        // Unix timestamp
        let formatter = TextFormatter::new()
            .with_timestamp_format(TimestampFormat::Unix);
        let output = formatter.format(&record).unwrap();
        // Should contain timestamp in brackets
        assert!(output.contains('[') && output.contains(']'));

        // No timestamp
        let formatter = TextFormatter::new()
            .with_timestamp_format(TimestampFormat::None);
        let output = formatter.format(&record).unwrap();
        // Output should still exist but without timestamp
        assert!(output.contains("INFO"));
    }

    #[test]
    fn test_formatter_content_type() {
        let text = TextFormatter::new();
        assert_eq!(text.content_type(), "text/plain");

        let json = JsonFormatter::new();
        assert_eq!(json.content_type(), "application/json");

        let compact = CompactFormatter::new();
        assert_eq!(compact.content_type(), "text/plain");
    }

    #[test]
    fn test_formatter_with_metadata() {
        let formatter = TextFormatter::new();
        let mut record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );
        record.metadata.insert("key1".to_string(), "value1".to_string());
        record.metadata.insert("key2".to_string(), "value2".to_string());

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("Metadata:"));
        assert!(output.contains("key1=value1"));
        assert!(output.contains("key2=value2"));
    }

    #[test]
    fn test_formatter_with_request_trace_ids() {
        let formatter = TextFormatter::new();
        let mut record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );
        record.request_id = Some("req-123".to_string());
        record.trace_id = Some("trace-456".to_string());

        let output = formatter.format(&record).unwrap();
        assert!(output.contains("Request-ID: req-123"));
        assert!(output.contains("Trace-ID: trace-456"));
    }

    #[test]
    fn test_batch_formatting() {
        let formatter = JsonFormatter::new();
        let records = vec![
            LogRecord::new(LogLevel::Info, None, None, None, "First".to_string()),
            LogRecord::new(LogLevel::Debug, None, None, None, "Second".to_string()),
        ];

        let results = formatter.format_batch(&records).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].contains("\"level\":\"INFO\""));
        assert!(results[1].contains("\"level\":\"DEBUG\""));
    }
}
