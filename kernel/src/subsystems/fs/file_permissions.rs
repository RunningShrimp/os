//! File Permissions Implementation
//!
//! This module implements a comprehensive file permissions system for NOS,
//! providing POSIX-compatible access control with user, group, and other permissions.
//! The implementation supports access control lists (ACLs), capability-based security,
//! and fine-grained permission checking.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
// use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::subsystems::process::{Process, ProcessId};
use crate::subsystems::fs::{InodeType, DiskInode};

/// Permission bits (POSIX-compatible)
pub const PERM_READ: u16 = 0o400;    // Owner read
pub const PERM_WRITE: u16 = 0o200;   // Owner write
pub const PERM_EXEC: u16 = 0o100;    // Owner execute
pub const PERM_GREAD: u16 = 0o040;   // Group read
pub const PERM_GWRITE: u16 = 0o020;  // Group write
pub const PERM_GEXEC: u16 = 0o010;   // Group execute
pub const PERM_OREAD: u16 = 0o004;   // Other read
pub const PERM_OWRITE: u16 = 0o002;  // Other write
pub const PERM_OEXEC: u16 = 0o001;   // Other execute

/// Special permission bits
pub const PERM_SETUID: u16 = 0o4000;  // Set user ID on execution
pub const PERM_SETGID: u16 = 0o2000;  // Set group ID on execution
pub const PERM_STICKY: u16 = 0o1000;  // Sticky bit

/// Default file permissions
pub const DEFAULT_FILE_PERMS: u16 = PERM_READ | PERM_WRITE | PERM_GREAD | PERM_OREAD;
pub const DEFAULT_DIR_PERMS: u16 = PERM_READ | PERM_WRITE | PERM_EXEC | 
                                   PERM_GREAD | PERM_GEXEC | PERM_OREAD | PERM_OEXEC;

/// User ID for root (superuser)
pub const ROOT_UID: u32 = 0;

/// Group ID for root group
pub const ROOT_GID: u32 = 0;

/// Access control entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AclType {
    User = 0,
    Group = 1,
    Other = 2,
    Mask = 3,
}

/// Access control entry
#[derive(Debug, Clone)]
pub struct AclEntry {
    /// Type of ACL entry
    pub acl_type: AclType,
    /// User or group ID (not used for Other)
    pub id: u32,
    /// Permission bits
    pub permissions: u16,
    /// Whether this entry is effective
    pub effective: bool,
}

impl AclEntry {
    /// Create a new ACL entry
    pub fn new(acl_type: AclType, id: u32, permissions: u16) -> Self {
        Self {
            acl_type,
            id,
            permissions,
            effective: true,
        }
    }

    /// Check if this entry grants read permission
    pub fn can_read(&self) -> bool {
        self.effective && (self.permissions & PERM_READ != 0 || 
                          self.permissions & PERM_GREAD != 0 || 
                          self.permissions & PERM_OREAD != 0)
    }

    /// Check if this entry grants write permission
    pub fn can_write(&self) -> bool {
        self.effective && (self.permissions & PERM_WRITE != 0 || 
                          self.permissions & PERM_GWRITE != 0 || 
                          self.permissions & PERM_OWRITE != 0)
    }

    /// Check if this entry grants execute permission
    pub fn can_execute(&self) -> bool {
        self.effective && (self.permissions & PERM_EXEC != 0 || 
                          self.permissions & PERM_GEXEC != 0 || 
                          self.permissions & PERM_OEXEC != 0)
    }
}

/// Access control list
#[derive(Debug, Clone)]
pub struct AccessControlList {
    /// List of ACL entries
    entries: Vec<AclEntry>,
    /// Default ACL entries for directories
    default_entries: Vec<AclEntry>,
}

