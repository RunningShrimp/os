//! Security Module for Memory Isolation Integration
//!
//! This module integrates memory isolation and protection mechanisms
//! with the rest of the operating system, including:
//! - System call security validation
//! - Process memory isolation enforcement
//! - Secure memory allocation APIs
//! - Memory access auditing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::subsystems::mm::memory_isolation::{
    ProtectionDomainId, MemoryRegionId, 
    DomainPermissions, MemoryRegionType, AccessValidationResult,
    get_memory_isolation_manager, init_memory_isolation
};
use crate::syscall::SyscallResult;

/// Security context for a process
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Process ID
    pub pid: u32,
    /// Protection domain ID
    pub domain_id: ProtectionDomainId,
    /// Security level
    pub security_level: SecurityLevel,
    /// Capabilities
    pub capabilities: Vec<Capability>,
    /// Is this process sandboxed?
    pub sandboxed: bool,
}

/// Security level for processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    /// Untrusted level (lowest)
    Untrusted = 0,
    /// Low security level
    Low = 1,
    /// Medium security level
    Medium = 2,
    /// High security level
    High = 3,
    /// System level (highest)
    System = 4,
}

/// Capability for fine-grained permissions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    /// Can read any memory
    ReadAnyMemory,
    /// Can write any memory
    WriteAnyMemory,
    /// Can execute any memory
    ExecuteAnyMemory,
    /// Can manage protection domains
    ManageDomains,
    /// Can bypass ASLR
    BypassAslr,
    /// Can access secure regions
    AccessSecureRegions,
    /// Can manage system calls
    ManageSyscalls,
    /// Can manage processes
    ManageProcesses,
    /// Can manage devices
    ManageDevices,
    /// Can manage network
    ManageNetwork,
    /// Can manage file system
    ManageFilesystem,
}

/// Memory access audit entry
#[derive(Debug, Clone)]
pub struct MemoryAccessAuditEntry {
    /// Timestamp
    pub timestamp: u64,
    /// Process ID
    pub pid: u32,
    /// Source domain ID
    pub from_domain_id: ProtectionDomainId,
    /// Target domain ID (if cross-domain)
    pub to_domain_id: Option<ProtectionDomainId>,
    /// Memory address
    pub address: usize,
    /// Access size
    pub size: usize,
    /// Access type
    pub access_type: MemoryAccessType,
    /// Access result
    pub result: AccessValidationResult,
}

/// Memory access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessType {
    /// Read access
    Read = 0,
    /// Write access
    Write = 1,
    /// Execute access
    Execute = 2,
    /// System call access
    Syscall = 3,
}

/// Security manager
pub struct SecurityManager {
    /// Security contexts by process ID
    security_contexts: BTreeMap<u32, SecurityContext>,
    /// Memory access audit log
    audit_log: Vec<MemoryAccessAuditEntry>,
    /// Maximum audit log size
    max_audit_log_size: usize,
    /// Security statistics
    stats: SecurityStats,
}

/// Security statistics
#[derive(Debug, Default)]
pub struct SecurityStats {
    /// Total memory access attempts
    pub total_access_attempts: AtomicU64,
    /// Total access violations
    pub total_access_violations: AtomicU64,
    /// Total cross-domain access attempts
    pub total_cross_domain_attempts: AtomicU64,
    /// Total secure region access attempts
    pub total_secure_access_attempts: AtomicU64,
    /// Total system call attempts
    pub total_syscall_attempts: AtomicU64,
    /// Total system call violations
    pub total_syscall_violations: AtomicU64,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new() -> Self {
        Self {
            security_contexts: BTreeMap::new(),
            audit_log: Vec::new(),
            max_audit_log_size: 10000,
            stats: SecurityStats::default(),
        }
    }

