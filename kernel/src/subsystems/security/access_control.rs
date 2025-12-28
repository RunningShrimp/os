//! Access Control and Permission System
//!
//! This module implements comprehensive access control and permission management:
//! - Access Control Lists (ACL)
//! - User and group management
//! - Filesystem permission checking
//! - Process permission inheritance
//! - Security policy engine
//!
//! Features:
//! - Fine-grained ACL entries
//! - Role-based access control (RBAC)
//! - POSIX-like permission bits (rwx)
//! - Capability-based permissions
//! - Audit trail for all access decisions

use spin::Mutex;
use core::sync::atomic;
use alloc::collections::BTreeMap;
use core::sync::atomic;
use alloc::string::String;
use core::sync::atomic;
use alloc::collections::BTreeSet;
use core::sync::atomic;
use alloc::sync::Arc;
use core::sync::atomic;
use alloc::vec::Vec;
use core::sync::atomic;
use alloc::string::{String, ToString};
use core::sync::atomic;

// ============================================================================
// Access Control Constants
// ============================================================================

/// Maximum number of ACL entries per object
pub const MAX_ACL_ENTRIES: usize = 32;

/// Maximum number of users
pub const MAX_USERS: usize = 65536;

/// Maximum number of groups
pub const MAX_GROUPS: usize = 4096;

/// Permission bit masks (POSIX-style)
pub const PERM_READ: u16 = 0o004; // r--
pub const PERM_WRITE: u16 = 0o002; // -w-
pub const PERM_EXECUTE: u16 = 0o001; // --x
pub const PERM_OWNER: u16 = 0o700; // rwx------

// Special permission bits
pub const PERM_SETUID: u16 = 0o4000; // set user ID on execution
pub const PERM_SETGID: u16 = 0o2000; // set group ID on execution
pub const PERM_STICKY: u16 = 0o1000; // delete restricted by owner only

// ============================================================================
// User Management
// ============================================================================

/// User information
#[derive(Debug, Clone)]
pub struct User {
    /// User ID
    pub uid: u32,
    
    /// User name
    pub name: String,
    
    /// Primary group ID
    pub gid: u32,
    
    /// Supplementary group IDs
    pub supplementary_gids: Vec<u32>,
    
    /// User home directory
    pub home_dir: String,
    
    /// Default shell
    pub shell: String,
    
    /// User flags (locked, disabled, etc.)
    pub flags: UserFlags,
    
    /// Creation timestamp
    pub created_at: u64,
}

/// User flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserFlags {
    /// Account is locked
    pub locked: bool,
    
    /// Account is disabled
    pub disabled: bool,
    
    /// Account has root privileges
    pub root: bool,
}

impl Default for UserFlags {
    fn default() -> Self {
        Self {
            locked: false,
            disabled: false,
            root: false,
        }
    }
}

/// User manager
pub struct UserManager {
    /// All users by UID
    users: Mutex<BTreeMap<u32, User>>,
    
    /// Next UID to allocate
    next_uid: AtomicU32,
    
    /// Root user UID (typically 0)
    pub root_uid: u32,
    
    /// Total users count
    pub total_users: AtomicUsize,
}

impl UserManager {
    /// Create new user manager
    pub fn new() -> Self {
        Self {
            users: Mutex::new(BTreeMap::new()),
            next_uid: AtomicU32::new(1), // Start from 1 (0 is typically reserved for root)
            pub root_uid: 0,
            pub total_users: AtomicUsize::new(0),
        }
    }
    
    /// Create new user
    pub fn create_user(&self, name: String, home_dir: String, shell: String, 
                  flags: UserFlags, is_root: bool) -> Result<u32, AccessError> {
        let uid = self.next_uid.fetch_add(1, Ordering::Relaxed);
        
        // Root user gets UID 0
        let actual_uid = if is_root {
            self.root_uid
        } else {
            uid
        };
        
        let mut user_flags = if is_root {
            UserFlags { root: true, ..Default::default() }
        } else {
            flags
        };
        
        let user = User {
            uid: actual_uid,
            name,
            gid: actual_uid, // Default to primary group same as UID
            supplementary_gids: Vec::new(),
            home_dir,
            shell,
            flags: user_flags,
            created_at: crate::subsystems::time::timestamp_nanos(),
        };
        
        let mut users = self.users.lock();
        users.insert(actual_uid, user);
        
        self.total_users.fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[access_control] Created user '{}' with UID {}",
                        name, actual_uid);
        
        Ok(actual_uid)
    }
    
    /// Get user by UID
    pub fn get_user(&self, uid: u32) -> Option<User> {
        let users = self.users.lock();
        users.get(&uid).cloned()
    }
    
    /// Get user by name
    pub fn get_user_by_name(&self, name: &str) -> Option<User> {
        let users = self.users.lock();
        
        for user in users.values() {
            if user.name == name {
                return Some(user.clone());
            }
        }
        
        None
    }
    
    /// Delete user
    pub fn delete_user(&self, uid: u32) -> Result<(), AccessError> {
        if uid == self.root_uid {
            return Err(AccessError::PermissionDenied {
                message: String::from("Cannot delete root user"),
            });
        }
        
        let mut users = self.users.lock();
        
        if users.remove(&uid).is_some() {
            self.total_users.fetch_sub(1, Ordering::Relaxed);
            crate::println!("[access_control] Deleted user with UID {}", uid);
            Ok(())
        } else {
            Err(AccessError::UserNotFound { uid })
        }
    }
    
    /// Check if user is root
    pub fn is_root(&self, uid: u32) -> bool {
        uid == self.root_uid
    }
    
    /// Get total users count
    pub fn total_users(&self) -> usize {
        self.total_users.load(Ordering::Relaxed)
    }
    
    /// Get all users
    pub fn get_all_users(&self) -> Vec<User> {
        let users = self.users.lock();
        users.values().cloned().collect()
    }
}

