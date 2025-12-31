//! Service driver module for syscalls
//!
//! This module provides driver management functionality for system calls

use nos_api::syscall::interface::SyscallHandler;
use nos_api::syscall::types::{SyscallNumber, SyscallArgs, SyscallResult};
use nos_api::{Result, error::Error};

/// Service driver handler
pub struct ServiceDriver;

impl ServiceDriver {
    pub fn new() -> Self {
        Self
    }
}

impl SyscallHandler for ServiceDriver {
    fn handle(&mut self, number: SyscallNumber, args: &SyscallArgs) -> Result<SyscallResult> {
        // GH-#1321: Implement proper syscall handling with argument processing
        // See: https://github.com/npos/kernel/issues/1321
        // For now, return error indicating the syscall is not implemented
        // The args parameter will be used to extract syscall-specific arguments
        // such as file descriptors, buffers, flags, etc.
        Err(Error::SystemError(format!(
            "Syscall {} not implemented (args: {:?})",
            number, args
        ))
        .into())
    }

    fn name(&self) -> &str {
        "service_driver"
    }

    fn supports(&self, number: SyscallNumber) -> bool {
        // GH-#1322: Implement syscall support detection
        // See: https://github.com/npos/kernel/issues/1322
        // This method should check if the given syscall number is supported
        // by this driver. For now, no syscalls are supported.
        // When implemented, this will return true for supported syscall numbers.
        let _ = number; // Prefix with underscore to explicitly mark as intentionally unused until implemented
        false
    }
}