impl AccessControlList {
    /// Create a new empty ACL
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            default_entries: Vec::new(),
        }
    }

    /// Add an ACL entry
    pub fn add_entry(&mut self, entry: AclEntry) {
        // Remove any existing entry for the same type and ID
        self.entries.retain(|e| !(e.acl_type == entry.acl_type && e.id == entry.id));
        self.entries.push(entry);
    }

    /// Add a default ACL entry (for directories)
    pub fn add_default_entry(&mut self, entry: AclEntry) {
        // Remove any existing default entry for the same type and ID
        self.default_entries.retain(|e| !(e.acl_type == entry.acl_type && e.id == entry.id));
        self.default_entries.push(entry);
    }

    /// Get ACL entries
    pub fn entries(&self) -> &[AclEntry] {
        &self.entries
    }

    /// Get default ACL entries
    pub fn default_entries(&self) -> &[AclEntry] {
        &self.default_entries
    }

    /// Check if a user has specific permissions
    pub fn check_permissions(&self, uid: u32, gid: u32, groups: &[u32], required_perms: u16) -> bool {
        // Check for exact user match
        for entry in &self.entries {
            if entry.acl_type == AclType::User && entry.id == uid {
                if (entry.permissions & required_perms) == required_perms {
                    return true;
                }
                return false; // Explicit user entry denies access
            }
        }

        // Check for group match
        for entry in &self.entries {
            if entry.acl_type == AclType::Group && groups.contains(&entry.id) {
                if (entry.permissions & required_perms) == required_perms {
                    return true;
                }
                return false; // Explicit group entry denies access
            }
        }

        // Check other permissions
        for entry in &self.entries {
            if entry.acl_type == AclType::Other {
                return (entry.permissions & required_perms) == required_perms;
            }
        }

        // No matching ACL entry, deny access
        false
    }
}

/// File capability
#[derive(Debug, Clone)]
pub struct FileCapability {
    /// Capability name
    pub name: String,
    /// Capability value
    pub value: u64,
    /// Whether this capability is inheritable
    pub inheritable: bool,
}

/// Extended file attributes
#[derive(Debug, Clone)]
pub struct ExtendedAttributes {
    /// User-defined attributes
    user_attrs: BTreeMap<String, Vec<u8>>,
    /// System attributes
    system_attrs: BTreeMap<String, Vec<u8>>,
    /// Security attributes
    security_attrs: BTreeMap<String, Vec<u8>>,
}

impl ExtendedAttributes {
    /// Create a new extended attributes structure
    pub fn new() -> Self {
        Self {
            user_attrs: BTreeMap::new(),
            system_attrs: BTreeMap::new(),
            security_attrs: BTreeMap::new(),
        }
    }

    /// Set a user attribute
    pub fn set_user_attr(&mut self, name: &str, value: &[u8]) {
        self.user_attrs.insert(name.to_string(), value.to_vec());
    }

    /// Get a user attribute
    pub fn get_user_attr(&self, name: &str) -> Option<&[u8]> {
        self.user_attrs.get(name).map(|v| v.as_slice())
    }

    /// Remove a user attribute
    pub fn remove_user_attr(&mut self, name: &str) -> Option<Vec<u8>> {
        self.user_attrs.remove(name)
    }

    /// Set a system attribute
    pub fn set_system_attr(&mut self, name: &str, value: &[u8]) {
        self.system_attrs.insert(name.to_string(), value.to_vec());
    }

    /// Get a system attribute
    pub fn get_system_attr(&self, name: &str) -> Option<&[u8]> {
        self.system_attrs.get(name).map(|v| v.as_slice())
    }

    /// Set a security attribute
    pub fn set_security_attr(&mut self, name: &str, value: &[u8]) {
        self.security_attrs.insert(name.to_string(), value.to_vec());
    }

    /// Get a security attribute
    pub fn get_security_attr(&self, name: &str) -> Option<&[u8]> {
        self.security_attrs.get(name).map(|v| v.as_slice())
    }
}

/// File permissions structure
#[derive(Debug, Clone)]
pub struct FilePermissions {
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Permission bits (POSIX-style)
    pub mode: u16,
    /// Access control list
    pub acl: AccessControlList,
    /// Extended attributes
    pub xattrs: ExtendedAttributes,
    /// File capabilities
    pub capabilities: Vec<FileCapability>,
    /// Creation timestamp
    pub creation_time: u64,
    /// Modification timestamp
    pub modification_time: u64,
    /// Access timestamp
    pub access_time: u64,
}

impl FilePermissions {
    /// Create new file permissions with default values
    pub fn new(uid: u32, gid: u32, is_directory: bool) -> Self {
        let mode = if is_directory { DEFAULT_DIR_PERMS } else { DEFAULT_FILE_PERMS };
        let now = crate::time::get_timestamp();
        
        Self {
            uid,
            gid,
            mode,
            acl: AccessControlList::new(),
            xattrs: ExtendedAttributes::new(),
            capabilities: Vec::new(),
            creation_time: now,
            modification_time: now,
            access_time: now,
        }
    }

    /// Check if a user has read permission
    pub fn can_read(&self, uid: u32, gid: u32, groups: &[u32]) -> bool {
        // Root can read anything
        if uid == ROOT_UID {
            return true;
        }

        // Check ACL first
        if !self.acl.entries().is_empty() {
            return self.acl.check_permissions(uid, gid, groups, PERM_READ);
        }

        // Check owner permissions
        if uid == self.uid {
            return self.mode & PERM_READ != 0;
        }

        // Check group permissions
        if gid == self.gid || groups.contains(&self.gid) {
            return self.mode & PERM_GREAD != 0;
        }

        // Check other permissions
        self.mode & PERM_OREAD != 0
    }