// ============================================================================
// Group Management
// ============================================================================

/// Group information
#[derive(Debug, Clone)]
pub struct Group {
    /// Group ID
    pub gid: u32,
    
    /// Group name
    pub name: String,
    
    /// Group members (UIDs)
    pub members: BTreeSet<u32>,
    
    /// Group permissions (for default file permissions)
    pub permissions: u16,
    
    /// Creation timestamp
    pub created_at: u64,
}

/// Group manager
pub struct GroupManager {
    /// All groups by GID
    groups: Mutex<BTreeMap<u32, Group>>,
    
    /// Next GID to allocate
    next_gid: AtomicU32,
    
    /// Root group GID (typically 0)
    pub root_gid: u32,
    
    /// Total groups count
    pub total_groups: AtomicUsize,
}

impl GroupManager {
    /// Create new group manager
    pub fn new() -> Self {
        Self {
            groups: Mutex::new(BTreeMap::new()),
            next_gid: AtomicU32::new(1), // Start from 1
            pub root_gid: 0,
            pub total_groups: AtomicUsize::new(0),
        }
    }
    
    /// Create new group
    pub fn create_group(&self, name: String, permissions: u16) -> Result<u32, AccessError> {
        let gid = self.next_gid.fetch_add(1, Ordering::Relaxed);
        
        let group = Group {
            gid,
            name,
            members: BTreeSet::new(),
            permissions,
            created_at: crate::subsystems::time::timestamp_nanos(),
        };
        
        let mut groups = self.groups.lock();
        groups.insert(gid, group);
        
        self.total_groups.fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[access_control] Created group '{}' with GID {}", name, gid);
        
        Ok(gid)
    }
    
    /// Get group by GID
    pub fn get_group(&self, gid: u32) -> Option<Group> {
        let groups = self.groups.lock();
        groups.get(&gid).cloned()
    }
    
    /// Get group by name
    pub fn get_group_by_name(&self, name: &str) -> Option<Group> {
        let groups = self.groups.lock();
        
        for group in groups.values() {
            if group.name == name {
                return Some(group.clone());
            }
        }
        
        None
    }
    
    /// Add user to group
    pub fn add_user_to_group(&self, uid: u32, gid: u32) -> Result<(), AccessError> {
        let mut groups = self.groups.lock();
        
        if let Some(group) = groups.get_mut(&gid) {
            group.members.insert(uid);
            
            crate::println!("[access_control] Added user {} to group {}", uid, gid);
            
            Ok(())
        } else {
            Err(AccessError::GroupNotFound { gid })
        }
    }
    
    /// Remove user from group
    pub fn remove_user_from_group(&self, uid: u32, gid: u32) -> Result<(), AccessError> {
        let mut groups = self.groups.lock();
        
        if let Some(group) = groups.get_mut(&gid) {
            group.members.remove(&uid);
            
            crate::println!("[access_control] Removed user {} from group {}", uid, gid);
            
            Ok(())
        } else {
            Err(AccessError::GroupNotFound { gid })
        }
    }
    
    /// Get total groups count
    pub fn total_groups(&self) -> usize {
        self.total_groups.load(Ordering::Relaxed)
    }
    
    /// Get all groups
    pub fn get_all_groups(&self) -> Vec<Group> {
        let groups = self.groups.lock();
        groups.values().cloned().collect()
    }
}

// ============================================================================
// Access Control List (ACL)
// ============================================================================

/// ACL entry
#[derive(Debug, Clone)]
pub struct AclEntry {
    /// Subject who has permission
    pub subject: AclSubject,
    
    /// Permission bits (read/write/execute)
    pub permissions: u16,
    
    /// Access type (allow/deny)
    pub access_type: AclAccess,
    
    /// Entry creation time
    pub created_at: u64,
}

/// ACL subject (who permission applies to)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AclSubject {
    /// Specific user
    User(u32),
    
    /// Specific group
    Group(u32),
    
    /// Any user (wildcard)
    AnyUser,
    
    /// Any group (wildcard)
    AnyGroup,
    
    /// Everyone
    Everyone,
}

/// ACL access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclAccess {
    /// Allow access
    Allow,
    
    /// Deny access
    Deny,
}

impl AclEntry {
    /// Create new allow entry
    pub fn new_allow(subject: AclSubject, permissions: u16) -> Self {
        Self {
            subject,
            permissions,
            access_type: AclAccess::Allow,
            created_at: crate::subsystems::time::timestamp_nanos(),
        }
    }
    
    /// Create new deny entry
    pub fn new_deny(subject: AclSubject, permissions: u16) -> Self {
        Self {
            subject,
            permissions,
            access_type: AclAccess::Deny,
            created_at: crate::subsystems::time::timestamp_nanos(),
        }
    }
    
