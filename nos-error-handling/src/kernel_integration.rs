//! Kernel error handling integration
//! 
//! This module provides integration between kernel error handling and the nos-error-handling crate.
//! It includes kernel-specific error types and handling logic.
//! 
//! DEPRECATED: Implementation should be in kernel/src/error, not here

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;
use spin::Once;

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    /// Info level
    Info = 0,
    /// Low level
    Low = 1,
    /// Warning level
    Warning = 2,
    /// Medium level
    Medium = 3,
    /// High level
    High = 4,
    /// Error level
    Error = 5,
    /// Critical error
    Critical = 6,
    /// Fatal error
    Fatal = 7,
}

/// Error categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorCategory {
    /// System error
    System,
    /// Memory error
    Memory,
    /// File system error
    FileSystem,
    /// Network error
    Network,
    /// Device error
    Device,
    /// Process error
    Process,
    /// Security error
    Security,
    /// Application error
    Application,
    /// Hardware error
    Hardware,
    /// Configuration error
    Configuration,
    /// User error
    User,
    /// Resource error
    Resource,
    /// Timeout error
    Timeout,
    /// Protocol error
    Protocol,
    /// Data error
    Data,
    /// Interface error
    Interface,
}

/// Error status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorStatus {
    /// New error
    New,
    /// Processing
    Processing,
    /// Active
    Active,
    /// Recovered
    Recovered,
    /// Handled
    Handled,
    /// Ignored
    Ignored,
    /// Escalated
    Escalated,
    /// Closed
    Closed,
}

/// Error types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorType {
    /// Runtime error
    RuntimeError,
    /// Logic error
    LogicError,
    /// Compile error
    CompileError,
    /// Configuration error
    ConfigurationError,
    /// Resource error
    ResourceError,
    /// Permission error
    PermissionError,
    /// Network error
    NetworkError,
    /// I/O error
    IOError,
    /// Memory error
    MemoryError,
    /// System call error
    SystemCallError,
    /// Validation error
    ValidationError,
    /// Timeout error
    TimeoutError,
    /// Cancellation error
    CancellationError,
    /// System error (compatibility with old code)
    SystemError,
}

impl core::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ErrorType::RuntimeError => write!(f, "RuntimeError"),
            ErrorType::LogicError => write!(f, "LogicError"),
            ErrorType::CompileError => write!(f, "CompileError"),
            ErrorType::ConfigurationError => write!(f, "ConfigurationError"),
            ErrorType::ResourceError => write!(f, "ResourceError"),
            ErrorType::PermissionError => write!(f, "PermissionError"),
            ErrorType::NetworkError => write!(f, "NetworkError"),
            ErrorType::IOError => write!(f, "IOError"),
            ErrorType::MemoryError => write!(f, "MemoryError"),
            ErrorType::SystemCallError => write!(f, "SystemCallError"),
            ErrorType::ValidationError => write!(f, "ValidationError"),
            ErrorType::TimeoutError => write!(f, "TimeoutError"),
            ErrorType::CancellationError => write!(f, "CancellationError"),
            ErrorType::SystemError => write!(f, "SystemError"),
        }
    }
}

/// Recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryStrategy {
    /// No recovery
    None,
    /// Automatic retry
    Retry,
    /// Degrade service
    Degrade,
    /// Restart component
    Restart,
    /// Release resources (compatibility with old code)
    Release,
    /// Failover to backup
    Failover,
    /// Isolate fault
    Isolate,
    /// Manual intervention
    Manual,
    /// Ignore error
    Ignore,
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
    /// Severity level
    pub severity: ErrorSeverity,
    /// Priority
    pub priority: ErrorPriority,
    /// Error status
    pub status: ErrorStatus,
    /// Error message
    pub message: String,
    /// Detailed description
    pub description: String,
    /// Error source
    pub source: ErrorSource,
    /// Timestamp
    pub timestamp: u64,
    /// Error context
    pub context: ErrorContext,
    /// Stack trace
    pub stack_trace: Vec<StackFrame>,
    /// System state
    pub system_state: SystemStateSnapshot,
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
    /// Metadata
    pub metadata: BTreeMap<String, String>,
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
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Process ID
    pub process_id: u32,
    /// Thread ID
    pub thread_id: u32,
    /// CPU ID
    pub cpu_id: u32,
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

/// Stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function name
    pub function: String,
    /// Module name
    pub module: String,
    /// File name
    pub file: String,
    /// Line number
    pub line: u32,
    /// Function address
    pub address: u64,
    /// Offset
    pub offset: u64,
}

/// System state snapshot
#[derive(Debug, Clone)]
pub struct SystemStateSnapshot {
    /// Memory usage
    pub memory_usage: MemoryUsage,
    /// CPU usage
    pub cpu_usage: CpuUsage,
    /// Process states
    pub process_states: Vec<ProcessState>,
    /// Network state
    pub network_state: NetworkState,
    /// File system state
    pub filesystem_state: FileSystemState,
    /// Device states
    pub device_states: Vec<DeviceState>,
    /// System load
    pub system_load: SystemLoad,
    /// Timestamp
    pub timestamp: u64,
}

impl Default for SystemStateSnapshot {
    fn default() -> Self {
        Self {
            memory_usage: MemoryUsage {
                total_memory: 0,
                used_memory: 0,
                available_memory: 0,
                cached_memory: 0,
                swap_used: 0,
                kernel_memory: 0,
            },
            cpu_usage: CpuUsage {
                usage_percent: 0.0,
                user_percent: 0.0,
                system_percent: 0.0,
                idle_percent: 0.0,
                wait_percent: 0.0,
                interrupt_percent: 0.0,
            },
            process_states: Vec::new(),
            network_state: NetworkState {
                active_connections: 0,
                listening_ports: 0,
                interfaces: Vec::new(),
                packet_stats: PacketStats {
                    total_rx: 0,
                    total_tx: 0,
                    dropped: 0,
                    errors: 0,
                },
            },
            filesystem_state: FileSystemState {
                mount_points: Vec::new(),
                disk_usage: Vec::new(),
                io_stats: IoStats {
                    read_operations: 0,
                    write_operations: 0,
                    read_bytes: 0,
                    write_bytes: 0,
                    io_wait_time: 0,
                },
            },
            device_states: Vec::new(),
            system_load: SystemLoad {
                load_1min: 0.0,
                load_5min: 0.0,
                load_15min: 0.0,
                run_queue_length: 0,
                blocked_processes: 0,
            },
            timestamp: 0,
        }
    }
}

/// Memory usage
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// Total memory
    pub total_memory: u64,
    /// Used memory
    pub used_memory: u64,
    /// Available memory
    pub available_memory: u64,
    /// Cached memory
    pub cached_memory: u64,
    /// Swap used
    pub swap_used: u64,
    /// Kernel memory
    pub kernel_memory: u64,
}

/// CPU usage
#[derive(Debug, Clone)]
pub struct CpuUsage {
    /// CPU usage percentage
    pub usage_percent: f64,
    /// User mode percentage
    pub user_percent: f64,
    /// System mode percentage
    pub system_percent: f64,
    /// Idle percentage
    pub idle_percent: f64,
    /// Wait percentage
    pub wait_percent: f64,
    /// Interrupt percentage
    pub interrupt_percent: f64,
}

/// Process state snapshot
#[derive(Debug, Clone)]
pub struct ProcessState {
    /// Process ID
    pub process_id: u32,
    /// Process name
    pub name: String,
    /// Process status
    pub status: String,
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage
    pub memory_usage: u64,
    /// Open files count
    pub open_files: u32,
    /// Thread count
    pub thread_count: u32,
    /// Runtime
    pub runtime: u64,
}

/// Network state snapshot
#[derive(Debug, Clone)]
pub struct NetworkState {
    /// Active connections
    pub active_connections: u32,
    /// Listening ports
    pub listening_ports: u32,
    /// Network interfaces
    pub interfaces: Vec<NetworkInterface>,
    /// Packet statistics
    pub packet_stats: PacketStats,
}

/// Network interface state
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// Interface status
    pub status: String,
    /// Received bytes
    pub rx_bytes: u64,
    /// Transmitted bytes
    pub tx_bytes: u64,
    /// Received packets
    pub rx_packets: u64,
    /// Transmitted packets
    pub tx_packets: u64,
    /// Error packets
    pub error_packets: u64,
}

/// Packet statistics
#[derive(Debug, Clone)]
pub struct PacketStats {
    /// Total received packets
    pub total_rx: u64,
    /// Total transmitted packets
    pub total_tx: u64,
    /// Dropped packets
    pub dropped: u64,
    /// Error packets
    pub errors: u64,
}

