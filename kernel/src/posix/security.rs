//! POSIX Security and Permissions Management
//!
//! This module implements POSIX security and permission features including:
//! - capget() / capset() - Capability management
//! - getpwnam() / getpwuid() - Password database queries
//! - getgrnam() / getgrgid() - Group database queries
//! - setuid() / setgid() - Set user/group ID
//! - seteuid() / setegid() - Set effective user/group ID
//! - setreuid() / setregid() - Set real/effective user/group ID

use crate::posix::{Uid, Gid, Pid, Mode};
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

/// POSIX capability structure
#[derive(Debug, Clone, Copy)]
pub struct CapHeader {
    /// Capability version
    pub version: u32,
    /// Number of capability entries
    pub pid: u32,
}

/// Capability data structure
#[derive(Debug, Clone, Copy)]
pub struct CapData {
    /// Effective capabilities
    pub effective: u32,
    /// Permitted capabilities
    pub permitted: u32,
    /// Inheritable capabilities
    pub inheritable: u32,
}

impl Default for CapData {
    fn default() -> Self {
        Self {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }
    }
}

/// Capability constants (Linux capability format)
pub const CAP_CHOWN: u32 = 0;            // Change file ownership
pub const CAP_DAC_OVERRIDE: u32 = 1;       // Override DAC access
pub const CAP_DAC_READ_SEARCH: u32 = 2;    // Read/search files/directories
pub const CAP_FOWNER: u32 = 3;            // Change file ownership
pub const CAP_FSETID: u32 = 4;            // Set file set ID
pub const CAP_KILL: u32 = 5;               // Kill processes
pub const CAP_SETGID: u32 = 6;             // Set group ID
pub const CAP_SETUID: u32 = 7;             // Set user ID
pub const CAP_SETPCAP: u32 = 8;            // Set process capabilities
pub const CAP_LINUX_IMMUTABLE: u32 = 9;    // Linux immutable
pub const CAP_NET_BIND_SERVICE: u32 = 10;   // Bind to privileged ports
pub const CAP_NET_BROADCAST: u32 = 11;    // Broadcast packets
pub const CAP_NET_ADMIN: u32 = 12;         // Network administration
pub const CAP_NET_RAW: u32 = 13;            // Use raw sockets
pub const CAP_IPC_LOCK: u32 = 14;          // Lock IPC resources
pub const CAP_IPC_OWNER: u32 = 15;         // Own IPC resources
pub const CAP_SYS_MODULE: u32 = 16;        // Load kernel modules
pub const CAP_SYS_RAWIO: u32 = 17;         // Raw I/O operations
pub const CAP_SYS_CHROOT: u32 = 18;        // Change root directory
pub const CAP_SYS_PTRACE: u32 = 19;        // Trace processes
pub const CAP_SYS_PACCT: u32 = 20;         // Process accounting
pub const CAP_SYS_ADMIN: u32 = 21;         // System administration
pub const CAP_SYS_BOOT: u32 = 22;          // Boot system
pub const CAP_SYS_NICE: u32 = 23;          // Change process priority
pub const CAP_SYS_RESOURCE: u32 = 24;       // Resource limits
pub const CAP_SYS_TIME: u32 = 25;          // Set system time
pub const CAP_SYS_TTY_CONFIG: u32 = 26;    // Configure terminals
pub const CAP_MKNOD: u32 = 27;             // Create device nodes
pub const CAP_LEASE: u32 = 28;              // File leases
pub const CAP_AUDIT_WRITE: u32 = 29;        // Write audit logs
pub const CAP_AUDIT_CONTROL: u32 = 30;       // Control audit subsystem
pub const CAP_SETFCAP: u32 = 31;           // Set file capabilities

/// Password database entry
#[derive(Debug, Clone)]
pub struct PasswdEntry {
    /// Username
    pub pw_name: String,
    /// User password (encrypted)
    pub pw_passwd: String,
    /// User ID
    pub pw_uid: Uid,
    /// Group ID
    pub pw_gid: Gid,
    /// User information (GECOS field)
    pub pw_gecos: String,
    /// Home directory
    pub pw_dir: String,
    /// Login shell
    pub pw_shell: String,
}

