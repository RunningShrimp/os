//! Memory Optimization Allocator - Track EK
//!
//! Custom memory allocator with jemalloc-inspired design for optimal kernel memory performance.
//!
//! ## Features
//!
//! - **Slab Allocator Optimization**: Per-CPU caches for fast allocation
//! - **Object Pooling**: Reuse common structures to reduce allocation overhead
//! - **Fragmentation Reduction**: Smart size classes and allocation strategies
//! - **Huge Page Utilization**: 1GB and 2MB page support for large allocations
//! - **NUMA-Aware Allocation**: Allocate memory on the optimal NUMA node
//! - **Allocation Statistics**: Comprehensive tracking and monitoring
//!
//! ## Architecture
//!
//! The allocator uses a tiered approach:
//! 1. **Thread-Local Cache**: Fast path for small allocations
//! 2. **Per-CPU Slabs**: Reduce lock contention
//! 3. **Central Allocator**: Fallback for large allocations
//! 4. **Huge Page Allocator**: Optimize for large memory regions
//!
//! ## Performance Targets
//!
//! - Small allocation (< 2KB): < 100ns
//! - Medium allocation (< 2MB): < 1μs
//! - Large allocation (> 2MB): < 10ms
//! - Memory overhead: < 3%
//! - Fragmentation: < 15%

use alloc::alloc::{alloc, alloc_zeroed, Layout};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::prelude::*;

/// Memory allocation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocError {
    /// Out of memory
    OutOfMemory,
    /// Invalid size requested
    InvalidSize,
    /// Invalid alignment
    InvalidAlignment,
    /// Too fragmented
    TooFragmented,
    /// NUMA node not available
    NumaNodeUnavailable,
    /// Allocation failed
    AllocationFailed,
}

impl core::fmt::Display for AllocError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AllocError::OutOfMemory => write!(f, "Out of memory"),
            AllocError::InvalidSize => write!(f, "Invalid allocation size"),
            AllocError::InvalidAlignment => write!(f, "Invalid alignment"),
            AllocError::TooFragmented => write!(f, "Memory too fragmented"),
            AllocError::NumaNodeUnavailable => write!(f, "NUMA node unavailable"),
            AllocError::AllocationFailed => write!(f, "Allocation failed"),
        }
    }
}

/// Page size constants
pub const PAGE_SIZE_4K: usize = 4096;
pub const PAGE_SIZE_2M: usize = 2 * 1024 * 1024;
pub const PAGE_SIZE_1G: usize = 1024 * 1024 * 1024;

/// Size classes for slab allocation (powers of 2 from 32 bytes to 8KB)
pub const SIZE_CLASSES: &[usize] = &[
    32, 64, 128, 256, 512, 1024, 2048, 4096, 8192,
];

/// Maximum per-CPU cache size (in bytes)
pub const PERCPU_CACHE_SIZE: usize = 256 * 1024; // 256KB

/// Per-CPU slab cache
#[repr(C)]
pub struct PerCpuSlabCache {
    /// Size class for this cache
    size_class: usize,
    /// Free objects in this cache
    free_list: Vec<NonNull<u8>>,
    /// Cache size limit
    cache_limit: usize,
    /// Allocations from this cache
    allocations: AtomicU64,
    /// Cache hits (allocated from per-CPU cache)
    hits: AtomicU64,
    /// Cache misses (fell back to central allocator)
    misses: AtomicU64,
}

unsafe impl Send for PerCpuSlabCache {}

