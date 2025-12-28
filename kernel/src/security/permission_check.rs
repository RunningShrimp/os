//! Unified Permission Checking Framework
//!
//! This module provides a unified interface for permission checking across
//! all security subsystems (ACL, Capabilities, SELinux, Seccomp, etc.)

extern crate alloc;
use alloc::string::ToString;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};



/// Types of resources that can be checked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    File,
    Directory,
    Socket,
    Process,
    Memory,
    Device,
    SystemCall,
    Network,
}

/// Requested operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Read,
    Write,
    Execute,
    Create,
    Delete,
    Modify,
    Access,
    Bind,
    Listen,
    Connect,
    Accept,
    Send,
    Receive,
    Map,
    Unmap,
    Allocate,
    Deallocate,
}

/// Permission check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionResult {
    /// Access granted
    Granted,
    /// Access denied
    Denied,
    /// Access granted but requires audit
    GrantedAudit,
    /// Access denied due to escalation attempt
    DeniedEscalation,
    /// Access denied due to security policy
    DeniedPolicy,
}

/// Unified permission check request
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Process ID requesting access
    pub pid: u64,
    /// User ID
    pub uid: u64,
    /// Group IDs
    pub gids: Vec<u64>,
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource identifier (path, fd, etc.)
    pub resource_id: String,
    /// Requested operation
    pub operation: Operation,
    /// Additional context
    pub context: PermissionContext,
}

/// Permission check context
#[derive(Debug, Clone, Default)]
pub struct PermissionContext {
    /// Whether the process is privileged
    pub privileged: bool,
    /// Capabilities (if applicable)
    pub capabilities: Vec<String>,
    /// SELinux context (if applicable)
    pub selinux_context: Option<String>,
    /// Additional metadata
    pub metadata: alloc::collections::BTreeMap<String, String>,
}

/// Unified permission checker
pub struct UnifiedPermissionChecker {
    /// Whether ACL checking is enabled
    acl_enabled: AtomicBool,
    /// Whether Capabilities checking is enabled
    capabilities_enabled: AtomicBool,
    /// Whether SELinux checking is enabled
    selinux_enabled: AtomicBool,
    /// Whether Seccomp checking is enabled
    seccomp_enabled: AtomicBool,
    /// Whether strict mode is enabled (all checks must pass)
    strict_mode: AtomicBool,
}

impl UnifiedPermissionChecker {
    /// Create a new unified permission checker
    pub fn new() -> Self {
        Self {
            acl_enabled: AtomicBool::new(true),
            capabilities_enabled: AtomicBool::new(true),
            selinux_enabled: AtomicBool::new(false), // SELinux disabled by default
            seccomp_enabled: AtomicBool::new(true),
            strict_mode: AtomicBool::new(false),
        }
    }

    /// Check permissions using unified framework
    pub fn check_permission(&self, request: &PermissionRequest) -> PermissionResult {
        // Check Seccomp first (system call filtering)
        if self.seccomp_enabled.load(Ordering::Relaxed) {
            if let Some(result) = self.check_seccomp(request) {
                return result;
            }
        }

        // Check SELinux if enabled
        if self.selinux_enabled.load(Ordering::Relaxed) {
            if let Some(result) = self.check_selinux(request) {
                return result;
            }
        }

        // Check Capabilities
        if self.capabilities_enabled.load(Ordering::Relaxed) {
            if let Some(result) = self.check_capabilities(request) {
                return result;
            }
        }

        // Check ACL (traditional Unix permissions + extended ACLs)
        if self.acl_enabled.load(Ordering::Relaxed) {
            return self.check_acl(request);
        }

        // Default: grant access if no security subsystem denies it
        PermissionResult::Granted
    }

    /// Check Seccomp filters
    fn check_seccomp(&self, request: &PermissionRequest) -> Option<PermissionResult> {
        // Only check for system call operations
        if request.resource_type != ResourceType::SystemCall {
            return None;
        }

        // Use seccomp module to check
        // Note: This is a placeholder - actual implementation would use seccomp subsystem
        // For now, default to granted if seccomp is not actively blocking
        
        // Convert operation to syscall number (simplified)
        let _syscall_num = self.operation_to_syscall(request.operation);
        
        // TODO: Integrate with actual seccomp subsystem
        // For now, return None to skip seccomp check
        None
    }

    /// Check SELinux policy
    fn check_selinux(&self, request: &PermissionRequest) -> Option<PermissionResult> {
        // Get SELinux context from request
        let _target_context = request.context.selinux_context.as_ref()?;
        
        // Convert operation to SELinux permission
        let _permission = self.operation_to_selinux_permission(request.operation);
        let _object_class = self.resource_type_to_selinux_class(request.resource_type);
        
        // TODO: Integrate with actual SELinux subsystem
        // For now, return None to skip SELinux check if not configured
        None
    }

