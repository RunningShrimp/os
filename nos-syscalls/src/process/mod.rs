//! Process system calls
//!
//! This module provides process management related system calls.

#[cfg(feature = "alloc")]
use alloc::string::ToString;
#[cfg(feature = "alloc")]
use alloc::boxed::Box;
use nos_api::Result;
use crate::core::SyscallHandler;
#[cfg(feature = "log")]
use log;
#[cfg(feature = "alloc")]
use crate::core::SyscallDispatcher;

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
    fn id(&self) -> u32 {
        crate::types::SYS_FORK
    }
    
    fn execute(&self, _args: &[usize]) -> Result<isize> {
        // TODO: Implement actual fork logic
        Ok(0) // Return child PID in parent, 0 in child
    }
    
    fn name(&self) -> &str {
        "fork"
    }
}

/// Exec system call handler
struct ExecHandler;

impl SyscallHandler for ExecHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EXEC
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 2 {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".into()));
        }
        
        let pathname = args[0] as *const u8;
        let argv = args[1] as *const *const u8;
        
        // TODO: Implement actual exec logic using parameters:
        // pathname: Path to executable file
        // argv: Array of argument strings
        #[cfg(feature = "log")]
        log::trace!("exec called with: pathname={:?}, argv={:?}", pathname, argv);
        
        // Ensure parameters are used even when logging is disabled
        let _ = (pathname, argv);
        
        Ok(0)
    }
    
    fn name(&self) -> &str {
        "exec"
    }
}

/// Wait system call handler
struct WaitHandler;

impl SyscallHandler for WaitHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_WAIT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 2 {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".into()));
        }
        
        let pid = args[0] as i32;
        let status = args[1] as *mut i32;
        
        // TODO: Implement actual wait logic using parameters:
        // pid: Process ID to wait for, or -1 for any child process
        // status: Pointer to store exit status information
        #[cfg(feature = "log")]
        log::trace!("wait called with: pid={}, status={:?}", pid, status);
        
        // Ensure status is used even when logging is disabled
        let _ = status;
        
        Ok(pid as isize) // Return child PID
    }
    
    fn name(&self) -> &str {
        "wait"
    }
}

/// Exit system call handler
struct ExitHandler;

impl SyscallHandler for ExitHandler {
    fn id(&self) -> u32 {
        crate::types::SYS_EXIT
    }
    
    fn execute(&self, args: &[usize]) -> Result<isize> {
        if args.len() < 1 {
            #[cfg(feature = "alloc")]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".to_string()));
            #[cfg(not(feature = "alloc"))]
            return Err(nos_api::Error::InvalidArgument("Insufficient arguments".into()));
        }
        
        let status = args[0] as i32;
        
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
}