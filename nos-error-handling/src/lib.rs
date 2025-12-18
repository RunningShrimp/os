//! NOS Error Handling
//!
//! This crate provides comprehensive error handling and recovery framework for NOS operating system.
//! It includes error classification, recovery strategies, diagnostic tools, and health monitoring.
//!
//! # Architecture
//!
//! The error handling module is organized into several functional domains:
//!
//! - **Core**: Core error handling infrastructure
//! - **Registry**: Error registration and lookup
//! - **Classifier**: Error classification and analysis
//! - **Recovery**: Error recovery strategies
//! - **Diagnostics**: Error diagnostic tools
//! - **Reporting**: Error reporting and logging
//! - **Health**: System health monitoring
//!
//! # Usage
//!
//! ```rust
//! use nos_error_handling::{ErrorHandlingEngine, ErrorRecord, ErrorSeverity};
//!
//! // Create an error handling engine
//! let mut engine = ErrorHandlingEngine::new(Default::default());
//! engine.init()?;
//!
//! // Record an error
//! let error = ErrorRecord {
//!     // ... error details
//! };
//! let error_id = engine.record_error(error)?;
//! ```

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

// Import and re-export Result and Error from nos_api
pub use nos_api::error::{Error, Result};

// Core modules
pub mod core;
pub mod registry;
pub mod classifier;
pub mod recovery;
pub mod diagnostics;
pub mod reporting;
pub mod health;
pub mod types;
pub mod common;

// Kernel integration module
pub mod kernel_integration;

// Re-export commonly used items
// Instead of using glob exports, explicitly re-export items to avoid conflicts
// Registry
pub use registry::{ErrorRegistry, init_registry, get_registry, shutdown_registry};

// Classifier
pub use classifier::{ErrorClassifier, get_classifier};

// Recovery
pub use recovery::{RecoveryManager, apply_recovery_strategy, get_manager, recovery_get_stats};

// Diagnostics
pub use diagnostics::{DiagnosticAnalyzer, analyze_error, get_analyzer, diagnostics_get_stats};

// Reporting
pub use reporting::{ErrorReporter, ReportDestination, ReportLevel, ReportingStats, report_error, generate_report, get_reporter, reporting_get_stats};

// Health
pub use health::{HealthMonitor, HealthMetric, HealthThreshold, HealthLevel, HealthSeverity, HealthStats, get_current_status, get_monitor, health_get_stats};

// Kernel integration
// Note: register_kernel_handlers, unregister_kernel_handlers, and KernelErrorHandler are not available in this module

// Common
pub use common::{get_timestamp, validate_error_record, format_error_message};

// Types
pub use types::*;

/// Initialize error handling subsystem
///
/// This function initializes the error handling engine
/// and related components.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
#[cfg(feature = "alloc")]
pub fn init_error_handling() -> nos_api::Result<()> {
    // Initialize error handling engine
    core::init_engine()?;
    
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
#[cfg(feature = "alloc")]
pub fn shutdown_error_handling() -> nos_api::Result<()> {
    // Shutdown error handling engine
    core::shutdown_engine()?;
    
    Ok(())
}

/// Get error handling statistics
///
/// # Returns
///
/// * `ErrorHandlingStats` - Error handling statistics
#[cfg(feature = "alloc")]
pub fn get_error_stats() -> types::ErrorHandlingStats {
    core::get_stats()
}



#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "alloc")]
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