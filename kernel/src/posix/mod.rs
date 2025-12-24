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
