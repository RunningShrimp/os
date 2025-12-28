#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
//! System Call Handler Wrapper Module
//!
//! This module provides wrapper functions to unify system call handler return types.
//! It automatically converts various error types (KernelError, UnifiedError, etc.)
//! to the standard SyscallResult type.
//! # Design Principles
//! - Zero-cost abstraction: minimal overhead
//! - Type safety: compile-time error checking
//! - Flexibility: works with different handler signatures
//! - Performance: inline where possible

use crate::subsystems::syscalls::interface::SyscallResult;
use alloc::string::ToString;
use crate::subsystems::syscalls::error_conversion::{
    IntoSyscallError, convert_result, convert_result_with_context,
};
use alloc::vec::Vec;

/// Wrap a legacy handler that returns Result<u64, KernelError>
///
/// This function wraps handlers from the old API and converts their
/// return type to the new unified SyscallResult.
/// # Arguments
/// * `f` - The legacy handler function
/// * `args` - System call arguments
/// # Returns
/// * `SyscallResult` - Unified system call result
#[inline(always)]
pub fn wrap_legacy_handler<F>(f: F, args: &[u64]) -> SyscallResult
where
    F: FnOnce(&[u64]) -> Result<u64, crate::api::KernelError>,
{
    f(args).map_err(|e| e.into_syscall_error())
}

/// Wrap a handler that returns Result<T, E> where T: Into<u64>
/// Generic wrapper for handlers that return different value types.
pub fn wrap_handler<T, E, F>(f: F, args: &[u64]) -> SyscallResult
where
    T: Into<u64>,
    E: IntoSyscallError,
    F: FnOnce(&[u64]) -> Result<T, E>,
{
    f(args).map(|v| v.into()).map_err(|e| e.into_syscall_error())
}

/// Wrap a handler with context for debugging
/// This version adds error context information for better debugging.
pub fn wrap_handler_with_context<T, E, F>(
    f: F,
    args: &[u64],
    context: &str,
) -> SyscallResult
where
    T: Into<u64>,
    E: IntoSyscallError,
    F: FnOnce(&[u64]) -> Result<T, E>,
{
    f(args)
        .map(|v| v.into())
        .map_err(|e| e.into_syscall_error_with_context(context))
}

/// Unified handler trait for wrapper compatibility
/// This trait allows existing handlers to be wrapped
/// without changing their implementation.
pub trait UnifiedHandler {
    /// Execute the handler with arguments
    fn execute(&self, args: &[u64]) -> SyscallResult;
    /// Get the handler name for debugging
    fn name(&self) -> &'static str;
}

/// Adapter for legacy handlers
/// This struct wraps a legacy handler function to implement
/// the UnifiedHandler trait.
pub struct LegacyHandlerAdapter<F>
where
    F: Fn(&[u64]) -> Result<u64, crate::api::KernelError>,
{
    func: F,
    handler_name: &'static str,
}

impl<F> LegacyHandlerAdapter<F>
where
    F: Fn(&[u64]) -> Result<u64, crate::api::KernelError>,
{
    /// Create a new adapter for a legacy handler
    pub fn new(func: F, handler_name: &'static str) -> Self {
        Self {
            func,
            handler_name,
        }
    }
}

impl<F> UnifiedHandler for LegacyHandlerAdapter<F>
where
    F: Fn(&[u64]) -> Result<u64, crate::api::KernelError>,
{
    fn execute(&self, args: &[u64]) -> SyscallResult {
        wrap_legacy_handler(&self.func, args)
    }

    fn name(&self) -> &'static str {
        self.handler_name
    }
}

/// Adapter for generic handlers
/// This struct wraps a generic handler function to implement
/// the UnifiedHandler trait.
pub struct GenericHandlerAdapter<T, E, F>
where
    F: Fn(&[u64]) -> Result<T, E>,
    T: Into<u64>,
    E: IntoSyscallError,
{
    func: F,
    handler_name: &'static str,
    _phantom: core::marker::PhantomData<(T, E)>,
}