impl PasswdEntry {
    /// Create a new password entry
    pub fn new(name: &str, uid: Uid, gid: Gid) -> Self {
        Self {
            pw_name: name.to_string(),
            pw_passwd: "x".to_string(), // Shadow password indicator
            pw_uid: uid,
            pw_gid: gid,
            pw_gecos: "".to_string(),
            pw_dir: format!("/home/{}", name),
            pw_shell: "/bin/sh".to_string(),
        }
    }

    /// Create a root password entry
    pub fn root() -> Self {
        Self::new("root", 0, 0)
    }

    /// Create a guest password entry
    pub fn guest() -> Self {
        Self::new("guest", 999, 999)
    }

    /// Create a nobody password entry
    pub fn nobody() -> Self {
        Self::new("nobody", 65534, 65534)
    }
}

/// Group database entry
#[derive(Debug, Clone)]
pub struct GroupEntry {
    /// Group name
    pub gr_name: String,
    /// Group password
    pub gr_passwd: String,
    /// Group ID
    pub gr_gid: Gid,
    /// Group members
    pub gr_mem: Vec<String>,
}

impl GroupEntry {
    /// Create a new group entry
    pub fn new(name: &str, gid: Gid) -> Self {
        Self {
            gr_name: name.to_string(),
            gr_passwd: "x".to_string(), // Shadow password indicator
            gr_gid: gid,
            gr_mem: Vec::new(),
        }
    }

    /// Create a root group entry
    pub fn root() -> Self {
        Self::new("root", 0)
    }

    /// Create a wheel group entry
    pub fn wheel() -> Self {
        let mut entry = Self::new("wheel", 1);
        entry.gr_mem.push("root".to_string());
        entry
    }

    /// Create a nobody group entry
    pub fn nobody() -> Self {
        Self::new("nobody", 65534)
    }

    /// Add a member to the group
    pub fn add_member(&mut self, username: &str) {
        if !self.gr_mem.contains(&username.to_string()) {
            self.gr_mem.push(username.to_string());
        }
    }
}

/// Process credentials structure
#[derive(Debug, Clone)]
pub struct ProcessCredentials {
    /// Real user ID
    pub real_uid: Uid,
    /// Effective user ID
    pub effective_uid: Uid,
    /// Saved set-user ID
    pub saved_uid: Uid,
    /// Real group ID
    pub real_gid: Gid,
    /// Effective group ID
    pub effective_gid: Gid,
    /// Saved set-group ID
    pub saved_gid: Gid,
    /// File creation mask
    pub umask: Mode,
    /// Capabilities
    pub capabilities: CapData,
}

impl ProcessCredentials {
    /// Create new process credentials
    pub fn new() -> Self {
        Self {
            real_uid: 0,
            effective_uid: 0,
            saved_uid: 0,
            real_gid: 0,
            effective_gid: 0,
            saved_gid: 0,
            umask: 0o022, // Default umask
            capabilities: CapData::default(),
        }
    }

    /// Set user IDs
    pub fn set_uids(&mut self, ruid: Uid, euid: Uid) {
        self.real_uid = ruid;
        self.effective_uid = euid;
    }

    /// Set group IDs
    pub fn set_gids(&mut self, rgid: Gid, egid: Gid) {
        self.real_gid = rgid;
        self.effective_gid = egid;
    }

    /// Save current IDs
    pub fn save_ids(&mut self) {
        self.saved_uid = self.effective_uid;
        self.saved_gid = self.effective_gid;
    }

    /// Restore saved IDs
    pub fn restore_ids(&mut self) {
        self.effective_uid = self.saved_uid;
        self.effective_gid = self.saved_gid;
    }

    /// Check if process has root privileges
    pub fn is_root(&self) -> bool {
        self.effective_uid == 0
    }

    /// Check if process has specific capability
    pub fn has_capability(&self, cap: u32) -> bool {
        (self.capabilities.effective & cap) != 0
    }

    /// Set file creation mask
    pub fn set_umask(&mut self, mask: Mode) {
        self.umask = mask & 0o777; // Only permission bits
    }

    /// Get file creation mask
    pub fn get_umask(&self) -> Mode {
        self.umask
    }
}