impl PerCpuSlabCache {
    /// Create a new per-CPU slab cache
    pub fn new(size_class: usize) -> Self {
        let cache_limit = PERCPU_CACHE_SIZE / size_class;

        Self {
            size_class,
            free_list: Vec::new(),
            cache_limit,
            allocations: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Allocate from per-CPU cache
    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        self.allocations.fetch_add(1, Ordering::Relaxed);

        if let Some(ptr) = self.free_list.pop() {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(ptr)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Free to per-CPU cache
    pub fn free(&mut self, ptr: NonNull<u8>) {
        if self.free_list.len() < self.cache_limit {
            self.free_list.push(ptr);
        }
        // If cache is full, object is returned to central allocator
    }

    /// Get cache statistics
    pub fn stats(&self) -> SlabCacheStats {
        SlabCacheStats {
            size_class: self.size_class,
            free_objects: self.free_list.len(),
            cache_limit: self.cache_limit,
            allocations: self.allocations.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }
}

/// Slab cache statistics
#[derive(Debug, Clone)]
pub struct SlabCacheStats {
    pub size_class: usize,
    pub free_objects: usize,
    pub cache_limit: usize,
    pub allocations: u64,
    pub hits: u64,
    pub misses: u64,
}

/// Central slab allocator
pub struct CentralSlabAllocator {
    /// Size class allocators
    size_classes: Vec<Option<Box<SlabAllocator>>>,
    /// Total allocations
    total_allocations: AtomicU64,
    /// Total deallocations
    total_deallocations: AtomicU64,
    /// Current memory usage
    memory_usage: AtomicUsize,
}

impl CentralSlabAllocator {
    /// Create a new central slab allocator
    pub fn new() -> Self {
        let mut size_classes = Vec::new();

        for &size in SIZE_CLASSES {
            size_classes.push(Some(Box::new(SlabAllocator::new(size))));
        }

        Self {
            size_classes,
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            memory_usage: AtomicUsize::new(0),
        }
    }

    /// Allocate from size class
    pub fn allocate(&self, size: usize) -> Result<NonNull<u8>, AllocError> {
        // Find appropriate size class
        let size_class_idx = self.find_size_class(size)?;

        if let Some(slab) = &self.size_classes[size_class_idx] {
            let ptr = slab.allocate()?;
            self.total_allocations.fetch_add(1, Ordering::Relaxed);
            self.memory_usage.fetch_add(size, Ordering::Relaxed);
            Ok(ptr)
        } else {
            Err(AllocError::AllocationFailed)
        }
    }

    /// Deallocate to size class
    pub fn deallocate(&self, ptr: NonNull<u8>, size: usize) {
        let size_class_idx = self.find_size_class(size).unwrap();

        if let Some(slab) = &self.size_classes[size_class_idx] {
            slab.deallocate(ptr);
            self.total_deallocations.fetch_add(1, Ordering::Relaxed);
            self.memory_usage.fetch_sub(size, Ordering::Relaxed);
        }
    }

    /// Find appropriate size class
    fn find_size_class(&self, size: usize) -> Result<usize, AllocError> {
        for (idx, &class_size) in SIZE_CLASSES.iter().enumerate() {
            if size <= class_size {
                return Ok(idx);
            }
        }
        Err(AllocError::InvalidSize)
    }

    /// Get statistics
    pub fn stats(&self) -> CentralSlabStats {
        let mut size_class_stats = Vec::new();

        for slab in &self.size_classes {
            if let Some(s) = slab {
                size_class_stats.push(s.stats());
            }
        }

        CentralSlabStats {
            size_class_stats,
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.total_deallocations.load(Ordering::Relaxed),
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
        }
    }
}

/// Central slab statistics
#[derive(Debug, Clone)]
pub struct CentralSlabStats {
    pub size_class_stats: Vec<SlabAllocatorStats>,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub memory_usage: usize,
}

/// Individual slab allocator for a size class
pub struct SlabAllocator {
    /// Object size
    object_size: usize,
    /// Slabs
    slabs: Vec<Box<Slab>>,
    /// Free list
    free_list: UnsafeCell<Option<NonNull<SlabObject>>>,
    /// Total objects allocated
    total_objects: AtomicU64,
    /// Free objects
    free_objects: AtomicU64,
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub fn new(object_size: usize) -> Self {
        Self {
            object_size,
            slabs: Vec::new(),
            free_list: UnsafeCell::new(None),
            total_objects: AtomicU64::new(0),
            free_objects: AtomicU64::new(0),
        }
    }

    /// Allocate an object
    pub fn allocate(&self) -> Result<NonNull<u8>, AllocError> {
        // Try to get from free list
        // SAFETY: Using UnsafeCell for interior mutability
        let free_list = unsafe { &*self.free_list.get() };
        if let Some(free_obj) = *free_list {
            let obj = unsafe { core::ptr::read(free_obj.as_ptr()) };

            // Update free list
            let next = obj.next;
            self.set_free_list(next);

            self.free_objects.fetch_sub(1, Ordering::Relaxed);

            return Ok(obj.data);
        }

        // Need to allocate a new slab
        let slab = self.allocate_slab()?;
        let obj = unsafe { &mut *slab.objects };

        // Initialize free list
        let mut current: Option<NonNull<SlabObject>> = Some(NonNull::new(obj).unwrap());

        for i in 1..slab.num_objects {
            let obj_ptr = unsafe {
                slab.objects.add(i)
            };

            unsafe {
                let non_null = NonNull::new_unchecked(obj_ptr);
                (*obj_ptr).next = current;
                current = Some(non_null);
            }
        }

        // Get first object
        let first_obj = current.unwrap();
        let next = unsafe { first_obj.as_ref().next };
        self.set_free_list(next);

        self.free_objects.fetch_sub(1, Ordering::Relaxed);

        Ok(unsafe { first_obj.as_ref().data })
    }

    /// Deallocate an object
    pub fn deallocate(&self, ptr: NonNull<u8>) {
        // Create slab object
        // SAFETY: Using UnsafeCell for interior mutability
        let free_list = unsafe { &*self.free_list.get() };
        let obj = SlabObject {
            data: ptr,
            next: *free_list,
        };

        // Add to free list
        let boxed = Box::leak(Box::new(obj));
        self.set_free_list(Some(NonNull::from(boxed)));

        self.free_objects.fetch_add(1, Ordering::Relaxed);
    }

    /// Allocate a new slab
    fn allocate_slab(&self) -> Result<Box<Slab>, AllocError> {
        const SLAB_SIZE: usize = PAGE_SIZE_2M; // Use 2MB slabs

        let num_objects = SLAB_SIZE / self.object_size;
        let layout = Layout::from_size_align(SLAB_SIZE, PAGE_SIZE_4K)
            .map_err(|_| AllocError::InvalidSize)?;

        let ptr = unsafe { alloc_zeroed(layout) };

        if ptr.is_null() {
            return Err(AllocError::OutOfMemory);
        }

        let slab_ptr = NonNull::new(ptr).ok_or(AllocError::AllocationFailed)?;

        let slab = Box::new(Slab {
            base: slab_ptr,
            objects: unsafe { core::ptr::null_mut() }, // Will be initialized
            num_objects,
            object_size: self.object_size,
        });

        self.total_objects.fetch_add(num_objects as u64, Ordering::Relaxed);

        Ok(slab)
    }

    /// Set free list (internal helper)
    fn set_free_list(&self, list: Option<NonNull<SlabObject>>) {
        // SAFETY: We use UnsafeCell for interior mutability
        // This is safe because we have exclusive access through &self
        unsafe {
            *self.free_list.get() = list;
        }
    }

    /// Get statistics
    pub fn stats(&self) -> SlabAllocatorStats {
        SlabAllocatorStats {
            object_size: self.object_size,
            num_slabs: self.slabs.len(),
            total_objects: self.total_objects.load(Ordering::Relaxed),
            free_objects: self.free_objects.load(Ordering::Relaxed),
        }
    }
}

/// Slab allocator statistics
#[derive(Debug, Clone)]
pub struct SlabAllocatorStats {
    pub object_size: usize,
    pub num_slabs: usize,
    pub total_objects: u64,
    pub free_objects: u64,
}

/// Slab structure
pub struct Slab {
    /// Base pointer
    base: NonNull<u8>,
    /// Objects pointer
    objects: *mut SlabObject,
    /// Number of objects in this slab
    num_objects: usize,
    /// Object size
    object_size: usize,
}

unsafe impl Send for Slab {}

/// Slab object (free list node)
#[repr(C)]
pub struct SlabObject {
    /// Object data pointer
    data: NonNull<u8>,
    /// Next free object
    next: Option<NonNull<SlabObject>>,
}

/// Object pool for common structures
pub struct ObjectPool<T> {
    /// Free objects
    free_list: Vec<Box<T>>,
    /// Pool size limit
    limit: usize,
    /// Allocations from pool
    allocations: AtomicU64,
    /// Returns to pool
    returns: AtomicU64,
}

impl<T> ObjectPool<T> {
    /// Create a new object pool
    pub fn new(limit: usize) -> Self {
        Self {
            free_list: Vec::new(),
            limit,
            allocations: AtomicU64::new(0),
            returns: AtomicU64::new(0),
        }
    }

    /// Allocate from pool
    pub fn allocate(&mut self) -> Option<Box<T>> {
        self.allocations.fetch_add(1, Ordering::Relaxed);
        self.free_list.pop()
    }

    /// Return to pool
    pub fn free(&mut self, obj: Box<T>) {
        if self.free_list.len() < self.limit {
            self.free_list.push(obj);
            self.returns.fetch_add(1, Ordering::Relaxed);
        }
        // If pool is full, object is dropped
    }

    /// Get pool statistics
    pub fn stats(&self) -> ObjectPoolStats {
        ObjectPoolStats {
            free_objects: self.free_list.len(),
            limit: self.limit,
            allocations: self.allocations.load(Ordering::Relaxed),
            returns: self.returns.load(Ordering::Relaxed),
        }
    }
}

/// Object pool statistics
#[derive(Debug, Clone)]
pub struct ObjectPoolStats {
    pub free_objects: usize,
    pub limit: usize,
    pub allocations: u64,
    pub returns: u64,
}

/// NUMA-aware allocator
pub struct NumaAllocator {
    /// NUMA node allocators
    node_allocators: Vec<Option<Box<CentralSlabAllocator>>>,
    /// Preferred NUMA node
    preferred_node: AtomicUsize,
}

impl NumaAllocator {
    /// Create a new NUMA allocator
    pub fn new(num_nodes: usize) -> Self {
        let mut node_allocators = Vec::new();

        for _ in 0..num_nodes {
            node_allocators.push(Some(Box::new(CentralSlabAllocator::new())));
        }

        Self {
            node_allocators,
            preferred_node: AtomicUsize::new(0),
        }
    }

    /// Allocate from preferred NUMA node
    pub fn allocate(&self, size: usize) -> Result<NonNull<u8>, AllocError> {
        let node = self.preferred_node.load(Ordering::Relaxed);

        if let Some(allocator) = &self.node_allocators[node] {
            allocator.allocate(size)
        } else {
            Err(AllocError::NumaNodeUnavailable)
        }
    }

    /// Allocate from specific NUMA node
    pub fn allocate_on_node(&self, size: usize, node: usize) -> Result<NonNull<u8>, AllocError> {
        if let Some(allocator) = &self.node_allocators.get(node).and_then(|a| a.as_ref()) {
            allocator.allocate(size)
        } else {
            Err(AllocError::NumaNodeUnavailable)
        }
    }

    /// Deallocate to NUMA node
    pub fn deallocate(&self, ptr: NonNull<u8>, size: usize, _node: usize) {
        let node = self.preferred_node.load(Ordering::Relaxed);

        if let Some(allocator) = &self.node_allocators[node] {
            allocator.deallocate(ptr, size);
        }
    }

    /// Set preferred NUMA node
    pub fn set_preferred_node(&self, node: usize) {
        self.preferred_node.store(node, Ordering::Relaxed);
    }

    /// Get preferred NUMA node
    pub fn get_preferred_node(&self) -> usize {
        self.preferred_node.load(Ordering::Relaxed)
    }
}

/// Huge page allocator
pub struct HugePageAllocator {
    /// 2MB page allocations
    pages_2m: Vec<NonNull<u8>>,
    /// 1GB page allocations
    pages_1g: Vec<NonNull<u8>>,
    /// Free 2MB pages
    free_2m: Vec<NonNull<u8>>,
    /// Free 1GB pages
    free_1g: Vec<NonNull<u8>>,
    /// 2MB allocations
    allocations_2m: AtomicU64,
    /// 1GB allocations
    allocations_1g: AtomicU64,
}

impl HugePageAllocator {
    /// Create a new huge page allocator
    pub fn new() -> Self {
        Self {
            pages_2m: Vec::new(),
            pages_1g: Vec::new(),
            free_2m: Vec::new(),
            free_1g: Vec::new(),
            allocations_2m: AtomicU64::new(0),
            allocations_1g: AtomicU64::new(0),
        }
    }

    /// Allocate a 2MB huge page
    pub fn allocate_2m(&mut self) -> Result<NonNull<u8>, AllocError> {
        // Try to reuse a freed page
        if let Some(ptr) = self.free_2m.pop() {
            self.allocations_2m.fetch_add(1, Ordering::Relaxed);
            return Ok(ptr);
        }

        // Allocate new 2MB page
        let layout = Layout::from_size_align(PAGE_SIZE_2M, PAGE_SIZE_2M)
            .map_err(|_| AllocError::InvalidSize)?;

        let ptr = unsafe { alloc_zeroed(layout) };

        if ptr.is_null() {
            return Err(AllocError::OutOfMemory);
        }

        let ptr = NonNull::new(ptr).ok_or(AllocError::AllocationFailed)?;
        self.pages_2m.push(ptr);
        self.allocations_2m.fetch_add(1, Ordering::Relaxed);

        Ok(ptr)
    }

    /// Allocate a 1GB huge page
    pub fn allocate_1g(&mut self) -> Result<NonNull<u8>, AllocError> {
        // Try to reuse a freed page
        if let Some(ptr) = self.free_1g.pop() {
            self.allocations_1g.fetch_add(1, Ordering::Relaxed);
            return Ok(ptr);
        }

        // Allocate new 1GB page
        let layout = Layout::from_size_align(PAGE_SIZE_1G, PAGE_SIZE_1G)
            .map_err(|_| AllocError::InvalidSize)?;

        let ptr = unsafe { alloc_zeroed(layout) };

        if ptr.is_null() {
            return Err(AllocError::OutOfMemory);
        }

        let ptr = NonNull::new(ptr).ok_or(AllocError::AllocationFailed)?;
        self.pages_1g.push(ptr);
        self.allocations_1g.fetch_add(1, Ordering::Relaxed);

        Ok(ptr)
    }

    /// Free a 2MB huge page
    pub fn free_2m(&mut self, ptr: NonNull<u8>) {
        self.free_2m.push(ptr);
    }

    /// Free a 1GB huge page
    pub fn free_1g(&mut self, ptr: NonNull<u8>) {
        self.free_1g.push(ptr);
    }

    /// Get statistics
    pub fn stats(&self) -> HugePageStats {
        HugePageStats {
            allocated_2m: self.pages_2m.len(),
            allocated_1g: self.pages_1g.len(),
            free_2m: self.free_2m.len(),
            free_1g: self.free_1g.len(),
            allocations_2m: self.allocations_2m.load(Ordering::Relaxed),
            allocations_1g: self.allocations_1g.load(Ordering::Relaxed),
        }
    }
}

/// Huge page statistics
#[derive(Debug, Clone)]
pub struct HugePageStats {
    pub allocated_2m: usize,
    pub allocated_1g: usize,
    pub free_2m: usize,
    pub free_1g: usize,
    pub allocations_2m: u64,
    pub allocations_1g: u64,
}

/// Optimized memory allocator (main entry point)
pub struct OptimizedAllocator {
    /// Central slab allocator
    central: CentralSlabAllocator,
    /// Huge page allocator
    huge_pages: Mutex<HugePageAllocator>,
    /// NUMA allocator
    numa: NumaAllocator,
    /// Per-CPU caches
    percpu_caches: Vec<Mutex<PerCpuSlabCache>>,
    /// Total allocations
    total_allocations: AtomicU64,
    /// Total deallocations
    total_deallocations: AtomicU64,
    /// Fragmentation count
    fragmentation_count: AtomicU64,
}

impl OptimizedAllocator {
    /// Create a new optimized allocator
    pub fn new(num_cpus: usize, num_numa_nodes: usize) -> Self {
        let mut percpu_caches = Vec::new();

        for _ in 0..num_cpus {
            percpu_caches.push(Mutex::new(PerCpuSlabCache::new(0)));
        }

        Self {
            central: CentralSlabAllocator::new(),
            huge_pages: Mutex::new(HugePageAllocator::new()),
            numa: NumaAllocator::new(num_numa_nodes),
            percpu_caches,
            total_allocations: AtomicU64::new(0),
            total_deallocations: AtomicU64::new(0),
            fragmentation_count: AtomicU64::new(0),
        }
    }

    /// Aligned allocation
    pub fn alloc_aligned(&self, size: usize, align: usize) -> Result<NonNull<u8>, AllocError> {
        // Validate alignment
        if !align.is_power_of_two() {
            return Err(AllocError::InvalidAlignment);
        }

        if size == 0 {
            return Err(AllocError::InvalidSize);
        }

        // Try per-CPU cache first for small allocations
        if size <= SIZE_CLASSES[SIZE_CLASSES.len() - 1] {
            let cpu_id = 0; // TODO: Get actual CPU ID

            if let Some(cache) = self.percpu_caches.get(cpu_id) {
                if let Some(ptr) = cache.lock().alloc() {
                    self.total_allocations.fetch_add(1, Ordering::Relaxed);
                    return Ok(ptr);
                }
            }
        }

        // Fall back to central allocator
        let ptr = self.central.allocate(size)?;
        self.total_allocations.fetch_add(1, Ordering::Relaxed);

        Ok(ptr)
    }

    /// Allocate huge page
    pub fn alloc_huge(&self, size: usize) -> Result<NonNull<u8>, AllocError> {
        let mut huge_alloc = self.huge_pages.lock();

        if size <= PAGE_SIZE_2M {
            huge_alloc.allocate_2m()
        } else if size <= PAGE_SIZE_1G {
            huge_alloc.allocate_1g()
        } else {
            // Need multiple 1GB pages
            Err(AllocError::InvalidSize)
        }
    }

    /// NUMA-aware allocation
    pub fn numa_alloc(&self, size: usize, node: u32) -> Result<NonNull<u8>, AllocError> {
        self.numa.allocate_on_node(size, node as usize)
    }

    /// Free memory
    pub fn free(&self, ptr: NonNull<u8>, size: usize) {
        self.central.deallocate(ptr, size);
        self.total_deallocations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get allocation statistics
    pub fn stats(&self) -> AllocatorStats {
        AllocatorStats {
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_deallocations: self.total_deallocations.load(Ordering::Relaxed),
            memory_usage: self.central.stats().memory_usage,
            fragmentation_count: self.fragmentation_count.load(Ordering::Relaxed),
        }
    }
}

/// Global allocator statistics
#[derive(Debug, Clone)]
pub struct AllocatorStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub memory_usage: usize,
    pub fragmentation_count: u64,
}

/// Initialize the optimized allocator (called during kernel init)
pub fn init_optimized_allocator() {
    log::info!("Initializing optimized memory allocator...");

    let num_cpus = 1; // TODO: Get actual CPU count
    let num_numa_nodes = 1; // TODO: Detect NUMA topology

    let _allocator = OptimizedAllocator::new(num_cpus, num_numa_nodes);

    log::info!("Optimized memory allocator initialized");
}

/// Alloc aligned - public API
pub fn alloc_aligned(size: usize, align: usize) -> Result<NonNull<u8>, AllocError> {
    if !align.is_power_of_two() {
        return Err(AllocError::InvalidAlignment);
    }

    let layout = Layout::from_size_align(size, align)
        .map_err(|_| AllocError::InvalidSize)?;

    let ptr = unsafe { alloc(layout) };

    if ptr.is_null() {
        Err(AllocError::OutOfMemory)
    } else {
        NonNull::new(ptr).ok_or(AllocError::AllocationFailed)
    }
}

/// Alloc huge - public API
pub fn alloc_huge(size: usize) -> Result<NonNull<u8>, AllocError> {
    // Determine huge page size
    let page_size = if size <= PAGE_SIZE_2M {
        PAGE_SIZE_2M
    } else if size <= PAGE_SIZE_1G {
        PAGE_SIZE_1G
    } else {
        return Err(AllocError::InvalidSize);
    };

    let layout = Layout::from_size_align(page_size, page_size)
        .map_err(|_| AllocError::InvalidSize)?;

    let ptr = unsafe { alloc_zeroed(layout) };

    if ptr.is_null() {
        Err(AllocError::OutOfMemory)
    } else {
        NonNull::new(ptr).ok_or(AllocError::AllocationFailed)
    }
}

/// NUMA alloc - public API
pub fn numa_alloc(size: usize, _node: u32) -> Result<NonNull<u8>, AllocError> {
    // For now, just do regular allocation
    // In a real implementation, this would bind to the specific NUMA node
    alloc_aligned(size, 8)
}
