//! Optimized Page Allocator
//!
//! This module implements an optimized page allocator with the following features:
//! - O(1) allocation and deallocation for single pages
//! - Fast buddy allocation for multiple pages
//! - Per-CPU caches for reduced contention
//! - Memory defragmentation
//! - NUMA awareness

use core::ptr::{null_mut};
use core::sync::atomic::{AtomicUsize, AtomicPtr, AtomicBool, Ordering};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::sync::Mutex;
use super::phys::{PAGE_SIZE, page_round_up, page_round_down};

/// Page order (log2 of number of pages)
pub type PageOrder = u8;

/// Maximum order supported (2^MAX_ORDER pages)
pub const MAX_ORDER: PageOrder = 10; // 2^10 = 1024 pages = 4MB

/// Page frame number
pub type PageFrame = usize;

/// Page descriptor
#[derive(Debug)]
pub struct PageDescriptor {
    /// Physical address of the page
    pub pfn: PageFrame,
    /// Order of the page (0 for single page, 1 for 2 pages, etc.)
    pub order: PageOrder,
    /// NUMA node ID
    pub numa_node: u8,
    /// Reference count
    pub ref_count: AtomicUsize,
    /// Flags
    pub flags: PageFlags,
    /// Last access time (for defragmentation)
    pub last_access: AtomicUsize,
}

/// Page flags
#[derive(Debug, Clone, Copy)]
pub struct PageFlags {
    /// Page is allocated
    pub allocated: bool,
    /// Page is reserved (cannot be allocated)
    pub reserved: bool,
    /// Page is dirty (needs to be written back)
    pub dirty: bool,
    /// Page is accessed recently
    pub accessed: bool,
    /// Page is part of a huge page
    pub huge_page: bool,
}

impl Default for PageFlags {
    fn default() -> Self {
        Self {
            allocated: false,
            reserved: false,
            dirty: false,
            accessed: false,
            huge_page: false,
        }
    }
}

/// Per-CPU page cache
#[derive(Debug)]
pub struct PerCpuPageCache {
    /// Single page cache
    single_pages: Vec<PageFrame>,
    /// Multi-page cache by order
    multi_pages: BTreeMap<PageOrder, Vec<PageFrame>>,
    /// Maximum cache size
    max_cache_size: usize,
    /// Current cache size
    current_cache_size: AtomicUsize,
}

impl PerCpuPageCache {
    /// Create a new per-CPU page cache
    pub const fn new(max_cache_size: usize) -> Self {
        Self {
            single_pages: Vec::new(),
            multi_pages: BTreeMap::new(),
            max_cache_size,
            current_cache_size: AtomicUsize::new(0),
        }
    }
    
    /// Get a single page from the cache
    pub fn get_single_page(&mut self) -> Option<PageFrame> {
        if let Some(pfn) = self.single_pages.pop() {
            self.current_cache_size.fetch_sub(1, Ordering::Relaxed);
            Some(pfn)
        } else {
            None
        }
    }
    
    /// Get multiple pages from the cache
    pub fn get_multi_pages(&mut self, order: PageOrder) -> Option<PageFrame> {
        if let Some(pages) = self.multi_pages.get_mut(&order) {
            if let Some(pfn) = pages.pop() {
                let page_count = 1usize << order;
                self.current_cache_size.fetch_sub(page_count, Ordering::Relaxed);
                return Some(pfn);
            }
        }
        None
    }
    
