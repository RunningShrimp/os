//! System call traits
//!
//! This module provides common traits for system calls.

use nos_api::Result;

/// System call handler trait
pub trait SyscallHandler: Send + Sync {
    /// Execute the system call
    fn execute(&self, args: &[usize]) -> Result<isize>;
    
    /// Get the system call name
    fn name(&self) -> &str;
    
    /// Get the system call ID
    fn id(&self) -> u32;
    
    /// Check if the system call is available
    fn is_available(&self) -> bool {
        true
    }
}

/// System call validator trait
pub trait SyscallValidator: Send + Sync {
    /// Validate system call arguments
    fn validate(&self, args: &[usize]) -> Result<()>;
    
    /// Get the validator name
    fn name(&self) -> &str;
}

/// System call logger trait
pub trait SyscallLogger: Send + Sync {
    /// Log a system call
    fn log(&self, id: u32, args: &[usize], result: Result<isize>);
    
    /// Get the logger name
    fn name(&self) -> &str;
}

/// System call interceptor trait
pub trait SyscallInterceptor: Send + Sync {
    /// Intercept a system call before execution
    fn before(&self, _id: u32, _args: &[usize]) -> Result<bool> {
        // Return true to continue execution, false to block
        Ok(true)
    }
    
    /// Intercept a system call after execution
    fn after(&self, _id: u32, _args: &[usize], _result: &Result<isize>) {
        // Default implementation does nothing
    }
    
    /// Get the interceptor name
    fn name(&self) -> &str;
}

/// System call filter trait
pub trait SyscallFilter: Send + Sync {
    /// Check if system call should be allowed
    fn allow(&self, id: u32, args: &[usize]) -> bool;
    
    /// Get filter name
    fn name(&self) -> &str;
}

/// System call context trait
pub trait SyscallContext: Send + Sync {
    /// Get the current process ID
    fn get_pid(&self) -> u32;
    
    /// Get the current user ID
    fn get_uid(&self) -> u32;
    
    /// Get the current thread ID
    fn get_tid(&self) -> u32;
    
    /// Check if the current context has permission to execute the system call
    fn has_permission(&self, id: u32) -> bool;
}

/// System call statistics trait
pub trait SyscallStatsTrait: Send + Sync {
    /// Record a system call execution
    fn record(&self, id: u32, execution_time: u64, success: bool);

    /// Get the total number of system calls
    fn get_total_calls(&self) -> u64;

    /// Get the number of system calls by type
    fn get_calls_by_type(&self, id: u32) -> u64;

    /// Get the average execution time
    fn get_avg_execution_time(&self) -> u64;

    /// Get the error rate
    fn get_error_rate(&self) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: &'static str,
        id: u32,
    }

    impl SyscallHandler for TestHandler {
        fn execute(&self, _args: &[usize]) -> Result<isize> {
            Ok(0)
        }
        
        fn name(&self) -> &str {
            self.name
        }
        
        fn id(&self) -> u32 {
            self.id
        }
    }

    #[test]
    fn test_syscall_handler() {
        let handler = TestHandler {
            name: "test",
            id: 100,
        };
        
        assert_eq!(handler.name(), "test");
        assert_eq!(handler.id(), 100);
        assert!(handler.is_available());
    }
}