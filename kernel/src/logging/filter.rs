//! Log filtering mechanisms
//!
//! This module provides various log filtering strategies:
//! - Component-level filtering (per-module log levels)
//! - Severity-based filtering
//! - Pattern-based filtering (regex and glob patterns)
//! - Dynamic filter updates
//! - Filter chains for complex filtering logic

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::logging::logger::LogRecord;

/// Filter trait for log record filtering
pub trait Filter: Send + Sync {
    /// Check if a log record should be filtered out (true = filter out)
    fn should_filter(&self, record: &LogRecord) -> bool;

    /// Get the filter name
    fn name(&self) -> &str {
        "unknown"
    }

    /// Check if filter is enabled
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Severity-based filter
pub struct SeverityFilter {
    /// Minimum log level to allow
    min_level: crate::logging::logger::LogLevel,
    /// Maximum log level to allow (optional)
    max_level: Option<crate::logging::logger::LogLevel>,
    /// Enabled flag
    enabled: AtomicBool,
}

impl SeverityFilter {
    /// Create a new severity filter
    pub fn new(min_level: crate::logging::logger::LogLevel) -> Self {
        Self {
            min_level,
            max_level: None,
            enabled: AtomicBool::new(true),
        }
    }

    /// Set maximum log level
    pub fn with_max_level(mut self, max_level: crate::logging::logger::LogLevel) -> Self {
        self.max_level = Some(max_level);
        self
    }

