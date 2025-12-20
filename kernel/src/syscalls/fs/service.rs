//! Filesystem System Call Service Implementation
//!
//! This module provides the filesystem service that manages all
//! filesystem-related system calls through the new modular service
//! architecture.

use alloc::{boxed::Box, string::String, vec::Vec};

use super::{
    handlers,
    types::{FilesystemError, FilesystemOperation},
};
use crate::error_handling::unified::KernelError;
// FilesystemRequest and FilesystemResponse are defined inline to avoid circular dependencies
use crate::syscalls::services::{Service, ServiceStatus, SyscallService};

/// Filesystem system call service
///
/// Implements SyscallService trait to provide filesystem operations handling
/// in the new modular service architecture.
pub struct FilesystemService {
    /// Service name
    name: String,
    /// Service version
    version: String,
    /// Service description
    description: String,
    /// Current service status
    status: ServiceStatus,
    /// Supported syscall numbers
    supported_syscalls: Vec<u32>,
    /// Filesystem statistics
    stats: FilesystemStats,
}

impl FilesystemService {
    /// Create a new filesystem service instance
    pub fn new() -> Self {
        Self {
            name: String::from("filesystem"),
            version: String::from("1.0.0"),
            description: String::from("Filesystem syscall service for managing file operations"),
            status: ServiceStatus::Uninitialized,
            supported_syscalls: Vec::new(), // TODO: Implement get_supported_syscalls
            stats: FilesystemStats::default(),
        }
    }

    /// Get filesystem statistics
    pub fn get_stats(&self) -> &FilesystemStats {
        &self.stats
    }

    /// Update statistics for an operation
    fn update_stats(&mut self, operation: FilesystemOperation) {
        match operation {
            FilesystemOperation::Stat | FilesystemOperation::Lstat => {
                self.stats.stat_calls += 1;
            },
            FilesystemOperation::Access => {
                self.stats.access_calls += 1;
            },
            FilesystemOperation::ChangeDirectory => {
                self.stats.chdir_calls += 1;
            },
            FilesystemOperation::GetCurrentDirectory => {
                self.stats.getcwd_calls += 1;
            },
            FilesystemOperation::MakeDirectory => {
                self.stats.mkdir_calls += 1;
            },
            FilesystemOperation::RemoveDirectory => {
                self.stats.rmdir_calls += 1;
            },
            FilesystemOperation::Unlink => {
                self.stats.unlink_calls += 1;
            },
            FilesystemOperation::Rename => {
                self.stats.rename_calls += 1;
            },
            FilesystemOperation::Link => {
                self.stats.link_calls += 1;
            },
            FilesystemOperation::Symlink => {
                self.stats.symlink_calls += 1;
            },
            FilesystemOperation::Readlink => {
                self.stats.readlink_calls += 1;
            },
            FilesystemOperation::ChangeMode => {
                self.stats.chmod_calls += 1;
            },
            FilesystemOperation::ChangeOwner => {
                self.stats.chown_calls += 1;
            },
            FilesystemOperation::SetUmask => {
                self.stats.umask_calls += 1;
            },
            _ => self.stats.other_calls += 1,
        }
        self.stats.total_calls += 1;
    }

    /// Reset filesystem statistics
    pub fn reset_stats(&mut self) {
        // println removed for no_std compatibility
    }

    /// Get the current working directory of a process
    pub fn get_process_cwd(&self, pid: u32) -> Option<String> {
        // Access process table to get current working directory
        // TODO: Implement proper process table access
        // println removed for no_std compatibility
        Some(String::from("/")) // Default to root directory for now
    }

    /// Validate filesystem path
    pub fn validate_path(&self, path: &str) -> Result<(), FilesystemError> {
        if path.is_empty() {
            // println removed for no_std compatibility
        }

        if path.contains('\0') {
            // println removed for no_std compatibility
        }

        // Check for directory traversal attempts
        if path.contains("../") {
            // println removed for no_std compatibility
        }

        // Check path length limits (POSIX PATH_MAX is typically 4096)
        if path.len() > 4096 {
            // println removed for no_std compatibility
        }

        Ok(())
    }