    /// Check if a user has write permission
    pub fn can_write(&self, uid: u32, gid: u32, groups: &[u32]) -> bool {
        // Root can write anything
        if uid == ROOT_UID {
            return true;
        }

        // Check ACL first
        if !self.acl.entries().is_empty() {
            return self.acl.check_permissions(uid, gid, groups, PERM_WRITE);
        }

        // Check owner permissions
        if uid == self.uid {
            return self.mode & PERM_WRITE != 0;
        }

        // Check group permissions
        if gid == self.gid || groups.contains(&self.gid) {
            return self.mode & PERM_GWRITE != 0;
        }

        // Check other permissions
        self.mode & PERM_OWRITE != 0
    }

    /// Check if a user has execute permission
    pub fn can_execute(&self, uid: u32, gid: u32, groups: &[u32]) -> bool {
        // Root can execute anything
        if uid == ROOT_UID {
            return true;
        }

        // Check ACL first
        if !self.acl.entries().is_empty() {
            return self.acl.check_permissions(uid, gid, groups, PERM_EXEC);
        }

        // Check owner permissions
        if uid == self.uid {
            return self.mode & PERM_EXEC != 0;
        }

        // Check group permissions
        if gid == self.gid || groups.contains(&self.gid) {
            return self.mode & PERM_GEXEC != 0;
        }

        // Check other permissions
        self.mode & PERM_OEXEC != 0
    }

    /// Check if a user has specific permissions
    pub fn has_permissions(&self, uid: u32, gid: u32, groups: &[u32], required_perms: u16) -> bool {
        // Root has all permissions
        if uid == ROOT_UID {
            return true;
        }

        // Check ACL first
        if !self.acl.entries().is_empty() {
            return self.acl.check_permissions(uid, gid, groups, required_perms);
        }

        // Check owner permissions
        if uid == self.uid {
            return (self.mode & required_perms) == required_perms;
        }

        // Check group permissions
        if gid == self.gid || groups.contains(&self.gid) {
            let group_perms = required_perms & (PERM_GREAD | PERM_GWRITE | PERM_GEXEC);
            return (self.mode & group_perms) == group_perms;
        }

        // Check other permissions
        let other_perms = required_perms & (PERM_OREAD | PERM_OWRITE | PERM_OEXEC);
        (self.mode & other_perms) == other_perms
    }

    /// Update access time
    pub fn update_access_time(&mut self) {
        self.access_time = crate::time::get_timestamp();
    }

    /// Update modification time
    pub fn update_modification_time(&mut self) {
        self.modification_time = crate::time::get_timestamp();
    }

    /// Set permissions
    pub fn set_mode(&mut self, mode: u16) {
        self.mode = mode;
        self.update_modification_time();
    }

    /// Change owner
    pub fn change_owner(&mut self, uid: u32, gid: u32) {
        self.uid = uid;
        self.gid = gid;
        self.update_modification_time();
    }

    /// Add a capability
    pub fn add_capability(&mut self, capability: FileCapability) {
        self.capabilities.push(capability);
        self.update_modification_time();
    }

    /// Get a capability by name
    pub fn get_capability(&self, name: &str) -> Option<&FileCapability> {
        self.capabilities.iter().find(|c| c.name == name)
    }

    /// Remove a capability by name
    pub fn remove_capability(&mut self, name: &str) -> Option<FileCapability> {
        let pos = self.capabilities.iter().position(|c| c.name == name)?;
        let cap = self.capabilities.remove(pos);
        self.update_modification_time();
        Some(cap)
    }

    /// Get permission string (like "rwxr-xr--")
    pub fn permission_string(&self) -> String {
        let mut result = String::with_capacity(10);

        // File type
        result.push('-'); // Default to regular file

        // Owner permissions
        result.push(if self.mode & PERM_READ != 0 { 'r' } else { '-' });
        result.push(if self.mode & PERM_WRITE != 0 { 'w' } else { '-' });
        result.push(if self.mode & PERM_EXEC != 0 { 'x' } else { '-' });

        // Group permissions
        result.push(if self.mode & PERM_GREAD != 0 { 'r' } else { '-' });
        result.push(if self.mode & PERM_GWRITE != 0 { 'w' } else { '-' });
        result.push(if self.mode & PERM_GEXEC != 0 { 'x' } else { '-' });

        // Other permissions
        result.push(if self.mode & PERM_OREAD != 0 { 'r' } else { '-' });
        result.push(if self.mode & PERM_OWRITE != 0 { 'w' } else { '-' });
        result.push(if self.mode & PERM_OEXEC != 0 { 'x' } else { '-' });

        result
    }
}