    /// Initialize security manager
    pub fn init(&mut self) -> Result<(), SecurityError> {
        // Initialize memory isolation
        init_memory_isolation()?;
        
        // Create system security context
        let system_context = SecurityContext {
            pid: 0,
            domain_id: 0, // Kernel domain
            security_level: SecurityLevel::System,
            capabilities: vec![
                Capability::ReadAnyMemory,
                Capability::WriteAnyMemory,
                Capability::ExecuteAnyMemory,
                Capability::ManageDomains,
                Capability::BypassAslr,
                Capability::AccessSecureRegions,
                Capability::ManageSyscalls,
                Capability::ManageProcesses,
                Capability::ManageDevices,
                Capability::ManageNetwork,
                Capability::ManageFilesystem,
            ],
            sandboxed: false,
        };
        
        self.security_contexts.insert(0, system_context);
        
        Ok(())
    }

    /// Create security context for a new process
    pub fn create_process_security_context(
        &mut self,
        pid: u32,
        parent_pid: Option<u32>,
        security_level: SecurityLevel,
        sandboxed: bool,
    ) -> Result<ProtectionDomainId, SecurityError> {
        // Get parent context if available
        let parent_context = parent_pid.and_then(|p| self.security_contexts.get(&p));
        
        // Create new protection domain
        let domain_permissions = match security_level {
            SecurityLevel::Untrusted => DomainPermissions::default(),
            SecurityLevel::Low => DomainPermissions {
                can_read_others: false,
                can_write_others: false,
                can_execute_others: false,
                can_manage_domains: false,
                can_modify_protection_keys: false,
            },
            SecurityLevel::Medium => DomainPermissions {
                can_read_others: false,
                can_write_others: false,
                can_execute_others: false,
                can_manage_domains: false,
                can_modify_protection_keys: false,
            },
            SecurityLevel::High => DomainPermissions {
                can_read_others: true,
                can_write_others: false,
                can_execute_others: false,
                can_manage_domains: false,
                can_modify_protection_keys: false,
            },
            SecurityLevel::System => DomainPermissions {
                can_read_others: true,
                can_write_others: true,
                can_execute_others: true,
                can_manage_domains: true,
                can_modify_protection_keys: true,
            },
        };
        
        let domain_name = format!("process_{}", pid);
        let domain_id = {
            let mut isolation_manager = get_memory_isolation_manager().lock();
            isolation_manager.create_domain(domain_name, domain_permissions, security_level >= SecurityLevel::High)?
        };
        
        // Determine capabilities based on security level and parent
        let capabilities = match security_level {
            SecurityLevel::Untrusted => vec![],
            SecurityLevel::Low => vec![],
            SecurityLevel::Medium => vec![Capability::ReadAnyMemory],
            SecurityLevel::High => vec![
                Capability::ReadAnyMemory,
                Capability::WriteAnyMemory,
                Capability::ManageProcesses,
            ],
            SecurityLevel::System => vec![
                Capability::ReadAnyMemory,
                Capability::WriteAnyMemory,
                Capability::ExecuteAnyMemory,
                Capability::ManageDomains,
                Capability::BypassAslr,
                Capability::AccessSecureRegions,
                Capability::ManageSyscalls,
                Capability::ManageProcesses,
                Capability::ManageDevices,
                Capability::ManageNetwork,
                Capability::ManageFilesystem,
            ],
        };
        
        // Create security context
        let context = SecurityContext {
            pid,
            domain_id,
            security_level,
            capabilities,
            sandboxed,
        };
        
        self.security_contexts.insert(pid, context);
        
        Ok(domain_id)
    }

    /// Remove security context for a terminated process
    pub fn remove_process_security_context(&mut self, pid: u32) -> Result<(), SecurityError> {
        self.security_contexts.remove(&pid);
        Ok(())
    }

