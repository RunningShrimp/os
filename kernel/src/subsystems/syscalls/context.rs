//! System Call Context Implementation
//!
//! This module provides a concrete implementation of SyscallContext trait.
//! It provides access to system call context information such as current
//! process, user credentials, and other execution context.

use crate::types::stubs::{pid_t, uid_t, gid_t};
use super::interface::SyscallContext;
use alloc::string::String;
/// System call context implementation
///
/// This struct provides access to system call context information.
pub struct SyscallContextImpl {
    /// Current process ID
    pid: pid_t,
    /// Current user ID
    uid: uid_t,
    /// Current group ID
    gid: gid_t,
    /// Current working directory
    cwd: String,
}

impl SyscallContextImpl {
    /// Create a new system call context
    ///
    /// # Returns
    /// * `Self` - New context instance
    pub fn new() -> Self {
        Self {
            pid: 1234, // Example PID
            uid: 1000, // Example UID
            gid: 1000, // Example GID
            cwd: "/".to_string(), // Example CWD
        }
    }
    
    /// Create a new system call context with specific values
    ///
    /// # Arguments
    /// * `pid` - Process ID
    /// * `uid` - User ID
    /// * `gid` - Group ID
    /// * `cwd` - Current working directory
    ///
    /// # Returns
    /// * `Self` - New context instance
    pub fn with_values(pid: pid_t, uid: uid_t, gid: gid_t, cwd: &str) -> Self {
        Self {
            pid,
            uid,
            gid,
            cwd: cwd.to_string(),
        }
    }
}

impl Default for SyscallContextImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallContext for SyscallContextImpl {
    fn get_pid(&self) -> pid_t {
        self.pid
    }
    
    fn get_uid(&self) -> uid_t {
        self.uid
    }
    
    fn get_gid(&self) -> gid_t {
        self.gid
    }
    
    fn has_permission(&self, operation: &str) -> bool {
        // In a real implementation, this would check permissions
        // For now, we'll just return true for all operations
        true
    }
    
    fn get_cwd(&self) -> &str {
        &self.cwd
    }
}