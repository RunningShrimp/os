// POSIX Capabilities Implementation
//
// This module implements POSIX capabilities to provide fine-grained privilege
// separation beyond the traditional root/user model.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// POSIX capability constants (Linux capability numbering)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Capability {
    /// Capability to change file ownership
    Chown = 0,
    /// Capability to override DAC restrictions
    DACOverride = 1,
    /// Capability to override file read/write/checks
    DACReadSearch = 2,
    /// Capability to override file access permissions
    Fowner = 3,
    /// Capability to override file creation/move/rename restrictions
    Fsetid = 4,
    /// Capability to kill processes
    Kill = 5,
    /// Capability to set group ID
    Setgid = 6,
    /// Capability to set user ID
    Setuid = 7,
    /// Capability to set process capabilities
    Setpcap = 8,
    /// Capability to override Linux Immutable/Append-only flags
    LinuxImmutable = 9,
    /// Capability to bind to privileged ports (<1024)
    NetBindService = 10,
    /// Capability to load kernel modules
    NetAdmin = 11,
    /// Capability to configure network interfaces
    NetRaw = 12,
    /// Capability to access IPC
    IpcOwner = 13,
    /// Capability to change system clock
    SysModule = 14,
    /// Capability to load/unload kernel modules
    SysRawio = 15,
    /// Capability to configure the kernel (sysctl, etc.)
    SysChroot = 16,
    /// Capability to configure process resource limits
    SysPtrace = 17,
    /// Capability to configure the system clock
    SysPacct = 18,
    /// Capability to manage system accounting
    SysAdmin = 19,
    /// Capability to configure the system clock
    SysBoot = 20,
    /// Capability to configure system time
    SysNice = 21,
    /// Capability to configure system resource limits
    SysResource = 22,
    /// Capability to configure system time
    SysTime = 23,
    /// Capability to configure TTY devices
    SysTtyConfig = 24,
    /// Capability to manage mknod devices
    Mknod = 25,
    /// Capability to lease files
    Lease = 26,
    /// Capability to override audit
    AuditWrite = 27,
    /// Capability to manage audit
    AuditControl = 28,
    /// Capability to set file attributes
    Setfcap = 29,
}

impl Capability {
    /// Get all capability values
    pub const ALL: [Capability; 30] = [
        Capability::Chown,
        Capability::DACOverride,
        Capability::DACReadSearch,
        Capability::Fowner,
        Capability::Fsetid,
        Capability::Kill,
        Capability::Setgid,
        Capability::Setuid,
        Capability::Setpcap,
        Capability::LinuxImmutable,
        Capability::NetBindService,
        Capability::NetAdmin,
        Capability::NetRaw,
        Capability::IpcOwner,
        Capability::SysModule,
        Capability::SysRawio,
        Capability::SysChroot,
        Capability::SysPtrace,
        Capability::SysPacct,
        Capability::SysAdmin,
        Capability::SysBoot,
        Capability::SysNice,
        Capability::SysResource,
        Capability::SysTime,
        Capability::SysTtyConfig,
        Capability::Mknod,
        Capability::Lease,
        Capability::AuditWrite,
        Capability::AuditControl,
        Capability::Setfcap,
    ];

