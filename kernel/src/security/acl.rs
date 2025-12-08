// Access Control List (ACL) implementation
//
// This module provides fine-grained access control for system resources including
// files, devices, network sockets, and IPC objects. ACLs allow for more detailed
// permission management beyond traditional Unix permissions.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::types::stubs::*;
use crate::vfs::{FileMode};
use crate::reliability::errno::{EACCES, EPERM};

/// ACL entry types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AclType {
    /// User-specific entry
    User,
    /// Group-specific entry
    Group,
    /// Other entry
    Other,
    /// Mask entry (maximum permissions)
    Mask,
}

/// ACL permission flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AclPermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
    /// Append permission (for files)
    pub append: bool,
    /// Delete permission
    pub delete: bool,
    /// Read attributes permission
    pub read_attributes: bool,
    /// Write attributes permission
    pub write_attributes: bool,
    /// Read ACL permission
    pub read_acl: bool,
    /// Write ACL permission
    pub write_acl: bool,
    /// Synchronize permission
    pub synchronize: bool,
}

impl Default for AclPermissions {
    fn default() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
            append: false,
            delete: false,
            read_attributes: true,
            write_attributes: false,
            read_acl: true,
            write_acl: false,
            synchronize: true,
        }
    }
}

/// ACL entry
#[derive(Debug, Clone)]
pub struct AclEntry {
    /// Entry type
    pub acl_type: AclType,
    /// User ID (for User type) or Group ID (for Group type)
    pub id: u32,
    /// Permissions
    pub permissions: AclPermissions,
    /// Tag for additional information
    pub tag: Option<String>,
}

/// ACL for a specific resource
#[derive(Debug, Clone)]
pub struct AccessControlList {
    /// Resource ID
    pub resource_id: u64,
    /// Resource type
    pub resource_type: ResourceType,
    /// ACL entries
    pub entries: Vec<AclEntry>,
    /// Default ACL (for directories)
    pub default_acl: Option<Vec<AclEntry>>,
    /// Whether ACL is enabled
    pub enabled: bool,
    /// Inheritance flags
    pub inheritance_flags: InheritanceFlags,
}

/// Resource types that can have ACLs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceType {
    File,
    Directory,
    Device,
    Socket,
    SharedMemory,
    MessageQueue,
    Semaphore,
    Process,
    NetworkInterface,
    Memory,
    SystemCall,
    Network,
}

/// ACL inheritance flags
#[derive(Debug, Clone, Copy)]
pub struct InheritanceFlags {
    /// Inherit to files
    pub file_inherit: bool,
    /// Inherit to directories
    pub directory_inherit: bool,
    /// Inherit only (don't apply to current object)
    pub inherit_only: bool,
    /// No propagate inheritance
    pub no_propagate: bool,
}

impl Default for InheritanceFlags {
    fn default() -> Self {
        Self {
            file_inherit: true,
            directory_inherit: true,
            inherit_only: false,
            no_propagate: false,
        }
    }
}

/// ACL configuration
#[derive(Debug, Clone)]
pub struct AclConfig {
    /// Whether ACLs are globally enabled
    pub enabled: bool,
    /// Default ACL for new files
    pub default_file_acl: Option<AccessControlList>,
    /// Default ACL for new directories
    pub default_dir_acl: Option<AccessControlList>,
    /// Whether to enforce strict ACL checking
    pub strict_mode: bool,
    /// Whether to log ACL decisions
    pub log_decisions: bool,
    /// Maximum number of ACL entries per resource
    pub max_entries: usize,
}

impl Default for AclConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_file_acl: None,
            default_dir_acl: None,
            strict_mode: false,
            log_decisions: true,
            max_entries: 32,
        }
    }
}

/// Access request
#[derive(Debug, Clone)]
pub struct AccessRequest {
    /// Process ID making the request
    pub pid: u64,
    /// User ID
    pub uid: u32,
    /// Group IDs
    pub gids: Vec<u32>,
    /// Resource being accessed
    pub resource_id: u64,
    /// Resource type
    pub resource_type: ResourceType,
    /// Requested permissions
    pub requested_permissions: AclPermissions,
    /// Access context (e.g., operation type)
    pub context: AccessContext,
}

