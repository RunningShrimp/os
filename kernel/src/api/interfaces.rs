//! Module Interface Specification
//!
//! This document defines the interface specification for all kernel modules.
//! It provides guidelines for implementing clear, consistent module boundaries.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// Module interface trait
///
/// This trait defines the common interface that all kernel modules should implement.
/// It provides a standardized way to initialize, configure, and query modules.
pub trait ModuleInterface {
    /// Get the module name
    ///
    /// # Returns
    /// * `&str` - Module name
    fn get_name(&self) -> &str;

    /// Get the module version
    ///
    /// # Returns
    /// * `&str` - Module version
    fn get_version(&self) -> &str;

    /// Get the module description
    ///
    /// # Returns
    /// * `&str` - Module description
    fn get_description(&self) -> &str;

    /// Get the module dependencies
    ///
    /// # Returns
    /// * `Vec<&str>` - Module dependencies
    fn get_dependencies(&self) -> Vec<&str>;

    /// Initialize the module
    ///
    /// # Returns
    /// * `Result<(), ModuleError>` - Success or error
    fn initialize(&mut self) -> Result<(), ModuleError>;

    /// Shutdown the module
    ///
    /// # Returns
    /// * `Result<(), ModuleError>` - Success or error
    fn shutdown(&mut self) -> Result<(), ModuleError>;

    /// Get the module status
    ///
    /// # Returns
    /// * `ModuleStatus` - Module status
    fn get_status(&self) -> ModuleStatus;

    /// Get the module configuration
    ///
    /// # Returns
    /// * `&ModuleConfig` - Module configuration
    fn get_config(&self) -> &ModuleConfig;

    /// Update the module configuration
    ///
    /// # Arguments
    /// * `config` - New module configuration
    ///
    /// # Returns
    /// * `Result<(), ModuleError>` - Success or error
    fn update_config(&mut self, config: ModuleConfig) -> Result<(), ModuleError>;

    /// Get the module statistics
    ///
    /// # Returns
    /// * `ModuleStats` - Module statistics
    fn get_stats(&self) -> ModuleStats;

    /// Reset the module statistics
    fn reset_stats(&mut self);
}

/// Module error
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleError {
    /// Initialization failed
    InitializationFailed,
    /// Shutdown failed
    ShutdownFailed,
    /// Configuration error
    ConfigurationError,
    /// Dependency not met
    DependencyNotMet,
    /// Resource not available
    ResourceUnavailable,
    /// Invalid state
    InvalidState,
    /// Operation not supported
    NotSupported,
    /// Unknown error
    Unknown,
}

/// Module status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModuleStatus {
    /// Module is not initialized
    Uninitialized,
    /// Module is being initialized
    Initializing,
    /// Module is initialized and ready
    Ready,
    /// Module is running
    Running,
    /// Module is being shut down
    ShuttingDown,
    /// Module is shut down
    Shutdown,
    /// Module is in error state
    Error,
}

/// Module configuration
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    /// Configuration parameters
    pub parameters: Vec<(String, String)>,
    /// Feature flags
    pub features: Vec<String>,
    /// Debug mode
    pub debug: bool,
}

/// Module statistics
#[derive(Debug, Clone)]
pub struct ModuleStats {
    /// Initialization time
    pub init_time: u64,
    /// Total operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Last operation time
    pub last_operation_time: u64,
    /// Average operation time
    pub avg_operation_time: f64,
}

/// Service interface trait
///
/// This trait defines the interface that all services should implement.
/// It provides a standardized way to register, configure, and manage services.
pub trait ServiceInterface {
    /// Get the service name
    ///
    /// # Returns
    /// * `&str` - Service name
    fn get_name(&self) -> &str;

    /// Get the service version
    ///
    /// # Returns
    /// * `&str` - Service version
    fn get_version(&self) -> &str;

    /// Get the service description
    ///
    /// # Returns
    /// * `&str` - Service description
    fn get_description(&self) -> &str;

    /// Get the service capabilities
    ///
    /// # Returns
    /// * `ServiceCapabilities` - Service capabilities
    fn get_capabilities(&self) -> ServiceCapabilities;

    /// Start the service
    ///
    /// # Returns
    /// * `Result<(), ServiceError>` - Success or error
    fn start(&mut self) -> Result<(), ServiceError>;

    /// Stop the service
    ///
    /// # Returns
    /// * `Result<(), ServiceError>` - Success or error
    fn stop(&mut self) -> Result<(), ServiceError>;

    /// Get the service status
    ///
    /// # Returns
    /// * `ServiceStatus` - Service status
    fn get_status(&self) -> ServiceStatus;

    /// Handle a service request
    ///
    /// # Arguments
    /// * `request` - Service request
    ///
    /// # Returns
    /// * `Result<ServiceResponse, ServiceError>` - Service response or error
    fn handle_request(&mut self, request: ServiceRequest) -> Result<ServiceResponse, ServiceError>;

    /// Get the service statistics
    ///
    /// # Returns
    /// * `ServiceStats` - Service statistics
    fn get_stats(&self) -> ServiceStats;