    /// Get capability name
    pub fn name(&self) -> &'static str {
        match self {
            Capability::Chown => "CAP_CHOWN",
            Capability::DACOverride => "CAP_DAC_OVERRIDE",
            Capability::DACReadSearch => "CAP_DAC_READ_SEARCH",
            Capability::Fowner => "CAP_FOWNER",
            Capability::Fsetid => "CAP_FSETID",
            Capability::Kill => "CAP_KILL",
            Capability::Setgid => "CAP_SETGID",
            Capability::Setuid => "CAP_SETUID",
            Capability::Setpcap => "CAP_SETPCAP",
            Capability::LinuxImmutable => "CAP_LINUX_IMMUTABLE",
            Capability::NetBindService => "CAP_NET_BIND_SERVICE",
            Capability::NetAdmin => "CAP_NET_ADMIN",
            Capability::NetRaw => "CAP_NET_RAW",
            Capability::IpcOwner => "CAP_IPC_OWNER",
            Capability::SysModule => "CAP_SYS_MODULE",
            Capability::SysRawio => "CAP_SYS_RAWIO",
            Capability::SysChroot => "CAP_SYS_CHROOT",
            Capability::SysPtrace => "CAP_SYS_PTRACE",
            Capability::SysPacct => "CAP_SYS_PACCT",
            Capability::SysAdmin => "CAP_SYS_ADMIN",
            Capability::SysBoot => "CAP_SYS_BOOT",
            Capability::SysNice => "CAP_SYS_NICE",
            Capability::SysResource => "CAP_SYS_RESOURCE",
            Capability::SysTime => "CAP_SYS_TIME",
            Capability::SysTtyConfig => "CAP_SYS_TTY_CONFIG",
            Capability::Mknod => "CAP_MKNOD",
            Capability::Lease => "CAP_LEASE",
            Capability::AuditWrite => "CAP_AUDIT_WRITE",
            Capability::AuditControl => "CAP_AUDIT_CONTROL",
            Capability::Setfcap => "CAP_SETFCAP",
        }
    }
}

/// Capability sets (effective, permitted, inheritable)
#[derive(Debug, Clone)]
pub struct CapabilitySets {
    /// Effective capabilities - currently in effect
    pub effective: u32,
    /// Permitted capabilities - allowed to be effective
    pub permitted: u32,
    /// Inheritable capabilities - inherited across exec
    pub inheritable: u32,
    /// Bounding set - maximum capabilities that can be possessed
    pub bounding: u32,
    /// Ambient capabilities - automatically granted on exec
    pub ambient: u32,
}

impl Default for CapabilitySets {
    fn default() -> Self {
        Self {
            effective: 0,
            permitted: 0,
            inheritable: 0,
            bounding: u32::MAX,
            ambient: 0,
        }
    }
}

impl CapabilitySets {
    /// Create new capability sets
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a capability is in the effective set
    pub fn has_effective(&self, cap: Capability) -> bool {
        (self.effective & (1 << (cap as u32))) != 0
    }

    /// Add a capability to the effective set
    pub fn add_effective(&mut self, cap: Capability) {
        self.effective |= 1 << (cap as u32);
    }

    /// Remove a capability from the effective set
    pub fn remove_effective(&mut self, cap: Capability) {
        self.effective &= !(1 << (cap as u32));
    }

    /// Check if a capability is in the permitted set
    pub fn has_permitted(&self, cap: Capability) -> bool {
        (self.permitted & (1 << (cap as u32))) != 0
    }

    /// Add a capability to the permitted set
    pub fn add_permitted(&mut self, cap: Capability) {
        self.permitted |= 1 << (cap as u32);
    }

    /// Remove a capability from the permitted set
    pub fn remove_permitted(&mut self, cap: Capability) {
        self.permitted &= !(1 << (cap as u32));
    }

    /// Check if a capability is in the inheritable set
    pub fn has_inheritable(&self, cap: Capability) -> bool {
        (self.inheritable & (1 << (cap as u32))) != 0
    }

    /// Add a capability to the inheritable set
    pub fn add_inheritable(&mut self, cap: Capability) {
        self.inheritable |= 1 << (cap as u32);
    }

    /// Remove a capability from the inheritable set
    pub fn remove_inheritable(&mut self, cap: Capability) {
        self.inheritable &= !(1 << (cap as u32));
    }

    /// Check if a capability is in the bounding set
    pub fn has_bounding(&self, cap: Capability) -> bool {
        (self.bounding & (1 << (cap as u32))) != 0
    }

