//! POSIX Types and Constants
//!
//! Standard types and constants for POSIX compliance
//!
//! This module is organized into submodules for better maintainability:

pub mod types;
pub mod file_modes;
pub mod open_flags;
pub mod seek;
pub mod fcntl;
pub mod aio;
pub mod stat;

// ============================================================================
// Public Exports
// ============================================================================

pub use self::types::*;
pub use self::file_modes::*;
pub use self::open_flags::*;
pub use self::seek::*;
pub use self::fcntl::*;
pub use self::aio::*;
pub use self::stat::*;

// ============================================================================
// Thread support
// ============================================================================

pub mod thread;
pub mod sync;

// ============================================================================
// IPC and Synchronization modules
// ============================================================================

pub mod semaphore;
pub mod mqueue;
pub mod shm;
pub mod timer;
pub mod advanced_signal;
pub mod realtime;
pub mod advanced_thread;
pub mod security;
pub mod session;
pub mod fd_flags;

pub use self::thread::*;
pub use self::semaphore::*;
pub use self::mqueue::*;
pub use self::shm::*;
pub use self::timer::*;

// ============================================================================
// Re-export from libc for compatibility
// ============================================================================

pub use crate::libc::interface::size_t;
pub use crate::libc::ssize_t;

// ============================================================================
// Memory Protection Constants
// ============================================================================
pub const PROT_READ: i32 = 0x1;
pub const PROT_WRITE: i32 = 0x2;
pub const PROT_EXEC: i32 = 0x4;
pub const PROT_NONE: i32 = 0x0;

// ============================================================================
// Mapping Flags
// ============================================================================
pub const MAP_SHARED: i32 = 0x01;
pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_ANONYMOUS: i32 = 0x20;
pub const MAP_FIXED: i32 = 0x10;

// ============================================================================
// Semaphore Types
// ============================================================================
pub type SemT = *mut core::ffi::c_void;

// ============================================================================
// Shared Memory Types
// ============================================================================
#[repr(C)]
pub struct ShmidDs {
    pub shm_perm: IpcPerm,
    pub shm_segsz: usize,
    pub shm_atime: i64,
    pub shm_dtime: i64,
    pub shm_ctime: i64,
    pub shm_cpid: i32,
    pub shm_lpid: i32,
    pub shm_nattch: u64,
}

#[repr(C)]
pub struct IpcPerm {
    pub key: i32,
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u32,
    pub seq: u16,
}

// ============================================================================
// Timer Types
// ============================================================================
pub type TimerT = i32;

#[repr(C)]
pub struct SigEvent {
    pub sigev_value: usize,
    pub sigev_signo: i32,
    pub sigev_notify: i32,
}

#[repr(C)]
pub struct Itimerspec {
    pub it_interval: Timespec,
    pub it_value: Timespec,
}

// ============================================================================
// Signal Types
// ============================================================================
#[repr(C)]
pub struct SigSet {
    pub bits: [u64; 1],
}

#[repr(C)]
pub struct SigInfoT {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
    pub _pad: [i32; 29],
}

pub const SIGRTMIN: i32 = 34;
pub const SIGRTMAX: i32 = 64;
