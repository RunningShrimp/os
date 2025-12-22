//! Error reporting
//!
//! This module provides error reporting and logging functionality.

use spin::Mutex;
use crate::Result;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::format;
use nos_api::collections::BTreeMap;

/// Error reporter
#[cfg(feature = "alloc")]
#[derive(Default)]
pub struct ErrorReporter {
    /// Report destinations
    destinations: Vec<ReportDestination>,
    /// Reporting statistics
    stats: Mutex<ReportingStats>,
}

#[cfg(feature = "alloc")]
impl ErrorReporter {
    /// Create a new error reporter
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a report destination
    pub fn add_destination(&mut self, destination: ReportDestination) {
        self.destinations.push(destination);
    }

    /// Report an error
    pub fn report_error(&self, error_record: &crate::types::ErrorRecord) -> Result<()> {
        // Report to all destinations
        for destination in &self.destinations {
            destination.report_error(error_record)?;
        }
        
        // Update statistics
        let mut stats = self.stats.lock();
        stats.total_reported += 1;
        *stats.reports_by_severity.entry(error_record.severity).or_insert(0) += 1;
        
        Ok(())
    }

    /// Generate an error report
    pub fn generate_report(&self, errors: &[crate::types::ErrorRecord], time_range: Option<(u64, u64)>) -> Result<String> {
        let mut report = String::from("# Error Report\n\n");
        
        // Add report header
        report.push_str(&format!("Generated at: {}\n", crate::common::get_timestamp()));
        report.push_str(&format!("Total errors: {}\n\n", errors.len()));
        
        // Add error summary
        let mut summary = BTreeMap::new();
        for error in errors {
            *summary.entry(error.category).or_insert(0) += 1;
        }
        
        report.push_str("## Error Summary\n\n");
        for (category, count) in summary.iter() {
            report.push_str(&format!("- {:?}: {}\n", category, count));
        }
        
        report.push_str("\n## Error Details\n\n");
        
        // Add error details
        for error in errors {
            // Filter by time range if specified
            if let Some((start, end)) = time_range {
                if error.timestamp < start || error.timestamp > end {
                    continue;
                }
            }
            
            report.push_str(&format!("### Error #{}\n", error.id));
            report.push_str(&format!("- Code: {}\n", error.code));
            report.push_str(&format!("- Type: {:?}\n", error.error_type));
            report.push_str(&format!("- Category: {:?}\n", error.category));
            report.push_str(&format!("- Severity: {:?}\n", error.severity));
            report.push_str(&format!("- Message: {}\n", error.message));
            report.push_str(&format!("- Description: {}\n", error.description));
            report.push_str(&format!("- Timestamp: {}\n", error.timestamp));
            report.push_str(&format!("- Source: {}:{}\n", error.source.file, error.source.line));
            report.push('\n');
        }
        
        Ok(report)
    }

    /// Get reporting statistics
    pub fn get_stats(&self) -> ReportingStats {
        self.stats.lock().clone()
    }

    /// Initialize reporter
    pub fn init(&mut self) -> Result<()> {
        // Add default destinations
        self.add_default_destinations();
        Ok(())
    }

    /// Shutdown reporter
    pub fn shutdown(&mut self) -> Result<()> {
        // TODO: Shutdown reporter
        Ok(())
    }

    /// Add default destinations
    fn add_default_destinations(&mut self) {
        // Add console destination
        self.add_destination(ReportDestination::Console {
            level: ReportLevel::Error,
        });
        
        // Add file destination
        self.add_destination(ReportDestination::File {
            path: "/var/log/nos_errors.log".to_string(),
            level: ReportLevel::Warning,
            max_file_size_mb: 100,
            max_files: 10,
        });
    }
}

/// Report destination
#[cfg(feature = "alloc")]
#[derive(Debug, Clone)]
pub enum ReportDestination {
    /// Console destination
    Console {
        /// Report level
        level: ReportLevel,
    },
    /// File destination
    File {
        /// File path
        path: String,
        /// Report level
        level: ReportLevel,
        /// Maximum file size (MB)
        max_file_size_mb: u32,
        /// Maximum number of files
        max_files: u32,
    },
    /// Network destination
    Network {
        /// Server address
        address: String,
        /// Server port
        port: u16,
        /// Protocol
        protocol: String,
        /// Report level
        level: ReportLevel,
        /// Authentication token
        auth_token: Option<String>,
    },
}