/// Access context information
#[derive(Debug, Clone)]
pub struct AccessContext {
    /// Operation being performed
    pub operation: String,
    /// File path (if applicable)
    pub path: Option<String>,
    /// Flags and options
    pub flags: u32,
    /// Whether this is a privileged operation
    pub privileged: bool,
}

/// Access decision result
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessDecision {
    /// Access granted
    Granted,
    /// Access denied
    Denied,
    /// Access denied with escalation (admin review required)
    DeniedEscalation,
    /// Access granted with audit (log required)
    GrantedAudit,
}

/// ACL statistics
#[derive(Debug, Default, Clone)]
pub struct AclStats {
    pub events_processed: u64,
    /// Total access checks performed
    pub total_checks: u64,
    /// Access granted
    pub access_granted: u64,
    /// Access denied
    pub access_denied: u64,
    /// Access denied with escalation
    pub escalation_denied: u64,
    /// Access granted with audit
    pub audit_granted: u64,
    /// Checks by resource type
    pub checks_by_type: BTreeMap<ResourceType, u64>,
    /// Checks by user
    pub checks_by_user: BTreeMap<u32, u64>,
    /// Average check time (in nanoseconds)
    pub avg_check_time_ns: u64,
}

/// ACL subsystem
pub struct AclSubsystem {
    /// Configuration
    config: AclConfig,
    /// ACLs by resource ID
    acls: BTreeMap<u64, AccessControlList>,
    /// Statistics
    stats: Arc<Mutex<AclStats>>,
    /// Next resource ID
    next_resource_id: AtomicU64,
}

impl AclSubsystem {
    /// Create a new ACL subsystem
    pub fn new(config: AclConfig) -> Self {
        Self {
            config,
            acls: BTreeMap::new(),
            stats: Arc::new(Mutex::new(AclStats::default())),
            next_resource_id: AtomicU64::new(1),
        }
    }

    /// Create a new ACL
    pub fn create_acl(
        &mut self,
        resource_type: ResourceType,
        entries: Vec<AclEntry>,
    ) -> Result<u64, &'static str> {
        if entries.len() > self.config.max_entries {
            return Err("Too many ACL entries");
        }

        let resource_id = self.next_resource_id.fetch_add(1, Ordering::SeqCst);

        let acl = AccessControlList {
            resource_id,
            resource_type,
            entries,
            default_acl: None,
            enabled: true,
            inheritance_flags: InheritanceFlags::default(),
        };