    /// Remove a capability from the bounding set
    pub fn remove_bounding(&mut self, cap: Capability) {
        self.bounding &= !(1 << (cap as u32));
    }

    /// Check if a capability is in the ambient set
    pub fn has_ambient(&self, cap: Capability) -> bool {
        (self.ambient & (1 << (cap as u32))) != 0
    }

    /// Add a capability to the ambient set
    pub fn add_ambient(&mut self, cap: Capability) {
        self.ambient |= 1 << (cap as u32);
    }

    /// Remove a capability from the ambient set
    pub fn remove_ambient(&mut self, cap: Capability) {
        self.ambient &= !(1 << (cap as u32));
    }

    /// Validate that effective capabilities are a subset of permitted
    pub fn validate(&self) -> bool {
        (self.effective & !self.permitted) == 0
    }

    /// Get all capabilities as vector
    pub fn get_effective_capabilities(&self) -> Vec<Capability> {
        Capability::ALL.iter()
            .filter(|&&cap| self.has_effective(cap))
            .copied()
            .collect()
    }

    /// Get all permitted capabilities as vector
    pub fn get_permitted_capabilities(&self) -> Vec<Capability> {
        Capability::ALL.iter()
            .filter(|&&cap| self.has_permitted(cap))
            .copied()
            .collect()
    }
}

/// Per-process capability state
#[derive(Debug, Clone)]
pub struct ProcessCapabilities {
    /// Process ID
    pub pid: u64,
    /// User ID
    pub uid: u32,
    /// Capability sets
    pub caps: CapabilitySets,
    /// Whether process is privileged (root)
    pub privileged: bool,
}

/// Capability subsystem
pub struct CapabilitySubsystem {
    /// Per-process capabilities
    process_caps: BTreeMap<u64, ProcessCapabilities>,
    /// Default capabilities for root
    root_caps: CapabilitySets,
    /// Default capabilities for regular users
    user_caps: CapabilitySets,
}

impl CapabilitySubsystem {
    /// Create new capability subsystem
    pub fn new() -> Self {
        let root_caps = CapabilitySets {
            effective: u32::MAX,
            permitted: u32::MAX,
            inheritable: u32::MAX,
            bounding: u32::MAX,
            ambient: 0,
        };

        let user_caps = CapabilitySets::default();

        Self {
            process_caps: BTreeMap::new(),
            root_caps,
            user_caps,
        }
    }

    /// Initialize capabilities for a process
    pub fn init_process(&mut self, pid: u64, uid: u32) -> Result<(), &'static str> {
        let caps = if uid == 0 {
            self.root_caps.clone()
        } else {
            self.user_caps.clone()
        };

        let process_caps = ProcessCapabilities {
            pid,
            uid,
            caps,
            privileged: uid == 0,
        };

