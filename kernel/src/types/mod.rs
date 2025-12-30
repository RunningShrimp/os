//! Types module
//!
//! Provides type definitions and stubs for the kernel.

use crate::prelude::*;

#[allow(dead_code)]

// Submodule for stub implementations
// Make stubs module public so external code can use crate::types::stubs::*
pub mod stubs;

// Re-export specific items from stubs that are not in prelude
// Note: FileSystemError, MemoryError, NetworkError, ProcessError, SyscallError
// are already exported from prelude, so we exclude them to avoid conflicts
pub use stubs::{
    // IPC types
    Message,
    MessageType,
    ServiceId,

    // POSIX types
    PidT,
    UidT,
    GidT,
    AfUnix,
    pid_t,
    uid_t,
    gid_t,

    // Sync types
    SyncMutex,

    // Socket constants
    AF_UNIX,
    AF_INET,
    AF_INET6,
    SOCK_STREAM,
    SOCK_DGRAM,
    SOCK_RAW,

    // Type stubs (not in prelude)
    VirtAddr,
    VfsNode,
    FileMode,
    FileType,
    FilesystemStats,
    VfsError,

    // RNG and utilities
    RNG,
    RNG_INSTANCE,
    get_timestamp,

    // IPC manager stubs
    IpcManager,
    IpcMessage,

    // Memory manager
    memory,

    // Block device trait
    BlockDevice,

    // Function stubs
    log_info,
    kill_process,

    // Constants
    AF_UNIX_CONST,
};


// ============================================================================
// Process and Thread Types
// ============================================================================

/// Process identifier type
pub type Pid = u64;

/// Process ID alias (for compatibility)
pub type ProcessId = Pid;

/// Thread identifier type
pub type Tid = u64;

/// User identifier type
pub type Uid = u32;

/// Group identifier type
pub type Gid = u32;

/// Session identifier type
pub type SessionId = u32;

/// Process group identifier type
pub type ProcessGroupId = u32;

// ============================================================================
// Memory Types
// ============================================================================

/// Physical address type
pub type PhysAddr = usize;

/// Page frame number type
pub type PageNumber = usize;

/// Memory offset type
pub type MemoryOffset = usize;

/// Cache key type
pub type CacheKey = u64;

// ============================================================================
// Time Types
// ============================================================================

/// Clock ID type
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ClockId {
    /// Realtime clock
    Realtime = 0,
    /// Monotonic clock
    Monotonic = 1,
    /// Process CPU time clock
    ProcessCPUTime = 2,
    /// Thread CPU time clock
    ThreadCPUTime = 3,
    /// Monotonic raw clock
    MonotonicRaw = 4,
    /// Realtime coarse clock
    RealtimeCoarse = 5,
    /// Monotonic coarse clock
    MonotonicCoarse = 6,
    /// Boot time clock
    BootTime = 7,
    /// Realtime alarm clock
    RealtimeAlarm = 8,
    /// Boottime alarm clock
    BoottimeAlarm = 9,
}

// ============================================================================
// Signal Types
// ============================================================================

/// Signal number type
pub type Signal = i32;

/// Signal set type
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SigSet {
    bits: [u64; 1],
}

impl SigSet {
    /// Create an empty signal set
    pub const fn empty() -> Self {
        Self { bits: [0] }
    }

    /// Create a full signal set
    pub const fn full() -> Self {
        Self { bits: [u64::MAX] }
    }

    /// Create a SigSet from raw bits
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits: [bits] }
    }

    /// Check if a signal is in the set
    pub fn has(&self, sig: u32) -> bool {
        if sig == 0 || sig > 64 {
            return false;
        }
        let index = (sig - 1) / 64;
        let bit = (sig - 1) % 64;
        (self.bits[index as usize] & (1 << bit)) != 0
    }

    /// Add a signal to the set
    pub fn add(&mut self, sig: u32) {
        if sig == 0 || sig > 64 {
            return;
        }
        let index = (sig - 1) / 64;
        let bit = (sig - 1) % 64;
        self.bits[index as usize] |= 1 << bit;
    }

    /// Remove a signal from the set
    pub fn remove(&mut self, sig: u32) {
        if sig == 0 || sig > 64 {
            return;
        }
        let index = (sig - 1) / 64;
        let bit = (sig - 1) % 64;
        self.bits[index as usize] &= !(1 << bit);
    }

    /// Check if set contains a signal (alias for has)
    pub fn contains(&self, sig: Signal) -> bool {
        self.has(sig as u32)
    }
}