    /// Put a single page back to the cache
    pub fn put_single_page(&mut self, pfn: PageFrame) {
        if self.current_cache_size.load(Ordering::Relaxed) >= self.max_cache_size {
            return; // Cache is full
        }
        
        self.single_pages.push(pfn);
        self.current_cache_size.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Put multiple pages back to the cache
    pub fn put_multi_pages(&mut self, pfn: PageFrame, order: PageOrder) {
        let page_count = 1usize << order;
        if self.current_cache_size.load(Ordering::Relaxed) + page_count > self.max_cache_size {
            return; // Cache is full
        }
        
        let pages = self.multi_pages.entry(order).or_insert_with(Vec::new);
        pages.push(pfn);
        self.current_cache_size.fetch_add(page_count, Ordering::Relaxed);
    }
    
    /// Flush the cache (return all pages to the main allocator)
    pub fn flush(&mut self) -> (Vec<PageFrame>, BTreeMap<PageOrder, Vec<PageFrame>>) {
        let single_pages = core::mem::take(&mut self.single_pages);
        let multi_pages = core::mem::take(&mut self.multi_pages);
        self.current_cache_size.store(0, Ordering::Relaxed);
        (single_pages, multi_pages)
    }
}

/// Buddy allocator for multi-page allocations
#[derive(Debug)]
pub struct BuddyAllocator {
    /// Free lists for each order
    free_lists: [AtomicPtr<PageFrame>; MAX_ORDER as usize + 1],
    /// Total pages managed
    total_pages: usize,
    /// Free pages count
    free_pages: AtomicUsize,
    /// Allocation statistics
    alloc_stats: AllocationStats,
}

/// Allocation statistics
#[derive(Debug, Default)]
pub struct AllocationStats {
    /// Total allocations
    pub total_allocations: AtomicUsize,
    /// Total deallocations
    pub total_deallocations: AtomicUsize,
    /// Failed allocations
    pub failed_allocations: AtomicUsize,
    /// Fast path hits (from cache)
    pub fast_path_hits: AtomicUsize,
    /// Slow path allocations (from buddy)
    pub slow_path_allocations: AtomicUsize,
    /// Defragmentation runs
    pub defragmentation_runs: AtomicUsize,
}

/// Free list node
#[repr(C)]
struct FreeListNode {
    next: *mut FreeListNode,
    prev: *mut FreeListNode,
}

impl BuddyAllocator {
    /// Create a new buddy allocator
    pub const fn new() -> Self {
        const NULL: AtomicPtr<PageFrame> = AtomicPtr::new(null_mut());
        Self {
            free_lists: [NULL; MAX_ORDER as usize + 1],
            total_pages: 0,
            free_pages: AtomicUsize::new(0),
            alloc_stats: AllocationStats::default(),
        }
    }
    
    /// Initialize the buddy allocator with a memory range
    pub unsafe fn init(&mut self, start: usize, end: usize) {
        let start_pfn = start / PAGE_SIZE;
        let end_pfn = end / PAGE_SIZE;
        self.total_pages = end_pfn - start_pfn;
        
        // Initialize all free lists to empty
        for i in 0..=MAX_ORDER {
            self.free_lists[i as usize] = AtomicPtr::new(null_mut());
        }
        
        // Add all pages to the free lists
        let mut remaining_pages = self.total_pages;
        let mut current_pfn = start_pfn;
        
        // Try to add pages in the largest possible chunks
        for order in (0..=MAX_ORDER).rev() {
            let chunk_size = 1usize << order;
            
            while remaining_pages >= chunk_size && (current_pfn % chunk_size == 0) {
                self.add_to_free_list(current_pfn, order);
                current_pfn += chunk_size;
                remaining_pages -= chunk_size;
            }
        }
        
        self.free_pages.store(self.total_pages, Ordering::Relaxed);
    }
    
    /// Allocate pages of the given order
    pub fn allocate(&self, order: PageOrder) -> Option<PageFrame> {
        if order > MAX_ORDER {
            return None;
        }
        
        // Try to find a free block of the requested order
        for current_order in order..=MAX_ORDER {
            let pfn = self.remove_from_free_list(current_order);
            if pfn.is_some() {
                let pfn = pfn.unwrap();
                
                // If we got a larger block, split it
                if current_order > order {
                    self.split_block(pfn, current_order, order);
                }
                
                self.free_pages.fetch_sub(1usize << order, Ordering::Relaxed);
                self.alloc_stats.total_allocations.fetch_add(1, Ordering::Relaxed);
                self.alloc_stats.slow_path_allocations.fetch_add(1, Ordering::Relaxed);
                
                return Some(pfn);
            }
        }
        
        // No suitable block found
        self.alloc_stats.failed_allocations.fetch_add(1, Ordering::Relaxed);
        None
    }
    