/// File system state snapshot
#[derive(Debug, Clone)]
pub struct FileSystemState {
    /// Mount points
    pub mount_points: Vec<MountPoint>,
    /// Disk usage
    pub disk_usage: Vec<DiskUsage>,
    /// I/O statistics
    pub io_stats: IoStats,
}

/// Mount point state
#[derive(Debug, Clone)]
pub struct MountPoint {
    /// Mount point path
    pub mount_point: String,
    /// Device name
    pub device: String,
    /// File system type
    pub filesystem_type: String,
    /// Mount options
    pub options: String,
    /// Status
    pub status: String,
}

/// Disk usage
#[derive(Debug, Clone)]
pub struct DiskUsage {
    /// Disk device
    pub device: String,
    /// Total size
    pub total_size: u64,
    /// Used size
    pub used_size: u64,
    /// Available size
    pub available_size: u64,
    /// Usage percentage
    pub usage_percent: f64,
}

/// I/O statistics
#[derive(Debug, Clone)]
pub struct IoStats {
    /// Read operations
    pub read_operations: u64,
    /// Write operations
    pub write_operations: u64,
    /// Read bytes
    pub read_bytes: u64,
    /// Write bytes
    pub write_bytes: u64,
    /// I/O wait time
    pub io_wait_time: u64,
}

/// Device state
#[derive(Debug, Clone)]
pub struct DeviceState {
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: String,
    /// Device status
    pub status: String,
    /// Driver
    pub driver: String,
    /// Device parameters
    pub parameters: BTreeMap<String, String>,
}

/// System load
#[derive(Debug, Clone)]
pub struct SystemLoad {
    /// 1 minute average load
    pub load_1min: f64,
    /// 5 minute average load
    pub load_5min: f64,
    /// 15 minute average load
    pub load_15min: f64,
    /// Run queue length
    pub run_queue_length: u32,
    /// Blocked processes
    pub blocked_processes: u32,
}

/// Recovery action
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryActionType {
    /// Retry operation
    Retry,
    /// Restart service
    Restart,
    /// Reset component
    Reset,
    /// Release resources
    Release,
    /// Allocate resources
    Allocate,
    /// Isolate component
    Isolate,
    /// Escalate handling
    Escalate,
    /// Log event
    Log,
    /// Send notification
    Notify,
    /// Rollback operation
    Rollback,
    /// Switch mode
    SwitchMode,
}

/// Error priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorPriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// Error handling configuration
#[derive(Debug, Clone)]
pub struct ErrorHandlingConfig {
    /// Enable error recovery
    pub enable_recovery: bool,
    /// Maximum retry count
    pub max_retries: u32,
    /// Retry interval (milliseconds)
    pub retry_interval_ms: u64,
    /// Error escalation threshold
    pub escalation_threshold: u32,
    /// Auto recovery strategies
    pub auto_recovery_strategies: Vec<RecoveryStrategy>,
    /// Error record retention time (seconds)
    pub retention_period_seconds: u64,
    /// Maximum error records
    pub max_error_records: usize,
    /// Enable error aggregation
    pub enable_error_aggregation: bool,
    /// Aggregation time window (seconds)
    pub aggregation_window_seconds: u64,
    /// Enable error prediction
    pub enable_error_prediction: bool,
    /// Enable health checks
    pub enable_health_checks: bool,
    /// Health check interval (seconds)
    pub health_check_interval_seconds: u64,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            enable_recovery: true,
            max_retries: 3,
            retry_interval_ms: 1000,
            escalation_threshold: 5,
            auto_recovery_strategies: vec![
                RecoveryStrategy::Retry,
                RecoveryStrategy::Degrade,
            ],
            retention_period_seconds: 86400 * 7, // 7 days
            max_error_records: 10000,
            enable_error_aggregation: true,
            aggregation_window_seconds: 300, // 5 minutes
            enable_error_prediction: false,
            enable_health_checks: true,
            health_check_interval_seconds: 60,
        }
    }
}

/// Error handling statistics
#[derive(Debug, Clone, Default)]
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
    /// Average recovery time (milliseconds)
    pub avg_recovery_time_ms: u64,
    /// Error prediction accuracy
    pub prediction_accuracy: f64,
    /// System health score
    pub health_score: f64,
}