        self.acls.insert(resource_id, acl);
        Ok(resource_id)
    }

    /// Get ACL for a resource
    pub fn get_acl(&self, resource_id: u64) -> Option<&AccessControlList> {
        self.acls.get(&resource_id)
    }

    /// Update ACL for a resource
    pub fn update_acl(
        &mut self,
        resource_id: u64,
        entries: Vec<AclEntry>,
    ) -> Result<(), &'static str> {
        if entries.len() > self.config.max_entries {
            return Err("Too many ACL entries");
        }

        let acl = self.acls.get_mut(&resource_id)
            .ok_or("ACL not found")?;

        acl.entries = entries;
        Ok(())
    }

    /// Delete ACL
    pub fn delete_acl(&mut self, resource_id: u64) -> Result<(), &'static str> {
        match self.acls.remove(&resource_id) {
            Some(_) => Ok(()),
            None => Err("ACL not found"),
        }
    }

    /// Check access permissions
    pub fn check_access(&self, request: &AccessRequest) -> AccessDecision {
        let start_time = crate::time::get_timestamp_nanos();

        let decision = if !self.config.enabled {
            AccessDecision::Granted
        } else {
            self.evaluate_access(request)
        };

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_checks += 1;
            *stats.checks_by_type.entry(request.resource_type).or_insert(0) += 1;
            *stats.checks_by_user.entry(request.uid).or_insert(0) += 1;

            match decision {
                AccessDecision::Granted => stats.access_granted += 1,
                AccessDecision::Denied => stats.access_denied += 1,
                AccessDecision::DeniedEscalation => stats.escalation_denied += 1,
                AccessDecision::GrantedAudit => stats.audit_granted += 1,
            }

            let elapsed = crate::time::get_timestamp_nanos() - start_time;
            stats.avg_check_time_ns = (stats.avg_check_time_ns + elapsed) / 2;
        }

        // Log decision if required
        if self.config.log_decisions || matches!(decision, AccessDecision::GrantedAudit | AccessDecision::DeniedEscalation) {
            self.log_access_decision(request, &decision);
        }

        decision
    }

    /// Evaluate access permissions
    fn evaluate_access(&self, request: &AccessRequest) -> AccessDecision {
        let acl = match self.acls.get(&request.resource_id) {
            Some(acl) => acl,
            None => {
                // No ACL means check traditional Unix permissions
                return self.check_traditional_permissions(request);
            }
        };

        if !acl.enabled {
            return AccessDecision::Granted;
        }

        // Check each relevant ACL entry
        let mut granted_permissions = AclPermissions::default();

        // Check user-specific entry
        if let Some(user_entry) = acl.entries.iter().find(|e| e.acl_type == AclType::User && e.id == request.uid) {
            self.merge_permissions(&mut granted_permissions, &user_entry.permissions);
        }

        // Check group entries
        for gid in &request.gids {
            if let Some(group_entry) = acl.entries.iter().find(|e| e.acl_type == AclType::Group && e.id == *gid) {
                self.merge_permissions(&mut granted_permissions, &group_entry.permissions);
            }
        }

        // Check mask entry
        if let Some(mask_entry) = acl.entries.iter().find(|e| e.acl_type == AclType::Mask) {
            self.apply_mask(&mut granted_permissions, &mask_entry.permissions);
        }

        // Check other entry if no specific permissions found
        if !self.has_any_permissions(&granted_permissions) {
            if let Some(other_entry) = acl.entries.iter().find(|e| e.acl_type == AclType::Other) {
                self.merge_permissions(&mut granted_permissions, &other_entry.permissions);
            }
        }

        // Evaluate if requested permissions are granted
        if self.has_requested_permissions(&granted_permissions, &request.requested_permissions) {
            if request.context.privileged || self.requires_audit(&granted_permissions) {
                AccessDecision::GrantedAudit
            } else {
                AccessDecision::Granted
            }
        } else {
            if self.config.strict_mode && self.is_escalation_attempt(&request.requested_permissions) {
                AccessDecision::DeniedEscalation
            } else {
                AccessDecision::Denied
            }
        }
    }

    /// Check traditional Unix permissions
    fn check_traditional_permissions(&self, request: &AccessRequest) -> AccessDecision {
        // This would integrate with the existing VFS permission checking
        // For now, we'll default to granted for backwards compatibility
        AccessDecision::Granted
    }

    /// Merge permissions from ACL entry
    fn merge_permissions(&self, target: &mut AclPermissions, source: &AclPermissions) {
        target.read |= source.read;
        target.write |= source.write;
        target.execute |= source.execute;
        target.append |= source.append;
        target.delete |= source.delete;
        target.read_attributes |= source.read_attributes;
        target.write_attributes |= source.write_attributes;
        target.read_acl |= source.read_acl;
        target.write_acl |= source.write_acl;
        target.synchronize |= source.synchronize;
    }

    /// Apply mask to permissions
    fn apply_mask(&self, permissions: &mut AclPermissions, mask: &AclPermissions) {
        permissions.read &= mask.read;
        permissions.write &= mask.write;
        permissions.execute &= mask.execute;
        permissions.append &= mask.append;
        permissions.delete &= mask.delete;
        permissions.read_attributes &= mask.read_attributes;
        permissions.write_attributes &= mask.write_attributes;
        permissions.read_acl &= mask.read_acl;
        permissions.write_acl &= mask.write_acl;
        permissions.synchronize &= mask.synchronize;
    }

    /// Check if permissions have any bits set
    fn has_any_permissions(&self, permissions: &AclPermissions) -> bool {
        permissions.read || permissions.write || permissions.execute ||
        permissions.append || permissions.delete || permissions.write_attributes ||
        permissions.write_acl || permissions.synchronize
    }

    /// Check if requested permissions are granted
    fn has_requested_permissions(
        &self,
        granted: &AclPermissions,
        requested: &AclPermissions,
    ) -> bool {
        (!requested.read || granted.read) &&
        (!requested.write || granted.write) &&
        (!requested.execute || granted.execute) &&
        (!requested.append || granted.append) &&
        (!requested.delete || granted.delete) &&
        (!requested.read_attributes || granted.read_attributes) &&
        (!requested.write_attributes || granted.write_attributes) &&
        (!requested.read_acl || granted.read_acl) &&
        (!requested.write_acl || granted.write_acl) &&
        (!requested.synchronize || granted.synchronize)
    }

    /// Check if operation requires audit
    fn requires_audit(&self, permissions: &AclPermissions) -> bool {
        permissions.delete || permissions.write_acl || permissions.write_attributes
    }

    /// Check if this is an escalation attempt
    fn is_escalation_attempt(&self, requested: &AclPermissions) -> bool {
        requested.write_acl || requested.delete
    }

    /// Log access decision
    fn log_access_decision(&self, request: &AccessRequest, decision: &AccessDecision) {
        let decision_str = match decision {
            AccessDecision::Granted => "GRANTED",
            AccessDecision::Denied => "DENIED",
            AccessDecision::DeniedEscalation => "DENIED_ESCALATION",
            AccessDecision::GrantedAudit => "GRANTED_AUDIT",
        };

        crate::println!(
            "[ACL] {}: uid={}, gid={:?}, resource={:?}:{}, operation={}",
            decision_str, request.uid, request.gids, request.resource_type,
            request.resource_id, request.context.operation
        );
    }

    /// Set default ACL for new resources
    pub fn set_default_acl(
        &mut self,
        resource_type: ResourceType,
        acl: AccessControlList,
    ) {
        match resource_type {
            ResourceType::File => {
                self.config.default_file_acl = Some(acl);
            }
            ResourceType::Directory => {
                self.config.default_dir_acl = Some(acl);
            }
            _ => {
                // Other resource types don't typically have defaults
            }
        }
    }

    /// Get default ACL for resource type
    pub fn get_default_acl(&self, resource_type: ResourceType) -> Option<&AccessControlList> {
        match resource_type {
            ResourceType::File => self.config.default_file_acl.as_ref(),
            ResourceType::Directory => self.config.default_dir_acl.as_ref(),
            _ => None,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &AclConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: AclConfig) {
        self.config = config;
    }

    /// Get statistics
    pub fn get_stats(&self) -> AclStats {
        self.stats.lock().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        *self.stats.lock() = AclStats::default();
    }

    /// List all ACLs
    pub fn list_acls(&self) -> Vec<&AccessControlList> {
        self.acls.values().collect()
    }

    /// Search ACLs by criteria
    pub fn search_acls<F>(&self, predicate: F) -> Vec<&AccessControlList>
    where
        F: Fn(&AccessControlList) -> bool,
    {
        self.acls.values().filter(|acl| predicate(acl)).collect()
    }
}