        self.process_caps.insert(pid, process_caps);
        Ok(())
    }

    /// Get capabilities for a process
    pub fn get_process_capabilities(&self, pid: u64) -> Option<&ProcessCapabilities> {
        self.process_caps.get(&pid)
    }

    /// Check if a process has a specific capability
    pub fn process_has_capability(&self, pid: u64, cap: Capability) -> bool {
        match self.process_caps.get(&pid) {
            Some(proc_caps) => proc_caps.caps.has_effective(cap),
            None => false,
        }
    }

    /// Update process capabilities
    pub fn update_process_capabilities(
        &mut self,
        pid: u64,
        caps: CapabilitySets,
    ) -> Result<(), &'static str> {
        let process_caps = self.process_caps.get_mut(&pid)
            .ok_or("Process not found")?;

        if !caps.validate() {
            return Err("Invalid capability sets");
        }

        process_caps.caps = caps;
        Ok(())
    }

    /// Add capability to process
    pub fn add_process_capability(
        &mut self,
        pid: u64,
        cap: Capability,
        cap_type: CapType,
    ) -> Result<(), &'static str> {
        let process_caps = self.process_caps.get_mut(&pid)
            .ok_or("Process not found")?;

        match cap_type {
            CapType::Effective => process_caps.caps.add_effective(cap),
            CapType::Permitted => process_caps.caps.add_permitted(cap),
            CapType::Inheritable => process_caps.caps.add_inheritable(cap),
            CapType::Ambient => process_caps.caps.add_ambient(cap),
        }

        if !process_caps.caps.validate() {
            // Remove the capability we just added
            match cap_type {
                CapType::Effective => process_caps.caps.remove_effective(cap),
                CapType::Permitted => process_caps.caps.remove_permitted(cap),
                CapType::Inheritable => process_caps.caps.remove_inheritable(cap),
                CapType::Ambient => process_caps.caps.remove_ambient(cap),
            }
            return Err("Cannot add capability to effective set without adding to permitted set");
        }

        Ok(())
    }

    /// Remove capability from process
    pub fn remove_process_capability(
        &mut self,
        pid: u64,
        cap: Capability,
        cap_type: CapType,
    ) -> Result<(), &'static str> {
        let process_caps = self.process_caps.get_mut(&pid)
            .ok_or("Process not found")?;

        match cap_type {
            CapType::Effective => process_caps.caps.remove_effective(cap),
            CapType::Permitted => process_caps.caps.remove_permitted(cap),
            CapType::Inheritable => process_caps.caps.remove_inheritable(cap),
            CapType::Ambient => process_caps.caps.remove_ambient(cap),
        }

        // Also remove from effective if removing from permitted
        if cap_type == CapType::Permitted {
            process_caps.caps.remove_effective(cap);
        }

        Ok(())
    }

    /// Cleanup process capabilities
    pub fn cleanup_process(&mut self, pid: u64) {
        self.process_caps.remove(&pid);
    }

    /// Fork capabilities from parent to child
    pub fn fork_capabilities(&mut self, parent_pid: u64, child_pid: u64) -> Result<(), &'static str> {
        let parent_caps = self.process_caps.get(&parent_pid)
            .ok_or("Parent process not found")?;

        let child_caps = ProcessCapabilities {
            pid: child_pid,
            uid: parent_caps.uid,
            caps: parent_caps.caps.clone(),
            privileged: parent_caps.privileged,
        };

        self.process_caps.insert(child_pid, child_caps);
        Ok(())
    }

    /// Apply capabilities after exec
    pub fn exec_capabilities(&mut self, pid: u64, setuid: bool, setgid: bool) -> Result<(), &'static str> {
        let process_caps = self.process_caps.get_mut(&pid)
            .ok_or("Process not found")?;

        if setuid {
            process_caps.uid = 0;
            process_caps.privileged = true;
        }

        // Clear capabilities if dropping privileges
        if !process_caps.privileged {
            process_caps.caps.effective = 0;
            process_caps.caps.permitted = 0;
            process_caps.caps.inheritable = 0;
            process_caps.caps.ambient = 0;
        }

        Ok(())
    }
}

/// Types of capability sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapType {
    Effective,
    Permitted,
    Inheritable,
    Ambient,
}

/// High-level capability interface functions

/// Initialize capabilities for a process
pub fn init_process_capabilities(pid: u64, uid: u32) -> Result<(), &'static str> {
    let mut guard = crate::security::CAPABILITIES.lock();
    if let Some(ref mut s) = *guard {
        s.init_process(pid, uid)
    } else {
        Ok(())
    }
}

/// Check if process has specific capability
pub fn process_has_capability(pid: u64, cap: Capability) -> bool {
    let guard = crate::security::CAPABILITIES.lock();
    guard.as_ref().map(|s| s.process_has_capability(pid, cap)).unwrap_or(false)
}

/// Get process capabilities
pub fn get_process_capabilities(pid: u64) -> Option<ProcessCapabilities> {
    let guard = crate::security::CAPABILITIES.lock();
    guard.as_ref().and_then(|s| s.get_process_capabilities(pid).cloned())
}