impl<T, E, F> GenericHandlerAdapter<T, E, F>
where
    F: Fn(&[u64]) -> Result<T, E>,
    T: Into<u64>,
    E: IntoSyscallError,
{
    /// Create a new adapter for a generic handler
    pub fn new(func: F, handler_name: &'static str) -> Self {
        Self {
            func,
            handler_name,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<T, E, F> UnifiedHandler for GenericHandlerAdapter<T, E, F>
where
    F: Fn(&[u64]) -> Result<T, E>,
    T: Into<u64>,
    E: IntoSyscallError,
{
    fn execute(&self, args: &[u64]) -> SyscallResult {
        wrap_handler(&self.func, args)
    }

    fn name(&self) -> &'static str {
        self.handler_name
    }
}

/// Convert a batch of handler results to SyscallResults
/// This function is useful for batch system call operations.
pub fn wrap_batch_results<T, E>(
    results: Vec<Result<T, E>>,
) -> Vec<SyscallResult>
where
    T: Into<u64>,
    E: IntoSyscallError,
{
    results
        .into_iter()
        .map(|r| r.map(|v| v.into()).map_err(|e| e.into_syscall_error()))
        .collect()
}

/// Helper to create a SyscallError with context
/// This function creates an error with detailed context information.
pub fn create_error<E>(error: E, syscall_name: &'static str, operation: &str) -> crate::subsystems::syscalls::interface::SyscallError
where
    E: IntoSyscallError,
{
    let context = alloc::format!("{}: {}", syscall_name, operation);
    error.into_syscall_error_with_context(&context)
}

/// Macro for creating wrapped handlers
/// This macro simplifies the creation of wrapped handlers
/// from existing handler functions.
#[macro_export]
macro_rules! make_wrapped_handler {
    ($name:expr, $func:path) => {
        LegacyHandlerAdapter::new(|args| $func(args), $name)
    };
    ($name:expr, $func:path, $context:expr) => {
        LegacyHandlerAdapter::new(
            |args| {
                $func(args).map_err(|e| e.into_syscall_error_with_context($context))
            },
            $name,
        )
    };
}

/// Macro for creating unified result handlers
/// This macro simplifies converting results to unified format.
#[macro_export]
macro_rules! to_syscall_result {
    ($expr:expr) => {
        $expr.map(|v| v.into()).map_err(|e| e.into_syscall_error())
    };
    ($expr:expr, $context:expr) => {
        $expr.map(|v| v.into())
            .map_err(|e| e.into_syscall_error_with_context($context))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_legacy_handler(args: &[u64]) -> Result<u64, crate::api::KernelError> {
        if args.is_empty() {
            Err(crate::api::KernelError::InvalidArgument)
        } else {
            Ok(args[0])
        }
    }

    fn test_generic_handler(args: &[u64]) -> Result<u32, crate::api::KernelError> {
        if args.is_empty() {
            Err(crate::api::KernelError::NotFoundKey)
        } else {
            Ok(args[0] as u32)
        }
    }

    #[test]
    fn test_wrap_legacy_handler() {
        let result = wrap_legacy_handler(&test_legacy_handler, &[42]);
        assert_eq!(result, Ok(42));
        let error_result = wrap_legacy_handler(&test_legacy_handler, &[]);
        assert!(error_result.is_err());
    }

    #[test]
    fn test_wrap_generic_handler() {
        let result = wrap_handler(&test_generic_handler, &[100]);
        assert_eq!(result, Ok(100u64));
        let error_result = wrap_handler(&test_generic_handler, &[]);
        assert!(error_result.is_err());
    }

    #[test]
    fn test_legacy_adapter() {
        let adapter = LegacyHandlerAdapter::new(&test_legacy_handler, "test_handler");
        assert_eq!(adapter.name(), "test_handler");
        let result = adapter.execute(&[99]);
        assert_eq!(result, Ok(99));
    }

    #[test]
    fn test_generic_adapter() {
        let adapter = GenericHandlerAdapter::new(&test_generic_handler, "generic_test");
        assert_eq!(adapter.name(), "generic_test");
        let result = adapter.execute(&[88]);
        assert_eq!(result, Ok(88u64));
    }

    #[test]
    fn test_wrap_batch_results() {
        let results = {
            let mut v = alloc::vec::Vec::new();
            v.push(Ok::<u32, crate::api::KernelError>(1u32));
            v.push(Err(crate::api::KernelError::NotFoundKey));
            v.push(Ok(2u32));
            v
        };
        let wrapped = wrap_batch_results(results);
        assert_eq!(wrapped[0], Ok(1u64));
        assert!(wrapped[1].is_err());
        assert_eq!(wrapped[2], Ok(2u64));
    }
}