    /// Check if subject matches
    pub fn subject_matches(&self, subject: AclSubject) -> bool {
        match (&self.subject, subject) {
            (AclSubject::Everyone, _) => true,
            (AclSubject::AnyUser, AclSubject::AnyUser) => true,
            (AclSubject::User(u1), AclSubject::User(u2)) => u1 == u2,
            (AclSubject::Group(g1), AclSubject::Group(g2)) => g1 == g2,
            (AclSubject::AnyGroup, AclSubject::AnyGroup) => true,
            (_, _) => false,
        }
    }
    
    /// Check if permissions are granted
    pub fn has_permission(&self, permissions: u16) -> bool {
        if self.access_type == AclAccess::Deny {
            false // Deny entries don't grant permissions
        } else {
            (self.permissions & permissions) == permissions
        }
    }
    
    /// Check permissions against required set
    pub fn check_permissions(&self, required: u16) -> bool {
        if self.access_type == AclAccess::Deny {
            false // Deny overrides all
        } else {
            (self.permissions & required) == required
        }
    }
}

/// ACL for an object
pub struct AccessControlList {
    /// ACL type
    pub acl_type: AclType,
    
    /// ACL entries
    pub entries: Vec<AclEntry>,
    
    /// Owner UID
    pub owner_uid: u32,
    
    /// Owner GID
    pub owner_gid: u32,
    
    /// Default permission for non-matching subjects
    pub default_permissions: u16,
}

/// ACL entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclType {
    /// Access ACL for filesystem object
    Access,
    
    /// Default ACL for new objects
    Default,
    
    /// Inherited ACL (inherited from parent directory)
    Inherited,
}

impl AccessControlList {
    /// Create new ACL
    pub fn new(owner_uid: u32, owner_gid: u32) -> Self {
        Self {
            acl_type: AclType::Default,
            entries: Vec::new(),
            owner_uid,
            owner_gid,
            default_permissions: PERM_READ | PERM_WRITE, // Default: read+write for owner
        }
    }
    
    /// Add allow entry
    pub fn add_allow(&mut self, subject: AclSubject, permissions: u16) {
        self.entries.push(AclEntry::new_allow(subject, permissions));
        crate::println!("[access_control] Added ACL entry: {:?} allowed {:#o}", 
                        subject, permissions);
    }
    
    /// Add deny entry
    pub fn add_deny(&mut self, subject: AclSubject, permissions: u16) {
        self.entries.push(AclEntry::new_deny(subject, permissions));
        crate::println!("[access_control] Added ACL entry: {:?} denied {:#o}", 
                        subject, permissions);
    }
    
    /// Remove entry by index
    pub fn remove_entry(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
        }
    }
    
    /// Check access for subject with requested permissions
    pub fn check_access(&self, subject: AclSubject, requested_perms: u16) -> AccessResult {
        // Check deny entries first
        for entry in &self.entries {
            if entry.subject_matches(subject) && entry.access_type == AclAccess::Deny {
                return AccessResult::Denied {
                    reason: String::from("Explicitly denied by ACL"),
                };
            }
        }
        
        // Check allow entries
        for entry in &self.entries {
            if entry.subject_matches(subject) && entry.access_type == AclAccess::Allow {
                if (entry.permissions & requested_perms) == requested_perms {
                    return AccessResult::Allowed;
                }
            }
        }
        
        // Check default permissions
        if (self.default_permissions & requested_perms) == requested_perms {
            // Check if subject matches owner
            match subject {
                AclSubject::User(uid) if *uid == self.owner_uid => {
                    return AccessResult::Allowed;
                }
                AclSubject::Group(gid) if *gid == self.owner_gid => {
                    return AccessResult::Allowed;
                }
                _ => {}
            }
        }
        
        AccessResult::Denied {
            reason: String::from("Permission denied by ACL"),
        }
    }
    
    /// Set ACL type
    pub fn set_acl_type(&mut self, acl_type: AclType) {
        self.acl_type = acl_type;
    }
}

/// Access check result
#[derive(Debug, Clone)]
pub enum AccessResult {
    /// Access allowed
    Allowed,
    
    /// Access denied
    Denied {
        reason: String,
    },
}

// ============================================================================
// Filesystem Permission Checker
// ============================================================================

/// Filesystem permission checker
pub struct FsPermissionChecker {
    /// User manager
    user_manager: Arc<UserManager>,
    
    /// Group manager
    group_manager: Arc<GroupManager>,
}

impl FsPermissionChecker {
    /// Create new permission checker
    pub fn new(user_manager: Arc<UserManager>, group_manager: Arc<GroupManager>) -> Self {
        Self {
            user_manager,
            group_manager,
        }
    }
    
    /// Check file read permission
    pub fn check_read(&self, uid: u32, gid: u32, file_mode: u16, acl: Option<&AccessControlList>) 
        -> AccessResult {
        
        // Check if user is root (root can read anything)
        if self.user_manager.is_root(uid) {
            return AccessResult::Allowed;
        }
        
        // Check file mode
        if (file_mode & PERM_READ) == 0 {
            return AccessResult::Denied {
                reason: String::from("File not readable (mode bit not set)"),
            };
        }
        
        // Check ACL if present
        if let Some(acl) = acl {
            let result = acl.check_access(AclSubject::User(uid), PERM_READ);
            if !matches!(result, AccessResult::Allowed) {
                return result;
            }
        }
        
        // Check if user owns the file or is in owning group
        let file_owner_gid = (file_mode & 0x1FF) >> 16; // Extract GID from file mode (simplified)
        
        if uid == acl.owner_uid || (file_owner_gid == gid) {
            // File is owned by user or user's group
            // Check if group has read permission
            if let Some(group) = self.group_manager.get_group(gid) {
                if (group.permissions & PERM_READ) != 0 {
                    return AccessResult::Allowed;
                }
            }
        }
        
        AccessResult::Denied {
            reason: String::from("Read permission denied"),
        }
    }
    
