//! Memory Management API Interface
//!
//! This module defines trait interfaces for memory management.
//! It provides abstractions that separate memory management interface
//! from its implementation, helping to break circular dependencies
//! between modules.

use alloc::string::String;
use alloc::vec::Vec;
use crate::types::stubs::{pid_t, uid_t, gid_t};
use crate::error::unified_framework::{FrameworkError, IntoFrameworkError, FrameworkResult};
use crate::error::unified::UnifiedError;

/// Memory manager trait
///
/// This trait defines the interface for memory management operations.
/// It provides methods for allocating, deallocating, and managing memory.
pub trait MemoryManager {
    /// Allocate physical memory
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate (in bytes)
    /// * `flags` - Allocation flags
    ///
    /// # Returns
    /// * `Ok(PhysicalAddress)` - Physical address of allocated memory
    /// * `Err(MemoryError)` - Memory allocation error
    fn allocate_physical(&self, size: usize, flags: AllocationFlags) -> Result<PhysicalAddress, MemoryError>;

    /// Deallocate physical memory
    ///
    /// # Arguments
    /// * `addr` - Physical address of memory to deallocate
    /// * `size` - Size of memory to deallocate (in bytes)
    fn deallocate_physical(&self, addr: PhysicalAddress, size: usize);

    /// Allocate virtual memory
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate (in bytes)
    /// * `flags` - Allocation flags
    ///
    /// # Returns
    /// * `Ok(VirtualAddress)` - Virtual address of allocated memory
    /// * `Err(MemoryError)` - Memory allocation error
    fn allocate_virtual(&self, size: usize, flags: AllocationFlags) -> Result<VirtualAddress, MemoryError>;

    /// Deallocate virtual memory
    ///
    /// # Arguments
    /// * `addr` - Virtual address of memory to deallocate
    /// * `size` - Size of memory to deallocate (in bytes)
    fn deallocate_virtual(&self, addr: VirtualAddress, size: usize);

    /// Map virtual memory to physical memory
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `paddr` - Physical address
    /// * `size` - Size of memory to map (in bytes)
    /// * `flags` - Mapping flags
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(MemoryError)` - Mapping error
    fn map_memory(&self, vaddr: VirtualAddress, paddr: PhysicalAddress, size: usize, flags: MappingFlags) -> Result<(), MemoryError>;

    /// Unmap virtual memory
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `size` - Size of memory to unmap (in bytes)
    fn unmap_memory(&self, vaddr: VirtualAddress, size: usize);

    /// Change memory protection
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `size` - Size of memory to change protection (in bytes)
    /// * `flags` - Protection flags
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(MemoryError)` - Protection change error
    fn protect_memory(&self, vaddr: VirtualAddress, size: usize, flags: ProtectionFlags) -> Result<(), MemoryError>;

    /// Get memory statistics
    ///
    /// # Returns
    /// * `MemoryStats` - Memory statistics
    fn get_memory_stats(&self) -> MemoryStats;

    /// Get memory map
    ///
    /// # Returns
    /// * `Vec<MemoryRegion>` - Memory regions
    fn get_memory_map(&self) -> Vec<MemoryRegion>;

    /// Get page size
    ///
    /// # Returns
    /// * `usize` - Page size (in bytes)
    fn get_page_size(&self) -> usize;

    /// Flush memory caches
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `size` - Size of memory to flush (in bytes)
    fn flush_caches(&self, vaddr: VirtualAddress, size: usize);

    /// Sync memory to disk
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `size` - Size of memory to sync (in bytes)
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(MemoryError)` - Sync error
    fn sync_memory(&self, vaddr: VirtualAddress, size: usize) -> Result<(), MemoryError>;

    /// Lock memory in RAM
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `size` - Size of memory to lock (in bytes)
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(MemoryError)` - Lock error
    fn lock_memory(&self, vaddr: VirtualAddress, size: usize) -> Result<(), MemoryError>;

    /// Unlock memory from RAM
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `size` - Size of memory to unlock (in bytes)
    fn unlock_memory(&self, vaddr: VirtualAddress, size: usize);

