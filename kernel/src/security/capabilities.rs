//! # Linux Capabilities Implementation
//!
//! This module provides comprehensive POSIX capabilities implementation with
//! capability bounding sets, ambient capabilities, and securebits.
//!
//! ## Features
//!
//! - **POSIX Capabilities**: Fine-grained privilege management
//! - **Capability Bounding Set**: Restrict capabilities for all processes
//! - **Ambient Capabilities**: Inherit capabilities across execve
//! - **Securebits**: Secure computation and integrity settings
//! - **Capability Sets**: Permitted, Effective, Inheritable sets

use crate::prelude::*;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Capability Definitions
// ============================================================================

/// All capabilities defined by Linux
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapFlag {
    /// Change file ownership
    Chown = 1 << 0,
    /// Override DAC (discretionary access control)
    DacOverride = 1 << 1,
    /// Perform read/write operations without DAC
    DacReadSearch = 1 << 2,
    /// Override file permissions
    Fowner = 1 << 3,
    /// Set file modification time
    Fsetid = 1 << 4,
    /// Send signals to other processes
    Kill = 1 << 5,
    /// Set group ID
    Setgid = 1 << 6,
    /// Set user ID
    Setuid = 1 << 7,
    /// Set process capabilities
    Setpcap = 1 << 8,
    /// Override immutable/append-only file attributes
    LinuxImmutable = 1 << 9,
    /// Bind to privileged network ports
    NetBindService = 1 << 10,
    /// Broadcast and listen to multicasts
    NetBroadcast = 1 << 11,
    /// Perform network administration
    NetAdmin = 1 << 12,
    /// Use raw sockets
    NetRaw = 1 << 13,
    /// Lock memory
    IpcLock = 1 << 14,
    /// Override IPC ownership checks
    IpcOwner = 1 << 15,
    /// Load and unload kernel modules
    SysModule = 1 << 16,
    /// Perform I/O port operations
    SysRawio = 1 << 17,
    /// Use chroot
    SysChroot = 1 << 18,
    /// Trace processes using ptrace
    SysPtrace = 1 << 19,
    /// Configure process accounting
    SysPacct = 1 << 20,
    /// Perform system administration
    SysAdmin = 1 << 21,
    /// Reboot system
    SysBoot = 1 << 22,
    /// Nice other processes
    SysNice = 1 << 23,
    /// Override resource limits
    SysResource = 1 << 24,
    /// Set system time
    SysTime = 1 << 25,
    /// Configure TTY devices
    SysTtyConfig = 1 << 26,
    /// Create special files using mknod
    Mknod = 1 << 27,
    /// Set file leases
    Lease = 1 << 28,
    /// Write to audit log
    AuditWrite = 1 << 29,
    /// Configure audit subsystem
    AuditControl = 1 << 30,
    /// Set file capabilities
    Setfcap = 1 << 31,
    /// Override MAC (mandatory access control)
    MacOverride = 1 << 32,
    /// Configure MAC
    MacAdmin = 1 << 33,
    /// Use syslog
    Syslog = 1 << 34,
    /// Wake up system with wake alarms
    WakeAlarm = 1 << 35,
    /// Block suspend and hibernation
    BlockSuspend = 1 << 36,
    /// Read audit log
    AuditRead = 1 << 37,
    /// Use performance events
    Perfmon = 1 << 38,
    /// Use BPF
    Bpf = 1 << 39,
    /// Perform checkpoint/restore
    CheckpointRestore = 1 << 40,
}

impl CapFlag {
    pub fn bits(&self) -> u64 {
        *self as u64
    }

    pub fn from_bits(bits: u64) -> CapFlags {
        CapFlags(bits)
    }
}

/// Collection of capability flags
#[derive(Debug, Clone, Copy)]
pub struct CapFlags(pub u64);

impl CapFlags {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn all() -> Self {
        Self(0xFFFFFFFFFFFFFFFF)
    }

    pub fn contains(&self, flag: CapFlag) -> bool {
        (self.0 & flag.bits()) != 0
    }

    pub fn insert(&mut self, flag: CapFlag) {
        self.0 |= flag.bits();
    }

    pub fn remove(&mut self, flag: CapFlag) {
        self.0 &= !flag.bits();
    }

    pub fn bits(&self) -> u64 {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn intersects(&self, other: CapFlags) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn union(&self, other: CapFlags) -> CapFlags {
        CapFlags(self.0 | other.0)
    }

    pub fn difference(&self, other: CapFlags) -> CapFlags {
        CapFlags(self.0 & !other.0)
    }
}

// ============================================================================
// Capability Sets
// ============================================================================

/// Process capability sets
#[derive(Debug, Clone)]
pub struct CapSets {
    /// Permitted capabilities (maximum that can be held)
    pub permitted: CapFlags,
    /// Effective capabilities (currently in effect)
    pub effective: CapFlags,
    /// Inheritable capabilities (preserved across execve)
    pub inheritable: CapFlags,
    /// Ambient capabilities (automatically inherited)
    pub ambient: CapFlags,
    /// Bounding set (restricts capabilities for all processes)
    pub bounding: CapFlags,
}

impl CapSets {
    pub fn new() -> Self {
        Self {
            permitted: CapFlags::empty(),
            effective: CapFlags::empty(),
            inheritable: CapFlags::empty(),
            ambient: CapFlags::empty(),
            bounding: CapFlags::all(),
        }
    }