    /// Check file write permission
    pub fn check_write(&self, uid: u32, gid: u32, file_mode: u16, acl: Option<&AccessControlList>) 
        -> AccessResult {
        
        // Check if user is root
        if self.user_manager.is_root(uid) {
            return AccessResult::Allowed;
        }
        
        // Check file mode
        if (file_mode & PERM_WRITE) == 0 {
            return AccessResult::Denied {
                reason: String::from("File not writable (mode bit not set)"),
            };
        }
        
        // Check sticky bit (only owner can write)
        if (file_mode & PERM_STICKY) != 0 {
            let file_owner_gid = (file_mode & 0x1FF) >> 16;
            if uid != acl.owner_uid && file_owner_gid != gid {
                return AccessResult::Denied {
                    reason: String::from("Sticky bit set: only owner can write"),
                };
            }
        }
        
        // Check ACL
        if let Some(acl) = acl {
            let result = acl.check_access(AclSubject::User(uid), PERM_WRITE);
            if !matches!(result, AccessResult::Allowed) {
                return result;
            }
        }
        
        // Check ownership
        let file_owner_gid = (file_mode & 0x1FF) >> 16;
        
        if uid == acl.owner_uid || (file_owner_gid == gid) {
            // File is owned by user or user's group
            // Check if group has write permission
            if let Some(group) = self.group_manager.get_group(gid) {
                if (group.permissions & PERM_WRITE) != 0 {
                    return AccessResult::Allowed;
                }
            }
        }
        
        AccessResult::Denied {
            reason: String::from("Write permission denied"),
        }
    }
    
    /// Check file execute permission
    pub fn check_execute(&self, uid: u32, gid: u32, file_mode: u16, acl: Option<&AccessControlList>) 
        -> AccessResult {
        
        // Check if user is root
        if self.user_manager.is_root(uid) {
            return AccessResult::Allowed;
        }
        
        // Check file mode
        if (file_mode & PERM_EXECUTE) == 0 {
            return AccessResult::Denied {
                reason: String::from("File not executable (mode bit not set)"),
            };
        }
        
        // Check SUID/SGID bits
        if (file_mode & PERM_SETUID) != 0 {
            // File will execute as owner - allow
            return AccessResult::Allowed;
        }
        
        if (file_mode & PERM_SETGID) != 0 {
            // File will execute as group - check if user is in group
            if let Some(group) = self.group_manager.get_group(gid) {
                if group.members.contains(&uid) {
                    return AccessResult::Allowed;
                }
            }
        }
        
        // Check ACL
        if let Some(acl) = acl {
            let result = acl.check_access(AclSubject::User(uid), PERM_EXECUTE);
            if !matches!(result, AccessResult::Allowed) {
                return result;
            }
        }
        
        // Check ownership
        let file_owner_gid = (file_mode & 0x1FF) >> 16;
        
        if uid == acl.owner_uid || (file_owner_gid == gid) {
            // File is owned by user or user's group
            // Check if group has execute permission
            if let Some(group) = self.group_manager.get_group(gid) {
                if (group.permissions & PERM_EXECUTE) != 0 {
                    return AccessResult::Allowed;
                }
            }
        }
        
        AccessResult::Denied {
            reason: String::from("Execute permission denied"),
        }
    }
}

// ============================================================================
// Access Error Types
// ============================================================================

/// Access control error types
#[derive(Debug, Clone)]
pub enum AccessError {
    /// User not found
    UserNotFound {
        uid: u32,
    },
    
    /// Group not found
    GroupNotFound {
        gid: u32,
    },
    
    /// Permission denied
    PermissionDenied {
        message: String,
    },
    
    /// Invalid arguments
    InvalidArguments,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_bits() {
        let rw = PERM_READ | PERM_WRITE;
        let rwx = PERM_READ | PERM_WRITE | PERM_EXECUTE;
        
        assert_eq!(rw, 0o006);
        assert_eq!(rwx, 0o007);
    }

    #[test]
    fn test_user_creation() {
        let manager = UserManager::new();
        
        let uid = manager.create_user(
            String::from("testuser"),
            String::from("/home/testuser"),
            String::from("/bin/sh"),
            UserFlags::default(),
            false
        ).unwrap();
        
        assert_eq!(manager.total_users(), 1);
        
        let user = manager.get_user(uid).unwrap();
        assert_eq!(user.name, "testuser");
    }

    #[test]
    fn test_group_creation() {
        let manager = GroupManager::new();
        
        let gid = manager.create_group(
            String::from("testgroup"),
            PERM_READ | PERM_WRITE
        ).unwrap();
        
        assert_eq!(manager.total_groups(), 1);
        
        let group = manager.get_group(gid).unwrap();
        assert_eq!(group.name, "testgroup");
    }

    #[test]
    fn test_acl_subject_matching() {
        let user_entry = AclEntry::new_allow(AclSubject::User(123), PERM_READ);
        
        assert!(user_entry.subject_matches(AclSubject::User(123)));
        assert!(!user_entry.subject_matches(AclSubject::User(456)));
        
        assert!(user_entry.subject_matches(AclSubject::Everyone));
        assert!(user_entry.subject_matches(AclSubject::AnyUser));
    }

