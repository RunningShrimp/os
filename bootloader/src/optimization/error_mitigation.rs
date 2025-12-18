//! Error Mitigation - Advanced Error Handling and Recovery
//!
//! Implements error mitigation strategies:
//! - Error classification
//! - Mitigation strategies
//! - Recovery tactics
//! - Error reporting

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Hardware,
    Software,
    Memory,
    IO,
    Validation,
    Configuration,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Hardware => write!(f, "Hardware"),
            ErrorCategory::Software => write!(f, "Software"),
            ErrorCategory::Memory => write!(f, "Memory"),
            ErrorCategory::IO => write!(f, "I/O"),
            ErrorCategory::Validation => write!(f, "Validation"),
            ErrorCategory::Configuration => write!(f, "Configuration"),
        }
    }
}

/// Error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Minor => write!(f, "Minor"),
            ErrorSeverity::Moderate => write!(f, "Moderate"),
            ErrorSeverity::Major => write!(f, "Major"),
            ErrorSeverity::Critical => write!(f, "Critical"),
        }
    }
}

/// Mitigation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MitigationStrategy {
    Ignore,
    Retry,
    Fallback,
    Reduce,
    Skip,
    Halt,
}

impl fmt::Display for MitigationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MitigationStrategy::Ignore => write!(f, "Ignore"),
            MitigationStrategy::Retry => write!(f, "Retry"),
            MitigationStrategy::Fallback => write!(f, "Fallback"),
            MitigationStrategy::Reduce => write!(f, "Reduce"),
            MitigationStrategy::Skip => write!(f, "Skip"),
            MitigationStrategy::Halt => write!(f, "Halt"),
        }
    }
}

/// Boot error
#[derive(Debug, Clone)]
pub struct BootError {
    pub error_code: u32,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub message: String,
    pub context: String,
    pub timestamp: u64,
    pub recovery_attempted: bool,
}

impl BootError {
    /// Create new error
    pub fn new(code: u32, category: ErrorCategory, msg: &str) -> Self {
        BootError {
            error_code: code,
            category,
            severity: ErrorSeverity::Moderate,
            message: String::from(msg),
            context: String::new(),
            timestamp: 0,
            recovery_attempted: false,
        }
    }

    /// Set severity
    pub fn set_severity(&mut self, severity: ErrorSeverity) {
        self.severity = severity;
    }

    /// Add context
    pub fn add_context(&mut self, ctx: &str) {
        self.context = String::from(ctx);
    }

    /// Mark recovery attempted
    pub fn mark_recovery_attempted(&mut self) {
        self.recovery_attempted = true;
    }
}

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Error 0x{:x}: {} [{}] {}",
            self.error_code, self.category, self.severity, self.message
        )
    }
}

/// Error mitigation record
#[derive(Debug, Clone)]
pub struct MitigationRecord {
    pub error: BootError,
    pub strategy: MitigationStrategy,
    pub success: bool,
    pub attempts: u32,
}

impl MitigationRecord {
    /// Create new record
    pub fn new(error: BootError, strategy: MitigationStrategy) -> Self {
        MitigationRecord {
            error,
            strategy,
            success: false,
            attempts: 0,
        }
    }

    /// Increment attempts
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }

    /// Mark success
    pub fn mark_success(&mut self) {
        self.success = true;
    }
}

impl fmt::Display for MitigationRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} [{}]",
            self.error, self.strategy, if self.success { "Success" } else { "Failed" }
        )
    }
}

/// Error Mitigator
pub struct ErrorMitigator {
    errors: Vec<BootError>,
    mitigations: Vec<MitigationRecord>,
    total_errors: u32,
    recovered_errors: u32,
    unrecoverable_errors: u32,
}

impl ErrorMitigator {
    /// Create new mitigator
    pub fn new() -> Self {
        ErrorMitigator {
            errors: Vec::new(),
            mitigations: Vec::new(),
            total_errors: 0,
            recovered_errors: 0,
            unrecoverable_errors: 0,
        }
    }

    /// Record error
    pub fn record_error(&mut self, error: BootError) -> u32 {
        self.total_errors += 1;
        self.errors.push(error);
        self.total_errors
    }

    /// Get mitigation strategy
    pub fn get_mitigation_strategy(&self, error: &BootError) -> MitigationStrategy {
        match error.category {
            ErrorCategory::Hardware => MitigationStrategy::Fallback,
            ErrorCategory::Memory => MitigationStrategy::Reduce,
            ErrorCategory::IO => MitigationStrategy::Retry,
            ErrorCategory::Validation => MitigationStrategy::Skip,
            ErrorCategory::Configuration => MitigationStrategy::Retry,
            ErrorCategory::Software => MitigationStrategy::Retry,
        }
    }

