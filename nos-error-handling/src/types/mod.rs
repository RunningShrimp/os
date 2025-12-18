//! Error types
//!
//! This module provides common types for error handling.

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(not(feature = "alloc"))]
use nos_api::collections::BTreeMap;
#[cfg(not(feature = "alloc"))]
use nos_api::interfaces::String;
#[cfg(not(feature = "alloc"))]
use nos_api::interfaces::Vec;

/// Error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(Default)]
pub enum ErrorSeverity {
    /// Informational
    #[default]
    Info = 0,
    /// Low severity
    Low = 1,
    /// Warning
    Warning = 2,
    /// Medium severity
    Medium = 3,
    /// High severity
    High = 4,
    /// Error
    Error = 5,
    /// Critical
    Critical = 6,
    /// Fatal
    Fatal = 7,
}

/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(Default)]
pub enum ErrorCategory {
    /// System error
    #[default]
    System = 0,
    /// Memory error
    Memory = 1,
    /// File system error
    FileSystem = 2,
    /// Network error
    Network = 3,
    /// Device error
    Device = 4,
    /// Process error
    Process = 5,
    /// Security error
    Security = 6,
    /// Application error
    Application = 7,
    /// Hardware error
    Hardware = 8,
    /// Configuration error
    Configuration = 9,
    /// User error
    User = 10,
    /// Resource error
    Resource = 11,
    /// Timeout error
    Timeout = 12,
    /// Protocol error
    Protocol = 13,
    /// Data error
    Data = 14,
    /// Interface error
    Interface = 15,
}

/// Error status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ErrorStatus {
    /// New error
    #[default]
    New = 0,
    /// Processing
    Processing = 1,
    /// Active
    Active = 2,
    /// Recovered
    Recovered = 3,
    /// Handled
    Handled = 4,
    /// Ignored
    Ignored = 5,
    /// Escalated
    Escalated = 6,
    /// Closed
    Closed = 7,
}

/// Error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ErrorType {
    /// Runtime error
    #[default]
    RuntimeError = 0,
    /// Logic error
    LogicError = 1,
    /// Compile error
    CompileError = 2,
    /// Configuration error
    ConfigurationError = 3,
    /// Resource error
    ResourceError = 4,
    /// Permission error
    PermissionError = 5,
    /// Network error
    NetworkError = 6,
    /// I/O error
    IOError = 7,
    /// Memory error
    MemoryError = 8,
    /// System call error
    SystemCallError = 9,
    /// Validation error
    ValidationError = 10,
    /// Timeout error
    TimeoutError = 11,
    /// Cancellation error
    CancellationError = 12,
}

/// Recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RecoveryStrategy {
    /// No recovery
    #[default]
    None = 0,
    /// Retry
    Retry = 1,
    /// Degrade service
    Degrade = 2,
    /// Restart component
    Restart = 3,
    /// Release resources
    Release = 4,
    /// Failover to backup
    Failover = 5,
    /// Isolate fault
    Isolate = 6,
    /// Manual intervention
    Manual = 7,
    /// Ignore error
    Ignore = 8,
}

/// Error record
#[derive(Debug, Clone)]
pub struct ErrorRecord {
    /// Error ID
    pub id: u64,
    /// Error code
    pub code: u32,
    /// Error type
    pub error_type: ErrorType,
    /// Error category
    pub category: ErrorCategory,
    /// Error severity
    pub severity: ErrorSeverity,
    /// Error status
    pub status: ErrorStatus,
    /// Error message
    pub message: String,
    /// Error description
    pub description: String,
    /// Error source
    pub source: ErrorSource,
    /// Error timestamp
    pub timestamp: u64,
    /// Error context
    pub context: ErrorContext,
    /// Recovery actions
    pub recovery_actions: Vec<RecoveryAction>,
    /// Occurrence count
    pub occurrence_count: u32,
    /// Last occurrence
    pub last_occurrence: u64,
    /// Resolved flag
    pub resolved: bool,
    /// Resolution time
    pub resolution_time: Option<u64>,
    /// Resolution method
    pub resolution_method: Option<String>,
    /// Error metadata
    pub metadata: BTreeMap<String, String>,
}