    #[test]
    fn test_acl_permission_check() {
        let mut acl = AccessControlList::new(0, 0);
        acl.add_allow(AclSubject::User(123), PERM_READ);
        acl.add_deny(AclSubject::User(456), PERM_WRITE);
        
        // Check allow
        assert!(matches!(acl.check_access(AclSubject::User(123), PERM_READ), 
                         AccessResult::Allowed));
        
        // Check deny
        assert!(matches!(acl.check_access(AclSubject::User(456), PERM_WRITE), 
                         AccessResult::Denied { .. }));
    }

    #[test]
    fn test_fs_permission_check() {
        let user_mgr = Arc::new(UserManager::new());
        let group_mgr = Arc::new(GroupManager::new());
        
        // Create group with read permission
        group_mgr.create_group(String::from("testgroup"), PERM_READ).unwrap();
        let gid = 0; // Will be assigned
        
        // Create user and add to group
        let uid = user_mgr.create_user(
            String::from("testuser"),
            String::from("/home/testuser"),
            String::from("/bin/sh"),
            UserFlags::default(),
            false
        ).unwrap();
        
        let mut groups = group_mgr.groups.lock();
        for (_, group) in groups.iter_mut() {
            group.members.insert(uid);
            break;
        }
        
        let checker = FsPermissionChecker::new(user_mgr, group_mgr);
        
        // Create ACL for file
        let mut acl = AccessControlList::new(uid, gid);
        acl.add_allow(AclSubject::User(uid), PERM_READ | PERM_WRITE);
        
        // Check read permission (user is in group, group has read)
        let result = checker.check_read(uid, gid, 0o644, Some(&acl));
        assert!(matches!(result, AccessResult::Allowed));
        
        // Check write permission (user has write permission)
        let result = checker.check_write(uid, gid, 0o644, Some(&acl));
        assert!(matches!(result, AccessResult::Allowed));
        
        // Check execute permission (no execute permission)
        let result = checker.check_execute(uid, gid, 0o755, Some(&acl));
        assert!(matches!(result, AccessResult::Denied { .. }));
    }
}

// ============================================================================
// Process Permission Inheritance
// ============================================================================

/// Process permissions
#[derive(Debug, Clone)]
pub struct ProcessPermissions {
    /// Effective UID
    pub euid: u32,
    
    /// Real UID
    pub ruid: u32,
    
    /// Saved set-user ID
    pub suid: u32,
    
    /// Effective GID
    pub egid: u32,
    
    /// Real GID
    pub rgid: u32,
    
    /// Saved set-group ID
    pub sgid: u32,
    
    /// Supplementary groups
    pub groups: Vec<u32>,
    
    /// Capability set
    pub capabilities: CapabilitySet,
    
    /// Inherited ACL entries
    pub inherited_acls: Vec<AclEntry>,
    
    /// Inheritance flags
    pub inheritance_flags: ProcessInheritFlags,
}

/// Process inheritance flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessInheritFlags {
    /// Inherit umask from parent
    pub inherit_umask: bool,
    
    /// Inherit supplementary groups
    pub inherit_groups: bool,
    
    /// Inherit capabilities
    pub inherit_capabilities: bool,
    
    /// Inherit ACL entries
    pub inherit_acls: bool,
    
    /// Inherit file descriptors
    pub inherit_fds: bool,
    
    /// Inherit signal disposition
    pub inherit_signals: bool,
    
    /// Clear capabilities on exec
    pub clear_capabilities_on_exec: bool,
    
    /// Keep capabilities on exec
    pub keep_capabilities_on_exec: bool,
}

impl Default for ProcessInheritFlags {
    fn default() -> Self {
        Self {
            inherit_umask: true,
            inherit_groups: true,
            inherit_capabilities: false, // Default: don't inherit capabilities
            inherit_acls: false,      // Default: don't inherit ACLs
            inherit_fds: true,
            inherit_signals: true,
            clear_capabilities_on_exec: false,
            keep_capabilities_on_exec: false,
        }
    }
}

impl ProcessPermissions {
    /// Create new process permissions
    pub fn new(euid: u32, ruid: u32, egid: u32, rgid: u32) -> Self {
        Self {
            euid,
            ruid,
            suid: 0,
            egid,
            rgid,
            sgid: 0,
            groups: Vec::new(),
            capabilities: CapabilitySet::new(),
            inherited_acls: Vec::new(),
            inheritance_flags: ProcessInheritFlags::default(),
        }
    }
    
    /// Create from parent process permissions
    pub fn from_parent(parent: &ProcessPermissions, flags: ProcessInheritFlags) -> Self {
        let mut capabilities = CapabilitySet::new();
        
        // Inherit capabilities if requested
        if flags.inherit_capabilities {
            // Copy capabilities from parent
            for cap_desc in parent.capabilities.all() {
                // In simplified version, just grant capability without full descriptor
                // In real implementation, would copy all metadata
            }
        }
        
        // Inherit ACL entries if requested
        let inherited_acls = if flags.inherit_acls {
            parent.inherited_acls.clone()
        } else {
            Vec::new()
        };
        
        // Inherit groups if requested
        let groups = if flags.inherit_groups {
            parent.groups.clone()
        } else {
            Vec::new()
        };
        
        Self {
            euid: parent.euid,  // Child inherits effective UID from parent
            ruid: parent.ruid,  // Child inherits real UID from parent
            suid: 0,              // Saved set-user ID is cleared
            egid: parent.egid,  // Child inherits effective GID from parent
            rgid: parent.rgid,  // Child inherits real GID from parent
            sgid: 0,              // Saved set-group ID is cleared
            groups,
            capabilities,
            inherited_acls,
            inheritance_flags: flags,
        }
    }
    
