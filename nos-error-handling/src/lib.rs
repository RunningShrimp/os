//! NOS Error Handling - Interface Definition Layer
//!
//! This crate provides trait definitions and type definitions for error handling
//! in the NOS operating system. Actual implementations are in kernel/src/error.
//!
//! # Architecture
//!
//! This crate serves as an interface definition layer:
//! - Defines traits for error handlers, recovery strategies, and health monitoring
//! - Provides type definitions for error records, severity levels, etc.
//! - Does NOT contain actual implementations (those are in kernel/src/error)
//!
//! # Usage
//!
//! ```rust
//! use nos_error_handling::{ErrorHandler, ErrorAction, ErrorSeverity};
//!
//! // Implement the ErrorHandler trait in kernel code
//! struct MyErrorHandler;
//! impl ErrorHandler for MyErrorHandler {
//!     fn handle_error(&self, error: &ErrorContext) -> ErrorAction {
//!         // Implementation in kernel
//!     }
//! }
//! ```

#![no_std]

extern crate alloc;

// Import and re-export Result and Error from nos_api
pub use nos_api::error::{Error, Result};

// Core trait definitions only
pub mod core {
    // Core traits for error handling
    pub mod traits;
    
    // Engine functions - these are implemented in kernel/src/error
    pub fn init_engine() -> nos_api::Result<()> {
        // Implementation in kernel/src/error
        Ok(())
    }
    
    pub fn shutdown_engine() -> nos_api::Result<()> {
        // Implementation in kernel/src/error
        Ok(())
    }
    
    pub fn get_stats() -> crate::types::ErrorHandlingStats {
        // Implementation in kernel/src/error
        crate::types::ErrorHandlingStats::default()
    }
}

// Type definitions
pub mod types;
pub mod common;

// Kernel integration types (deprecated - use kernel/src/error instead)
#[deprecated(note = "Implementation should be in kernel/src/error, not here")]
pub mod kernel_integration;

// Note: The following modules contain implementation details that should be
// moved to kernel/src/error. They are kept here temporarily for backward
// compatibility but will be deprecated.
#[deprecated(note = "Implementation should be in kernel/src/error, not here")]
pub mod registry;

#[deprecated(note = "Implementation should be in kernel/src/error, not here")]
pub mod classifier;

#[deprecated(note = "Implementation should be in kernel/src/error, not here")]
pub mod recovery;

#[deprecated(note = "Implementation should be in kernel/src/error, not here")]
pub mod diagnostics;

#[deprecated(note = "Implementation should be in kernel/src/error, not here")]
pub mod reporting;

#[deprecated(note = "Implementation should be in kernel/src/error, not here")]
pub mod health;

// Re-export trait definitions
pub use core::traits::{
    ErrorHandler, ErrorAction, ErrorContext,
    RecoveryStrategy, RecoveryResult,
    ErrorClassifier, HealthMonitor, HealthStatus
};

// Re-export type definitions
pub use types::*;
pub use common::{get_timestamp, validate_error_record, format_error_message};

// Deprecated: Implementation exports (should use kernel/src/error instead)
#[deprecated(note = "Use kernel/src/error implementations instead")]
pub use registry::{ErrorRegistry, init_registry, get_registry, shutdown_registry};

#[deprecated(note = "Use kernel/src/error implementations instead")]
pub use classifier::{ErrorClassifier as ImplErrorClassifier, get_classifier};

#[deprecated(note = "Use kernel/src/error implementations instead")]
pub use recovery::{RecoveryManager, apply_recovery_strategy, get_manager, recovery_get_stats};

#[deprecated(note = "Use kernel/src/error implementations instead")]
pub use diagnostics::{DiagnosticAnalyzer, analyze_error, get_analyzer, diagnostics_get_stats};

#[deprecated(note = "Use kernel/src/error implementations instead")]
pub use reporting::{ErrorReporter, ReportDestination, ReportLevel, ReportingStats, report_error, generate_report, get_reporter, reporting_get_stats};

#[deprecated(note = "Use kernel/src/error implementations instead")]
pub use health::{HealthMonitor as ImplHealthMonitor, HealthMetric, HealthThreshold, HealthLevel, HealthSeverity, HealthStats, get_current_status, get_monitor, health_get_stats};

/// Initialize error handling subsystem
///
/// This function initializes the error handling engine
/// and related components.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn init_error_handling() -> nos_api::Result<()> {
    // Note: Actual implementation should be in kernel/src/error
    // This is just an interface definition layer
    Ok(())
}

/// Shutdown error handling subsystem
///
/// This function shuts down the error handling engine
/// and related components.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_error_handling() -> nos_api::Result<()> {
    // Note: Actual implementation should be in kernel/src/error
    // This is just an interface definition layer
    Ok(())
}

/// Get error handling statistics
///
/// # Returns
///
/// * `ErrorHandlingStats` - Error handling statistics
pub fn get_error_stats() -> types::ErrorHandlingStats {
    // Note: Actual implementation should be in kernel/src/error
    // This is just an interface definition layer
    types::ErrorHandlingStats {
        total_errors: 0,
        errors_by_category: alloc::collections::BTreeMap::new(),
        errors_by_severity: alloc::collections::BTreeMap::new(),
        recovered_errors: 0,
        recovery_success_rate: 0.0,
        avg_recovery_time: 0,
        health_score: 100.0,
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_stats() {
        let stats = ErrorHandlingStats::default();
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.recovered_errors, 0);
        assert_eq!(stats.recovery_success_rate, 0.0);
        assert_eq!(stats.avg_recovery_time, 0);
        assert_eq!(stats.health_score, 100.0);
        assert!(stats.errors_by_category.is_empty());
        assert!(stats.errors_by_severity.is_empty());
    }
}