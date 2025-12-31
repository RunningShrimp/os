//! # NOS Kernel Prelude
//!
//! This module re-exports all commonly used types and functions
//! to make them available throughout the kernel without requiring
//! explicit imports in every module.
//!
//! ## Purpose
//!
//! The prelude dramatically reduces import boilerplate and prevents
//! compilation errors from missing type imports. By including this
//! prelude, modules automatically get access to:
//!
//! - Core allocation types (String, Box, Vec, etc.)
//! - Error handling types
//! - Synchronization primitives
//! - Common traits
//! - Memory management constants
//! - API types
//!
//! ## Usage
//!
//! ```rust
//! // At the top of your kernel module:
//! use crate::prelude::*;
//!
//! // Now you can use common types directly:
//! let text = String::from("Hello");
//! let numbers = Vec::new();
//! let boxed = Box::new(42);
//! ```
//!
//! ## Design Principles
//!
//! 1. **Minimal but Comprehensive**: Include only what's truly needed
//! 2. **No Surprises**: All exports should be intuitive
//! 3. **Conflict-Free**: Avoid names that could conflict with local code
//! 4. **Well-Organized**: Group related exports together

// ============================================================================
// Core Allocation Types
// ============================================================================

/// Heap-allocated pointer type
pub use alloc::boxed::Box;

/// Heap-allocated string type
pub use alloc::string::String;

/// ToString trait for string conversions
pub use alloc::string::ToString;

/// Heap-allocated growable vector type
pub use alloc::vec::Vec;

/// Ordered map type (B-tree)
pub use alloc::collections::BTreeMap;

/// Ordered set type (B-tree)
pub use alloc::collections::BTreeSet;

/// Format macro for string formatting
pub use alloc::format;

/// Vec macro for vector creation
pub use alloc::vec;

// ============================================================================
// Error Handling
// ============================================================================

/// Unified error type from the error module
pub use crate::error::Error;

/// Result type alias for kernel operations
pub use crate::error::Result;

/// Unified error type with extended error information
pub use crate::error::UnifiedError;

/// Unified result type
pub use crate::error::UnifiedResult;

// ============================================================================
// Synchronization Primitives
// ============================================================================

/// Mutex type for mutual exclusion
pub use crate::sync::Mutex;

/// Read-write lock type
pub use crate::sync::RwLock;

/// Spinlock type for short-term critical sections
pub use crate::sync::SpinLock;

/// Mutex that disables interrupts while held
pub use crate::sync::MutexIrq;

/// Spinlock that disables interrupts while held
pub use crate::sync::SpinLockIrq;

/// Sleep lock for operations that may sleep
pub use crate::sync::Sleeplock;

/// One-time initialization primitive
pub use crate::sync::Once;

/// Lazy-initialized value
pub use crate::sync::Lazy;

// ============================================================================
// Common Traits
// ============================================================================

/// String formatting trait
pub use core::fmt::Display;

/// Debug formatting trait
pub use core::fmt::Debug;

/// Clone trait for duplicating values
pub use core::clone::Clone;

/// Partial equality comparison trait
pub use core::cmp::PartialEq;

/// Equality comparison trait
pub use core::cmp::Eq;

/// Partial ordering trait
pub use core::cmp::PartialOrd;

/// Ordering trait
pub use core::cmp::Ord;

// Note: Async features disabled - no "async" feature defined in Cargo.toml
// /// Async iterator trait
// #[cfg(feature = "async")]
// pub use core::async_iter::AsyncIterator;
//
// /// Async iterator trait (extended)
// #[cfg(feature = "async")]
// pub use core::future::Future;

// ============================================================================
// Memory Management
// ============================================================================

/// Standard page size (4096 bytes on most architectures)
pub use crate::subsystems::mm::PAGE_SIZE;

/// Physical memory allocation
pub use crate::subsystems::mm::kalloc;

/// Physical memory deallocation
pub use crate::subsystems::mm::kfree;

/// Virtual memory permissions
pub use crate::subsystems::mm::VmPerm;

/// Virtual memory area
pub use crate::subsystems::mm::VmArea;

/// Map virtual pages to physical pages
pub use crate::subsystems::mm::map_pages;

/// Activate a page table
pub use crate::subsystems::mm::activate;

/// Flush a single TLB entry
pub use crate::subsystems::mm::flush_tlb_page;

/// Copy data from kernel to user space
pub use crate::subsystems::mm::copyout;

/// Get page table entry flags
pub use crate::subsystems::mm::flags;

/// Number of page table entries
pub use crate::subsystems::mm::PTE_COUNT;

