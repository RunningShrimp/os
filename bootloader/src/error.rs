//! Bootloader error handling
//!
//! This module defines the error types used throughout the bootloader
//! for consistent error reporting and handling.

use core::fmt;

/// Bootloader error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootError {
    /// Generic error with custom code
    Generic(u32),

    /// Initialization failed
    InitializationFailed(&'static str),

    /// Memory management errors
    MemoryAllocationFailed,
    MemoryMapError,
    OutOfMemory,

    /// Protocol errors
    ProtocolNotSupported,
    ProtocolDetectionFailed,
    ProtocolInitializationFailed(&'static str),

    /// UEFI-specific errors
    UefiError(uefi::Status),
    UefiNotFound,
    UefiUnsupported,

    /// BIOS-specific errors
    BiosInterruptFailed(u8),
    BiosNotSupported,

    /// Device errors
    DeviceNotFound,
    DeviceError(&'static str),

    /// Filesystem errors
    FileNotFound,
    FileSystemError,
    InvalidFileFormat,

    /// Network errors
    NetworkError,
    ConnectionFailed,
    Timeout,

    /// Kernel loading errors
    KernelNotFound,
    KernelLoadFailed,
    InvalidKernelFormat,

    /// Boot configuration errors
    InvalidBootConfig,
    ConfigurationError(&'static str),

    /// Architecture errors
    UnsupportedArchitecture,

    /// Recovery errors
    RecoveryModeFailed,
    RecoveryNotSupported,
    UserRequestedRecovery,

    /// System errors
    SystemHalted,
    SystemRebooted,
    SystemShutdown,

    /// Internal errors
    NotInitialized,
    InvalidState,
    CorruptionDetected,

    /// Kernel unexpectedly returned
    KernelReturned,

    /// Feature not enabled
    FeatureNotEnabled(&'static str),
}

impl BootError {
    /// Convert to an error code suitable for passing to firmware/OS
    pub fn as_error_code(&self) -> u32 {
        match self {
            BootError::Generic(code) => *code,
            BootError::InitializationFailed(_) => 0x1000,
            BootError::MemoryAllocationFailed => 0x2000,
            BootError::MemoryMapError => 0x2001,
            BootError::OutOfMemory => 0x2002,
            BootError::ProtocolNotSupported => 0x3000,
            BootError::ProtocolDetectionFailed => 0x3001,
            BootError::ProtocolInitializationFailed(_) => 0x3002,
            BootError::UefiError(status) => status.as_usize() as u32,
            BootError::UefiNotFound => 0x4001,
            BootError::UefiUnsupported => 0x4002,
            BootError::BiosInterruptFailed(int) => 0x5000 + (*int as u32),
            BootError::BiosNotSupported => 0x50FF,
            BootError::DeviceNotFound => 0x6000,
            BootError::DeviceError(_) => 0x6001,
            BootError::FileNotFound => 0x7000,
            BootError::FileSystemError => 0x7001,
            BootError::InvalidFileFormat => 0x7002,
            BootError::NetworkError => 0x8000,
            BootError::ConnectionFailed => 0x8001,
            BootError::Timeout => 0x8002,
            BootError::KernelNotFound => 0x9000,
            BootError::KernelLoadFailed => 0x9001,
            BootError::InvalidKernelFormat => 0x9002,
            BootError::InvalidBootConfig => 0xA000,
            BootError::ConfigurationError(_) => 0xA001,
            BootError::UnsupportedArchitecture => 0xB000,
            BootError::RecoveryModeFailed => 0xC000,
            BootError::RecoveryNotSupported => 0xC001,
            BootError::UserRequestedRecovery => 0xC002,
            BootError::SystemHalted => 0xD000,
            BootError::SystemRebooted => 0xD001,
            BootError::SystemShutdown => 0xD002,
            BootError::NotInitialized => 0xE000,
            BootError::InvalidState => 0xE001,
            BootError::CorruptionDetected => 0xE002,
            BootError::KernelReturned => 0xF000,
            BootError::FeatureNotEnabled(_) => 0xF001,
        }
    }

    /// Get a human-readable description of the error
    pub fn description(&self) -> &'static str {
        match self {
            BootError::Generic(_) => "Generic error",
            BootError::InitializationFailed(msg) => msg,
            BootError::MemoryAllocationFailed => "Failed to allocate memory",
            BootError::MemoryMapError => "Memory map error",
            BootError::OutOfMemory => "Out of memory",
            BootError::ProtocolNotSupported => "Boot protocol not supported",
            BootError::ProtocolDetectionFailed => "Failed to detect boot protocol",
            BootError::ProtocolInitializationFailed(msg) => msg,
            BootError::UefiError(_) => "UEFI error",
            BootError::UefiNotFound => "UEFI not found",
            BootError::UefiUnsupported => "UEFI not supported",
            BootError::BiosInterruptFailed(_) => "BIOS interrupt failed",
            BootError::BiosNotSupported => "BIOS not supported",
            BootError::DeviceNotFound => "Device not found",
            BootError::DeviceError(msg) => msg,
            BootError::FileNotFound => "File not found",
            BootError::FileSystemError => "File system error",
            BootError::InvalidFileFormat => "Invalid file format",
            BootError::NetworkError => "Network error",
            BootError::ConnectionFailed => "Connection failed",
            BootError::Timeout => "Operation timed out",
            BootError::KernelNotFound => "Kernel not found",
            BootError::KernelLoadFailed => "Failed to load kernel",
            BootError::InvalidKernelFormat => "Invalid kernel format",
            BootError::InvalidBootConfig => "Invalid boot configuration",
            BootError::ConfigurationError(msg) => msg,
            BootError::UnsupportedArchitecture => "Unsupported architecture",
            BootError::RecoveryModeFailed => "Recovery mode failed",
            BootError::RecoveryNotSupported => "Recovery not supported",
            BootError::UserRequestedRecovery => "User requested recovery mode",
            BootError::SystemHalted => "System halted",
            BootError::SystemRebooted => "System rebooted",
            BootError::SystemShutdown => "System shutdown",
            BootError::NotInitialized => "Bootloader not initialized",
            BootError::InvalidState => "Invalid bootloader state",
            BootError::CorruptionDetected => "Data corruption detected",
            BootError::KernelReturned => "Kernel unexpectedly returned",
            BootError::FeatureNotEnabled(feature) => feature,
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            BootError::Timeout
            | BootError::ConnectionFailed
            | BootError::DeviceNotFound
            | BootError::FileNotFound => true,
            BootError::UserRequestedRecovery => true,
            _ => false,
        }
    }

    /// Check if this error should trigger recovery mode
    pub fn should_enter_recovery(&self) -> bool {
        match self {
            BootError::KernelLoadFailed
            | BootError::KernelNotFound
            | BootError::InvalidKernelFormat
            | BootError::CorruptionDetected
            | BootError::DeviceError(_) => true,
            _ => false,
        }
    }
}

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BootError: {} (code: {:#x})", self.description(), self.as_error_code())
    }
}

/// Result type used throughout the bootloader
pub type Result<T = ()> = core::result::Result<T, BootError>;

/// Convert UEFI status to bootloader error
#[cfg(feature = "uefi_support")]
impl From<uefi::Status> for BootError {
    fn from(status: uefi::Status) -> Self {
        match status {
            uefi::Status::NOT_FOUND => BootError::UefiNotFound,
            uefi::Status::UNSUPPORTED => BootError::UefiUnsupported,
            _ => BootError::UefiError(status),
        }
    }
}

/// Convert BIOS interrupt return to bootloader error
#[cfg(feature = "bios_support")]
pub fn bios_interrupt_error(interrupt: u8, ax: u16) -> BootError {
    if ax & 0x8000 != 0 {
        BootError::BiosInterruptFailed(interrupt)
    } else {
        BootError::Generic(ax as u32)
    }
}

/// Macro for creating formatted bootloader errors
#[macro_export]
macro_rules! boot_error {
    ($error:expr, $($arg:tt)*) => {
        $crate::error::BootError::Generic(format_args!($($arg)*).as_str().as_ptr() as u32)
    };
}