/// Security errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityError {
    /// Invalid capability
    InvalidCapability,
    /// Permission denied
    PermissionDenied,
    /// User not found
    UserNotFound,
    /// Group not found
    GroupNotFound,
    /// Invalid argument
    InvalidArgument,
    /// Operation not supported
    NotSupported,
    /// Resource busy
    ResourceBusy,
    /// Invalid credentials
    InvalidCredentials,
}

/// Global security registry
pub static SECURITY_REGISTRY: Mutex<SecurityRegistry> = Mutex::new(SecurityRegistry::new());

/// Security registry for managing system security
#[derive(Debug)]
pub struct SecurityRegistry {
    /// Map from process ID to credentials
    pub process_credentials: BTreeMap<Pid, ProcessCredentials>,
    /// Password database (username -> entry)
    pub password_db: BTreeMap<String, PasswdEntry>,
    /// Group database (name -> entry)
    pub group_db: BTreeMap<String, GroupEntry>,
    /// Next user ID to allocate
    pub next_uid: Uid,
    /// Next group ID to allocate
    pub next_gid: Gid,
}

impl SecurityRegistry {
    /// Create a new security registry
    pub const fn new() -> Self {
        Self {
            process_credentials: BTreeMap::new(),
            password_db: BTreeMap::new(),
            group_db: BTreeMap::new(),
            next_uid: 1000, // Start from 1000 to avoid conflicts
            next_gid: 1000,
        }
    }

    /// Initialize the registry with default entries
    pub fn init(&mut self) {
        // Add default password entries
        self.password_db.insert("root".to_string(), PasswdEntry::root());
        self.password_db.insert("guest".to_string(), PasswdEntry::guest());
        self.password_db.insert("nobody".to_string(), PasswdEntry::nobody());
        
        // Add default group entries
        self.group_db.insert("root".to_string(), GroupEntry::root());
        self.group_db.insert("wheel".to_string(), GroupEntry::wheel());
        self.group_db.insert("nogroup".to_string(), GroupEntry::nobody());
        
        crate::println!("[security] Security registry initialized with default users and groups");
    }

    /// Allocate a new user ID
    pub fn allocate_uid(&mut self) -> Uid {
        let uid = self.next_uid;
        self.next_uid += 1;
        uid
    }

    /// Allocate a new group ID
    pub fn allocate_gid(&mut self) -> Gid {
        let gid = self.next_gid;
        self.next_gid += 1;
        gid
    }

    /// Get process credentials
    pub fn get_process_credentials(&self, pid: Pid) -> Option<&ProcessCredentials> {
        self.process_credentials.get(&pid)
    }

    /// Set process credentials
    pub fn set_process_credentials(&mut self, pid: Pid, creds: ProcessCredentials) -> Result<(), SecurityError> {
        if self.process_credentials.contains_key(&pid) {
            return Err(SecurityError::ResourceBusy);
        }
        self.process_credentials.insert(pid, creds);
        Ok(())
    }

    /// Remove process credentials
    pub fn remove_process_credentials(&mut self, pid: Pid) -> Option<ProcessCredentials> {
        self.process_credentials.remove(&pid)
    }

    /// Get password entry by name
    pub fn getpwnam(&self, name: &str) -> Option<&PasswdEntry> {
        self.password_db.get(&name.to_string())
    }

    /// Get password entry by UID
    pub fn getpwuid(&self, uid: Uid) -> Option<&PasswdEntry> {
        self.password_db.values().find(|entry| entry.pw_uid == uid)
    }

    /// Get group entry by name
    pub fn getgrnam(&self, name: &str) -> Option<&GroupEntry> {
        self.group_db.get(&name.to_string())
    }

    /// Get group entry by GID
    pub fn getgrgid(&self, gid: Gid) -> Option<&GroupEntry> {
        self.group_db.values().find(|entry| entry.gr_gid == gid)
    }

    /// Add a password entry
    pub fn add_password_entry(&mut self, entry: PasswdEntry) -> Result<(), SecurityError> {
        if self.password_db.contains_key(&entry.pw_name) {
            return Err(SecurityError::ResourceBusy);
        }
        self.password_db.insert(entry.pw_name.clone(), entry);
        Ok(())
    }