    /// Advise on memory usage
    ///
    /// # Arguments
    /// * `vaddr` - Virtual address
    /// * `size` - Size of memory to advise (in bytes)
    /// * `advice` - Memory advice
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(MemoryError)` - Advice error
    fn advise_memory(&self, vaddr: VirtualAddress, size: usize, advice: MemoryAdvice) -> Result<(), MemoryError>;
}

/// Process memory manager trait
///
/// This trait defines the interface for process-specific memory management.
pub trait ProcessMemoryManager {
    /// Get the memory manager for a specific process
    ///
    /// # Arguments
    /// * `pid` - Process ID
    ///
    /// # Returns
    /// * `Option<&dyn ProcessMemoryManager>` - Process memory manager if exists
    fn get_process_memory_manager(&self, pid: pid_t) -> Option<&dyn ProcessMemoryManager>;

    /// Create a new process memory manager
    ///
    /// # Arguments
    /// * `pid` - Process ID
    ///
    /// # Returns
    /// * `Ok(())` - Success
    /// * `Err(MemoryError)` - Creation error
    fn create_process_memory_manager(&mut self, pid: pid_t) -> Result<(), MemoryError>;

    /// Destroy a process memory manager
    ///
    /// # Arguments
    /// * `pid` - Process ID
    fn destroy_process_memory_manager(&mut self, pid: pid_t);

    /// Get the current process memory manager
    ///
    /// # Returns
    /// * `Option<&dyn ProcessMemoryManager>` - Current process memory manager if exists
    fn get_current_process_memory_manager(&self) -> Option<&dyn ProcessMemoryManager>;

    /// Allocate memory in the current process
    ///
    /// # Arguments
    /// * `size` - Size of memory to allocate (in bytes)
    /// * `flags` - Allocation flags
    ///
    /// # Returns
    /// * `Ok(VirtualAddress)` - Virtual address of allocated memory
    /// * `Err(MemoryError)` - Memory allocation error
    fn allocate_current_process_memory(&self, size: usize, flags: AllocationFlags) -> Result<VirtualAddress, MemoryError>;

    /// Deallocate memory in the current process
    ///
    /// # Arguments
    /// * `addr` - Virtual address of memory to deallocate
    /// * `size` - Size of memory to deallocate (in bytes)
    fn deallocate_current_process_memory(&self, addr: VirtualAddress, size: usize);

    /// Get the memory map of the current process
    ///
    /// # Returns
    /// * `Vec<MemoryRegion>` - Memory regions
    fn get_current_process_memory_map(&self) -> Vec<MemoryRegion>;

    /// Get the memory usage of the current process
    ///
    /// # Returns
    /// * `ProcessMemoryUsage` - Memory usage
    fn get_current_process_memory_usage(&self) -> ProcessMemoryUsage;
}

/// Physical address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalAddress(pub usize);

/// Virtual address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualAddress(pub usize);

/// Allocation flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllocationFlags {
    /// Zero the allocated memory
    pub zero: bool,
    /// Allocate contiguous memory
    pub contiguous: bool,
    /// Allocate below 4GB
    pub below_4gb: bool,
    /// Allocate below 16MB
    pub below_16mb: bool,
    /// Allocate cache-aligned memory
    pub cache_aligned: bool,
    /// Allocate page-aligned memory
    pub page_aligned: bool,
    /// Allocate non-cached memory
    pub non_cached: bool,
    /// Allocate write-combining memory
    pub write_combining: bool,
    /// Allocate write-through memory
    pub write_through: bool,
}

/// Mapping flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MappingFlags {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
    /// User accessible
    pub user: bool,
    /// Write-through caching
    pub write_through: bool,
    /// Cache disabled
    pub cache_disable: bool,
    /// Global mapping
    pub global: bool,
    /// Huge page
    pub huge_page: bool,
    /// Write-combining
    pub write_combining: bool,
}

/// Protection flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtectionFlags {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

/// Memory advice
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryAdvice {
    /// No special advice
    Normal,
    /// Expect random access pattern
    Random,
    /// Expect sequential access pattern
    Sequential,
    /// Will need this memory
    WillNeed,
    /// Won't need this memory
    DontNeed,
    /// Expect this memory to be accessed only once
    SequentialOnly,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total physical memory
    pub total_physical: usize,
    /// Available physical memory
    pub available_physical: usize,
    /// Total virtual memory
    pub total_virtual: usize,
    /// Available virtual memory
    pub available_virtual: usize,
    /// Total page frames
    pub total_page_frames: usize,
    /// Free page frames
    pub free_page_frames: usize,
    /// Active page frames
    pub active_page_frames: usize,
    /// Inactive page frames
    pub inactive_page_frames: usize,
    /// Dirty page frames
    pub dirty_page_frames: usize,
    /// Page cache size
    pub page_cache_size: usize,
    /// Swap total
    pub swap_total: usize,
    /// Swap free
    pub swap_free: usize,
}

/// Memory region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Start address
    pub start: VirtualAddress,
    /// End address
    pub end: VirtualAddress,
    /// Size
    pub size: usize,
    /// Permissions
    pub permissions: ProtectionFlags,
    /// Type
    pub region_type: MemoryRegionType,
    /// Name
    pub name: String,
    /// File path (if file-backed)
    pub file_path: Option<String>,
    /// Offset in file (if file-backed)
    pub file_offset: Option<u64>,
    /// Device (if device-mapped)
    pub device: Option<u64>,
    /// Inode (if file-backed)
    pub inode: Option<u64>,
}

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryRegionType {
    /// Anonymous memory
    Anonymous,
    /// File-backed memory
    File,
    /// Stack memory
    Stack,
    /// Heap memory
    Heap,
    /// Code memory
    Code,
    /// Data memory
    Data,
    /// Shared memory
    Shared,
    /// Device memory
    Device,
}

/// Process memory usage
#[derive(Debug, Clone)]
pub struct ProcessMemoryUsage {
    /// Virtual memory size
    pub virtual_size: usize,
    /// Resident set size
    pub resident_set_size: usize,
    /// Shared memory size
    pub shared_size: usize,
    /// Text size
    pub text_size: usize,
    /// Data size
    pub data_size: usize,
    /// Stack size
    pub stack_size: usize,
    /// Heap size
    pub heap_size: usize,
    /// Number of page faults
    pub page_faults: u64,
    /// Number of major page faults
    pub major_page_faults: u64,
    /// Number of minor page faults
    pub minor_page_faults: u64,
}

/// Memory error
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryError {
    /// Out of memory
    OutOfMemory,
    /// Invalid address
    InvalidAddress,
    /// Invalid size
    InvalidSize,
    /// Invalid flags
    InvalidFlags,
    /// Permission denied
    PermissionDenied,
    /// Already mapped
    AlreadyMapped,
    /// Not mapped
    NotMapped,
    /// Mapping conflict
    MappingConflict,
    /// Resource busy
    ResourceBusy,
    /// Resource unavailable
    ResourceUnavailable,
    /// Invalid argument
    InvalidArgument,
    /// Operation not supported
    NotSupported,
    /// Unknown error
    Unknown,
}

impl IntoFrameworkError for MemoryError {
    fn into_framework_error(self) -> FrameworkError {
        match self {
            MemoryError::OutOfMemory => UnifiedError::OutOfMemory.into_framework_error(),
            MemoryError::InvalidAddress => UnifiedError::InvalidAddress.into_framework_error(),
            MemoryError::InvalidSize => UnifiedError::InvalidArgument.into_framework_error(),
            MemoryError::InvalidFlags => UnifiedError::InvalidArgument.into_framework_error(),
            MemoryError::PermissionDenied => UnifiedError::PermissionDenied.into_framework_error(),
            MemoryError::AlreadyMapped => UnifiedError::AlreadyExists.into_framework_error(),
            MemoryError::NotMapped => UnifiedError::NotFound.into_framework_error(),
            MemoryError::MappingConflict => UnifiedError::InvalidState.into_framework_error(),
            MemoryError::ResourceBusy => UnifiedError::ResourceBusy.into_framework_error(),
            MemoryError::ResourceUnavailable => UnifiedError::ResourceUnavailable.into_framework_error(),
            MemoryError::InvalidArgument => UnifiedError::InvalidArgument.into_framework_error(),
            MemoryError::NotSupported => UnifiedError::NotSupported.into_framework_error(),
            MemoryError::Unknown => UnifiedError::Unknown.into_framework_error(),
        }
    }
    
    fn with_context(self, context: &str, location: &str) -> FrameworkError {
        match self {
            MemoryError::OutOfMemory => UnifiedError::OutOfMemory.with_context(context, location),
            MemoryError::InvalidAddress => UnifiedError::InvalidAddress.with_context(context, location),
            MemoryError::InvalidSize => UnifiedError::InvalidArgument.with_context(context, location),
            MemoryError::InvalidFlags => UnifiedError::InvalidArgument.with_context(context, location),
            MemoryError::PermissionDenied => UnifiedError::PermissionDenied.with_context(context, location),
            MemoryError::AlreadyMapped => UnifiedError::AlreadyExists.with_context(context, location),
            MemoryError::NotMapped => UnifiedError::NotFound.with_context(context, location),
            MemoryError::MappingConflict => UnifiedError::InvalidState.with_context(context, location),
            MemoryError::ResourceBusy => UnifiedError::ResourceBusy.with_context(context, location),
            MemoryError::ResourceUnavailable => UnifiedError::ResourceUnavailable.with_context(context, location),
            MemoryError::InvalidArgument => UnifiedError::InvalidArgument.with_context(context, location),
            MemoryError::NotSupported => UnifiedError::NotSupported.with_context(context, location),
            MemoryError::Unknown => UnifiedError::Unknown.with_context(context, location),
        }
    }
}

impl Default for AllocationFlags {
    fn default() -> Self {
        Self {
            zero: false,
            contiguous: false,
            below_4gb: false,
            below_16mb: false,
            cache_aligned: false,
            page_aligned: true,
            non_cached: false,
            write_combining: false,
            write_through: false,
        }
    }
}

impl Default for MappingFlags {
    fn default() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
            user: true,
            write_through: false,
            cache_disable: false,
            global: false,
            huge_page: false,
            write_combining: false,
        }
    }
}

impl Default for ProtectionFlags {
    fn default() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
        }
    }
}