    /// Deallocate pages of the given order
    pub fn deallocate(&self, pfn: PageFrame, order: PageOrder) {
        if order > MAX_ORDER {
            return;
        }
        
        // Try to merge with buddies
        let merged_pfn = self.try_merge(pfn, order);
        self.add_to_free_list(merged_pfn.0, merged_pfn.1);
        
        self.free_pages.fetch_add(1usize << merged_pfn.1, Ordering::Relaxed);
        self.alloc_stats.total_deallocations.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Get allocation statistics
    pub fn get_stats(&self) -> &AllocationStats {
        &self.alloc_stats
    }
    
    /// Get the number of free pages
    pub fn free_pages(&self) -> usize {
        self.free_pages.load(Ordering::Relaxed)
    }
    
    /// Get the total number of pages
    pub fn total_pages(&self) -> usize {
        self.total_pages
    }
    
    /// Add a block to the free list
    fn add_to_free_list(&self, pfn: PageFrame, order: PageOrder) {
        let node = pfn as *mut FreeListNode;
        
        unsafe {
            (*node).next = self.free_lists[order as usize].load(Ordering::Relaxed) as *mut FreeListNode;
            (*node).prev = null_mut();
            
            if !(*node).next.is_null() {
                (*(*node).next).prev = node;
            }
        }
        
        self.free_lists[order as usize].store(node as *mut PageFrame, Ordering::Relaxed);
    }
    
    /// Remove a block from the free list
    fn remove_from_free_list(&self, order: PageOrder) -> Option<PageFrame> {
        let node = self.free_lists[order as usize].load(Ordering::Relaxed) as *mut FreeListNode;
        
        if node.is_null() {
            return None;
        }
        
        unsafe {
            let next = (*node).next;
            
            if !next.is_null() {
                (*next).prev = null_mut();
            }
            
            self.free_lists[order as usize].store(next as *mut PageFrame, Ordering::Relaxed);
        }
        
        Some(node as PageFrame)
    }
    
    /// Split a block into smaller blocks
    fn split_block(&self, pfn: PageFrame, current_order: PageOrder, target_order: PageOrder) {
        let mut current_pfn = pfn;
        let mut current_order = current_order;
        
        while current_order > target_order {
            current_order -= 1;
            let buddy_pfn = current_pfn + (1usize << current_order);
            
            // Add the buddy to the free list
            self.add_to_free_list(buddy_pfn, current_order);
            
            // Continue with the first half
            current_pfn = current_pfn;
        }
    }
    
    /// Try to merge a block with its buddies
    fn try_merge(&self, pfn: PageFrame, order: PageOrder) -> (PageFrame, PageOrder) {
        let mut current_pfn = pfn;
        let mut current_order = order;
        
        while current_order < MAX_ORDER {
            let buddy_pfn = current_pfn ^ (1usize << current_order); // XOR to find buddy
            
            // Check if buddy is free
            if !self.is_buddy_free(buddy_pfn, current_order) {
                break;
            }
            
            // Remove buddy from free list
            self.remove_from_free_list(current_order);
            
            // Merge with buddy
            if buddy_pfn < current_pfn {
                current_pfn = buddy_pfn;
            }
            
            current_order += 1;
        }
        
        (current_pfn, current_order)
    }
    
    /// Check if a buddy is free
    fn is_buddy_free(&self, pfn: PageFrame, order: PageOrder) -> bool {
        let node = self.free_lists[order as usize].load(Ordering::Relaxed) as *mut FreeListNode;
        let mut current = node;
        
        while !current.is_null() {
            if current as PageFrame == pfn {
                return true;
            }
            
            unsafe {
                current = (*current).next;
            }
        }
        
        false
    }
}

/// Optimized page allocator with per-CPU caches
pub struct OptimizedPageAllocator {
    /// Buddy allocator for multi-page allocations
    buddy: BuddyAllocator,
    /// Per-CPU caches
    per_cpu_caches: Vec<Mutex<PerCpuPageCache>>,
    /// Current CPU ID
    current_cpu: AtomicUsize,
    /// Page descriptors
    page_descriptors: Vec<PageDescriptor>,
    /// Defragmentation enabled
    defragmentation_enabled: AtomicBool,
    /// NUMA node ID
    numa_node: u8,
}

impl OptimizedPageAllocator {
    /// Create a new optimized page allocator
    pub fn new(num_cpus: usize, numa_node: u8) -> Self {
        let mut per_cpu_caches = Vec::with_capacity(num_cpus);
        for _ in 0..num_cpus {
            per_cpu_caches.push(Mutex::new(PerCpuPageCache::new(64))); // 64 pages per CPU cache
        }
        
        Self {
            buddy: BuddyAllocator::new(),
            per_cpu_caches,
            current_cpu: AtomicUsize::new(0),
            page_descriptors: Vec::new(),
            defragmentation_enabled: AtomicBool::new(true),
            numa_node,
        }
    }
    