/// Permission manager
pub struct PermissionManager {
    /// Next available user ID
    next_uid: AtomicU32,
    /// Next available group ID
    next_gid: AtomicU32,
    /// User database
    users: Mutex<BTreeMap<u32, UserInfo>>,
    /// Group database
    groups: Mutex<BTreeMap<u32, GroupInfo>>,
    /// Permission statistics
    stats: Mutex<PermissionStats>,
}

/// User information
#[derive(Debug, Clone)]
pub struct UserInfo {
    /// User ID
    pub uid: u32,
    /// Username
    pub name: String,
    /// Primary group ID
    pub gid: u32,
    /// Secondary group IDs
    pub secondary_groups: Vec<u32>,
    /// Home directory
    pub home_dir: String,
    /// Shell
    pub shell: String,
}

/// Group information
#[derive(Debug, Clone)]
pub struct GroupInfo {
    /// Group ID
    pub gid: u32,
    /// Group name
    pub name: String,
    /// Member user IDs
    pub members: Vec<u32>,
}

/// Permission statistics
#[derive(Debug, Default)]
pub struct PermissionStats {
    /// Total permission checks
    pub total_checks: u64,
    /// Permission denials
    pub denials: u64,
    /// ACL checks
    pub acl_checks: u64,
    /// Capability checks
    pub capability_checks: u64,
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new() -> Self {
        Self {
            next_uid: AtomicU32::new(1000), // Start from 1000
            next_gid: AtomicU32::new(1000), // Start from 1000
            users: Mutex::new(BTreeMap::new()),
            groups: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(PermissionStats::default()),
        }
    }

    /// Initialize the permission manager with default users and groups
    pub fn init(&self) {
        // Add root user
        let root_user = UserInfo {
            uid: ROOT_UID,
            name: "root".to_string(),
            gid: ROOT_GID,
            secondary_groups: Vec::new(),
            home_dir: "/root".to_string(),
            shell: "/bin/sh".to_string(),
        };
        
        let mut users = self.users.lock();
        users.insert(ROOT_UID, root_user);
        drop(users);

        // Add root group
        let root_group = GroupInfo {
            gid: ROOT_GID,
            name: "root".to_string(),
            members: vec![ROOT_UID],
        };
        
        let mut groups = self.groups.lock();
        groups.insert(ROOT_GID, root_group);
        drop(groups);

        crate::println!("perm: initialized permission manager");
    }

    /// Create a new user
    pub fn create_user(&self, name: &str, gid: u32, home_dir: &str, shell: &str) -> u32 {
        let uid = self.next_uid.fetch_add(1, Ordering::SeqCst);
        
        let user = UserInfo {
            uid,
            name: name.to_string(),
            gid,
            secondary_groups: Vec::new(),
            home_dir: home_dir.to_string(),
            shell: shell.to_string(),
        };
        
        let mut users = self.users.lock();
        users.insert(uid, user);
        drop(users);

        // Add user to primary group
        let mut groups = self.groups.lock();
        if let Some(group) = groups.get_mut(&gid) {
            if !group.members.contains(&uid) {
                group.members.push(uid);
            }
        }
        drop(groups);

        crate::println!("perm: created user '{}' with UID {}", name, uid);
        uid
    }

    /// Create a new group
    pub fn create_group(&self, name: &str) -> u32 {
        let gid = self.next_gid.fetch_add(1, Ordering::SeqCst);
        
        let group = GroupInfo {
            gid,
            name: name.to_string(),
            members: Vec::new(),
        };
        
        let mut groups = self.groups.lock();
        groups.insert(gid, group);
        drop(groups);

        crate::println!("perm: created group '{}' with GID {}", name, gid);
        gid
    }

    /// Get user information
    pub fn get_user(&self, uid: u32) -> Option<UserInfo> {
        let users = self.users.lock();
        users.get(&uid).cloned()
    }

    /// Get user by name
    pub fn get_user_by_name(&self, name: &str) -> Option<UserInfo> {
        let users = self.users.lock();
        users.values().find(|u| u.name == name).cloned()
    }

    /// Get group information
    pub fn get_group(&self, gid: u32) -> Option<GroupInfo> {
        let groups = self.groups.lock();
        groups.get(&gid).cloned()
    }