    /// Normalize filesystem path
    pub fn normalize_path(&self, path: &str, cwd: Option<&str>) -> Result<String, FilesystemError> {
        self.validate_path(path)?;

        let abs_path = if path.starts_with('/') {
            path.to_string()
        } else if let Some(cwd) = cwd {
            format!("{}/{}", cwd, path)
        } else {
            format!("/{}", path)
        };

        // Normalize path components
        let mut components = Vec::new();
        // println removed for no_std compatibility
        for component in abs_path.split('/') {
            match component {
                "" | "." => continue,
                ".." => {
                    if !components.is_empty() {
                        components.pop();
                        // println removed for no_std compatibility
                    }
                },
                _ => components.push(component.to_string()),
            }
        }

        if abs_path.starts_with('/') {
            Ok(format!("/{}", components.join("/")))
        } else {
            Ok(components.join("/"))
        }
    }
}

impl Default for FilesystemService {
    fn default() -> Self {
        Self::new()
    }
}

impl Service for FilesystemService {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn initialize(&mut self) -> Result<(), KernelError> {
        // println removed for no_std compatibility
        self.status = ServiceStatus::Initializing;

        // Initialize VFS if needed
        if !crate::vfs::is_root_mounted() {
            // println removed for no_std compatibility
        }

        self.status = ServiceStatus::Initialized;
        // println removed for no_std compatibility
        Ok(())
    }

    fn start(&mut self) -> Result<(), KernelError> {
        // println removed for no_std compatibility
        self.status = ServiceStatus::Starting;

        // Perform any startup operations (caching, etc.)
        // TODO: Initialize filesystem caches, register VFS operations

        self.status = ServiceStatus::Running;
        // println removed for no_std compatibility
        Ok(())
    }

    fn stop(&mut self) -> Result<(), KernelError> {
        // println removed for no_std compatibility
        self.status = ServiceStatus::Stopping;

        // Perform cleanup operations
        // TODO: Flush caches, clean up temporary files

        self.status = ServiceStatus::Stopped;
        // println removed for no_std compatibility
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), KernelError> {
        // println removed for no_std compatibility

        // Perform final cleanup
        // println removed for no_std compatibility

        self.status = ServiceStatus::Uninitialized;
        // println removed for no_std compatibility
        Ok(())
    }

    fn status(&self) -> ServiceStatus {
        self.status
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["process", "vfs", "memory"]
    }
}

impl SyscallService for FilesystemService {
    fn supported_syscalls(&self) -> Vec<u32> {
        self.supported_syscalls.clone()
    }

    fn handle_syscall(&mut self, syscall_number: u32, args: &[u64]) -> Result<u64, KernelError> {
        // println removed for no_std compatibility

        // Update statistics
        use super::types::FilesystemOperation;
        let operation = match syscall_number {
            0x7000 => FilesystemOperation::ChangeDirectory,
            0x7001 => FilesystemOperation::ChangeDirectory, // fchdir
            0x7002 => FilesystemOperation::GetCurrentDirectory,
            0x7003 => FilesystemOperation::MakeDirectory,
            0x7004 => FilesystemOperation::RemoveDirectory,
            0x7005 => FilesystemOperation::Unlink,
            0x7006 => FilesystemOperation::Rename,
            0x7007 => FilesystemOperation::Link,
            0x7008 => FilesystemOperation::Symlink,
            0x7009 => FilesystemOperation::Readlink,
            0x700A => FilesystemOperation::ChangeMode,
            0x700B => FilesystemOperation::ChangeMode, // fchmod
            0x700C => FilesystemOperation::ChangeOwner,
            0x700D => FilesystemOperation::ChangeOwner, // fchown
            0x700F => FilesystemOperation::SetUmask,
            0x7010 => FilesystemOperation::Stat,
            0x7011 => FilesystemOperation::Lstat,
            0x7012 => FilesystemOperation::Access,
            _ => FilesystemOperation::Mount, // Default for unsupported
        };
        // println removed for no_std compatibility

        // Dispatch to appropriate handler
        handlers::dispatch_syscall(syscall_number, args)
    }