    /// Initialize the allocator with a memory range
    pub unsafe fn init(&mut self, start: usize, end: usize) {
        self.buddy.init(start, end);
        
        // Initialize page descriptors
        let start_pfn = start / PAGE_SIZE;
        let end_pfn = end / PAGE_SIZE;
        self.page_descriptors.reserve(end_pfn - start_pfn);
        
        for pfn in start_pfn..end_pfn {
            self.page_descriptors.push(PageDescriptor {
                pfn,
                order: 0,
                numa_node: self.numa_node,
                ref_count: AtomicUsize::new(0),
                flags: PageFlags::default(),
                last_access: AtomicUsize::new(0),
            });
        }
    }
    
    /// Allocate a single page
    pub fn allocate_page(&self) -> Option<PageFrame> {
        let cpu_id = self.current_cpu.load(Ordering::Relaxed) % self.per_cpu_caches.len();
        
        // Try per-CPU cache first
        {
            let mut cache = self.per_cpu_caches[cpu_id].lock();
            if let Some(pfn) = cache.get_single_page() {
                self.update_page_descriptor(pfn, 0, true);
                self.buddy.get_stats().fast_path_hits.fetch_add(1, Ordering::Relaxed);
                return Some(pfn);
            }
        }
        
        // Fall back to buddy allocator
        if let Some(pfn) = self.buddy.allocate(0) {
            self.update_page_descriptor(pfn, 0, true);
            return Some(pfn);
        }
        
        None
    }
    
    /// Allocate multiple pages
    pub fn allocate_pages(&self, order: PageOrder) -> Option<PageFrame> {
        let cpu_id = self.current_cpu.load(Ordering::Relaxed) % self.per_cpu_caches.len();
        
        // Try per-CPU cache first
        {
            let mut cache = self.per_cpu_caches[cpu_id].lock();
            if let Some(pfn) = cache.get_multi_pages(order) {
                self.update_page_descriptor(pfn, order, true);
                self.buddy.get_stats().fast_path_hits.fetch_add(1, Ordering::Relaxed);
                return Some(pfn);
            }
        }
        
        // Fall back to buddy allocator
        if let Some(pfn) = self.buddy.allocate(order) {
            self.update_page_descriptor(pfn, order, true);
            return Some(pfn);
        }
        
        None
    }
    
    /// Deallocate a single page
    pub fn deallocate_page(&self, pfn: PageFrame) {
        if pfn >= self.page_descriptors.len() {
            return;
        }
        
        let descriptor = &self.page_descriptors[pfn];
        if !descriptor.flags.allocated {
            return; // Already free
        }
        
        self.update_page_descriptor(pfn, 0, false);
        
        let cpu_id = self.current_cpu.load(Ordering::Relaxed) % self.per_cpu_caches.len();
        
        // Try to return to per-CPU cache
        {
            let mut cache = self.per_cpu_caches[cpu_id].lock();
            cache.put_single_page(pfn);
            return;
        }
        
        // Fall back to buddy allocator
        self.buddy.deallocate(pfn, 0);
    }
    
    /// Deallocate multiple pages
    pub fn deallocate_pages(&self, pfn: PageFrame, order: PageOrder) {
        if pfn >= self.page_descriptors.len() {
            return;
        }
        
        let descriptor = &self.page_descriptors[pfn];
        if !descriptor.flags.allocated {
            return; // Already free
        }
        
        self.update_page_descriptor(pfn, order, false);
        
        let cpu_id = self.current_cpu.load(Ordering::Relaxed) % self.per_cpu_caches.len();
        
        // Try to return to per-CPU cache
        {
            let mut cache = self.per_cpu_caches[cpu_id].lock();
            cache.put_multi_pages(pfn, order);
            return;
        }
        
        // Fall back to buddy allocator
        self.buddy.deallocate(pfn, order);
    }
    
    /// Set the current CPU ID
    pub fn set_current_cpu(&self, cpu_id: usize) {
        self.current_cpu.store(cpu_id, Ordering::Relaxed);
    }
    
    /// Get allocation statistics
    pub fn get_stats(&self) -> &AllocationStats {
        self.buddy.get_stats()
    }
    
    /// Get the number of free pages
    pub fn free_pages(&self) -> usize {
        self.buddy.free_pages()
    }
    
    /// Get the total number of pages
    pub fn total_pages(&self) -> usize {
        self.buddy.total_pages()
    }
    
    /// Enable or disable defragmentation
    pub fn set_defragmentation(&self, enabled: bool) {
        self.defragmentation_enabled.store(enabled, Ordering::Relaxed);
    }
    