    /// Set effective UID (requires CAP_SETUID)
    pub fn seteuid(&mut self, euid: u32, cap_check: impl Fn(u32) -> bool) -> bool {
        if !cap_check(self.euid) {
            return false;
        }
        
        self.euid = euid;
        crate::println!("[access_control] Process seteuid to {}", euid);
        
        true
    }
    
    /// Set real UID (requires CAP_SETUID)
    pub fn setruid(&mut self, ruid: u32, cap_check: impl Fn(u32) -> bool) -> bool {
        if !cap_check(self.euid) {
            return false;
        }
        
        self.ruid = ruid;
        crate::println!("[access_control] Process setruid to {}", ruid);
        
        true
    }
    
    /// Add supplementary group
    pub fn add_group(&mut self, gid: u32) {
        if !self.groups.contains(&gid) {
            self.groups.push(gid);
        }
    }
    
    /// Remove supplementary group
    pub fn remove_group(&mut self, gid: u32) {
        self.groups.retain(|&g| g != gid);
    }
    
    /// Grant capability
    pub fn grant_capability(&mut self, capability: Capability, flags: CapabilityFlags) {
        self.capabilities.grant(capability, flags, self.euid, None);
    }
    
    /// Revoke capability
    pub fn revoke_capability(&mut self, capability: Capability) -> bool {
        self.capabilities.revoke(capability)
    }
    
    /// Check if process has capability
    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities.has(capability)
    }
    
    /// Inherit ACL entry
    pub fn inherit_acl(&mut self, entry: AclEntry) {
        if self.inheritance_flags.inherit_acls {
            self.inherited_acls.push(entry);
        }
    }
    
    /// Get effective UID
    pub fn euid(&self) -> u32 {
        self.euid
    }
    
    /// Get real UID
    pub fn ruid(&self) -> u32 {
        self.ruid
    }
    
    /// Get effective GID
    pub fn egid(&self) -> u32 {
        self.egid
    }
    
    /// Get real GID
    pub fn rgid(&self) -> u32 {
        self.rgid
    }
    
    /// Check if process is privileged (running as root)
    pub fn is_privileged(&self) -> bool {
        self.euid == 0
    }
}

// ============================================================================
// Security Policy Engine
// ============================================================================

/// Security policy rule
#[derive(Debug, Clone)]
pub struct SecurityPolicyRule {
    /// Rule ID
    pub rule_id: u32,
    
    /// Rule priority (higher = checked first)
    pub priority: u32,
    
    /// Subject selector (who the rule applies to)
    pub subject: PolicySubject,
    
    /// Resource selector (what the rule applies to)
    pub resource: PolicyResource,
    
    /// Action to take
    pub action: PolicyAction,
    
    /// Rule conditions (must all be true)
    pub conditions: Vec<PolicyCondition>,
    
    /// Rule enabled flag
    pub enabled: bool,
    
    /// Rule description
    pub description: String,
    
    /// Rule creation time
    pub created_at: u64,
}

/// Policy subject (who)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySubject {
    /// All subjects
    All,
    
    /// Specific user
    User(u32),
    
    /// Specific group
    Group(u32),
    
    /// Process with specific capability
    HasCapability(Capability),
    
    /// Process running as root
    Root,
    
    /// Process with specific PID
    Process(usize),
}

/// Policy resource (what)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResource {
    /// All resources
    All,
    
    /// Filesystem resource
    Filesystem {
        path_pattern: String,
        operation: FileOperation,
    },
    
    /// Network resource
    Network {
        address_pattern: String,
        port: Option<u16>,
    },
    
    /// Process resource
    Process {
        pid: usize,
        operation: ProcessOperation,
    },
    
    /// System call
    Syscall {
        syscall_number: u32,
    },
}

/// File operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOperation {
    Read,
    Write,
    Execute,
    Delete,
    Create,
}

/// Process operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOperation {
    Create,
    Kill,
    Modify,
    Signal,
}

/// Policy actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    /// Allow operation
    Allow,
    
    /// Deny operation
    Deny,
    
    /// Audit operation (log but allow)
    Audit,
    
    /// Ask user for permission
    Ask,
    
    /// Quarantine resource
    Quarantine,
}

/// Policy condition
#[derive(Debug, Clone)]
pub struct PolicyCondition {
    /// Condition type
    pub condition_type: ConditionType,
    
    /// Condition value
    pub value: ConditionValue,
}

/// Condition types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionType {
    /// Time of day (hour)
    TimeOfDay,
    
    /// Day of week (0-6, Sunday=0)
    DayOfWeek,
    
    /// Time range (start-end seconds since epoch)
    TimeRange,
    
    /// Network location (local/remote)
    NetworkLocation,
    
    /// User location (physical)
    UserLocation,
    
    /// System load threshold
    SystemLoad,
}

/// Condition value
#[derive(Debug, Clone)]
pub enum ConditionValue {
    /// Integer value
    Integer(i64),
    
    /// String value
    String(String),
    
    /// Range (start-end)
    Range(u64, u64),
    
    /// Set of values
    Set(alloc::collections::BTreeSet<String>),
}