    /// Check Capabilities
    fn check_capabilities(&self, request: &PermissionRequest) -> Option<PermissionResult> {
        // Check if operation requires specific capability
        if let Some(required_cap) = self.operation_to_capability(request.operation, request.resource_type) {
            // TODO: Integrate with actual capabilities subsystem
            // For now, check if process is privileged
            if request.context.privileged {
                Some(PermissionResult::Granted)
            } else {
                Some(PermissionResult::DeniedEscalation)
            }
        } else {
            None // No capability required for this operation
        }
    }

    /// Check ACL (traditional + extended)
    fn check_acl(&self, request: &PermissionRequest) -> PermissionResult {
        use crate::security::acl::{AccessRequest, ResourceType as AclResourceType};
        
        // Convert to ACL request format
        let acl_request = AccessRequest {
            pid: request.pid as u64,
            uid: request.uid as u32,
            gids: request.gids.iter().map(|&gid| gid as u32).collect(),
            resource_type: match request.resource_type {
                ResourceType::File => AclResourceType::File,
                ResourceType::Directory => AclResourceType::Directory,
                ResourceType::Socket => AclResourceType::Socket,
                ResourceType::Process => AclResourceType::Process,
                ResourceType::Memory => AclResourceType::Memory,
                ResourceType::Device => AclResourceType::Device,
                ResourceType::SystemCall => AclResourceType::SystemCall,
                ResourceType::Network => AclResourceType::Network,
            },
            // Try to interpret resource_id as a numeric identifier (inode/file descriptor)
            resource_id: match request.resource_id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => 0,
            },
            requested_permissions: self.operation_to_acl_permissions(request.operation),
                context: crate::security::acl::AccessContext {
                    operation: format!("{:?}", request.operation),
                    // If resource looks like a path and resource type is file/dir, include it
                    path: if matches!(request.resource_type, ResourceType::File | ResourceType::Directory) && !request.resource_id.is_empty() {
                        Some(request.resource_id.clone())
                    } else {
                        None
                    },
                    flags: 0, // future: map additional flags from request
                    privileged: request.context.privileged,
                },
        };

