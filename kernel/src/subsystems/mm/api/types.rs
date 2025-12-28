// Memory module public types
// These types follow the MM_MODULE_API_BOUNDARIES_DESIGN.md

/// Memory protection properties
/// 
/// Define memory page protection attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryProtection {
    /// Readable
    pub readable: bool,
    /// Writable
    pub writable: bool,
    /// Executable
    pub executable: bool,
}

impl MemoryProtection {
    /// Create read-only protection
    pub const fn read_only() -> Self {
        Self { readable: true, writable: false, executable: false }
    }
    
    /// Create read-write protection
    pub const fn read_write() -> Self {
        Self { readable: true, writable: true, executable: false }
    }
    
    /// Create read-write-execute protection
    pub const fn read_write_execute() -> Self {
        Self { readable: true, writable: true, executable: true }
    }
    
    /// Create execute-only protection
    pub const fn execute_only() -> Self {
        Self { readable: false, writable: false, executable: true }
    }
}

/// Memory mapping flags
/// 
/// Define memory mapping behavior flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapFlags {
    /// Private mapping
    pub private: bool,
    /// Fixed mapping
    pub fixed: bool,
    /// Anonymous mapping
    pub anonymous: bool,
    /// No cache
    pub no_cache: bool,
}

impl Default for MapFlags {
    fn default() -> Self {
        Self {
            private: true,
            fixed: false,
            anonymous: false,
            no_cache: false,
        }
    }
}

/// Memory mapping object
/// 
/// Represent a memory mapping region
#[derive(Debug, Clone)]
pub struct MemoryMapping {
    /// Mapping address
    pub addr: usize,
    /// Mapping size
    pub size: usize,
    /// Memory protection attributes
    pub protection: MemoryProtection,
    /// Mapping flags
    pub flags: MapFlags,
    /// Mapping type
    pub mapping_type: MappingType,
    /// Reference count
    pub ref_count: u32,
}

/// Mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingType {
    /// Anonymous mapping
    Anonymous,
    /// File mapping
    File,
    /// Shared memory
    SharedMemory,
    /// Device mapping
    Device,
}

/// Memory stats
/// 
/// System memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total physical memory
    pub total_physical: usize,
    /// Available physical memory
    pub available_physical: usize,
    /// Used physical memory
    pub used_physical: usize,
    /// Total virtual memory
    pub total_virtual: usize,
    /// Used virtual memory
    pub used_virtual: usize,
}

/// Memory pool stats
/// 
/// Memory pool usage statistics
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    /// Number of objects in pool
    pub object_count: u32,
    /// Number of available objects
    pub available_objects: u32,
    /// Pool size in bytes
    pub pool_size: usize,
    /// Used size in bytes
    pub used_size: usize,
    /// Hit rate percentage
    pub hit_rate: f64,
}

/// Physical page structure
/// 
/// Represents a physical page in memory
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPage {
    /// Page frame number
    pub pfn: u64,
    /// Page flags
    pub flags: u32,
}

/// Allocator statistics
#[derive(Debug, Clone, Default)]
pub struct AllocatorStats {
    /// Total number of allocations
    pub total_allocations: usize,
    /// Total number of deallocations
    pub total_deallocations: usize,
    /// Currently allocated bytes
    pub current_allocated_bytes: usize,
    /// Peak allocated bytes
    pub peak_allocated_bytes: usize,
    /// Number of failed allocations
    pub failed_allocations: usize,
}

/// Physical memory stats
#[derive(Debug, Clone, Default)]
pub struct PhysicalMemoryStats {
    /// Total physical memory in bytes
    pub total_memory: usize,
    /// Available physical memory in bytes
    pub available_memory: usize,
    /// Number of free pages
    pub free_pages: usize,
    /// Number of used pages
    pub used_pages: usize,
    /// Number of reserved pages
    pub reserved_pages: usize,
}