    /// Get group by name
    pub fn get_group_by_name(&self, name: &str) -> Option<GroupInfo> {
        let groups = self.groups.lock();
        groups.values().find(|g| g.name == name).cloned()
    }

    /// Get all groups for a user
    pub fn get_user_groups(&self, uid: u32) -> Vec<u32> {
        let mut groups = Vec::new();
        
        // Get primary group
        if let Some(user) = self.get_user(uid) {
            groups.push(user.gid);
            
            // Get secondary groups
            groups.extend_from_slice(&user.secondary_groups);
        }
        
        // Check group memberships
        let groups_db = self.groups.lock();
        for (gid, group) in groups_db.iter() {
            if group.members.contains(&uid) && !groups.contains(gid) {
                groups.push(*gid);
            }
        }
        
        groups
    }

    /// Add user to group
    pub fn add_user_to_group(&self, uid: u32, gid: u32) -> bool {
        let mut groups = self.groups.lock();
        if let Some(group) = groups.get_mut(&gid) {
            if !group.members.contains(&uid) {
                group.members.push(uid);
                return true;
            }
        }
        false
    }

    /// Remove user from group
    pub fn remove_user_from_group(&self, uid: u32, gid: u32) -> bool {
        let mut groups = self.groups.lock();
        if let Some(group) = groups.get_mut(&gid) {
            if let Some(pos) = group.members.iter().position(|&u| u == uid) {
                group.members.remove(pos);
                return true;
            }
        }
        false
    }

    /// Check file permissions
    pub fn check_permissions(&self, perms: &FilePermissions, required_perms: u16) -> bool {
        // Get current process information
        let current_process = crate::subsystems::process::get_current_process();
        let (uid, gid, groups) = if let Some(process) = current_process {
            let process_info = process.lock();
            let groups = self.get_user_groups(process_info.uid);
            (process_info.uid, process_info.gid, groups)
        } else {
            // Default to root if no current process
            (ROOT_UID, ROOT_GID, Vec::new())
        };

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_checks += 1;
            if !perms.acl.entries().is_empty() {
                stats.acl_checks += 1;
            }
        }

        let result = perms.has_permissions(uid, gid, &groups, required_perms);
        
        if !result {
            let mut stats = self.stats.lock();
            stats.denials += 1;
        }

        result
    }

    /// Get permission statistics
    pub fn get_stats(&self) -> PermissionStats {
        self.stats.lock().clone()
    }
}

/// Global permission manager instance
static mut PERMISSION_MANAGER: Option<PermissionManager> = None;

/// Initialize the permission manager
pub fn init() {
    unsafe {
        let pm = PermissionManager::new();
        pm.init();
        PERMISSION_MANAGER = Some(pm);
    }
}

/// Get the permission manager instance
pub fn get_permission_manager() -> Option<&'static PermissionManager> {
    unsafe { PERMISSION_MANAGER.as_ref() }
}

/// Helper function to check if current process can read a file
pub fn can_read_file(perms: &FilePermissions) -> bool {
    if let Some(pm) = get_permission_manager() {
        pm.check_permissions(perms, PERM_READ)
    } else {
        // Default to allowing read if no permission manager
        true
    }
}

/// Helper function to check if current process can write a file
pub fn can_write_file(perms: &FilePermissions) -> bool {
    if let Some(pm) = get_permission_manager() {
        pm.check_permissions(perms, PERM_WRITE)
    } else {
        // Default to allowing write if no permission manager
        true
    }
}

/// Helper function to check if current process can execute a file
pub fn can_execute_file(perms: &FilePermissions) -> bool {
    if let Some(pm) = get_permission_manager() {
        pm.check_permissions(perms, PERM_EXEC)
    } else {
        // Default to allowing execute if no permission manager
        true
    }
}

/// Helper function to check if current process can delete a file
pub fn can_delete_file(perms: &FilePermissions, parent_perms: &FilePermissions) -> bool {
    // Need write permission on parent directory
    if !can_write_file(parent_perms) {
        return false;
    }
    
    // Owner can delete their own files
    if let Some(pm) = get_permission_manager() {
        let current_process = crate::subsystems::process::get_current_process();
        if let Some(process) = current_process {
            let process_info = process.lock();
            if process_info.uid == perms.uid || process_info.uid == ROOT_UID {
                return true;
            }
        }
    }
    
    // Check sticky bit
    if perms.mode & PERM_STICKY != 0 {
        // With sticky bit, only owner (or root) can delete
        return false;
    }
    
    // Otherwise, write permission on parent is sufficient
    true
}

use crate::sync::Mutex;