/// High-level ACL interface functions

/// Check file access using ACL
/// Initialize ACL subsystem
pub fn initialize_acl() -> Result<(), i32> {
    // ACL is already initialized via global static instance
    Ok(())
}

/// Cleanup ACL subsystem
pub fn cleanup_acl() {
    // Placeholder: In a real implementation, this would clean up ACL resources
}

pub fn check_file_access(
    pid: u64,
    uid: u32,
    gids: Vec<u32>,
    file_id: u64,
    requested_read: bool,
    requested_write: bool,
    requested_execute: bool,
) -> Result<(), isize> {
    let requested_permissions = AclPermissions {
        read: requested_read,
        write: requested_write,
        execute: requested_execute,
        append: false,
        delete: false,
        read_attributes: true,
        write_attributes: false,
        read_acl: false,
        write_acl: false,
        synchronize: true,
    };

    let request = AccessRequest {
        pid,
        uid,
        gids,
        resource_id: file_id,
        resource_type: ResourceType::File,
        requested_permissions,
        context: AccessContext {
            operation: "file_access".to_string(),
            path: None,
            flags: 0,
            privileged: false,
        },
    };

    let decision = if let Some(ref s) = *crate::security::ACL.lock() {
        s.check_access(&request)
    } else {
        AccessDecision::Denied
    };

    match decision {
        AccessDecision::Granted | AccessDecision::GrantedAudit => Ok(()),
        AccessDecision::Denied => Err(EACCES as isize),
        AccessDecision::DeniedEscalation => Err(EPERM as isize),
    }
}