/// Security policy
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    
    /// Policy rules
    pub rules: Mutex<Vec<SecurityPolicyRule>>,
    
    /// Policy enabled flag
    pub enabled: AtomicBool,
    
    /// Default action when no rules match
    pub default_action: PolicyAction,
    
    /// Total evaluations
    pub total_evaluations: AtomicU64,
    
    /// Total allows
    pub total_allows: AtomicU64,
    
    /// Total denies
    pub total_denies: AtomicU64,
}

impl SecurityPolicy {
    /// Create new security policy
    pub fn new(name: String, default_action: PolicyAction) -> Self {
        Self {
            name,
            rules: Mutex::new(Vec::new()),
            enabled: AtomicBool::new(true),
            default_action,
            total_evaluations: AtomicU64::new(0),
            total_allows: AtomicU64::new(0),
            total_denies: AtomicU64::new(0),
        }
    }
    
    /// Add policy rule
    pub fn add_rule(&self, rule: SecurityPolicyRule) {
        let mut rules = self.rules.lock();
        rules.push(rule);
        
        // Sort by priority (higher first)
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        crate::println!("[access_control] Added rule {} to policy '{}'",
                        rule.rule_id, self.name);
    }
    
    /// Remove policy rule by ID
    pub fn remove_rule(&self, rule_id: u32) -> bool {
        let mut rules = self.rules.lock();
        let initial_len = rules.len();
        
        rules.retain(|r| r.rule_id != rule_id);
        
        let removed = rules.len() < initial_len;
        
        if removed {
            crate::println!("[access_control] Removed rule {} from policy '{}'",
                            rule_id, self.name);
        }
        
        removed
    }
    
    /// Enable/disable policy
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
        
        crate::println!("[access_control] Policy '{}' {}",
                        self.name, if enabled { "enabled" } else { "disabled" });
    }
    
    /// Check if policy is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
    
    /// Evaluate policy against a request
    pub fn evaluate(&self, subject: PolicySubject, resource: PolicyResource) -> PolicyAction {
        // Check if policy is enabled
        if !self.is_enabled() {
            return PolicyAction::Allow; // Default to allow if disabled
        }
        
        self.total_evaluations.fetch_add(1, Ordering::Relaxed);
        
        let rules = self.rules.lock();
        
        // Check each rule in priority order
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }
            
            // Check if subject matches
            if !self.subject_matches(rule, &subject) {
                continue;
            }
            
            // Check if resource matches
            if !self.resource_matches(rule, &resource) {
                continue;
            }
            
            // Check conditions (must all be true)
            if !self.conditions_satisfied(rule) {
                continue;
            }
            
            // Rule matched - return action
            let action = rule.action;
            
            match action {
                PolicyAction::Allow => {
                    self.total_allows.fetch_add(1, Ordering::Relaxed);
                }
                PolicyAction::Deny => {
                    self.total_denies.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
            
            crate::println!("[access_control] Policy '{}' matched rule {} with action {:?}",
                            self.name, rule.rule_id, action);
            
            return action;
        }
        
        // No rule matched - return default action
        self.default_action
    }
    
    /// Check if subject matches rule subject
    fn subject_matches(&self, rule: &SecurityPolicyRule, subject: &PolicySubject) -> bool {
        match (&rule.subject, subject) {
            (PolicySubject::All, _) => true,
            
            (PolicySubject::User(r1), PolicySubject::User(r2)) => r1 == r2,
            
            (PolicySubject::Group(g1), PolicySubject::Group(g2)) => g1 == g2,
            
            (PolicySubject::HasCapability(_), PolicySubject::HasCapability(_)) => {
                // Would check actual capabilities
                true
            }
            
            (PolicySubject::Root, PolicySubject::Root) => true,
            
            (PolicySubject::Root, _) => false, // Rule requires root, subject isn't
            
            (_, PolicySubject::Root) => false, // Subject is root, rule doesn't require
            
            (PolicySubject::Process(p1), PolicySubject::Process(p2)) => p1 == p2,
            
            (_, _) => false,
        }
    }
    
    /// Check if resource matches rule resource
    fn resource_matches(&self, rule: &SecurityPolicyRule, resource: &PolicyResource) -> bool {
        match (&rule.resource, resource) {
            (PolicyResource::All, _) => true,
            
            (PolicyResource::Filesystem { path_pattern: _, operation: op1 },
             PolicyResource::Filesystem { path_pattern: _, operation: op2 }) => op1 == op2,
            
            (PolicyResource::Network { address_pattern: _, port: p1 },
             PolicyResource::Network { address_pattern: _, port: p2 }) => {
                p1 == p2 || p1.is_none() || p2.is_none()
            }
            
            (PolicyResource::Process { pid: p1, operation: op1 },
             PolicyResource::Process { pid: p2, operation: op2 }) => {
                p1 == p2 && op1 == op2
            }
            
            (PolicyResource::Syscall { syscall_number: s1 },
             PolicyResource::Syscall { syscall_number: s2 }) => s1 == s2,
            
            (_, _) => false,
        }
    }
    
    /// Check if all conditions are satisfied
    fn conditions_satisfied(&self, rule: &SecurityPolicyRule) -> bool {
        for condition in &rule.conditions {
            if !self.condition_satisfied(condition) {
                return false;
            }
        }
        
        true
    }
    
    /// Check if a single condition is satisfied
    fn condition_satisfied(&self, condition: &PolicyCondition) -> bool {
        match &condition.condition_type {
            ConditionType::TimeOfDay => {
                // Get current hour
                let timestamp = crate::subsystems::time::timestamp_nanos();
                let hours = (timestamp / 3_600_000_000_000) % 24;
                
                match &condition.value {
                    ConditionValue::Integer(h) => hours as i64 == *h,
                    _ => false,
                }
            }
            
            ConditionType::DayOfWeek => {
                // Get current day of week
                let timestamp = crate::subsystems::time::timestamp_nanos();
                let days = (timestamp / 86_400_000_000_000) % 7;
                
                match &condition.value {
                    ConditionValue::Integer(d) => days as i64 == *d,
                    ConditionValue::Set(days_set) => {
                        let day_str = { let mut s = alloc::string::String::from(""); s.push_str(&days.to_string()); s };
                        days_set.contains(&day_str)
                    }
                    _ => false,
                }
            }
            
            ConditionType::TimeRange => {
                match &condition.value {
                    ConditionValue::Range(start, end) => {
                        let timestamp = crate::subsystems::time::timestamp_nanos();
                        timestamp >= *start && timestamp < *end
                    }
                    _ => false,
                }
            }
            
            ConditionType::SystemLoad => {
                // In real implementation, would get actual system load
                true
            }
            
            ConditionType::NetworkLocation | ConditionType::UserLocation => {
                // In real implementation, would check actual location
                true
            }
        }
    }
    
    /// Get policy statistics
    pub fn get_stats(&self) -> PolicyStats {
        PolicyStats {
            enabled: self.is_enabled(),
            total_rules: self.rules.lock().len(),
            total_evaluations: self.total_evaluations.load(Ordering::Relaxed),
            total_allows: self.total_allows.load(Ordering::Relaxed),
            total_denies: self.total_denies.load(Ordering::Relaxed),
        }
    }
}