    /// Add a group entry
    pub fn add_group_entry(&mut self, entry: GroupEntry) -> Result<(), SecurityError> {
        if self.group_db.contains_key(&entry.gr_name) {
            return Err(SecurityError::ResourceBusy);
        }
        self.group_db.insert(entry.gr_name.clone(), entry);
        Ok(())
    }

    /// Get security statistics
    pub fn get_stats(&self) -> SecurityStats {
        SecurityStats {
            total_processes: self.process_credentials.len(),
            total_users: self.password_db.len(),
            total_groups: self.group_db.len(),
            next_uid: self.next_uid,
            next_gid: self.next_gid,
        }
    }
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    /// Total number of processes with credentials
    pub total_processes: usize,
    /// Total number of users in password database
    pub total_users: usize,
    /// Total number of groups in group database
    pub total_groups: usize,
    /// Next user ID to be allocated
    pub next_uid: Uid,
    /// Next group ID to be allocated
    pub next_gid: Gid,
}

/// Get process capabilities
pub fn capget(pid: Pid, header: &mut CapHeader, data: &mut CapData) -> Result<(), SecurityError> {
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Get process credentials
    let creds = registry.get_process_credentials(pid)
        .ok_or(SecurityError::UserNotFound)?;
    
    // Fill header
    header.version = 0x20080522; // Linux capability version 3
    header.pid = 1;
    
    // Fill data
    *data = creds.capabilities;
    
    Ok(())
}

/// Set process capabilities
pub fn capset(pid: Pid, header: &CapHeader, data: &CapData) -> Result<(), SecurityError> {
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Check permissions
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SecurityError::PermissionDenied),
    };
    
    // Only root can set capabilities for other processes
    if pid != current_pid {
        let current_creds = registry.get_process_credentials(current_pid)
            .ok_or(SecurityError::UserNotFound)?;
        
        if !current_creds.is_root() {
            return Err(SecurityError::PermissionDenied);
        }
    }
    
    // Get and update process credentials
    let creds = registry.process_credentials.get_mut(&pid)
        .ok_or(SecurityError::UserNotFound)?;
    
    creds.capabilities = *data;
    
    Ok(())
}

/// Get password entry by name
pub fn getpwnam(name: &str) -> Result<PasswdEntry, SecurityError> {
    let registry = SECURITY_REGISTRY.lock();
    
    match registry.getpwnam(name) {
        Some(entry) => Ok(entry.clone()),
        None => Err(SecurityError::UserNotFound),
    }
}

/// Get password entry by UID
pub fn getpwuid(uid: Uid) -> Result<PasswdEntry, SecurityError> {
    let registry = SECURITY_REGISTRY.lock();
    
    match registry.getpwuid(uid) {
        Some(entry) => Ok(entry.clone()),
        None => Err(SecurityError::UserNotFound),
    }
}

/// Get group entry by name
pub fn getgrnam(name: &str) -> Result<GroupEntry, SecurityError> {
    let registry = SECURITY_REGISTRY.lock();
    
    match registry.getgrnam(name) {
        Some(entry) => Ok(entry.clone()),
        None => Err(SecurityError::GroupNotFound),
    }
}

/// Get group entry by GID
pub fn getgrgid(gid: Gid) -> Result<GroupEntry, SecurityError> {
    let registry = SECURITY_REGISTRY.lock();
    
    match registry.getgrgid(gid) {
        Some(entry) => Ok(entry.clone()),
        None => Err(SecurityError::GroupNotFound),
    }
}

/// Set real and effective user ID
pub fn setuid(ruid: Uid, euid: Uid) -> Result<(), SecurityError> {
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SecurityError::PermissionDenied),
    };
    
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Get or create process credentials
    let creds = if registry.process_credentials.contains_key(&current_pid) {
        registry.process_credentials.get_mut(&current_pid).unwrap()
    } else {
        let mut creds = ProcessCredentials::new();
        creds.set_uids(ruid, euid);
        registry.set_process_credentials(current_pid, creds).unwrap();
        creds
    };
    
    creds.set_uids(ruid, euid);
    Ok(())
}