/// Error handling engine
pub struct ErrorHandlingEngine {
    /// Engine ID
    pub id: u64,
    /// Engine configuration
    pub config: ErrorHandlingConfig,
    /// Error records
    pub error_records: Vec<ErrorRecord>,
    /// Statistics
    pub stats: Arc<Mutex<ErrorHandlingStats>>,
    /// Error counter
    pub error_counter: AtomicU64,
    /// Running flag
    pub running: AtomicBool,
}

impl ErrorHandlingEngine {
    /// Create a new error handling engine
    pub fn new(config: ErrorHandlingConfig) -> Self {
        Self {
            id: 1,
            config,
            error_records: Vec::new(),
            stats: Arc::new(Mutex::new(ErrorHandlingStats::default())),
            error_counter: AtomicU64::new(1),
            running: AtomicBool::new(false),
        }
    }

    /// Initialize the error handling engine
    pub fn init(&mut self) -> nos_api::Result<()> {
        self.running.store(true, Ordering::SeqCst);
        // TODO: Implement proper logging - nos_api::log_info! not available
        Ok(())
    }

    /// Record an error
    pub fn record_error(&mut self, error_record: ErrorRecord) -> nos_api::Result<u64> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(nos_api::error::system_error("Error handling engine is not running"));
        }

        let error_id = self.error_counter.fetch_add(1, Ordering::SeqCst);
        
        // Update error record with ID
        let mut error_record = error_record;
        error_record.id = error_id;
        
        // Add to records list
        self.error_records.push(error_record.clone());
        
        // Limit record count
        if self.error_records.len() > self.config.max_error_records {
            self.error_records.remove(0);
        }
        
        // Update statistics
        self.update_statistics(&error_record);
        
        Ok(error_id)
    }

    /// Get error records
    pub fn get_error_records(&self, limit: Option<usize>, category: Option<ErrorCategory>, severity: Option<ErrorSeverity>) -> Vec<ErrorRecord> {
        let mut records = self.error_records.clone();
        
        // Filter by category
        if let Some(cat) = category {
            records.retain(|r| r.category == cat);
        }
        
        // Filter by severity
        if let Some(sev) = severity {
            records.retain(|r| r.severity == sev);
        }
        
        // Sort by timestamp (newest first)
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Limit count
        if let Some(limit) = limit {
            records.truncate(limit);
        }
        
        records
    }

    /// Get statistics
    pub fn get_statistics(&self) -> ErrorHandlingStats {
        self.stats.lock().clone()
    }

    /// Update configuration
    pub fn update_config(&mut self, config: ErrorHandlingConfig) -> nos_api::Result<()> {
        self.config = config;
        Ok(())
    }

    /// Shutdown the error handling engine
    pub fn shutdown(&mut self) -> nos_api::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        // TODO: Implement proper logging - nos_api::log_info! not available
        Ok(())
    }

    /// Update statistics
    fn update_statistics(&self, error_record: &ErrorRecord) {
        let mut stats = self.stats.lock();
        
        stats.total_errors += 1;
        
        *stats.errors_by_category.entry(error_record.category).or_insert(0) += 1;
        *stats.errors_by_severity.entry(error_record.severity).or_insert(0) += 1;
        
        if error_record.resolved {
            stats.recovered_errors += 1;
        }
    }
}

/// Global error handling engine instance
pub static ERROR_HANDLING_ENGINE: Once<spin::Mutex<ErrorHandlingEngine>> = Once::new();

/// Initialize global error handling
pub fn init_global_error_handling() {
    ERROR_HANDLING_ENGINE.call_once(||
        spin::Mutex::new(ErrorHandlingEngine::new(ErrorHandlingConfig::default()))
    );
}

/// Get the global error handling engine instance
pub fn get_error_handling_engine() -> &'static spin::Mutex<ErrorHandlingEngine> {
    ERROR_HANDLING_ENGINE.call_once(||
        spin::Mutex::new(ErrorHandlingEngine::new(ErrorHandlingConfig::default()))
    );
    ERROR_HANDLING_ENGINE.get().unwrap()
}

/// Initialize error handling system
pub fn init_error_handling() -> nos_api::Result<()> {
    let config = ErrorHandlingConfig::default();
    let mut engine = get_error_handling_engine().lock();
    engine.update_config(config)?;
    engine.init()
}