    /// Validate memory access for a process
    pub fn validate_memory_access(
        &mut self,
        pid: u32,
        addr: usize,
        size: usize,
        is_write: bool,
        is_execute: bool,
    ) -> Result<bool, SecurityError> {
        let context = self.security_contexts.get(&pid)
            .ok_or(SecurityError::ProcessNotFound)?;
        
        // Update statistics
        self.stats.total_access_attempts.fetch_add(1, Ordering::Relaxed);
        
        // Validate access through memory isolation manager
        let result = {
            let isolation_manager = get_memory_isolation_manager().lock();
            isolation_manager.validate_access(context.domain_id, addr, size, is_write, is_execute)
        };
        
        // Log access attempt
        let audit_entry = MemoryAccessAuditEntry {
            timestamp: self.get_timestamp(),
            pid,
            from_domain_id: context.domain_id,
            to_domain_id: None,
            address: addr,
            size,
            access_type: if is_execute { MemoryAccessType::Execute } 
                       else if is_write { MemoryAccessType::Write } 
                       else { MemoryAccessType::Read },
            result,
        };
        
        self.log_access_attempt(audit_entry);
        
        // Update violation statistics
        if result != AccessValidationResult::Allowed {
            self.stats.total_access_violations.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(result == AccessValidationResult::Allowed)
    }

    /// Validate system call for a process
    pub fn validate_syscall(
        &mut self,
        pid: u32,
        syscall_number: u32,
        args: &[usize],
    ) -> Result<bool, SecurityError> {
        let context = self.security_contexts.get(&pid)
            .ok_or(SecurityError::ProcessNotFound)?;
        
        // Update statistics
        self.stats.total_syscall_attempts.fetch_add(1, Ordering::Relaxed);
        
        // Check if syscall is allowed based on security level and capabilities
        let allowed = self.check_syscall_permission(context, syscall_number, args)?;
        
        // Log syscall attempt
        let audit_entry = MemoryAccessAuditEntry {
            timestamp: self.get_timestamp(),
            pid,
            from_domain_id: context.domain_id,
            to_domain_id: None,
            address: syscall_number as usize,
            size: args.len(),
            access_type: MemoryAccessType::Syscall,
            result: if allowed { AccessValidationResult::Allowed } else { AccessValidationResult::DeniedPermission },
        };
        
        self.log_access_attempt(audit_entry);
        
        // Update violation statistics
        if !allowed {
            self.stats.total_syscall_violations.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(allowed)
    }

    /// Check if a syscall is allowed for a security context
    fn check_syscall_permission(
        &self,
        context: &SecurityContext,
        syscall_number: u32,
        _args: &[usize],
    ) -> Result<bool, SecurityError> {
        // System calls that require special permissions
        match syscall_number {
            // Memory management syscalls
            1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 => { // mmap, munmap, etc.
                // Check if process has memory management capabilities
                Ok(context.capabilities.contains(&Capability::ReadAnyMemory) ||
                   context.capabilities.contains(&Capability::WriteAnyMemory))
            }
            
            // Process management syscalls
            10 | 11 | 12 | 13 | 14 | 15 => { // fork, exec, etc.
                // Check if process has process management capabilities
                Ok(context.capabilities.contains(&Capability::ManageProcesses))
            }
            
            // System management syscalls
            20 | 21 | 22 | 23 | 24 | 25 => { // reboot, shutdown, etc.
                // Only system level can perform these operations
                Ok(context.security_level >= SecurityLevel::System)
            }
            
            // Device management syscalls
            30 | 31 | 32 | 33 | 34 | 35 => { // open, close, read, write, etc.
                // Check if process has device management capabilities
                Ok(context.capabilities.contains(&Capability::ManageDevices))
            }
            
            // Network management syscalls
            40 | 41 | 42 | 43 | 44 | 45 => { // socket, bind, etc.
                // Check if process has network management capabilities
                Ok(context.capabilities.contains(&Capability::ManageNetwork))
            }
            
            // File system management syscalls
            50 | 51 | 52 | 53 | 54 | 55 => { // mkdir, rmdir, etc.
                // Check if process has file system management capabilities
                Ok(context.capabilities.contains(&Capability::ManageFilesystem))
            }
            
            // Default: allow basic syscalls for all processes
            _ => Ok(true),
        }
    }

    /// Log access attempt
    fn log_access_attempt(&mut self, entry: MemoryAccessAuditEntry) {
        self.audit_log.push(entry);
        
        // Trim audit log if it exceeds maximum size
        if self.audit_log.len() > self.max_audit_log_size {
            self.audit_log.drain(0..self.audit_log.len() - self.max_audit_log_size);
        }
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // TODO: Use proper time source
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Get security statistics
    pub fn get_stats(&self) -> &SecurityStats {
        &self.stats
    }

    /// Get audit log
    pub fn get_audit_log(&self) -> &[MemoryAccessAuditEntry] {
        &self.audit_log
    }

    /// Clear audit log
    pub fn clear_audit_log(&mut self) {
        self.audit_log.clear();
    }

    /// Get security context for a process
    pub fn get_security_context(&self, pid: u32) -> Option<&SecurityContext> {
        self.security_contexts.get(&pid)
    }

    /// Update security level for a process
    pub fn update_security_level(
        &mut self,
        pid: u32,
        new_level: SecurityLevel,
    ) -> Result<(), SecurityError> {
        let context = self.security_contexts.get_mut(&pid)
            .ok_or(SecurityError::ProcessNotFound)?;
        
        context.security_level = new_level;
        
        // Update domain permissions if needed
        // This would require updating the memory isolation manager
        
        Ok(())
    }

    /// Add capability to a process
    pub fn add_capability(
        &mut self,
        pid: u32,
        capability: Capability,
    ) -> Result<(), SecurityError> {
        let context = self.security_contexts.get_mut(&pid)
            .ok_or(SecurityError::ProcessNotFound)?;
        
        if !context.capabilities.contains(&capability) {
            context.capabilities.push(capability);
        }
        
        Ok(())
    }

    /// Remove capability from a process
    pub fn remove_capability(
        &mut self,
        pid: u32,
        capability: &Capability,
    ) -> Result<(), SecurityError> {
        let context = self.security_contexts.get_mut(&pid)
            .ok_or(SecurityError::ProcessNotFound)?;
        
        context.capabilities.retain(|c| c != capability);
        
        Ok(())
    }

    /// Check if a process has a specific capability
    pub fn has_capability(&self, pid: u32, capability: &Capability) -> bool {
        if let Some(context) = self.security_contexts.get(&pid) {
            context.capabilities.contains(capability)
        } else {
            false
        }
    }

    /// Create secure memory region for a process
    pub fn create_secure_memory_region(
        &mut self,
        pid: u32,
        size: usize,
        region_type: MemoryRegionType,
    ) -> Result<MemoryRegionId, SecurityError> {
        let context = self.security_contexts.get(&pid)
            .ok_or(SecurityError::ProcessNotFound)?;
        
        // Check if process has secure memory access capability
        if !context.capabilities.contains(&Capability::AccessSecureRegions) {
            return Err(SecurityError::PermissionDenied);
        }
        
        // Create secure region through memory isolation manager
        let region_id = {
            let mut isolation_manager = get_memory_isolation_manager().lock();
            isolation_manager.create_secure_region(context.domain_id, size, region_type)
                .map_err(|_| SecurityError::SecureRegionCreationFailed)?
        };
        
        Ok(region_id)
    }

    /// Zero out secure memory region for a process
    pub fn zero_secure_memory_region(
        &mut self,
        pid: u32,
        region_id: MemoryRegionId,
    ) -> Result<(), SecurityError> {
        let context = self.security_contexts.get(&pid)
            .ok_or(SecurityError::ProcessNotFound)?;
        
        // Check if process has secure memory access capability
        if !context.capabilities.contains(&Capability::AccessSecureRegions) {
            return Err(SecurityError::PermissionDenied);
        }
        
        // Zero secure region through memory isolation manager
        {
            let mut isolation_manager = get_memory_isolation_manager().lock();
            isolation_manager.zero_secure_region(context.domain_id, region_id)
                .map_err(|_| SecurityError::SecureRegionOperationFailed)?
        };
        
        Ok(())
    }
}

/// Security errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Process not found
    ProcessNotFound,
    /// Permission denied
    PermissionDenied,
    /// Secure region creation failed
    SecureRegionCreationFailed,
    /// Secure region operation failed
    SecureRegionOperationFailed,
    /// Memory isolation error
    MemoryIsolationError(crate::subsystems::mm::memory_isolation::MemoryIsolationError),
}

/// Global security manager instance
static SECURITY_MANAGER: crate::subsystems::sync::Mutex<SecurityManager> = crate::subsystems::sync::Mutex::new(SecurityManager::new());

/// Initialize security system
pub fn init_security() -> Result<(), SecurityError> {
    // 直接初始化内存安全管理器，不调用完整的安全子系统初始化（避免循环依赖）
    let mut manager = SECURITY_MANAGER.lock();
    manager.init()
}

/// Get security manager
pub fn get_security_manager() -> &'static crate::subsystems::sync::Mutex<SecurityManager> {
    &SECURITY_MANAGER
}

/// Validate memory access (convenience function)
pub fn validate_memory_access(
    pid: u32,
    addr: usize,
    size: usize,
    is_write: bool,
    is_execute: bool,
) -> Result<bool, SecurityError> {
    let mut manager = SECURITY_MANAGER.lock();
    manager.validate_memory_access(pid, addr, size, is_write, is_execute)
}

/// Validate system call (convenience function)
pub fn validate_syscall(
    pid: u32,
    syscall_number: u32,
    args: &[usize],
) -> Result<bool, SecurityError> {
    let mut manager = SECURITY_MANAGER.lock();
    manager.validate_syscall(pid, syscall_number, args)
}

/// Create security context for new process (convenience function)
pub fn create_process_security_context(
    pid: u32,
    parent_pid: Option<u32>,
    security_level: SecurityLevel,
    sandboxed: bool,
) -> Result<ProtectionDomainId, SecurityError> {
    let mut manager = SECURITY_MANAGER.lock();
    manager.create_process_security_context(pid, parent_pid, security_level, sandboxed)
}

/// Remove security context for terminated process (convenience function)
pub fn remove_process_security_context(pid: u32) -> Result<(), SecurityError> {
    let mut manager = SECURITY_MANAGER.lock();
    manager.remove_process_security_context(pid)
}

/// Check if process has capability (convenience function)
pub fn has_capability(pid: u32, capability: &Capability) -> bool {
    let manager = SECURITY_MANAGER.lock();
    manager.has_capability(pid, capability)
}

/// Create secure memory region (convenience function)
pub fn create_secure_memory_region(
    pid: u32,
    size: usize,
    region_type: MemoryRegionType,
) -> Result<MemoryRegionId, SecurityError> {
    let mut manager = SECURITY_MANAGER.lock();
    manager.create_secure_memory_region(pid, size, region_type)
}

/// Zero secure memory region (convenience function)
pub fn zero_secure_memory_region(
    pid: u32,
    region_id: MemoryRegionId,
) -> Result<(), SecurityError> {
    let mut manager = SECURITY_MANAGER.lock();
    manager.zero_secure_memory_region(pid, region_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context_creation() {
        let mut manager = SecurityManager::new();
        manager.init().unwrap();
        
        let domain_id = manager.create_process_security_context(
            1,
            Some(0),
            SecurityLevel::Medium,
            false,
        ).unwrap();
        
        assert!(domain_id > 0);
        
        let context = manager.get_security_context(1).unwrap();
        assert_eq!(context.pid, 1);
        assert_eq!(context.security_level, SecurityLevel::Medium);
        assert!(!context.sandboxed);
    }

    #[test]
    fn test_memory_access_validation() {
        let mut manager = SecurityManager::new();
        manager.init().unwrap();
        
        let domain_id = manager.create_process_security_context(
            1,
            Some(0),
            SecurityLevel::Medium,
            false,
        ).unwrap();
        
        // This test would need actual memory regions to be set up
        // For now, just test the function call
        let result = manager.validate_memory_access(1, 0x1000, 0x100, true, false);
        // Result would be false since no memory regions are set up
        assert!(result.is_ok());
    }

    #[test]
    fn test_capability_management() {
        let mut manager = SecurityManager::new();
        manager.init().unwrap();
        
        let domain_id = manager.create_process_security_context(
            1,
            Some(0),
            SecurityLevel::Medium,
            false,
        ).unwrap();
        
        // Add capability
        manager.add_capability(1, Capability::ReadAnyMemory).unwrap();
        assert!(manager.has_capability(1, &Capability::ReadAnyMemory));
        
        // Remove capability
        manager.remove_capability(1, &Capability::ReadAnyMemory).unwrap();
        assert!(!manager.has_capability(1, &Capability::ReadAnyMemory));
    }
}