/// Set real and effective group ID
pub fn setgid(rgid: Gid, egid: Gid) -> Result<(), SecurityError> {
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SecurityError::PermissionDenied),
    };
    
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Get or create process credentials
    let creds = if registry.process_credentials.contains_key(&current_pid) {
        registry.process_credentials.get_mut(&current_pid).unwrap()
    } else {
        let mut creds = ProcessCredentials::new();
        creds.set_gids(rgid, egid);
        registry.set_process_credentials(current_pid, creds).unwrap();
        creds
    };
    
    creds.set_gids(rgid, egid);
    Ok(())
}

/// Set effective user ID
pub fn seteuid(euid: Uid) -> Result<(), SecurityError> {
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SecurityError::PermissionDenied),
    };
    
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Get or create process credentials
    let creds = if registry.process_credentials.contains_key(&current_pid) {
        registry.process_credentials.get_mut(&current_pid).unwrap()
    } else {
        let mut creds = ProcessCredentials::new();
        creds.effective_uid = euid;
        registry.set_process_credentials(current_pid, creds).unwrap();
        creds
    };
    
    creds.effective_uid = euid;
    Ok(())
}

/// Set effective group ID
pub fn setegid(egid: Gid) -> Result<(), SecurityError> {
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SecurityError::PermissionDenied),
    };
    
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Get or create process credentials
    let creds = if registry.process_credentials.contains_key(&current_pid) {
        registry.process_credentials.get_mut(&current_pid).unwrap()
    } else {
        let mut creds = ProcessCredentials::new();
        creds.effective_gid = egid;
        registry.set_process_credentials(current_pid, creds).unwrap();
        creds
    };
    
    creds.effective_gid = egid;
    Ok(())
}

/// Set real and effective user ID
pub fn setreuid(ruid: Uid, euid: Uid) -> Result<(), SecurityError> {
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SecurityError::PermissionDenied),
    };
    
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Get or create process credentials
    let creds = if registry.process_credentials.contains_key(&current_pid) {
        registry.process_credentials.get_mut(&current_pid).unwrap()
    } else {
        let mut creds = ProcessCredentials::new();
        creds.set_uids(ruid, euid);
        creds.save_ids();
        registry.set_process_credentials(current_pid, creds).unwrap();
        creds
    };
    
    creds.set_uids(ruid, euid);
    Ok(())
}

/// Set real and effective group ID
pub fn setregid(rgid: Gid, egid: Gid) -> Result<(), SecurityError> {
    let current_pid = match crate::process::myproc() {
        Some(p) => p,
        None => return Err(SecurityError::PermissionDenied),
    };
    
    let mut registry = SECURITY_REGISTRY.lock();
    
    // Get or create process credentials
    let creds = if registry.process_credentials.contains_key(&current_pid) {
        registry.process_credentials.get_mut(&current_pid).unwrap()
    } else {
        let mut creds = ProcessCredentials::new();
        creds.set_gids(rgid, egid);
        creds.save_ids();
        registry.set_process_credentials(current_pid, creds).unwrap();
        creds
    };
    
    creds.set_gids(rgid, egid);
    Ok(())
}

/// Initialize POSIX security subsystem
pub fn init_security() {
    crate::println!("[security] Initializing POSIX security subsystem");
    
    let mut registry = SECURITY_REGISTRY.lock();
    registry.init();
    
    crate::println!("[security] POSIX security subsystem initialized");
    crate::println!("[security] Capability management enabled");
    crate::println!("[security] Password database queries enabled");
    crate::println!("[security] Group database queries enabled");
    crate::println!("[security] User/group ID management enabled");
}

/// Cleanup POSIX security subsystem
pub fn cleanup_security() {
    crate::println!("[security] Cleaning up POSIX security subsystem");
    
    let registry = SECURITY_REGISTRY.lock();
    let stats = registry.get_stats();
    
    crate::println!("[security] Cleanup stats:");
    crate::println!("[security]   Total processes: {}", stats.total_processes);
    crate::println!("[security]   Total users: {}", stats.total_users);
    crate::println!("[security]   Total groups: {}", stats.total_groups);
    crate::println!("[security]   Next UID: {}", stats.next_uid);
    crate::println!("[security]   Next GID: {}", stats.next_gid);
    
    // Note: We don't clear the registry here as it might be needed for cleanup
}