    /// Enable or disable the filter
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Check if filter is enabled
    pub fn is_enabled_filter(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Filter for SeverityFilter {
    fn should_filter(&self, record: &LogRecord) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        // Filter if below minimum level
        if record.level < self.min_level {
            return true;
        }

        // Filter if above maximum level (if set)
        if let Some(max) = self.max_level {
            if record.level > max {
                return true;
            }
        }

        false
    }

    fn name(&self) -> &str {
        "severity"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Module-level filter
pub struct ModuleFilter {
    /// Module level configurations (module_path -> log_level)
    module_levels: alloc::collections::BTreeMap<String, crate::logging::logger::LogLevel>,
    /// Default log level for unmatched modules
    default_level: crate::logging::logger::LogLevel,
    /// Exact match only (no prefix matching)
    exact_match: bool,
    /// Enabled flag
    enabled: AtomicBool,
}

impl ModuleFilter {
    /// Create a new module filter
    pub fn new(default_level: crate::logging::logger::LogLevel) -> Self {
        Self {
            module_levels: alloc::collections::BTreeMap::new(),
            default_level,
            exact_match: false,
            enabled: AtomicBool::new(true),
        }
    }

    /// Add a module-level configuration
    pub fn add_module(&mut self, module: String, level: crate::logging::logger::LogLevel) {
        self.module_levels.insert(module, level);
    }

    /// Set exact match mode
    pub fn with_exact_match(mut self, exact: bool) -> Self {
        self.exact_match = exact;
        self
    }

    /// Enable or disable the filter
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Get the log level for a module
    fn get_level_for_module(&self, module_path: &Option<String>) -> crate::logging::logger::LogLevel {
        let module = match module_path {
            Some(m) => m,
            None => return self.default_level,
        };

        if self.exact_match {
            // Exact match only
            if let Some(&level) = self.module_levels.get(module) {
                return level;
            }
        } else {
            // Try exact match first
            if let Some(&level) = self.module_levels.get(module) {
                return level;
            }

            // Try prefix match (longest match wins)
            let mut best_match: Option<&String> = None;
            for known_module in self.module_levels.keys() {
                if module.starts_with(known_module) {
                    match best_match {
                        None => best_match = Some(known_module),
                        Some(best) if known_module.len() > best.len() => {
                            best_match = Some(known_module);
                        }
                        _ => {}
                    }
                }
            }

            if let Some(best) = best_match {
                return self.module_levels[best];
            }
        }

        self.default_level
    }
}

impl Filter for ModuleFilter {
    fn should_filter(&self, record: &LogRecord) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        let level = self.get_level_for_module(&record.module_path);
        record.level < level
    }

    fn name(&self) -> &str {
        "module"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Pattern-based filter (simple glob matching)
pub struct PatternFilter {
    /// Patterns to filter out
    exclude_patterns: Vec<String>,
    /// Patterns to include (overrides exclude)
    include_patterns: Vec<String>,
    /// Match against message content
    match_message: bool,
    /// Match against module path
    match_module: bool,
    /// Case sensitive matching
    case_sensitive: bool,
    /// Enabled flag
    enabled: AtomicBool,
}

impl PatternFilter {
    /// Create a new pattern filter
    pub fn new() -> Self {
        Self {
            exclude_patterns: Vec::new(),
            include_patterns: Vec::new(),
            match_message: true,
            match_module: false,
            case_sensitive: false,
            enabled: AtomicBool::new(true),
        }
    }

    /// Add an exclude pattern
    pub fn add_exclude(&mut self, pattern: String) {
        self.exclude_patterns.push(pattern);
    }

    /// Add an include pattern
    pub fn add_include(&mut self, pattern: String) {
        self.include_patterns.push(pattern);
    }

    /// Set what to match against
    pub fn match_message(mut self, match_message: bool) -> Self {
        self.match_message = match_message;
        self
    }

    pub fn match_module(mut self, match_module: bool) -> Self {
        self.match_module = match_module;
        self
    }

    /// Set case sensitivity
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Enable or disable the filter
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Simple glob pattern matching
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        let text = if self.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };
        let pattern = if self.case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        // Convert glob pattern to simple regex-like matching
        // * matches any sequence
        // ? matches any single character

        self.glob_match(&text, &pattern)
    }

    /// Glob matching implementation
    fn glob_match(&self, text: &str, pattern: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        self.glob_match_helper(&text_chars, 0, &pattern_chars, 0)
    }

    fn glob_match_helper(
        &self,
        text: &[char],
        text_pos: usize,
        pattern: &[char],
        pattern_pos: usize,
    ) -> bool {
        // If we've reached the end of both pattern and text, it's a match
        if pattern_pos == pattern.len() {
            return text_pos == text.len();
        }

        // If pattern has more chars but text is done, match only if remaining pattern is all *
        if text_pos == text.len() {
            return pattern[pattern_pos..].iter().all(|&c| c == '*');
        }

        // Handle wildcard *
        if pattern[pattern_pos] == '*' {
            // Try skipping the * and matching from current text position
            if self.glob_match_helper(text, text_pos, pattern, pattern_pos + 1) {
                return true;
            }
            // Try consuming one text char and keeping the *
            return self.glob_match_helper(text, text_pos + 1, pattern, pattern_pos);
        }

        // Handle wildcard ?
        if pattern[pattern_pos] == '?' || pattern[pattern_pos] == text[text_pos] {
            return self.glob_match_helper(text, text_pos + 1, pattern, pattern_pos + 1);
        }

        false
    }
}

impl Default for PatternFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for PatternFilter {
    fn should_filter(&self, record: &LogRecord) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        // Check include patterns first (whitelist)
        for pattern in &self.include_patterns {
            let matches = if self.match_message && self.match_module {
                self.matches_pattern(&record.message, pattern)
                    || self.matches_pattern(
                        record.module_path.as_ref().map(|s| s.as_str()).unwrap_or(""),
                        pattern,
                    )
            } else if self.match_message {
                self.matches_pattern(&record.message, pattern)
            } else if self.match_module {
                self.matches_pattern(
                    record.module_path.as_ref().map(|s| s.as_str()).unwrap_or(""),
                    pattern,
                )
            } else {
                false
            };

            if matches {
                return false; // Don't filter if it matches include pattern
            }
        }

        // Check exclude patterns (blacklist)
        for pattern in &self.exclude_patterns {
            let matches = if self.match_message && self.match_module {
                self.matches_pattern(&record.message, pattern)
                    || self.matches_pattern(
                        record.module_path.as_ref().map(|s| s.as_str()).unwrap_or(""),
                        pattern,
                    )
            } else if self.match_message {
                self.matches_pattern(&record.message, pattern)
            } else if self.match_module {
                self.matches_pattern(
                    record.module_path.as_ref().map(|s| s.as_str()).unwrap_or(""),
                    pattern,
                )
            } else {
                false
            };

            if matches {
                return true; // Filter out if it matches exclude pattern
            }
        }

        false
    }

    fn name(&self) -> &str {
        "pattern"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Metadata-based filter
pub struct MetadataFilter {
    /// Required metadata keys (must all be present)
    required_keys: Vec<String>,
    /// Required metadata key-value pairs
    required_values: alloc::collections::BTreeMap<String, String>,
    /// Excluded metadata keys
    excluded_keys: Vec<String>,
    /// Enabled flag
    enabled: AtomicBool,
}

impl MetadataFilter {
    /// Create a new metadata filter
    pub fn new() -> Self {
        Self {
            required_keys: Vec::new(),
            required_values: alloc::collections::BTreeMap::new(),
            excluded_keys: Vec::new(),
            enabled: AtomicBool::new(true),
        }
    }

    /// Add a required metadata key
    pub fn add_required_key(&mut self, key: String) {
        self.required_keys.push(key);
    }

    /// Add a required metadata key-value pair
    pub fn add_required_value(&mut self, key: String, value: String) {
        self.required_values.insert(key, value);
    }

    /// Add an excluded metadata key
    pub fn add_excluded_key(&mut self, key: String) {
        self.excluded_keys.push(key);
    }

    /// Enable or disable the filter
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl Default for MetadataFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for MetadataFilter {
    fn should_filter(&self, record: &LogRecord) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        // Check required keys
        for key in &self.required_keys {
            if !record.metadata.contains(key) {
                return true; // Filter out if required key is missing
            }
        }

        // Check required values
        for (key, expected_value) in &self.required_values {
            match record.metadata.get(key) {
                Some(actual_value) if actual_value == expected_value => {}
                _ => return true, // Filter out if value doesn't match
            }
        }

        // Check excluded keys
        for key in &self.excluded_keys {
            if record.metadata.contains(key) {
                return true; // Filter out if excluded key is present
            }
        }

        false
    }

    fn name(&self) -> &str {
        "metadata"
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Filter stack that applies multiple filters in sequence
pub struct FilterStack {
    /// Filters to apply
    filters: Vec<Box<dyn Filter>>,
    /// Combination mode: true = AND, false = OR
    combine_mode: bool, // true = all must match, false = any must match
}

impl FilterStack {
    /// Create a new filter stack
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            combine_mode: true, // AND mode by default
        }
    }

    /// Add a filter to the stack
    pub fn add(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    /// Set combination mode
    pub fn with_combine_mode(mut self, and_mode: bool) -> Self {
        self.combine_mode = and_mode;
        self
    }

    /// Remove all filters
    pub fn clear(&mut self) {
        self.filters.clear();
    }

    /// Get number of filters
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

impl Default for FilterStack {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for FilterStack {
    fn should_filter(&self, record: &LogRecord) -> bool {
        if self.filters.is_empty() {
            return false;
        }

        if self.combine_mode {
            // AND mode: filter if all filters say to filter
            self.filters.iter().all(|f| f.should_filter(record))
        } else {
            // OR mode: filter if any filter says to filter
            self.filters.iter().any(|f| f.should_filter(record))
        }
    }

    fn name(&self) -> &str {
        "stack"
    }

    fn is_enabled(&self) -> bool {
        self.filters.iter().all(|f| f.is_enabled())
    }
}

/// Dynamic filter that can be updated at runtime
pub struct DynamicFilter {
    /// Current filter (can be swapped out)
    current_filter: spin::Mutex<Box<dyn Filter>>,
    /// Filter name
    name_str: String,
}

impl DynamicFilter {
    /// Create a new dynamic filter
    pub fn new(filter: Box<dyn Filter>) -> Self {
        Self {
            current_filter: spin::Mutex::new(filter),
            name_str: "dynamic".to_string(),
        }
    }

    /// Replace the current filter
    pub fn replace(&self, new_filter: Box<dyn Filter>) {
        let mut guard = self.current_filter.lock();
        *guard = new_filter;
    }

    /// Update filter with a name
    pub fn with_name(mut self, name: String) -> Self {
        self.name_str = name;
        self
    }
}

impl Filter for DynamicFilter {
    fn should_filter(&self, record: &LogRecord) -> bool {
        let guard = self.current_filter.lock();
        guard.should_filter(record)
    }

    fn name(&self) -> &str {
        &self.name_str
    }

    fn is_enabled(&self) -> bool {
        let guard = self.current_filter.lock();
        guard.is_enabled()
    }
}

/// Builder for creating complex filters
pub struct FilterBuilder {
    stack: FilterStack,
}

impl FilterBuilder {
    /// Create a new filter builder
    pub fn new() -> Self {
        Self {
            stack: FilterStack::new(),
        }
    }

    /// Add a severity filter
    pub fn severity(
        mut self,
        min_level: crate::logging::logger::LogLevel,
    ) -> Self {
        let filter = SeverityFilter::new(min_level);
        self.stack.add(Box::new(filter));
        self
    }

    /// Add a severity filter with max level
    pub fn severity_range(
        mut self,
        min_level: crate::logging::logger::LogLevel,
        max_level: crate::logging::logger::LogLevel,
    ) -> Self {
        let filter = SeverityFilter::new(min_level).with_max_level(max_level);
        self.stack.add(Box::new(filter));
        self
    }

    /// Add a module filter
    pub fn module(mut self, default_level: crate::logging::logger::LogLevel) -> Self {
        let filter = ModuleFilter::new(default_level);
        self.stack.add(Box::new(filter));
        self
    }

    /// Add a pattern filter
    pub fn pattern(mut self) -> Self {
        let filter = PatternFilter::new();
        self.stack.add(Box::new(filter));
        self
    }

    /// Add a metadata filter
    pub fn metadata(mut self) -> Self {
        let filter = MetadataFilter::new();
        self.stack.add(Box::new(filter));
        self
    }

    /// Set combination mode
    pub fn combine_and(mut self) -> Self {
        self.stack.combine_mode = true;
        self
    }

    pub fn combine_or(mut self) -> Self {
        self.stack.combine_mode = false;
        self
    }

    /// Build the filter stack
    pub fn build(self) -> FilterStack {
        self.stack
    }
}

impl Default for FilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::logger::{LogLevel, LogRecord};

    #[test]
    fn test_severity_filter() {
        let filter = SeverityFilter::new(LogLevel::Info);

        let debug_record = LogRecord::new(
            LogLevel::Debug,
            None,
            None,
            None,
            "Test".to_string(),
        );
        assert!(filter.should_filter(&debug_record));

        let info_record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );
        assert!(!filter.should_filter(&info_record));
    }

    #[test]
    fn test_severity_filter_with_max() {
        let filter = SeverityFilter::new(LogLevel::Info)
            .with_max_level(LogLevel::Warn);

        let debug_record = LogRecord::new(LogLevel::Debug, None, None, None, "Test".to_string());
        assert!(filter.should_filter(&debug_record));

        let warn_record = LogRecord::new(LogLevel::Warn, None, None, None, "Test".to_string());
        assert!(!filter.should_filter(&warn_record));

        let error_record = LogRecord::new(LogLevel::Error, None, None, None, "Test".to_string());
        assert!(filter.should_filter(&error_record));
    }

    #[test]
    fn test_severity_filter_enable_disable() {
        let filter = SeverityFilter::new(LogLevel::Info);

        let info_record = LogRecord::new(LogLevel::Info, None, None, None, "Test".to_string());
        assert!(!filter.should_filter(&info_record));

        filter.set_enabled(false);
        assert!(!filter.should_filter(&info_record));
        assert!(!filter.is_enabled_filter());

        filter.set_enabled(true);
        assert!(!filter.should_filter(&info_record));
        assert!(filter.is_enabled_filter());
    }

    #[test]
    fn test_module_filter() {
        let mut filter = ModuleFilter::new(LogLevel::Info);
        filter.add_module("verbose_module".to_string(), LogLevel::Trace);

        let verbose_record = LogRecord::new(
            LogLevel::Trace,
            Some("verbose_module".to_string()),
            None,
            None,
            "Test".to_string(),
        );
        assert!(!filter.should_filter(&verbose_record));

        let normal_record = LogRecord::new(
            LogLevel::Debug,
            Some("normal_module".to_string()),
            None,
            None,
            "Test".to_string(),
        );
        assert!(filter.should_filter(&normal_record));
    }

    #[test]
    fn test_module_filter_prefix_match() {
        let mut filter = ModuleFilter::new(LogLevel::Info);
        filter.add_module("myapp".to_string(), LogLevel::Debug);

        let record = LogRecord::new(
            LogLevel::Debug,
            Some("myapp::submodule".to_string()),
            None,
            None,
            "Test".to_string(),
        );
        assert!(!filter.should_filter(&record));
    }

    #[test]
    fn test_module_filter_exact_match() {
        let mut filter = ModuleFilter::new(LogLevel::Info).with_exact_match(true);
        filter.add_module("myapp".to_string(), LogLevel::Debug);

        let record = LogRecord::new(
            LogLevel::Debug,
            Some("myapp::submodule".to_string()),
            None,
            None,
            "Test".to_string(),
        );
        // Should be filtered because exact match required
        assert!(filter.should_filter(&record));
    }

    #[test]
    fn test_pattern_filter_exclude() {
        let mut filter = PatternFilter::new();
        filter.add_exclude("password".to_string());

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "User password is secret".to_string(),
        );
        assert!(filter.should_filter(&record));
    }

    #[test]
    fn test_pattern_filter_include() {
        let mut filter = PatternFilter::new();
        filter.add_exclude("debug".to_string());
        filter.add_include("important".to_string());

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "This is important debug info".to_string(),
        );
        // Should not be filtered because it matches include pattern
        assert!(!filter.should_filter(&record));
    }

    #[test]
    fn test_pattern_filter_glob() {
        let mut filter = PatternFilter::new();
        filter.add_exclude("test*".to_string());

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "test message".to_string(),
        );
        assert!(filter.should_filter(&record));
    }

    #[test]
    fn test_metadata_filter_required_keys() {
        let mut filter = MetadataFilter::new();
        filter.add_required_key("user_id".to_string());

        let mut record1 = LogRecord::new(LogLevel::Info, None, None, None, "Test".to_string());
        record1.metadata.insert("user_id".to_string(), "123".to_string());
        assert!(!filter.should_filter(&record1));

        let record2 = LogRecord::new(LogLevel::Info, None, None, None, "Test".to_string());
        assert!(filter.should_filter(&record2));
    }

    #[test]
    fn test_metadata_filter_required_values() {
        let mut filter = MetadataFilter::new();
        filter.add_required_value("environment".to_string(), "production".to_string());

        let mut record1 = LogRecord::new(LogLevel::Info, None, None, None, "Test".to_string());
        record1.metadata.insert("environment".to_string(), "production".to_string());
        assert!(!filter.should_filter(&record1));

        let mut record2 = LogRecord::new(LogLevel::Info, None, None, None, "Test".to_string());
        record2.metadata.insert("environment".to_string(), "development".to_string());
        assert!(filter.should_filter(&record2));
    }

    #[test]
    fn test_metadata_filter_excluded_keys() {
        let mut filter = MetadataFilter::new();
        filter.add_excluded_key("sensitive".to_string());

        let mut record1 = LogRecord::new(LogLevel::Info, None, None, None, "Test".to_string());
        record1.metadata.insert("sensitive".to_string(), "true".to_string());
        assert!(filter.should_filter(&record1));

        let record2 = LogRecord::new(LogLevel::Info, None, None, None, "Test".to_string());
        assert!(!filter.should_filter(&record2));
    }

    #[test]
    fn test_filter_stack_and() {
        let mut stack = FilterStack::new().with_combine_mode(true);

        let severity_filter = Box::new(SeverityFilter::new(LogLevel::Info)) as Box<dyn Filter>;
        let pattern_filter = {
            let mut f = PatternFilter::new();
            f.add_exclude("password".to_string());
            Box::new(f) as Box<dyn Filter>
        };

        stack.add(severity_filter);
        stack.add(pattern_filter);

        let debug_record = LogRecord::new(
            LogLevel::Debug,
            None,
            None,
            None,
            "Test".to_string(),
        );
        assert!(stack.should_filter(&debug_record)); // Filtered by severity

        let password_record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "User password is secret".to_string(),
        );
        assert!(stack.should_filter(&password_record)); // Filtered by pattern

        let normal_record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Normal message".to_string(),
        );
        assert!(!stack.should_filter(&normal_record)); // Not filtered
    }