/// Policy statistics
#[derive(Debug, Clone, Copy)]
pub struct PolicyStats {
    pub enabled: bool,
    pub total_rules: usize,
    pub total_evaluations: u64,
    pub total_allows: u64,
    pub total_denies: u64,
}

// ============================================================================
// Security Policy Manager
// ============================================================================

/// Global security policy manager
pub struct SecurityPolicyManager {
    /// All security policies
    policies: Mutex<BTreeMap<String, Arc<SecurityPolicy>>>,
    
    /// Active policy name (if any)
    active_policy: Mutex<Option<String>>,
    
    /// Default policy
    default_policy: Option<Arc<SecurityPolicy>>,
    
    /// Total policy evaluations
    total_evaluations: AtomicU64,
}

impl SecurityPolicyManager {
    /// Create new policy manager
    pub fn new() -> Self {
        Self {
            policies: Mutex::new(BTreeMap::new()),
            active_policy: Mutex::new(None),
            default_policy: None,
            total_evaluations: AtomicU64::new(0),
        }
    }
    
    /// Add policy
    pub fn add_policy(&self, policy: Arc<SecurityPolicy>) -> Result<(), AccessError> {
        let name = policy.name.clone();
        
        let mut policies = self.policies.lock();
        
        if policies.contains_key(&name) {
            return Err(AccessError::PermissionDenied {
                message: alloc::string::String::from("Policy '") + &name.to_string() + alloc::string::String::from("' already exists"),
            });
        }
        
        policies.insert(name, policy);
        
        crate::println!("[access_control] Added policy '{}'", name);
        
        Ok(())
    }
    
    /// Remove policy
    pub fn remove_policy(&self, name: String) -> Result<(), AccessError> {
        let mut policies = self.policies.lock();
        
        if policies.remove(&name).is_some() {
            // Check if this was the active policy
            let mut active = self.active_policy.lock();
            if active.as_ref().map(|n| n == &name) == Some(true) {
                *active = None;
            }
            
            crate::println!("[access_control] Removed policy '{}'", name);
            
            Ok(())
        } else {
            Err(AccessError::PermissionDenied {
                message: alloc::string::String::from("Policy '") + &name.to_string() + alloc::string::String::from("' not found"),
            })
        }
    }
    
    /// Set active policy
    pub fn set_active_policy(&self, name: String) -> Result<(), AccessError> {
        let policies = self.policies.lock();
        
        if policies.contains_key(&name) {
            let mut active = self.active_policy.lock();
            *active = Some(name);
            
            crate::println!("[access_control] Set active policy to '{}'", name);
            
            Ok(())
        } else {
            Err(AccessError::PermissionDenied {
                message: alloc::string::String::from("Policy '") + &name.to_string() + alloc::string::String::from("' not found"),
            })
        }
    }
    
    /// Get active policy
    pub fn get_active_policy(&self) -> Option<Arc<SecurityPolicy>> {
        let active = self.active_policy.lock();
        
        if let Some(name) = active.as_ref() {
            let policies = self.policies.lock();
            policies.get(name).cloned()
        } else {
            self.default_policy.clone()
        }
    }
    
    /// Evaluate all active policies
    pub fn evaluate(&self, subject: PolicySubject, resource: PolicyResource) -> PolicyAction {
        self.total_evaluations.fetch_add(1, Ordering::Relaxed);
        
        // Get active policy
        if let Some(policy) = self.get_active_policy() {
            policy.evaluate(subject, resource)
        } else {
            // No active policy - allow by default
            PolicyAction::Allow
        }
    }
    
    /// Get all policies
    pub fn get_all_policies(&self) -> Vec<String> {
        let policies = self.policies.lock();
        policies.keys().cloned().collect()
    }
}