    /// Attempt mitigation
    pub fn attempt_mitigation(&mut self, error: BootError) -> bool {
        let strategy = self.get_mitigation_strategy(&error);
        let mut record = MitigationRecord::new(error, strategy);

        match strategy {
            MitigationStrategy::Retry => {
                record.increment_attempts();
                if record.attempts < 3 {
                    record.mark_success();
                    self.recovered_errors += 1;
                }
            }
            MitigationStrategy::Fallback => {
                record.mark_success();
                self.recovered_errors += 1;
            }
            MitigationStrategy::Skip => {
                record.mark_success();
                self.recovered_errors += 1;
            }
            MitigationStrategy::Reduce => {
                record.mark_success();
                self.recovered_errors += 1;
            }
            MitigationStrategy::Ignore => {
                record.mark_success();
                self.recovered_errors += 1;
            }
            MitigationStrategy::Halt => {
                self.unrecoverable_errors += 1;
            }
        }

        let success = record.success;
        self.mitigations.push(record);
        success
    }

    /// Get error count
    pub fn get_error_count(&self) -> u32 {
        self.total_errors
    }

    /// Get recovery rate
    pub fn get_recovery_rate(&self) -> u32 {
        if self.total_errors == 0 {
            return 0;
        }
        ((self.recovered_errors as u64 * 100) / (self.total_errors as u64)) as u32
    }

    /// Get mitigation report
    pub fn mitigation_report(&self) -> String {
        let mut report = String::from("=== Error Mitigation Report ===\n");

        report.push_str(&format!("Total Errors: {}\n", self.total_errors));
        report.push_str(&format!("Recovered: {}\n", self.recovered_errors));
        report.push_str(&format!("Unrecoverable: {}\n", self.unrecoverable_errors));
        report.push_str(&format!("Recovery Rate: {}%\n\n", self.get_recovery_rate()));

        report.push_str("--- Error Log ---\n");
        for error in &self.errors {
            report.push_str(&format!("{}\n", error));
        }

        report.push_str("\n--- Mitigations ---\n");
        for mitigation in &self.mitigations {
            report.push_str(&format!("{}\n", mitigation));
        }

        report
    }
}

impl fmt::Display for ErrorMitigator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ErrorMitigator {{ errors: {}, recovered: {}, rate: {}% }}",
            self.total_errors,
            self.recovered_errors,
            self.get_recovery_rate()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_error() {
        let error = BootError::new(1, ErrorCategory::Hardware, "Test");
        assert_eq!(error.error_code, 1);
    }

    #[test]
    fn test_boot_error_severity() {
        let mut error = BootError::new(1, ErrorCategory::Hardware, "Test");
        error.set_severity(ErrorSeverity::Critical);
        assert_eq!(error.severity, ErrorSeverity::Critical);
    }

    #[test]
    fn test_boot_error_context() {
        let mut error = BootError::new(1, ErrorCategory::Hardware, "Test");
        error.add_context("CPU failed");
        assert!(!error.context.is_empty());
    }

    #[test]
    fn test_mitigation_record() {
        let error = BootError::new(1, ErrorCategory::IO, "Test");
        let record = MitigationRecord::new(error, MitigationStrategy::Retry);
        assert!(!record.success);
    }

    #[test]
    fn test_mitigation_record_attempts() {
        let error = BootError::new(1, ErrorCategory::IO, "Test");
        let mut record = MitigationRecord::new(error, MitigationStrategy::Retry);
        record.increment_attempts();
        assert_eq!(record.attempts, 1);
    }

    #[test]
    fn test_error_mitigator() {
        let mitigator = ErrorMitigator::new();
        assert_eq!(mitigator.get_error_count(), 0);
    }

    #[test]
    fn test_error_mitigator_record() {
        let mut mitigator = ErrorMitigator::new();
        let error = BootError::new(1, ErrorCategory::Hardware, "Test");
        mitigator.record_error(error);
        assert_eq!(mitigator.get_error_count(), 1);
    }

    #[test]
    fn test_error_mitigator_strategy() {
        let mitigator = ErrorMitigator::new();
        let error = BootError::new(1, ErrorCategory::IO, "Test");
        let strategy = mitigator.get_mitigation_strategy(&error);
        assert_eq!(strategy, MitigationStrategy::Retry);
    }

    #[test]
    fn test_error_mitigator_attempt() {
        let mut mitigator = ErrorMitigator::new();
        let error = BootError::new(1, ErrorCategory::Memory, "Test");
        mitigator.attempt_mitigation(error);
        assert_eq!(mitigator.recovered_errors, 1);
    }

    #[test]
    fn test_error_mitigator_recovery_rate() {
        let mut mitigator = ErrorMitigator::new();
        let e1 = BootError::new(1, ErrorCategory::Memory, "Test");
        let e2 = BootError::new(2, ErrorCategory::Memory, "Test");
        mitigator.record_error(e1.clone());
        mitigator.record_error(e2.clone());
        mitigator.attempt_mitigation(e1);
        mitigator.attempt_mitigation(e2);
        assert_eq!(mitigator.get_recovery_rate(), 100);
    }

    #[test]
    fn test_error_mitigator_report() {
        let mitigator = ErrorMitigator::new();
        let report = mitigator.mitigation_report();
        assert!(report.contains("Error Mitigation Report"));
    }
}