    /// Reset the service statistics
    fn reset_stats(&mut self);
}

/// Service error
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceError {
    /// Start failed
    StartFailed,
    /// Stop failed
    StopFailed,
    /// Request failed
    RequestFailed,
    /// Invalid request
    InvalidRequest,
    /// Resource not available
    ResourceUnavailable,
    /// Permission denied
    PermissionDenied,
    /// Timeout
    Timeout,
    /// Unknown error
    Unknown,
}

/// Service capabilities
#[derive(Debug, Clone)]
pub struct ServiceCapabilities {
    /// Supported operations
    pub operations: Vec<String>,
    /// Maximum concurrent requests
    pub max_concurrent_requests: u32,
    /// Request timeout
    pub request_timeout: u32,
    /// Supports asynchronous operations
    pub supports_async: bool,
}

/// Service request
#[derive(Debug, Clone)]
pub struct ServiceRequest {
    /// Request ID
    pub request_id: u64,
    /// Operation
    pub operation: String,
    /// Parameters
    pub parameters: Vec<(String, String)>,
    /// Timestamp
    pub timestamp: u64,
}

/// Service response
#[derive(Debug, Clone)]
pub struct ServiceResponse {
    /// Request ID
    pub request_id: u64,
    /// Result
    pub result: ServiceResult,
    /// Error message
    pub error_message: Option<String>,
    /// Timestamp
    pub timestamp: u64,
}

/// Service result
#[derive(Debug, Clone)]
pub enum ServiceResult {
    /// Success with data
    Success(String),
    /// Success with no data
    Acknowledged,
    /// In progress
    InProgress,
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    /// Service is not initialized
    Uninitialized,
    /// Service is being initialized
    Initializing,
    /// Service is initialized and ready
    Ready,
    /// Service is running
    Running,
    /// Service is being shut down
    ShuttingDown,
    /// Service is shut down
    Shutdown,
    /// Service is in error state
    Error,
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStats {
    /// Start time
    pub start_time: u64,
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time
    pub avg_response_time: f64,
    /// Last request time
    pub last_request_time: u64,
}

/// Driver interface trait
///
/// This trait defines the interface that all drivers should implement.
/// It provides a standardized way to initialize, configure, and manage drivers.
pub trait DriverInterface {
    /// Get the driver name
    ///
    /// # Returns
    /// * `&str` - Driver name
    fn get_name(&self) -> &str;

    /// Get the driver version
    ///
    /// # Returns
    /// * `&str` - Driver version
    fn get_version(&self) -> &str;

    /// Get the driver description
    ///
    /// # Returns
    /// * `&str` - Driver description
    fn get_description(&self) -> &str;

    /// Get the supported devices
    ///
    /// # Returns
    /// * `Vec<String>` - Supported devices
    fn get_supported_devices(&self) -> Vec<String>;

    /// Initialize the driver
    ///
    /// # Returns
    /// * `Result<(), DriverError>` - Success or error
    fn initialize(&mut self) -> Result<(), DriverError>;

    /// Shutdown the driver
    ///
    /// # Returns
    /// * `Result<(), DriverError>` - Success or error
    fn shutdown(&mut self) -> Result<(), DriverError>;

    /// Get the driver status
    ///
    /// # Returns
    /// * `DriverStatus` - Driver status
    fn get_status(&self) -> DriverStatus;

    /// Probe for a device
    ///
    /// # Arguments
    /// * `device_id` - Device ID
    ///
    /// # Returns
    /// * `Result<bool, DriverError>` - True if device is supported
    fn probe_device(&self, device_id: &str) -> Result<bool, DriverError>;

    /// Open a device
    ///
    /// # Arguments
    /// * `device_id` - Device ID
    ///
    /// # Returns
    /// * `Result<DeviceHandle, DriverError>` - Device handle or error
    fn open_device(&mut self, device_id: &str) -> Result<DeviceHandle, DriverError>;

    /// Close a device
    ///
    /// # Arguments
    /// * `handle` - Device handle
    ///
    /// # Returns
    /// * `Result<(), DriverError>` - Success or error
    fn close_device(&mut self, handle: DeviceHandle) -> Result<(), DriverError>;

    /// Read from a device
    ///
    /// # Arguments
    /// * `handle` - Device handle
    /// * `buffer` - Buffer to read into
    /// * `count` - Number of bytes to read
    ///
    /// # Returns
    /// * `Result<usize, DriverError>` - Number of bytes read or error
    fn read_device(&mut self, handle: DeviceHandle, buffer: &mut [u8], count: usize) -> Result<usize, DriverError>;

    /// Write to a device
    ///
    /// # Arguments
    /// * `handle` - Device handle
    /// * `buffer` - Buffer to write from
    /// * `count` - Number of bytes to write
    ///
    /// # Returns
    /// * `Result<usize, DriverError>` - Number of bytes written or error
    fn write_device(&mut self, handle: DeviceHandle, buffer: &[u8], count: usize) -> Result<usize, DriverError>;