/// Add capability to process
pub fn add_process_capability(
    pid: u64,
    cap: Capability,
    cap_type: CapType,
) -> Result<(), &'static str> {
    let mut guard = crate::security::CAPABILITIES.lock();
    if let Some(ref mut s) = *guard {
        s.add_process_capability(pid, cap, cap_type)
    } else {
        Ok(())
    }
}

/// Remove capability from process
pub fn remove_process_capability(
    pid: u64,
    cap: Capability,
    cap_type: CapType,
) -> Result<(), &'static str> {
    let mut guard = crate::security::CAPABILITIES.lock();
    if let Some(ref mut s) = *guard {
        s.remove_process_capability(pid, cap, cap_type)
    } else {
        Ok(())
    }
}

/// Fork capabilities from parent to child
pub fn fork_capabilities(parent_pid: u64, child_pid: u64) -> Result<(), &'static str> {
    let mut guard = crate::security::CAPABILITIES.lock();
    if let Some(ref mut s) = *guard {
        s.fork_capabilities(parent_pid, child_pid)
    } else {
        Ok(())
    }
}

/// Apply capabilities after exec
pub fn exec_capabilities(pid: u64, setuid: bool, setgid: bool) -> Result<(), &'static str> {
    let mut guard = crate::security::CAPABILITIES.lock();
    if let Some(ref mut s) = *guard {
        s.exec_capabilities(pid, setuid, setgid)
    } else {
        Ok(())
    }
}

/// Initialize capabilities subsystem
pub fn initialize_capabilities() -> Result<(), i32> {
    // Capabilities is already initialized via global static instance
    Ok(())
}

/// Cleanup capabilities subsystem
pub fn cleanup_capabilities() {
    // Placeholder: In a real implementation, this would clean up capabilities resources
}

/// Cleanup process capabilities
pub fn cleanup_process_capabilities(pid: u64) {
    if let Some(ref mut s) = *crate::security::CAPABILITIES.lock() {
        s.cleanup_process(pid);
    }
}

// Global instance moved to crate::security::CAPABILITIES (Option)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_names() {
        assert_eq!(Capability::Chown.name(), "CAP_CHOWN");
        assert_eq!(Capability::Kill.name(), "CAP_KILL");
        assert_eq!(Capability::SysAdmin.name(), "CAP_SYS_ADMIN");
    }

    #[test]
    fn test_capability_sets_creation() {
        let caps = CapabilitySets::new();
        assert_eq!(caps.effective, 0);
        assert_eq!(caps.permitted, 0);
        assert_eq!(caps.inheritable, 0);
        assert_eq!(caps.bounding, u32::MAX);
    }

    #[test]
    fn test_capability_operations() {
        let mut caps = CapabilitySets::new();

        assert!(!caps.has_effective(Capability::Kill));
        caps.add_effective(Capability::Kill);
        assert!(caps.has_effective(Capability::Kill));
        caps.remove_effective(Capability::Kill);
        assert!(!caps.has_effective(Capability::Kill));
    }

    #[test]
    fn test_capability_validation() {
        let mut caps = CapabilitySets::new();

        // Valid: empty effective set is subset of empty permitted set
        assert!(caps.validate());

        // Invalid: effective without permitted
        caps.add_effective(Capability::Kill);
        assert!(!caps.validate());

        // Valid: add permitted then effective
        caps.add_permitted(Capability::Kill);
        assert!(caps.validate());
    }

    #[test]
    fn test_capability_subsystem() {
        let mut subsystem = CapabilitySubsystem::new();

        let result = subsystem.init_process(1234, 1000);
        assert!(result.is_ok());

        assert!(!subsystem.process_has_capability(1234, Capability::Kill));

        let result = subsystem.init_process(0, 0);
        assert!(result.is_ok());

        assert!(subsystem.process_has_capability(0, Capability::Kill));
    }
}