/// Free a page table
pub use crate::subsystems::mm::free_pagetable;

/// Page table type
pub use crate::subsystems::mm::PageTable;

/// Memory statistics
pub use crate::subsystems::mm::AllocationStats;

/// Memory management statistics
pub use crate::subsystems::mm::MemoryManagementStats;

/// C library statistics
pub use crate::libc::CLibStats;

/// Numeric statistics
pub use crate::subsystems::mm::NumStats;

// ============================================================================
// API Types
// ============================================================================

/// API error type from nos_api crate
pub use nos_api::Error as ApiError;

/// API result type
pub use nos_api::Result as ApiResult;

// ============================================================================
// Atomic Types (re-exported from alloc)
// ============================================================================

/// Atomic boolean type
pub use core::sync::atomic::AtomicBool;

/// Atomic usize type
pub use core::sync::atomic::AtomicUsize;

/// Atomic u64 type
pub use core::sync::atomic::AtomicU64;

/// Atomic u32 type
pub use core::sync::atomic::AtomicU32;

/// Atomic i32 type
pub use core::sync::atomic::AtomicI32;

/// Atomic pointer type
pub use core::sync::atomic::AtomicPtr;

/// Atomic u8 type (if available)
#[cfg(feature = "atomic_u8")]
pub use core::sync::atomic::AtomicU8;

// ============================================================================
// Once Lock Types
// ============================================================================

/// One-time initialization lock (Rust 1.70+)
/// Note: OnceLock is not available in core::sync in no_std, using lazy_static with Mutex instead
pub use crate::sync::OnceLock;

// ============================================================================
// Smart Pointer Types
// ============================================================================

/// Atomically reference-counted pointer
pub use alloc::sync::Arc;

// ============================================================================
// Common Operations
// ============================================================================

/// Align address up
pub use crate::subsystems::mm::align_up;

/// Align address down
pub use crate::subsystems::mm::align_down;

/// Check if address is aligned
pub use crate::subsystems::mm::is_aligned;

/// Round up to power of 2
pub use crate::subsystems::mm::round_up_power_of_2;

/// Get log2 of power-of-2
pub use crate::subsystems::mm::log2_pow2;

/// Get allocation order
pub use crate::subsystems::mm::get_order;

// ============================================================================
// Additional Common Types
// ============================================================================

/// Memory region type
pub use crate::memory::MemoryRegion;

/// Memory permissions type
pub use crate::memory::MemoryPermissions;

/// Memory region type for processes
pub use crate::api::MemoryRegionType;

/// Syscall error type
pub use crate::error::SyscallError;

/// Syscall result type
pub use crate::error::SyscallResult;

/// Kernel error type
pub use crate::error::KernelError;

/// Kernel result type
pub use crate::error::KernelResult;

/// File system error type
pub use crate::error::FileSystemError;

/// Network error type
pub use crate::error::NetworkError;

/// Process error type
pub use crate::error::ProcessError;

/// Memory error type
pub use crate::error::MemoryError;

/// Security error type
pub use crate::error::SecurityError;

/// Driver error type
pub use crate::error::DriverError;

/// Process ID type
pub use crate::types::ProcessId;

/// Access result type
pub use crate::security::enhanced_permissions::AccessResult;

// Note: CallingConvention is in subsystems::formal_verification, not sync
// pub use crate::subsystems::sync::CallingConvention;

// Note: Timestamp type doesn't exist in subsystems::time
// pub use crate::subsystems::time::Timestamp;

/// Container type
pub use crate::subsystems::cloud_native::container::Container;

// Note: ContainerService doesn't exist
// pub use crate::subsystems::cloud_native::container::ContainerService;

// ============================================================================
// Test the prelude
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_types() {
        // Test that all prelude types are accessible
        let _string: String = String::new();
        let _vec: Vec<u8> = Vec::new();
        let _boxed: Box<u32> = Box::new(42);

        // Test that constants are accessible
        assert!(PAGE_SIZE > 0);

        // Test that traits are accessible
        let _display: &dyn Display = &_string;
        let _debug: &dyn Debug = &_vec;
    }

    #[test]
    fn test_memory_types() {
        // Test memory management types
        let _align = align_up(1000, PAGE_SIZE);
        let _rounded = round_up_power_of_2(1000);

        assert!(_align >= 1000);
        assert!(_rounded >= 1000);
    }

    #[test]
    fn test_error_types() {
        // Test that error types are accessible
        use crate::error::{Error, UnifiedError};

        // These should compile if types are properly exported
        let _: Result<(), Error> = Ok(());
        let _: UnifiedResult<()> = Ok(());
    }
}