    /// Control a device
    ///
    /// # Arguments
    /// * `handle` - Device handle
    /// * `command` - Control command
    /// * `arg` - Command argument
    ///
    /// # Returns
    /// * `Result<u64, DriverError>` - Control result or error
    fn control_device(&mut self, handle: DeviceHandle, command: u32, arg: u64) -> Result<u64, DriverError>;

    /// Get the driver statistics
    ///
    /// # Returns
    /// * `DriverStats` - Driver statistics
    fn get_stats(&self) -> DriverStats;

    /// Reset the driver statistics
    fn reset_stats(&mut self);
}

/// Driver error
#[derive(Debug, Clone, PartialEq)]
pub enum DriverError {
    /// Initialization failed
    InitializationFailed,
    /// Shutdown failed
    ShutdownFailed,
    /// Device not found
    DeviceNotFound,
    /// Device not supported
    DeviceNotSupported,
    /// Device busy
    DeviceBusy,
    /// Invalid handle
    InvalidHandle,
    /// Read failed
    ReadFailed,
    /// Write failed
    WriteFailed,
    /// Control failed
    ControlFailed,
    /// Permission denied
    PermissionDenied,
    /// Timeout
    Timeout,
    /// Unknown error
    Unknown,
}

/// Driver status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriverStatus {
    /// Driver is not initialized
    Uninitialized,
    /// Driver is being initialized
    Initializing,
    /// Driver is initialized and ready
    Ready,
    /// Driver is active
    Active,
    /// Driver is being shut down
    ShuttingDown,
    /// Driver is shut down
    Shutdown,
    /// Driver is in error state
    Error,
}

/// Device handle
#[derive(Debug, Clone, Copy)]
pub struct DeviceHandle {
    /// Handle value
    pub value: u64,
}

/// Driver statistics
#[derive(Debug, Clone)]
pub struct DriverStats {
    /// Initialization time
    pub init_time: u64,
    /// Total operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Last operation time
    pub last_operation_time: u64,
}

/// Module registry
///
/// This struct provides a registry for all kernel modules.
/// It is used to register, initialize, and manage modules.
pub struct ModuleRegistry {
    /// Registered modules
    modules: Vec<Box<dyn ModuleInterface>>,
    /// Initialization order
    init_order: Vec<String>,
}

impl ModuleRegistry {
    /// Create a new module registry
    ///
    /// # Returns
    /// * `ModuleRegistry` - New registry
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            init_order: Vec::new(),
        }
    }

    /// Register a module
    ///
    /// # Arguments
    /// * `module` - Module to register
    ///
    /// # Returns
    /// * `Result<(), ModuleError>` - Success or error
    pub fn register(&mut self, module: Box<dyn ModuleInterface>) -> Result<(), ModuleError> {
        let name = module.get_name();
        
        // Check if module is already registered
        if self.modules.iter().any(|m| m.get_name() == name) {
            return Err(ModuleError::InitializationFailed);
        }
        
        // Check dependencies
        for dep in module.get_dependencies() {
            if !self.modules.iter().any(|m| m.get_name() == dep) {
                return Err(ModuleError::DependencyNotMet);
            }
        }
        
        self.modules.push(module);
        self.init_order.push(name.to_string());
        
        Ok(())
    }

    /// Initialize all modules
    ///
    /// # Returns
    /// * `Result<(), ModuleError>` - Success or error
    pub fn initialize_all(&mut self) -> Result<(), ModuleError> {
        // Initialize modules in dependency order
        for name in &self.init_order.clone() {
            if let Some(module) = self.modules.iter_mut()
                .find(|m| m.get_name() == name) {
                module.initialize()?;
            }
        }
        
        Ok(())
    }

    /// Shutdown all modules
    ///
    /// # Returns
    /// * `Result<(), ModuleError>` - Success or error
    pub fn shutdown_all(&mut self) -> Result<(), ModuleError> {
        // Shutdown modules in reverse dependency order
        for name in self.init_order.iter().rev() {
            if let Some(module) = self.modules.iter_mut()
                .find(|m| m.get_name() == name) {
                module.shutdown()?;
            }
        }
        
        Ok(())
    }

    /// Get a module by name
    ///
    /// # Arguments
    /// * `name` - Module name
    ///
    /// # Returns
    /// * `Option<&dyn ModuleInterface>` - Module if found
    pub fn get_module(&self, name: &str) -> Option<&dyn ModuleInterface> {
        self.modules.iter()
            .find(|m| m.get_name() == name)
            .map(|m| m.as_ref())
    }

    /// Get all modules
    ///
    /// # Returns
    /// * `Vec<&dyn ModuleInterface>` - All modules
    pub fn get_all_modules(&self) -> Vec<&dyn ModuleInterface> {
        self.modules.iter()
            .map(|m| m.as_ref())
            .collect()
    }

    /// Get module statistics
    ///
    /// # Returns
    /// * `Vec<(&str, ModuleStats)>` - Module statistics
    pub fn get_all_stats(&self) -> Vec<(&str, ModuleStats)> {
        self.modules.iter()
            .map(|m| (m.get_name(), m.get_stats()))
            .collect()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}