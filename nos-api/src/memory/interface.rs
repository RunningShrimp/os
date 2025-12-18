//! Memory management interface

use crate::error::Result;
use crate::core::types::{PhysAddr, VirtAddr, Size, PageNum, MemoryProtection, MemoryMappingType};

/// Trait for memory manager
pub trait MemoryManager {
    /// Allocates physical memory
    fn alloc_phys(&mut self, size: Size) -> Result<PhysAddr>;
    
    /// Frees physical memory
    fn free_phys(&mut self, addr: PhysAddr, size: Size) -> Result<()>;
    
    /// Allocates virtual memory
    fn alloc_virt(&mut self, size: Size) -> Result<VirtAddr>;
    
    /// Frees virtual memory
    fn free_virt(&mut self, addr: VirtAddr, size: Size) -> Result<()>;
    
    /// Maps physical memory to virtual memory
    fn map(&mut self, phys: PhysAddr, virt: VirtAddr, size: Size, prot: MemoryProtection) -> Result<()>;
    
    /// Unmaps virtual memory
    fn unmap(&mut self, virt: VirtAddr, size: Size) -> Result<()>;
    
    /// Changes memory protection
    fn protect(&mut self, virt: VirtAddr, size: Size, prot: MemoryProtection) -> Result<()>;
    
    /// Returns physical address for virtual address
    fn virt_to_phys(&self, virt: VirtAddr) -> Option<PhysAddr>;
    
    /// Returns virtual address for physical address
    fn phys_to_virt(&self, phys: PhysAddr) -> Option<VirtAddr>;
    
    /// Returns available physical memory
    fn available_phys(&self) -> Size;
    
    /// Returns available virtual memory
    fn available_virt(&self) -> Size;
    
    /// Returns total physical memory
    fn total_phys(&self) -> Size;
    
    /// Returns total virtual memory
    fn total_virt(&self) -> Size;
}

/// Trait for page allocator
pub trait PageAllocator {
    /// Allocates a page
    fn alloc_page(&mut self) -> Result<PageNum>;
    
    /// Allocates multiple pages
    fn alloc_pages(&mut self, count: PageNum) -> Result<PageNum>;
    
    /// Frees a page
    fn free_page(&mut self, page: PageNum) -> Result<()>;
    
    /// Frees multiple pages
    fn free_pages(&mut self, start: PageNum, count: PageNum) -> Result<()>;
    
    /// Returns number of free pages
    fn free_pages_count(&self) -> PageNum;
    
    /// Returns total number of pages
    fn total_pages(&self) -> PageNum;
    
    /// Returns page size
    fn page_size(&self) -> Size;
}

/// Trait for memory mapper
pub trait MemoryMapper {
    /// Maps a memory region
    fn map_region(&mut self, phys: PhysAddr, virt: VirtAddr, size: Size, 
                   prot: MemoryProtection, mapping_type: MemoryMappingType) -> Result<()>;
    
    /// Unmaps a memory region
    fn unmap_region(&mut self, virt: VirtAddr, size: Size) -> Result<()>;
    
    /// Changes protection of a memory region
    fn protect_region(&mut self, virt: VirtAddr, size: Size, prot: MemoryProtection) -> Result<()>;
    
    /// Flushes memory mappings
    fn flush(&mut self, virt: VirtAddr, size: Size) -> Result<()>;
    
    /// Invalidates memory mappings
    fn invalidate(&mut self, virt: VirtAddr, size: Size) -> Result<()>;
}

/// Trait for memory cache
pub trait MemoryCache {
    /// Flushes cache
    fn flush(&mut self) -> Result<()>;
    
    /// Invalidates cache
    fn invalidate(&mut self) -> Result<()>;
    
    /// Flushes specific cache line
    fn flush_line(&mut self, addr: VirtAddr) -> Result<()>;
    
    /// Invalidates specific cache line
    fn invalidate_line(&mut self, addr: VirtAddr) -> Result<()>;
    
    /// Returns cache size
    fn size(&self) -> Size;
    
    /// Returns cache line size
    fn line_size(&self) -> Size;
    
    /// Returns cache associativity
    fn associativity(&self) -> u32;
}

/// Trait for memory protection
pub trait MemoryProtectionUnit {
    /// Sets memory protection for a region
    fn set_protection(&mut self, addr: VirtAddr, size: Size, prot: MemoryProtection) -> Result<()>;
    
    /// Gets memory protection for a region
    fn get_protection(&self, addr: VirtAddr) -> Option<MemoryProtection>;
    
    /// Enables memory protection
    fn enable(&mut self) -> Result<()>;
    
    /// Disables memory protection
    fn disable(&mut self) -> Result<()>;
    
    /// Returns true if memory protection is enabled
    fn is_enabled(&self) -> bool;
}

/// Trait for memory statistics
pub trait MemoryStats {
    /// Returns memory usage statistics
    fn usage(&self) -> MemoryUsage;
    
    /// Returns memory allocation statistics
    fn allocation(&self) -> MemoryAllocation;
    
    /// Returns memory fragmentation statistics
    fn fragmentation(&self) -> MemoryFragmentation;
    
    /// Returns memory error statistics
    fn errors(&self) -> MemoryErrors;
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// Used physical memory
    pub used_phys: Size,
    /// Free physical memory
    pub free_phys: Size,
    /// Used virtual memory
    pub used_virt: Size,
    /// Free virtual memory
    pub free_virt: Size,
    /// Shared memory
    pub shared: Size,
    /// Buffer memory
    pub buffer: Size,
    /// Cached memory
    pub cached: Size,
}

/// Memory allocation statistics
#[derive(Debug, Clone)]
pub struct MemoryAllocation {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Current allocations
    pub current_allocations: u64,
    /// Peak allocations
    pub peak_allocations: u64,
    /// Total allocated bytes
    pub total_allocated_bytes: u64,
    /// Total deallocated bytes
    pub total_deallocated_bytes: u64,
    /// Current allocated bytes
    pub current_allocated_bytes: u64,
    /// Peak allocated bytes
    pub peak_allocated_bytes: u64,
}

/// Memory fragmentation statistics
#[derive(Debug, Clone)]
pub struct MemoryFragmentation {
    /// Fragmentation ratio
    pub ratio: f32,
    /// Largest free block
    pub largest_free_block: Size,
    /// Smallest free block
    pub smallest_free_block: Size,
    /// Average free block size
    pub average_free_block_size: Size,
    /// Number of free blocks
    pub free_blocks_count: u64,
}

/// Memory error statistics
#[derive(Debug, Clone)]
pub struct MemoryErrors {
    /// Out of memory errors
    pub out_of_memory: u64,
    /// Invalid address errors
    pub invalid_address: u64,
    /// Protection violations
    pub protection_violations: u64,
    /// Alignment errors
    pub alignment_errors: u64,
    /// Size errors
    pub size_errors: u64,
    /// Permission errors
    pub permission_errors: u64,
}