impl Default for SigSet {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Memory Mapping Types
// ============================================================================

/// Memory mapping flags
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MapFlags(u32);

impl MapFlags {
    /// Create empty flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create flags from raw value
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Get raw bits
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Check if flag is set
    pub const fn contains(&self, bits: u32) -> bool {
        (self.0 & bits) == bits
    }

    /// Map readable
    pub const fn readable() -> u32 {
        0x1
    }

    /// Map writable
    pub const fn writable() -> u32 {
        0x2
    }

    /// Map executable
    pub const fn executable() -> u32 {
        0x4
    }

    /// Shared mapping
    pub const fn shared() -> u32 {
        0x8
    }

    /// Private mapping
    pub const fn private() -> u32 {
        0x10
    }

    /// Fixed mapping
    pub const fn fixed() -> u32 {
        0x20
    }

    /// Anonymous mapping
    pub const fn anonymous() -> u32 {
        0x40
    }

    /// PROT_READ flag
    pub const PROT_READ: Self = Self(0x1);

    /// PROT_WRITE flag
    pub const PROT_WRITE: Self = Self(0x2);

    /// PROT_EXEC flag
    pub const PROT_EXEC: Self = Self(0x4);
}

impl core::ops::BitOr for MapFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// ============================================================================
// Network Types
// ============================================================================

/// Interface service statistics
#[derive(Debug, Clone)]
pub struct InterfaceServiceStats {
    /// Bytes received
    pub bytes_rx: u64,
    /// Bytes transmitted
    pub bytes_tx: u64,
    /// Packets received
    pub packets_rx: u64,
    /// Packets transmitted
    pub packets_tx: u64,
    /// Errors receive
    pub errors_rx: u64,
    /// Errors transmit
    pub errors_tx: u64,
    /// Drops receive
    pub drops_rx: u64,
    /// Drops transmit
    pub drops_tx: u64,
}

impl Default for InterfaceServiceStats {
    fn default() -> Self {
        Self {
            bytes_rx: 0,
            bytes_tx: 0,
            packets_rx: 0,
            packets_tx: 0,
            errors_rx: 0,
            errors_tx: 0,
            drops_rx: 0,
            drops_tx: 0,
        }
    }
}

// ============================================================================
// Binary Types
// ============================================================================

/// Loaded binary information
#[derive(Debug, Clone)]
pub struct LoadedBinary {
    /// Entry point address
    pub entry_point: VirtAddr,
    /// Base address
    pub base_address: VirtAddr,
    /// Binary size
    pub size: usize,
    /// Binary name
    pub name: alloc::string::String,
    /// Binary type (ELF, PE, etc.)
    pub binary_type: BinaryType,
}

/// Binary type enumeration
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinaryType {
    /// ELF binary
    Elf,
    /// PE binary
    Pe,
    /// Mach-O binary
    MachO,
    /// Unknown binary
    Unknown,
}

// ============================================================================
// Memory Region Types
// ============================================================================

/// Memory region descriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Region start address
    pub start: PhysAddr,
    /// Region end address
    pub end: PhysAddr,
    /// Region type
    pub region_type: MemoryRegionType,
    /// Region is reserved
    pub is_reserved: bool,
}

/// Memory region type
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Available memory
    Available,
    /// Reserved memory
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS memory
    AcpiNvs,
    /// Unusable memory
    Unusable,
    /// Device memory
    Device,
    /// Kernel code
    KernelCode,
    /// Kernel data
    KernelData,
}

// ============================================================================
// Compatibility Types
// ============================================================================

/// Memory manager compatibility wrapper
#[derive(Debug)]
pub struct MemoryManager {
    _private: (),
}

impl MemoryManager {
    /// Create a new memory manager (stub)
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

// ============================================================================
// Statistics Types
// ============================================================================

/// CPU statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct CpuStats {
    /// User time
    pub user: u64,
    /// System time
    pub system: u64,
    /// Idle time
    pub idle: u64,
    /// I/O wait time
    pub iowait: u64,
}

/// Memory statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryStats {
    /// Total memory
    pub total: u64,
    /// Used memory
    pub used: u64,
    /// Free memory
    pub free: u64,
    /// Cached memory
    pub cached: u64,
    /// Buffer memory
    pub buffers: u64,
}

// ============================================================================
// End of types module
// ============================================================================
// Note: C type aliases (pid_t, uid_t, gid_t, PidT, UidT, GidT) are re-exported from stubs
// Note: VFS types (VfsNode) are re-exported from stubs
// Note: Utility functions (get_timestamp) and globals (RNG_INSTANCE) are in stubs
