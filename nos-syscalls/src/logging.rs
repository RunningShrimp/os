//! Unified logging support for nos-syscalls
//!
//! This module provides a unified logging interface that handles different
//! feature combinations (log, std, alloc) without requiring repetitive
//! #[cfg] attributes throughout the codebase.

#[cfg(feature = "log")]
use log as external_log;

/// Unified trace-level logging
#[macro_export]
macro_rules! sys_trace {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::trace!($($arg)*);
    }
}

/// Unified debug-level logging
#[macro_export]
macro_rules! sys_debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::debug!($($arg)*);
    }
}

/// Unified info-level logging
#[macro_export]
macro_rules! sys_info {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::info!($($arg)*);
    }
}

/// Unified warn-level logging
#[macro_export]
macro_rules! sys_warn {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::warn!($($arg)*);
    }
}

/// Unified error-level logging
#[macro_export]
macro_rules! sys_error {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::error!($($arg)*);
    }
}

/// Unified trace-level logging with automatic parameter marking
/// This macro automatically marks parameters as used when logging is disabled
#[macro_export]
macro_rules! sys_trace_with_args {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::trace!($($arg)*);
        #[cfg(not(feature = "log"))]
        { let _ = ($($arg)*); }
    }
}

/// Unified debug-level logging with automatic parameter marking
#[macro_export]
macro_rules! sys_debug_with_args {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::debug!($($arg)*);
        #[cfg(not(feature = "log"))]
        { let _ = ($($arg)*); }
    }
}

/// Unified info-level logging with automatic parameter marking
#[macro_export]
macro_rules! sys_info_with_args {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::info!($($arg)*);
        #[cfg(not(feature = "log"))]
        { let _ = ($($arg)*); }
    }
}

/// Unified warn-level logging with automatic parameter marking
#[macro_export]
macro_rules! sys_warn_with_args {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::warn!($($arg)*);
        #[cfg(not(feature = "log"))]
        { let _ = ($($arg)*); }
    }
}

/// Unified error-level logging with automatic parameter marking
#[macro_export]
macro_rules! sys_error_with_args {
    ($($arg:tt)*) => {
        #[cfg(feature = "log")]
        log::error!($($arg)*);
        #[cfg(not(feature = "log"))]
        { let _ = ($($arg)*); }
    }
}

/// Unified report output function
/// Handles different feature combinations for outputting reports
#[inline]
pub fn output_report(report: &str) {
    #[cfg(feature = "log")]
    external_log::info!("{}", report);
    
    #[cfg(all(feature = "std", not(feature = "log")))]
    println!("{}", report);
    
    #[cfg(not(any(feature = "log", feature = "std")))]
    core::hint::black_box(report);
}

/// Mark a value as used to suppress unused variable warnings
/// Useful for parameters that are only used in logging
#[inline]
pub fn mark_used<T>(value: T) -> T {
    value
}

/// Helper macro to mark multiple values as used
#[macro_export]
macro_rules! mark_used {
    ($($val:expr),*) => {
        let _ = ($($crate::logging::mark_used($val)),*);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_used() {
        let x = 42;
        let y = "test";
        mark_used!(x, y);
    }

    #[test]
    fn test_output_report() {
        output_report("Test report");
    }
}
