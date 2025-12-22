//! Memory management types

use crate::core::types::Size;


/// Page size constants
pub mod page_size {
    use super::Size;
    /// 4KB page size
    pub const KB4: Size = 4096;
    /// 16KB page size
    pub const KB16: Size = 16384;
    /// 64KB page size
    pub const KB64: Size = 65536;
    /// 2MB page size
    pub const MB2: Size = 2097152;
    /// 4MB page size
    pub const MB4: Size = 4194304;
    /// 1GB page size
    pub const GB1: Size = 1073741824;
}

/// Memory alignment constants
pub mod alignment {
    use super::Size;
    /// 8-byte alignment
    pub const B8: Size = 8;
    /// 16-byte alignment
    pub const B16: Size = 16;
    /// 32-byte alignment
    pub const B32: Size = 32;
    /// 64-byte alignment
    pub const B64: Size = 64;
    /// 128-byte alignment
    pub const B128: Size = 128;
    /// 256-byte alignment
    pub const B256: Size = 256;
    /// 512-byte alignment
    pub const B512: Size = 512;
    /// 1KB alignment
    pub const KB1: Size = 1024;
    /// 4KB alignment
    pub const KB4: Size = 4096;
}

/// Memory region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Start address
    pub start: usize,
    /// End address
    pub end: usize,
    /// Size of region
    pub size: Size,
    /// Protection flags
    pub protection: u32,
    /// Mapping type
    pub mapping_type: u32,
}

impl MemoryRegion {
    /// Creates a new memory region
    pub fn new(start: usize, end: usize, size: Size, protection: u32, mapping_type: u32) -> Self {
        Self {
            start,
            end,
            size,
            protection,
            mapping_type,
        }
    }
    
    /// Returns true if address is in region
    pub fn contains(&self, addr: usize) -> bool {
        addr >= self.start && addr < self.end
    }
    
    /// Returns true if region overlaps with another region
    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.start < other.end && other.start < self.end
    }
    
    /// Returns the intersection of two regions
    pub fn intersection(&self, other: &MemoryRegion) -> Option<MemoryRegion> {
        if !self.overlaps(other) {
            return None;
        }
        
        let start = core::cmp::max(self.start, other.start);
        let end = core::cmp::min(self.end, other.end);
        let size = end - start;
        
        Some(MemoryRegion::new(
            start,
            end,
            size,
            self.protection | other.protection,
            self.mapping_type | other.mapping_type,
        ))
    }
}

/// Page table entry
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry {
    /// Physical address
    pub phys_addr: usize,
    /// Protection flags
    pub protection: u32,
    /// Mapping flags
    pub mapping_flags: u32,
    /// Access flags
    pub access_flags: u32,
}

impl PageTableEntry {
    /// Creates a new page table entry
    pub fn new(phys_addr: usize, protection: u32, mapping_flags: u32, access_flags: u32) -> Self {
        Self {
            phys_addr,
            protection,
            mapping_flags,
            access_flags,
        }
    }
    
    /// Returns true if entry is present
    pub fn is_present(&self) -> bool {
        (self.protection & 0x1) != 0
    }
    
    /// Returns true if entry is writable
    pub fn is_writable(&self) -> bool {
        (self.protection & 0x2) != 0
    }
    
    /// Returns true if entry is executable
    pub fn is_executable(&self) -> bool {
        (self.protection & 0x4) != 0
    }
    
    /// Returns true if entry is user accessible
    pub fn is_user(&self) -> bool {
        (self.protection & 0x8) != 0
    }
    
    /// Returns true if entry is write-through
    pub fn is_write_through(&self) -> bool {
        (self.mapping_flags & 0x1) != 0
    }
    
    /// Returns true if entry is cache disabled
    pub fn is_cache_disabled(&self) -> bool {
        (self.mapping_flags & 0x2) != 0
    }
    
    /// Returns true if entry is accessed
    pub fn is_accessed(&self) -> bool {
        (self.access_flags & 0x1) != 0
    }
    