    /// Run defragmentation
    pub fn defragment(&self) -> usize {
        if !self.defragmentation_enabled.load(Ordering::Relaxed) {
            return 0;
        }
        
        self.buddy.get_stats().defragmentation_runs.fetch_add(1, Ordering::Relaxed);
        
        // Simple defragmentation: flush all per-CPU caches
        let mut total_freed = 0;
        
        for cache in &self.per_cpu_caches {
            let mut cache_guard = cache.lock();
            let (single_pages, multi_pages) = cache_guard.flush();
            
            // Return single pages to buddy allocator
            for pfn in single_pages {
                self.buddy.deallocate(pfn, 0);
                total_freed += 1;
            }
            
            // Return multi-pages to buddy allocator
            for (order, pages) in multi_pages {
                for pfn in pages {
                    self.buddy.deallocate(pfn, order);
                    total_freed += 1usize << order;
                }
            }
        }
        
        total_freed
    }
    
    /// Update page descriptor
    fn update_page_descriptor(&self, pfn: PageFrame, order: PageOrder, allocated: bool) {
        if pfn < self.page_descriptors.len() {
            let descriptor = &self.page_descriptors[pfn];
            descriptor.order = order;
            descriptor.flags.allocated = allocated;
            
            if allocated {
                descriptor.last_access.store(crate::time::get_ticks(), Ordering::Relaxed);
            }
        }
    }
    
    /// Get page descriptor
    pub fn get_page_descriptor(&self, pfn: PageFrame) -> Option<&PageDescriptor> {
        self.page_descriptors.get(pfn)
    }
}

// Implement Send and Sync for the allocator
unsafe impl Send for OptimizedPageAllocator {}
unsafe impl Sync for OptimizedPageAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_cpu_cache() {
        let mut cache = PerCpuPageCache::new(10);
        
        // Test single page cache
        assert_eq!(cache.get_single_page(), None);
        
        cache.put_single_page(100);
        assert_eq!(cache.get_single_page(), Some(100));
        assert_eq!(cache.get_single_page(), None);
        
        // Test multi-page cache
        assert_eq!(cache.get_multi_pages(2), None);
        
        cache.put_multi_pages(200, 2);
        assert_eq!(cache.get_multi_pages(2), Some(200));
        assert_eq!(cache.get_multi_pages(2), None);
    }
    
    #[test]
    fn test_buddy_allocator() {
        let mut buddy = BuddyAllocator::new();
        
        // Initialize with a small range
        unsafe {
            buddy.init(0x100000, 0x200000); // 1MB range
        }
        
        let initial_free = buddy.free_pages();
        
        // Allocate a single page
        let pfn1 = buddy.allocate(0);
        assert!(pfn1.is_some());
        assert_eq!(buddy.free_pages(), initial_free - 1);
        
        // Allocate multiple pages
        let pfn2 = buddy.allocate(2); // 4 pages
        assert!(pfn2.is_some());
        assert_eq!(buddy.free_pages(), initial_free - 1 - 4);
        
        // Deallocate pages
        buddy.deallocate(pfn1.unwrap(), 0);
        assert_eq!(buddy.free_pages(), initial_free - 4);
        
        buddy.deallocate(pfn2.unwrap(), 2);
        assert_eq!(buddy.free_pages(), initial_free);
    }
    
    #[test]
    fn test_optimized_page_allocator() {
        let mut allocator = OptimizedPageAllocator::new(2, 0);
        
        // Initialize with a small range
        unsafe {
            allocator.init(0x100000, 0x200000); // 1MB range
        }
        
        let initial_free = allocator.free_pages();
        
        // Set current CPU
        allocator.set_current_cpu(0);
        
        // Allocate a single page
        let pfn1 = allocator.allocate_page();
        assert!(pfn1.is_some());
        assert_eq!(allocator.free_pages(), initial_free - 1);
        
        // Allocate multiple pages
        let pfn2 = allocator.allocate_pages(2); // 4 pages
        assert!(pfn2.is_some());
        assert_eq!(allocator.free_pages(), initial_free - 1 - 4);
        
        // Deallocate pages
        allocator.deallocate_page(pfn1.unwrap());
        assert_eq!(allocator.free_pages(), initial_free - 4);
        
        allocator.deallocate_pages(pfn2.unwrap(), 2);
        assert_eq!(allocator.free_pages(), initial_free);
        
        // Test defragmentation
        allocator.set_current_cpu(1);
        let pfn3 = allocator.allocate_page();
        assert!(pfn3.is_some());
        
        let freed = allocator.defragment();
        assert!(freed > 0);
    }
}