/// Create new ACL for resource
pub fn create_acl(
    resource_type: ResourceType,
    entries: Vec<AclEntry>,
) -> Result<u64, &'static str> {
    let mut guard = crate::security::ACL.lock();
    if let Some(ref mut s) = *guard {
        s.create_acl(resource_type, entries)
    } else {
        Err("ACL subsystem not initialized")
    }
}

/// Get ACL statistics
pub fn get_acl_statistics() -> AclStats {
    let guard = crate::security::ACL.lock();
    guard.as_ref().map(|s| s.get_stats()).unwrap_or_default()
}

/// Update ACL configuration
pub fn update_acl_config(config: AclConfig) {
    let mut guard = crate::security::ACL.lock();
    if let Some(ref mut s) = *guard {
        s.update_config(config);
    } else {
        *guard = Some(AclSubsystem::new(config));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acl_permissions_default() {
        let perms = AclPermissions::default();
        assert!(!perms.read);
        assert!(!perms.write);
        assert!(!perms.execute);
        assert!(perms.read_attributes);
        assert!(perms.read_acl);
        assert!(perms.synchronize);
    }

    #[test]
    fn test_acl_entry_creation() {
        let entry = AclEntry {
            acl_type: AclType::User,
            id: 1000,
            permissions: AclPermissions {
                read: true,
                write: true,
                execute: false,
                ..Default::default()
            },
            tag: Some("test".to_string()),
        };

        assert_eq!(entry.acl_type, AclType::User);
        assert_eq!(entry.id, 1000);
        assert!(entry.permissions.read);
        assert!(entry.permissions.write);
        assert!(!entry.permissions.execute);
    }

    #[test]
    fn test_acl_config_default() {
        let config = AclConfig::default();
        assert!(config.enabled);
        assert!(!config.strict_mode);
        assert!(config.log_decisions);
        assert_eq!(config.max_entries, 32);
    }

    #[test]
    fn test_inheritance_flags_default() {
        let flags = InheritanceFlags::default();
        assert!(flags.file_inherit);
        assert!(flags.directory_inherit);
        assert!(!flags.inherit_only);
        assert!(!flags.no_propagate);
    }

    #[test]
    fn test_acl_subsystem_creation() {
        let config = AclConfig::default();
        let subsystem = AclSubsystem::new(config);

        assert!(subsystem.config().enabled);
        let stats = subsystem.get_stats();
        assert_eq!(stats.total_checks, 0);
        assert_eq!(stats.access_granted, 0);
        assert_eq!(stats.access_denied, 0);
    }

    #[test]
    fn test_permissions_merge() {
        let config = AclConfig::default();
        let subsystem = AclSubsystem::new(config);

        let mut target = AclPermissions {
            read: true,
            write: false,
            execute: false,
            ..Default::default()
        };

        let source = AclPermissions {
            read: false,
            write: true,
            execute: true,
            ..Default::default()
        };

        subsystem.merge_permissions(&mut target, &source);
        assert!(target.read);
        assert!(target.write);
        assert!(target.execute);
    }

    #[test]
    fn test_requested_permissions_check() {
        let config = AclConfig::default();
        let subsystem = AclSubsystem::new(config);

        let granted = AclPermissions {
            read: true,
            write: true,
            execute: false,
            ..Default::default()
        };

        let requested = AclPermissions {
            read: true,
            write: false,
            execute: false,
            ..Default::default()
        };

        assert!(subsystem.has_requested_permissions(&granted, &requested));

        let requested_all = AclPermissions {
            read: true,
            write: true,
            execute: true,
            ..Default::default()
        };

        assert!(!subsystem.has_requested_permissions(&granted, &requested_all));
    }
}