    /// Returns true if entry is dirty
    pub fn is_dirty(&self) -> bool {
        (self.access_flags & 0x2) != 0
    }
}

/// Memory allocation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationFlags {
    /// No special flags
    None = 0x0,
    /// Zero memory after allocation
    Zero = 0x1,
    /// Align to page boundary
    PageAlign = 0x2,
    /// Align to cache line boundary
    CacheAlign = 0x4,
    /// Don't commit memory immediately
    Reserve = 0x8,
    /// Allow execution
    Executable = 0x10,
    /// Allow write access
    Writable = 0x20,
    /// Allow user access
    User = 0x40,
    /// Guard pages at boundaries
    Guard = 0x80,
}

/// Memory mapping flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingFlags {
    /// No special flags
    None = 0x0,
    /// Shared mapping
    Shared = 0x1,
    /// Private mapping
    Private = 0x2,
    /// Fixed address mapping
    Fixed = 0x4,
    /// Anonymous mapping
    Anonymous = 0x8,
    /// Grow down mapping
    GrowDown = 0x10,
    /// Deny execute
    DenyExecute = 0x20,
    /// Execute on write
    ExecuteOnWrite = 0x40,
    /// Lock mapping
    Locked = 0x80,
}

/// Memory cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    /// No cache
    None = 0x0,
    /// Write-back cache
    WriteBack = 0x1,
    /// Write-through cache
    WriteThrough = 0x2,
    /// Write-protected cache
    WriteProtected = 0x3,
    /// Uncacheable
    Uncacheable = 0x4,
}

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// Free region
    Free = 0x0,
    /// Reserved region
    Reserved = 0x1,
    /// Code region
    Code = 0x2,
    /// Data region
    Data = 0x3,
    /// Stack region
    Stack = 0x4,
    /// Heap region
    Heap = 0x5,
    /// Mapped region
    Mapped = 0x6,
    /// Device region
    Device = 0x7,
}

/// Memory allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// First fit strategy
    FirstFit = 0x0,
    /// Best fit strategy
    BestFit = 0x1,
    /// Worst fit strategy
    WorstFit = 0x2,
    /// Next fit strategy
    NextFit = 0x3,
    /// Buddy system strategy
    Buddy = 0x4,
    /// Slab allocator strategy
    Slab = 0x5,
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memory
    pub total: Size,
    /// Free memory
    pub free: Size,
    /// Used memory
    pub used: Size,
    /// Allocated memory
    pub allocated: Size,
    /// Committed memory
    pub committed: Size,
    /// Peak memory usage
    pub peak: Size,
    /// Number of allocations
    pub allocations: u64,
    /// Number of deallocations
    pub deallocations: u64,
    /// Number of page faults
    pub page_faults: u64,
    /// Number of major page faults
    pub major_page_faults: u64,
    /// Number of minor page faults
    pub minor_page_faults: u64,
}

impl MemoryStats {
    /// Creates new memory statistics
    pub fn new(total: Size) -> Self {
        Self {
            total,
            free: total,
            used: 0,
            allocated: 0,
            committed: 0,
            peak: 0,
            allocations: 0,
            deallocations: 0,
            page_faults: 0,
            major_page_faults: 0,
            minor_page_faults: 0,
        }
    }
    
    /// Updates allocation statistics
    pub fn allocate(&mut self, size: Size) {
        self.used += size;
        self.allocated += size;
        self.allocations += 1;
        if self.used > self.peak {
            self.peak = self.used;
        }
        if self.used > self.free {
            self.free = 0;
        } else {
            self.free -= size;
        }
    }
    
    /// Updates deallocation statistics
    pub fn deallocate(&mut self, size: Size) {
        self.used -= size;
        self.deallocations += 1;
        self.free += size;
    }
    
    /// Updates page fault statistics
    pub fn page_fault(&mut self, major: bool) {
        self.page_faults += 1;
        if major {
            self.major_page_faults += 1;
        } else {
            self.minor_page_faults += 1;
        }
    }
}