    pub fn with_full_caps() -> Self {
        let all = CapFlags::all();
        Self {
            permitted: all,
            effective: all,
            inheritable: all,
            ambient: CapFlags::empty(),
            bounding: all,
        }
    }

    pub fn has_cap(&self, cap: CapFlag) -> bool {
        self.effective.contains(cap) && self.permitted.contains(cap)
    }

    pub fn can_raise(&self, cap: CapFlag) -> bool {
        self.permitted.contains(cap) && self.bounding.contains(cap)
    }

    pub fn can_inherit(&self, cap: CapFlag) -> bool {
        self.inheritable.contains(cap) || self.ambient.contains(cap)
    }

    pub fn drop_effective(&mut self) {
        self.effective = CapFlags::empty();
    }

    pub fn drop_ambient(&mut self) {
        self.ambient = CapFlags::empty();
    }
}

// ============================================================================
// Securebits
// ============================================================================

/// Securebits flags
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBit {
    /// Keep capabilities across uid 0
    NoRoot = 0,
    /// Setuid to uid 0 doesn't grant capabilities
    NoRootLocked = 1,
    /// Setuid fixed across all uid 0 processes
    NoRootSetuidFixed = 2,
    /// Setuid fixed and locked
    NoRootSetuidFixedLocked = 3,
    /// Don't allow ambient capabilities to be raised
    NoCapAmbientRaise = 4,
    /// Ambient capabilities raise locked
    NoCapAmbientRaiseLocked = 5,
    /// Keep capabilities across execve for non-root
    KeepCaps = 6,
    /// Keep caps locked
    KeepCapsLocked = 7,
}

impl SecureBit {
    pub fn bits(&self) -> u32 {
        1 << (*self as u32)
    }
}

/// Securebits management
#[derive(Debug, Clone, Copy)]
pub struct SecureBits(pub u32);

impl SecureBits {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn is_set(&self, bit: SecureBit) -> bool {
        (self.0 & bit.bits()) != 0
    }

    pub fn set(&mut self, bit: SecureBit) {
        self.0 |= bit.bits();
    }

    pub fn clear(&mut self, bit: SecureBit) {
        self.0 &= !bit.bits();
    }

    pub fn is_locked(&self, bit: SecureBit) -> bool {
        match bit {
            SecureBit::NoRoot => self.is_set(SecureBit::NoRootLocked),
            SecureBit::NoRootSetuidFixed => self.is_set(SecureBit::NoRootSetuidFixedLocked),
            SecureBit::NoCapAmbientRaise => self.is_set(SecureBit::NoCapAmbientRaiseLocked),
            SecureBit::KeepCaps => self.is_set(SecureBit::KeepCapsLocked),
            _ => false,
        }
    }
}

// ============================================================================
// Capability Manager
// ============================================================================

/// Capability manager
#[derive(Debug)]
pub struct CapManager {
    pub process_caps: Mutex<BTreeMap<u32, CapSets>>,
    pub system_bounding: Mutex<CapFlags>,
    pub securebits: Mutex<SecureBits>,
    pub checks_performed: AtomicU64,
    pub denials: AtomicU64,
}

impl CapManager {
    pub fn new() -> Self {
        Self {
            process_caps: Mutex::new(BTreeMap::new()),
            system_bounding: Mutex::new(CapFlags::all()),
            securebits: Mutex::new(SecureBits::new()),
            checks_performed: AtomicU64::new(0),
            denials: AtomicU64::new(0),
        }
    }

    pub fn initialize(&self) -> Result<()> {
        log_info!("[caps] Capability manager initialized");
        Ok(())
    }

    pub fn register_process(&self, pid: u32, caps: CapSets) {
        self.process_caps.lock().insert(pid, caps);
    }

    pub fn unregister_process(&self, pid: u32) {
        self.process_caps.lock().remove(&pid);
    }

    pub fn check_capability(&self, pid: u32, cap: CapFlag) -> bool {
        self.checks_performed.fetch_add(1, Ordering::Relaxed);

        let caps = self.process_caps.lock();
        if let Some(process_caps) = caps.get(&pid) {
            let allowed = process_caps.has_cap(cap);
            if !allowed {
                self.denials.fetch_add(1, Ordering::Relaxed);
            }
            return allowed;
        }

        false
    }

    pub fn has_capability(&self, pid: u32, cap: CapFlag) -> Result<bool> {
        let caps = self.process_caps.lock();
        if let Some(process_caps) = caps.get(&pid) {
            return Ok(process_caps.has_cap(cap));
        }
        Err(Error::NotFound)
    }