#[cfg(feature = "alloc")]
impl Default for ErrorRecord {
    fn default() -> Self {
        Self {
            id: 0,
            code: 0,
            error_type: ErrorType::default(),
            category: ErrorCategory::default(),
            severity: ErrorSeverity::default(),
            status: ErrorStatus::default(),
            message: String::new(),
            description: String::new(),
            source: ErrorSource::default(),
            timestamp: 0,
            context: ErrorContext::default(),
            recovery_actions: Vec::new(),
            occurrence_count: 1,
            last_occurrence: 0,
            resolved: false,
            resolution_time: None,
            resolution_method: None,
            metadata: BTreeMap::new(),
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for ErrorRecord {
    fn default() -> Self {
        Self {
            id: 0,
            code: 0,
            error_type: ErrorType::default(),
            category: ErrorCategory::default(),
            severity: ErrorSeverity::default(),
            status: ErrorStatus::default(),
            message: "",
            description: "",
            source: ErrorSource::default(),
            timestamp: 0,
            context: ErrorContext::default(),
            recovery_actions: &[],
            occurrence_count: 1,
            last_occurrence: 0,
            resolved: false,
            resolution_time: None,
            resolution_method: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Error source
#[derive(Debug, Clone)]
pub struct ErrorSource {
    /// Source module
    pub module: String,
    /// Source function
    pub function: String,
    /// Source file
    pub file: String,
    /// Source line
    pub line: u32,
    /// Source column
    pub column: u32,
    /// Process ID
    pub process_id: u32,
    /// Thread ID
    pub thread_id: u32,
    /// CPU ID
    pub cpu_id: u32,
}

#[cfg(feature = "alloc")]
impl Default for ErrorSource {
    fn default() -> Self {
        Self {
            module: String::new(),
            function: String::new(),
            file: String::new(),
            line: 0,
            column: 0,
            process_id: 0,
            thread_id: 0,
            cpu_id: 0,
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for ErrorSource {
    fn default() -> Self {
        Self {
            module: "",
            function: "",
            file: "",
            line: 0,
            column: 0,
            process_id: 0,
            thread_id: 0,
            cpu_id: 0,
        }
    }
}

/// Error context
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Environment variables
    pub environment_variables: BTreeMap<String, String>,
    /// System configuration
    pub system_config: BTreeMap<String, String>,
    /// User input
    pub user_input: Option<String>,
    /// Related data
    pub related_data: Vec<u8>,
    /// Operation sequence
    pub operation_sequence: Vec<String>,
    /// Preconditions
    pub preconditions: Vec<String>,
    /// Postconditions
    pub postconditions: Vec<String>,
}

#[cfg(feature = "alloc")]
impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            environment_variables: BTreeMap::new(),
            system_config: BTreeMap::new(),
            user_input: None,
            related_data: Vec::new(),
            operation_sequence: Vec::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }
    }
}

#[cfg(not(feature = "alloc"))]
impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            environment_variables: BTreeMap::new(),
            system_config: BTreeMap::new(),
            user_input: None,
            related_data: &[],
            operation_sequence: &[],
            preconditions: &[],
            postconditions: &[],
        }
    }
}

/// Recovery action
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAction {
    /// Action ID
    pub id: u64,
    /// Action type
    pub action_type: RecoveryActionType,
    /// Action name
    pub name: String,
    /// Action description
    pub description: String,
    /// Execution time
    pub execution_time: u64,
    /// Success flag
    pub success: bool,
    /// Result message
    pub result_message: String,
    /// Action parameters
    pub parameters: BTreeMap<String, String>,
}

/// Recovery action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RecoveryActionType {
    /// Retry operation
    #[default]
    Retry = 0,
    /// Restart service
    Restart = 1,
    /// Reset component
    Reset = 2,
    /// Release resources
    Release = 3,
    /// Allocate resources
    Allocate = 4,
    /// Isolate component
    Isolate = 5,
    /// Escalate handling
    Escalate = 6,
    /// Log event
    Log = 7,
    /// Send notification
    Notify = 8,
    /// Rollback operation
    Rollback = 9,
    /// Switch mode
    SwitchMode = 10,
}

/// Error handling statistics
#[derive(Debug, Clone)]
pub struct ErrorHandlingStats {
    /// Total errors
    pub total_errors: u64,
    /// Errors by category
    pub errors_by_category: BTreeMap<ErrorCategory, u64>,
    /// Errors by severity
    pub errors_by_severity: BTreeMap<ErrorSeverity, u64>,
    /// Recovered errors
    pub recovered_errors: u64,
    /// Recovery success rate
    pub recovery_success_rate: f64,
    /// Average recovery time
    pub avg_recovery_time: u64,
    /// System health score
    pub health_score: f64,
}

impl Default for ErrorHandlingStats {
    fn default() -> Self {
        Self {
            total_errors: 0,
            errors_by_category: BTreeMap::new(),
            errors_by_severity: BTreeMap::new(),
            recovered_errors: 0,
            recovery_success_rate: 0.0,
            avg_recovery_time: 0,
            health_score: 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
        assert!(ErrorSeverity::Critical < ErrorSeverity::Fatal);
        
        assert_eq!(ErrorSeverity::default(), ErrorSeverity::Info);
    }

    #[test]
    fn test_error_category() {
        assert_eq!(ErrorCategory::System, ErrorCategory::System);
        assert_ne!(ErrorCategory::System, ErrorCategory::Memory);
        
        assert_eq!(ErrorCategory::default(), ErrorCategory::System);
    }

    #[test]
    fn test_error_record() {
        let record = ErrorRecord::default();
        assert_eq!(record.id, 0);
        assert_eq!(record.code, 0);
        assert_eq!(record.error_type, ErrorType::RuntimeError);
        assert_eq!(record.category, ErrorCategory::System);
        assert_eq!(record.severity, ErrorSeverity::Info);
        assert_eq!(record.status, ErrorStatus::New);
        assert_eq!(record.message, "");
        assert_eq!(record.description, "");
        assert_eq!(record.occurrence_count, 1);
        assert_eq!(record.last_occurrence, 0);
        assert!(!record.resolved);
        assert_eq!(record.resolution_time, None);
        assert_eq!(record.resolution_method, None);
        assert!(record.metadata.is_empty());
    }
}