/// Record an error
pub fn record_error(error_code: u32, error_type: ErrorType, category: ErrorCategory, severity: ErrorSeverity, message: &str, source: &ErrorSource) -> nos_api::Result<u64> {
    let error_record = ErrorRecord {
        id: 0, // Will be assigned in record_error
        code: error_code,
        error_type,
        category,
        severity,
        priority: ErrorPriority::Normal,
        status: ErrorStatus::New,
        message: message.to_string(),
        description: String::new(),
        source: source.clone(),
        timestamp: nos_api::event::get_time_ns(),
        context: ErrorContext {
            environment_variables: BTreeMap::new(),
            system_config: BTreeMap::new(),
            user_input: None,
            related_data: Vec::new(),
            operation_sequence: Vec::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        },
        stack_trace: Vec::new(),
        system_state: SystemStateSnapshot {
            memory_usage: MemoryUsage {
                total_memory: 0,
                used_memory: 0,
                available_memory: 0,
                cached_memory: 0,
                swap_used: 0,
                kernel_memory: 0,
            },
            cpu_usage: CpuUsage {
                usage_percent: 0.0,
                user_percent: 0.0,
                system_percent: 0.0,
                idle_percent: 0.0,
                wait_percent: 0.0,
                interrupt_percent: 0.0,
            },
            process_states: Vec::new(),
            network_state: NetworkState {
                active_connections: 0,
                listening_ports: 0,
                interfaces: Vec::new(),
                packet_stats: PacketStats {
                    total_rx: 0,
                    total_tx: 0,
                    dropped: 0,
                    errors: 0,
                },
            },
            filesystem_state: FileSystemState {
                mount_points: Vec::new(),
                disk_usage: Vec::new(),
                io_stats: IoStats {
                    read_operations: 0,
                    write_operations: 0,
                    read_bytes: 0,
                    write_bytes: 0,
                    io_wait_time: 0,
                },
            },
            device_states: Vec::new(),
            system_load: SystemLoad {
                load_1min: 0.0,
                load_5min: 0.0,
                load_15min: 0.0,
                run_queue_length: 0,
                blocked_processes: 0,
            },
            timestamp: nos_api::event::get_time_ns(),
        },
        recovery_actions: Vec::new(),
        occurrence_count: 1,
        last_occurrence: nos_api::event::get_time_ns(),
        resolved: false,
        resolution_time: None,
        resolution_method: None,
        metadata: BTreeMap::new(),
    };

    get_error_handling_engine().lock().record_error(error_record)
}

/// Get error statistics
pub fn get_error_statistics() -> ErrorHandlingStats {
    get_error_handling_engine().lock().get_statistics()
}

/// Shutdown error handling system
pub fn shutdown_error_handling() -> nos_api::Result<()> {
    get_error_handling_engine().lock().shutdown()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
        assert!(ErrorSeverity::Critical < ErrorSeverity::Fatal);
    }

    #[test]
    fn test_error_category() {
        assert_ne!(ErrorCategory::System, ErrorCategory::Memory);
        assert_eq!(ErrorCategory::Network, ErrorCategory::Network);
    }

    #[test]
    fn test_error_config_default() {
        let config = ErrorHandlingConfig::default();
        assert!(config.enable_recovery);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_interval_ms, 1000);
    }

    #[test]
    fn test_error_record_creation() {
        let source = ErrorSource {
            module: "test".to_string(),
            function: "test_func".to_string(),
            file: "test.rs".to_string(),
            line: 10,
            column: 5,
            process_id: 1,
            thread_id: 1,
            cpu_id: 0,
        };

        let record = ErrorRecord {
            id: 1,
            code: 1001,
            error_type: ErrorType::RuntimeError,
            category: ErrorCategory::System,
            severity: ErrorSeverity::Error,
            priority: ErrorPriority::Normal,
            status: ErrorStatus::New,
            message: "Test error".to_string(),
            description: "Test error description".to_string(),
            source: source.clone(),
            timestamp: 0,
            context: ErrorContext {
                environment_variables: BTreeMap::new(),
                system_config: BTreeMap::new(),
                user_input: None,
                related_data: Vec::new(),
                operation_sequence: Vec::new(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
            stack_trace: Vec::new(),
            system_state: SystemStateSnapshot::default(),
            recovery_actions: Vec::new(),
            occurrence_count: 1,
            last_occurrence: 0,
            resolved: false,
            resolution_time: None,
            resolution_method: None,
            metadata: BTreeMap::new(),
        };

        assert_eq!(record.id, 1);
        assert_eq!(record.severity, ErrorSeverity::Error);
        assert_eq!(record.source.module, "test");
    }
}