    fn priority(&self) -> u32 {
        20 // Filesystem operations are moderately critical
    }
}

/// Filesystem operation counters for statistics
#[derive(Debug, Clone, Default)]
pub struct FilesystemStats {
    /// Total number of filesystem syscalls handled
    pub total_calls: u64,
    /// Number of stat/lstat calls
    pub stat_calls: u64,
    /// Number of access calls
    pub access_calls: u64,
    /// Number of chdir calls
    pub chdir_calls: u64,
    /// Number of getcwd calls
    pub getcwd_calls: u64,
    /// Number of mkdir calls
    pub mkdir_calls: u64,
    /// Number of rmdir calls
    pub rmdir_calls: u64,
    /// Number of unlink calls
    pub unlink_calls: u64,
    /// Number of rename calls
    pub rename_calls: u64,
    /// Number of link calls
    pub link_calls: u64,
    /// Number of symlink calls
    pub symlink_calls: u64,
    /// Number of readlink calls
    pub readlink_calls: u64,
    /// Number of chmod calls
    pub chmod_calls: u64,
    /// Number of chown calls
    pub chown_calls: u64,
    /// Number of umask calls
    pub umask_calls: u64,
    /// Number of other calls
    pub other_calls: u64,
}

/// Filesystem service factory
///
/// Factory for creating filesystem service instances
pub struct FilesystemServiceFactory;

impl FilesystemServiceFactory {
    /// Create a new filesystem service instance
    pub fn create() -> Box<dyn SyscallService> {
        Box::new(FilesystemService::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_service_creation() {
        // println removed for no_std compatibility
        assert_eq!(service.name(), "filesystem");
        assert_eq!(service.version(), "1.0.0");
        assert_eq!(service.status(), ServiceStatus::Uninitialized);
        assert!(!service.supported_syscalls().is_empty());
    }

    #[test]
    fn test_filesystem_service_lifecycle() {
        // println removed for no_std compatibility

        // Test initialization
        assert!(service.initialize().is_ok());
        assert_eq!(service.status(), ServiceStatus::Initialized);

        // Test startup
        assert!(service.start().is_ok());
        assert_eq!(service.status(), ServiceStatus::Running);

        // Test shutdown
        assert!(service.stop().is_ok());
        assert_eq!(service.status(), ServiceStatus::Stopped);
    }

    #[test]
    fn test_path_validation() {
        // println removed for no_std compatibility

        // Valid paths
        assert!(service.validate_path("/tmp/test").is_ok());
        assert!(service.validate_path("relative/path").is_ok());

        // Invalid paths
        assert!(service.validate_path("").is_err());
        assert!(service.validate_path("path\0with\0null").is_err());
        assert!(service.validate_path(&"a".repeat(5000)).is_err());
    }

    #[test]
    fn test_path_normalization() {
        // println removed for no_std compatibility

        assert_eq!(service.normalize_path("/tmp/../usr", None).unwrap(), "/usr");
        assert_eq!(service.normalize_path("./test", Some("/tmp")).unwrap(), "/tmp/test");
        assert_eq!(service.normalize_path("/a/b//c", None).unwrap(), "/a/b/c");
    }

    #[test]
    fn test_statistics() {
        // println removed for no_std compatibility

        // Initially all zero
        assert_eq!(service.get_stats().total_calls, 0);

        // Update stats
        // println removed for no_std compatibility
        assert_eq!(service.get_stats().stat_calls, 1);
        assert_eq!(service.get_stats().total_calls, 1);

        // Reset stats
        // println removed for no_std compatibility
        assert_eq!(service.get_stats().total_calls, 0);
    }
}