        // Use ACL subsystem to check
        let guard = crate::security::ACL.lock();
        if let Some(ref acl_subsystem) = *guard {
            match acl_subsystem.check_access(&acl_request) {
                crate::security::acl::AccessDecision::Granted => PermissionResult::Granted,
                crate::security::acl::AccessDecision::Denied => PermissionResult::Denied,
                crate::security::acl::AccessDecision::GrantedAudit => PermissionResult::GrantedAudit,
                crate::security::acl::AccessDecision::DeniedEscalation => PermissionResult::DeniedEscalation,
            }
        } else {
            // No ACL subsystem, default to granted for backwards compatibility
            PermissionResult::Granted
        }
    }

    /// Convert operation to syscall number (simplified mapping)
    fn operation_to_syscall(&self, op: Operation) -> u64 {
        match op {
            Operation::Read => 0, // SYS_READ
            Operation::Write => 1, // SYS_WRITE
            Operation::Execute => 59, // SYS_EXECVE
            Operation::Create => 85, // SYS_CREAT
            Operation::Delete => 87, // SYS_UNLINK
            Operation::Modify => 90, // SYS_CHMOD
            Operation::Access => 21, // SYS_ACCESS
            Operation::Bind => 49, // SYS_BIND
            Operation::Listen => 50, // SYS_LISTEN
            Operation::Connect => 42, // SYS_CONNECT
            Operation::Accept => 43, // SYS_ACCEPT
            Operation::Send => 44, // SYS_SENDTO
            Operation::Receive => 45, // SYS_RECVFROM
            Operation::Map => 9, // SYS_MMAP
            Operation::Unmap => 11, // SYS_MUNMAP
            Operation::Allocate => 9, // SYS_MMAP
            Operation::Deallocate => 11, // SYS_MUNMAP
        }
    }

    /// Convert operation to SELinux permission
    fn operation_to_selinux_permission(&self, op: Operation) -> String {
        match op {
            Operation::Read => "read".to_string(),
            Operation::Write => "write".to_string(),
            Operation::Execute => "execute".to_string(),
            Operation::Create => "create".to_string(),
            Operation::Delete => "unlink".to_string(),
            Operation::Modify => "setattr".to_string(),
            Operation::Access => "getattr".to_string(),
            Operation::Bind => "bind".to_string(),
            Operation::Listen => "listen".to_string(),
            Operation::Connect => "connectto".to_string(),
            Operation::Accept => "accept".to_string(),
            Operation::Send => "sendto".to_string(),
            Operation::Receive => "recvfrom".to_string(),
            Operation::Map => "map".to_string(),
            Operation::Unmap => "unmap".to_string(),
            Operation::Allocate => "allocate".to_string(),
            Operation::Deallocate => "deallocate".to_string(),
        }
    }

    /// Convert resource type to SELinux object class
    fn resource_type_to_selinux_class(&self, rt: ResourceType) -> String {
        match rt {
            ResourceType::File => "file".to_string(),
            ResourceType::Directory => "dir".to_string(),
            ResourceType::Socket => "socket".to_string(),
            ResourceType::Process => "process".to_string(),
            ResourceType::Memory => "mem".to_string(),
            ResourceType::Device => "chr_file".to_string(),
            ResourceType::SystemCall => "system".to_string(),
            ResourceType::Network => "netif".to_string(),
        }
    }

    /// Convert operation to required capability
    fn operation_to_capability(&self, op: Operation, rt: ResourceType) -> Option<String> {
        match (op, rt) {
            (Operation::Modify, ResourceType::File) if op == Operation::Modify => {
                Some("CAP_FOWNER".to_string())
            }
            (Operation::Delete, _) => Some("CAP_DAC_OVERRIDE".to_string()),
            (Operation::Execute, ResourceType::Process) => Some("CAP_SYS_ADMIN".to_string()),
            (Operation::Bind, ResourceType::Socket) => Some("CAP_NET_BIND_SERVICE".to_string()),
            (Operation::Map, ResourceType::Memory) => Some("CAP_SYS_ADMIN".to_string()),
            _ => None,
        }
    }

    /// Convert operation to ACL permissions
    fn operation_to_acl_permissions(&self, op: Operation) -> crate::security::acl::AclPermissions {
        use crate::security::acl::AclPermissions;
        
        match op {
            Operation::Read => AclPermissions {
                read: true,
                ..Default::default()
            },
            Operation::Write => AclPermissions {
                write: true,
                ..Default::default()
            },
            Operation::Execute => AclPermissions {
                execute: true,
                ..Default::default()
            },
            Operation::Create => AclPermissions {
                write: true,
                ..Default::default()
            },
            Operation::Delete => AclPermissions {
                delete: true,
                ..Default::default()
            },
            Operation::Modify => AclPermissions {
                write: true,
                write_attributes: true,
                ..Default::default()
            },
            Operation::Access => AclPermissions {
                read_attributes: true,
                ..Default::default()
            },
            _ => AclPermissions::default(),
        }
    }

    /// Enable/disable ACL checking
    pub fn set_acl_enabled(&self, enabled: bool) {
        self.acl_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Enable/disable Capabilities checking
    pub fn set_capabilities_enabled(&self, enabled: bool) {
        self.capabilities_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Enable/disable SELinux checking
    pub fn set_selinux_enabled(&self, enabled: bool) {
        self.selinux_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Enable/disable Seccomp checking
    pub fn set_seccomp_enabled(&self, enabled: bool) {
        self.seccomp_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Enable/disable strict mode
    pub fn set_strict_mode(&self, enabled: bool) {
        self.strict_mode.store(enabled, Ordering::Relaxed);
    }
}

/// Global unified permission checker instance
static UNIFIED_CHECKER: spin::Mutex<Option<UnifiedPermissionChecker>> = spin::Mutex::new(None);

/// Initialize unified permission checker
pub fn init_unified_permission_checker() {
    *UNIFIED_CHECKER.lock() = Some(UnifiedPermissionChecker::new());
}

/// Check permission using unified framework
pub fn check_permission(request: &PermissionRequest) -> PermissionResult {
    let guard = UNIFIED_CHECKER.lock();
    guard.as_ref()
        .map(|checker| checker.check_permission(request))
        .unwrap_or(PermissionResult::Granted) // Default: grant if not initialized
}

/// Get unified permission checker instance
pub fn get_unified_checker() -> Option<spin::MutexGuard<'static, Option<UnifiedPermissionChecker>>> {
    Some(UNIFIED_CHECKER.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_checker_creation() {
        let checker = UnifiedPermissionChecker::new();
        assert!(checker.acl_enabled.load(Ordering::Relaxed));
        assert!(checker.capabilities_enabled.load(Ordering::Relaxed));
        assert!(!checker.selinux_enabled.load(Ordering::Relaxed));
    }

    #[test]
    fn test_permission_request() {
        let request = PermissionRequest {
            pid: 1234,
            uid: 1000,
            gids: vec![1000, 1001],
            resource_type: ResourceType::File,
            resource_id: "/tmp/test".to_string(),
            operation: Operation::Read,
            context: PermissionContext::default(),
        };

        assert_eq!(request.pid, 1234);
        assert_eq!(request.operation, Operation::Read);
    }
}