    pub fn raise_capability(&self, pid: u32, cap: CapFlag) -> Result<()> {
        let mut caps = self.process_caps.lock();
        if let Some(process_caps) = caps.get_mut(&pid) {
            if process_caps.can_raise(cap) {
                process_caps.effective.insert(cap);
                return Ok(());
            }
            return Err(Error::PermissionDenied);
        }
        Err(Error::NotFound)
    }

    pub fn drop_capability(&self, pid: u32, cap: CapFlag) -> Result<()> {
        let mut caps = self.process_caps.lock();
        if let Some(process_caps) = caps.get_mut(&pid) {
            process_caps.effective.remove(cap);
            return Ok(());
        }
        Err(Error::NotFound)
    }

    pub fn set_ambient(&self, pid: u32, cap: CapFlag, raise: bool) -> Result<()> {
        let securebits = self.securebits.lock();
        if securebits.is_set(SecureBit::NoCapAmbientRaise) && raise {
            return Err(Error::PermissionDenied);
        }

        let mut caps = self.process_caps.lock();
        if let Some(process_caps) = caps.get_mut(&pid) {
            if raise {
                process_caps.ambient.insert(cap);
            } else {
                process_caps.ambient.remove(cap);
            }
            return Ok(());
        }
        Err(Error::NotFound)
    }

    pub fn drop_bounding(&self, cap: CapFlag) -> Result<()> {
        let mut bounding = self.system_bounding.lock();
        bounding.remove(cap);
        Ok(())
    }

    pub fn get_process_caps(&self, pid: u32) -> Option<CapSets> {
        self.process_caps.lock().get(&pid).cloned()
    }

    pub fn set_securebit(&self, bit: SecureBit) -> Result<()> {
        let mut bits = self.securebits.lock();
        if bits.is_locked(bit) {
            return Err(Error::PermissionDenied);
        }
        bits.set(bit);
        Ok(())
    }

    pub fn get_securebits(&self) -> SecureBits {
        *self.securebits.lock()
    }

    pub fn get_stats(&self) -> CapStats {
        CapStats {
            checks_performed: self.checks_performed.load(Ordering::Relaxed),
            denials: self.denials.load(Ordering::Relaxed),
            active_processes: self.process_caps.lock().len(),
        }
    }
}

/// Capability statistics
#[derive(Debug, Clone)]
pub struct CapStats {
    pub checks_performed: u64,
    pub denials: u64,
    pub active_processes: usize,
}

// ============================================================================
// Global State
// ============================================================================

static GLOBAL_CAPS: Mutex<Option<CapManager>> = Mutex::new(None);

pub fn init_capabilities() -> Result<()> {
    let mut global = GLOBAL_CAPS.lock();
    if global.is_some() {
        return Ok(());
    }

    let manager = CapManager::new();
    manager.initialize()?;
    *global = Some(manager);
    Ok(())
}

pub fn get_cap_manager() -> Result<&'static Mutex<Option<CapManager>>> {
    Ok(&GLOBAL_CAPS)
}

pub fn check_capability(pid: u32, cap: CapFlag) -> bool {
    let global = GLOBAL_CAPS.lock();
    global
        .as_ref()
        .map(|m| m.check_capability(pid, cap))
        .unwrap_or(false)
}

pub fn raise_capability(pid: u32, cap: CapFlag) -> Result<()> {
    let global = GLOBAL_CAPS.lock();
    let manager = global.as_ref().ok_or(Error::NotFound)?;
    manager.raise_capability(pid, cap)
}

pub fn drop_capability(pid: u32, cap: CapFlag) -> Result<()> {
    let global = GLOBAL_CAPS.lock();
    let manager = global.as_ref().ok_or(Error::NotFound)?;
    manager.drop_capability(pid, cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cap_flags() {
        let mut flags = CapFlags::empty();
        flags.insert(CapFlag::Chown);
        assert!(flags.contains(CapFlag::Chown));
        assert!(!flags.contains(CapFlag::Kill));
    }

    #[test]
    fn test_cap_sets() {
        let caps = CapSets::with_full_caps();
        assert!(caps.has_cap(CapFlag::SysAdmin));
    }

    #[test]
    fn test_securebits() {
        let mut bits = SecureBits::new();
        bits.set(SecureBit::NoRoot);
        assert!(bits.is_set(SecureBit::NoRoot));
    }

    #[test]
    fn test_cap_manager() {
        let manager = CapManager::new();
        assert!(manager.initialize().is_ok());

        let caps = CapSets::with_full_caps();
        manager.register_process(1234, caps);

        assert!(manager.check_capability(1234, CapFlag::SysAdmin));
    }

    #[test]
    fn test_raise_drop_cap() {
        let manager = CapManager::new();
        manager.initialize().unwrap();

        let mut caps = CapSets::new();
        caps.permitted.insert(CapFlag::NetAdmin);
        caps.bounding.insert(CapFlag::NetAdmin);
        manager.register_process(1234, caps);

        // Raise capability
        assert!(manager.raise_capability(1234, CapFlag::NetAdmin).is_ok());

        // Drop capability
        assert!(manager.drop_capability(1234, CapFlag::NetAdmin).is_ok());
    }
}