    #[test]
    fn test_filter_stack_or() {
        let mut stack = FilterStack::new().with_combine_mode(false);

        let severity_filter = Box::new(SeverityFilter::new(LogLevel::Error)) as Box<dyn Filter>;
        let pattern_filter = {
            let mut f = PatternFilter::new();
            f.add_exclude("password".to_string());
            Box::new(f) as Box<dyn Filter>
        };

        stack.add(severity_filter);
        stack.add(pattern_filter);

        let info_record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "User message".to_string(),
        );
        assert!(!stack.should_filter(&info_record)); // Not filtered by either

        let error_record = LogRecord::new(
            LogLevel::Error,
            None,
            None,
            None,
            "Error occurred".to_string(),
        );
        assert!(stack.should_filter(&error_record)); // Filtered by severity

        let password_record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "User password".to_string(),
        );
        assert!(stack.should_filter(&password_record)); // Filtered by pattern
    }

    #[test]
    fn test_filter_builder() {
        let stack = FilterBuilder::new()
            .severity(LogLevel::Info)
            .combine_and()
            .build();

        let debug_record = LogRecord::new(
            LogLevel::Debug,
            None,
            None,
            None,
            "Test".to_string(),
        );
        assert!(stack.should_filter(&debug_record));
    }

    #[test]
    fn test_dynamic_filter() {
        let initial_filter = Box::new(SeverityFilter::new(LogLevel::Info)) as Box<dyn Filter>;
        let dynamic = DynamicFilter::new(initial_filter);

        let record = LogRecord::new(LogLevel::Debug, None, None, None, "Test".to_string());
        assert!(dynamic.should_filter(&record));

        let new_filter = Box::new(SeverityFilter::new(LogLevel::Trace)) as Box<dyn Filter>;
        dynamic.replace(new_filter);

        assert!(!dynamic.should_filter(&record));
    }
}
