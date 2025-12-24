//! Process system calls
//!
//! This module provides process management related system calls.

#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use nos_api::Result;
use crate::core::traits::SyscallHandler;
use nos_api::syscall::types::{SyscallNumber, SyscallArgs, SyscallResult};
#[cfg(feature = "log")]
use log;
#[cfg(feature = "alloc")]
use crate::core::dispatcher::SyscallDispatcher;

/// Register process system call handlers
#[cfg(feature = "alloc")]
pub fn register_handlers(dispatcher: &mut SyscallDispatcher) -> Result<()> {
    // Register fork system call
    dispatcher.register_handler(
        crate::types::SYS_FORK,
        Box::new(ForkHandler)
    );
    
    // Register exec system call
    dispatcher.register_handler(
        crate::types::SYS_EXEC,
        Box::new(ExecHandler)
    );
    
    // Register wait system call
    dispatcher.register_handler(
        crate::types::SYS_WAIT,
        Box::new(WaitHandler)
    );
    
    // Register exit system call
    dispatcher.register_handler(
        crate::types::SYS_EXIT,
        Box::new(ExitHandler)
    );
    
    Ok(())
}

/// Fork system call handler
struct ForkHandler;

impl SyscallHandler for ForkHandler {
    fn execute(&self, _args: &[usize]) -> Result<isize> {
        // TODO: Implement actual fork logic
        Ok(0) // Return child PID in parent, 0 in child
    }

    fn name(&self) -> &str {
        "fork"
    }

    fn id(&self) -> u32 {
        crate::types::SYS_FORK
    }
}

/// Exec system call handler
struct ExecHandler;

impl SyscallHandler for ExecHandler {
    fn execute(&self, args: &[usize]) -> Result<isize> {
        // TODO: Implement actual exec logic using parameters:
        // pathname: Path to executable file
        // argv: Array of argument strings
        #[cfg(feature = "log")]
        log::trace!("exec called with: arg0={:?}, arg1={:?}", args.get(0), args.get(1));

        Ok(0)
    }

    fn name(&self) -> &str {
        "exec"
    }

    fn id(&self) -> u32 {
        crate::types::SYS_EXEC
    }
}

/// Wait system call handler
struct WaitHandler;

impl SyscallHandler for WaitHandler {
    fn execute(&self, args: &[usize]) -> Result<isize> {
        let pid = args.get(0).copied().unwrap_or(0) as i32;
        let status = args.get(1).copied().unwrap_or(0) as *mut i32;

        // TODO: Implement actual wait logic using parameters:
        // pid: Process ID to wait for, or -1 for any child process
        // status: Pointer to store exit status information
        #[cfg(feature = "log")]
        log::trace!("wait called with: pid={}, status={:?}", pid, status);

        // Ensure status is used even when logging is disabled
        let _ = status;

        Ok(pid as isize)
    }

    fn name(&self) -> &str {
        "wait"
    }

    fn id(&self) -> u32 {
        crate::types::SYS_WAIT
    }
}

/// Exit system call handler
struct ExitHandler;

impl SyscallHandler for ExitHandler {
    fn execute(&self, args: &[usize]) -> Result<isize> {
        let status = args.get(0).copied().unwrap_or(0) as i32;

        // TODO: Implement actual exit logic using parameters:
        // status: Exit status value to return to parent process
        #[cfg(feature = "log")]
        log::trace!("exit called with: status={}", status);

        // Ensure status is used even when logging is disabled
        let _ = status;

        Ok(0)
    }

    fn name(&self) -> &str {
        "exit"
    }

    fn id(&self) -> u32 {
        crate::types::SYS_EXIT
    }
}