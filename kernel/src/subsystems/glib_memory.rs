//! GLib memory management extension system calls
//!
//! Provides high-performance memory management support for GLib, including:
//! - Dedicated memory pool creation and management
//! - Fast memory allocation and deallocation
//! - Memory pool statistics and debug information
//! - Thread-safe memory operations

extern crate alloc;

use crate::syscalls::SyscallResult;
use crate::alloc::allocator::FixedSizeAllocator;
use crate::sync::Mutex;
use alloc::collections::BTreeMap;
use core::ffi::c_int;
use core::sync::atomic::AtomicUsize;

/// GLib memory error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    InvalidArgument,
    OutOfMemory,
    PoolNotFound,
    PoolExists,
    PoolFull,
    InvalidSize,
    AlignmentError,
}

pub type MemoryResult<T> = Result<T, MemoryError>;

/// Memory pool information
#[derive(Debug, Clone)]
pub struct MemoryPoolInfo {
    /// Memory pool size
    pub size: usize,
    /// Alignment requirements
    pub alignment: usize,
    /// Number of allocated blocks
    pub allocated_blocks: AtomicUsize,
    /// Number of freed blocks
    pub freed_blocks: AtomicUsize,
    /// Current active blocks
    pub active_blocks: AtomicUsize,
    /// Creation timestamp
    pub created_timestamp: u64,
}

/// Global memory pool registry
static MEMORY_POOLS: Mutex<BTreeMap<c_int, (FixedSizeAllocator, MemoryPoolInfo)>> =
    Mutex::new(BTreeMap::new());

/// Next available memory pool ID
static NEXT_POOL_ID: AtomicUsize = AtomicUsize::new(1);

/// GLib memory manager singleton
pub static mut GLIB_MEMORY_MANAGER: () = ();

/// Get GLib memory manager reference
pub fn get_glib_memory_manager() -> &'static dyn super::allocator::GLibMemoryAllocator {
    unsafe { &GLIB_MEMORY_MANAGER }
}

pub mod pool;
pub mod allocator;
pub mod adapter;