#[cfg(feature = "alloc")]
impl ReportDestination {
    /// Report an error to this destination
    pub fn report_error(&self, error_record: &crate::types::ErrorRecord) -> Result<()> {
        match self {
            ReportDestination::Console { level } => {
                // Check if the error severity is high enough
                if error_record.severity as u8 >= (*level) as u8 {
                    // In a real implementation, we would print to console
                }
                Ok(())
            },
            ReportDestination::File { path: _, level, max_file_size_mb: _, max_files: _ } => {
                // Check if the error severity is high enough
                if error_record.severity as u8 >= (*level) as u8 {
                    // In a real implementation, we would write to the file
                }
                Ok(())
            },
            ReportDestination::Network { address: _, port: _, protocol: _, level, auth_token: _ } => {
                // Check if the error severity is high enough
                if error_record.severity as u8 >= (*level) as u8 {
                    // In a real implementation, we would send to the network
                }
                Ok(())
            },
        }
    }
}

/// Report level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ReportLevel {
    /// Debug level
    Debug = 0,
    /// Informational level
    Info = 1,
    /// Warning level
    Warning = 2,
    /// Error level
    #[default]
    Error = 3,
    /// Critical level
    Critical = 4,
}

/// Reporting statistics
#[cfg(feature = "alloc")]
#[derive(Debug, Clone, Default)]
pub struct ReportingStats {
    /// Total reported errors
    pub total_reported: u64,
    /// Reports by severity
    pub reports_by_severity: BTreeMap<crate::types::ErrorSeverity, u64>,
    /// Reports by destination
    pub reports_by_destination: BTreeMap<String, u64>,
    /// Average reporting time (microseconds)
    pub avg_reporting_time: u64,
}

/// Global error reporter
#[cfg(feature = "alloc")]
static GLOBAL_REPORTER: spin::Once<Mutex<ErrorReporter>> = spin::Once::new();

/// Initialize the global error reporter
#[cfg(feature = "alloc")]
pub fn init_reporter() -> Result<()> {
    GLOBAL_REPORTER.call_once(|| {
        Mutex::new(ErrorReporter::new())
    });
    
    // Initialize the reporter
    GLOBAL_REPORTER.get().unwrap().lock().init()
}

/// Get the global error reporter
#[cfg(feature = "alloc")]
pub fn get_reporter() -> &'static Mutex<ErrorReporter> {
    GLOBAL_REPORTER.get().expect("Error reporter not initialized")
}

/// Internal function to get the global error reporter
#[cfg(feature = "alloc")]
fn get_reporter_internal() -> &'static Mutex<ErrorReporter> {
    get_reporter()
}

/// Shutdown the global error reporter
#[cfg(feature = "alloc")]
pub fn shutdown_reporter() -> Result<()> {
    // Note: spin::Once doesn't provide a way to reset, so we just return Ok(())
    // In a real implementation, you might want to provide a different approach
    Ok(())
}

/// Report an error
#[cfg(feature = "alloc")]
pub fn report_error(error: &crate::types::ErrorRecord) -> Result<()> {
    let reporter = get_reporter_internal().lock();
    reporter.report_error(error)
}

/// Generate an error report
#[cfg(feature = "alloc")]
pub fn generate_report(errors: &[crate::types::ErrorRecord], time_range: Option<(u64, u64)>) -> Result<String> {
    let reporter = get_reporter_internal().lock();
    reporter.generate_report(errors, time_range)
}

/// Get reporting statistics
#[cfg(feature = "alloc")]
pub fn reporting_get_stats() -> ReportingStats {
    get_reporter_internal().lock().get_stats()
}



#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    #[test]
    fn test_error_reporter() {
        let mut reporter = ErrorReporter::new();
        
        // Add a test destination
        let destination = ReportDestination::Console {
            level: ReportLevel::Error,
        };
        reporter.add_destination(destination);
        
        // Report a test error
        let error_record = crate::types::ErrorRecord::default();
        assert!(reporter.report_error(&error_record).is_ok());
        
        // Check statistics
        let stats = reporter.get_stats();
        assert_eq!(stats.total_reported, 1);
    }

    #[test]
    fn test_report_level() {
        assert!(ReportLevel::Debug < ReportLevel::Info);
        assert!(ReportLevel::Info < ReportLevel::Warning);
        assert!(ReportLevel::Warning < ReportLevel::Error);
        assert!(ReportLevel::Error < ReportLevel::Critical);
        
        assert_eq!(ReportLevel::default(), ReportLevel::Error);
    }

    #[test]
    fn test_reporting_stats() {
        let stats = ReportingStats::default();
        assert_eq!(stats.total_reported, 0);
        assert_eq!(stats.avg_reporting_time, 0);
        assert!(stats.reports_by_severity.is_empty());
        assert!(stats.reports_by_destination.is_empty());
    }
}