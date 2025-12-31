//! Kernel Memory Optimization - Track EK
//!
//! Optimized kernel memory allocation with per-CPU caches and slab allocators.
//!
//! ## Features
//!
//! - **Per-CPU Allocator**: Lock-free allocation for kernel objects
//! - **Atomic Allocation Pools**: Lock-free data structures for parallel access
//! - **Quicklists**: Fast allocation for small objects
//! - **Memory cgroup Optimization**: Per-cgroup memory tracking
//! - **Slab Cache Tuning**: Dynamic slab cache management (reap, shrink)
//! - **Kernel Memory Statistics**: Comprehensive tracking
//!
//! ## Architecture
//!
//! The kernel memory allocator uses a tiered approach:
//! 1. **Per-CPU Caches**: Fast lock-free allocation
//! 2. **Quicklists**: Medium-speed allocation for small objects
//! 3. **Slab Allocators**: General-purpose allocation
//! 4. **Page Allocator**: Fallback for large allocations
//!
//! ## Performance Targets
//!
//! - Small allocation (< 128 bytes): < 30ns
//! - Medium allocation (< 4KB): < 100ns
//! - Large allocation (> 4KB): < 1μs
//! - Memory overhead: < 10%
//! - Lock contention: < 5%

use alloc::alloc::{alloc, dealloc, Layout};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::prelude::*;
use crate::subsystems::mm::PAGE_SIZE;

/// Kernel memory errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KmemError {
    /// Out of memory
    OutOfMemory,
    /// Invalid size
    InvalidSize,
    /// Slab cache error
    SlabCacheError,
    /// Per-CPU cache error
    PerCpuError,
    /// Memory cgroup limit exceeded
    CgroupLimitExceeded,
    /// Allocation failed
    AllocationFailed,
}

impl core::fmt::Display for KmemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KmemError::OutOfMemory => write!(f, "Out of memory"),
            KmemError::InvalidSize => write!(f, "Invalid allocation size"),
            KmemError::SlabCacheError => write!(f, "Slab cache error"),
            KmemError::PerCpuError => write!(f, "Per-CPU cache error"),
            KmemError::CgroupLimitExceeded => write!(f, "Memory cgroup limit exceeded"),
            KmemError::AllocationFailed => write!(f, "Allocation failed"),
        }
    }
}

/// Per-CPU kernel memory cache
#[repr(C)]
pub struct PerCpuKmemCache {
    /// CPU ID
    cpu_id: usize,
    /// Free objects
    free_list: Vec<NonNull<u8>>,
    /// Batch size for refill/drain
    batch_size: usize,
    /// Allocations from this cache
    allocations: AtomicU64,
    /// Cache hits
    hits: AtomicU64,
    /// Cache misses
    misses: AtomicU64,
}

unsafe impl Send for PerCpuKmemCache {}

impl PerCpuKmemCache {
    /// Create a new per-CPU cache
    pub fn new(cpu_id: usize, batch_size: usize) -> Self {
        Self {
            cpu_id,
            free_list: Vec::new(),
            batch_size,
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
        self.free_list.push(ptr);
    }

    /// Refill cache from central allocator
    pub fn refill(&mut self, central: &mut CentralKmemAllocator, size: usize) {
        for _ in 0..self.batch_size {
            if let Some(ptr) = central.allocate(size) {
                self.free_list.push(ptr);
            } else {
                break;
            }
        }
    }

    /// Drain cache to central allocator
    pub fn drain(&mut self, central: &mut CentralKmemAllocator) {
        while let Some(ptr) = self.free_list.pop() {
            central.deallocate(ptr, 0); // Size not tracked for simplicity
        }
    }

    /// Get statistics
    pub fn stats(&self) -> PerCpuStats {
        PerCpuStats {
            cpu_id: self.cpu_id,
            free_objects: self.free_list.len(),
            batch_size: self.batch_size,
            allocations: self.allocations.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }
}

/// Per-CPU statistics
#[derive(Debug, Clone)]
pub struct PerCpuStats {
    pub cpu_id: usize,
    pub free_objects: usize,
    pub batch_size: usize,
    pub allocations: u64,
    pub hits: u64,
    pub misses: u64,
}

/// Atomic allocation pool (lock-free)
pub struct AtomicAllocationPool {
    /// Free objects (atomic stack)
    free_list: AtomicOption,
    /// Pool size
    size: AtomicUsize,
    /// Allocations
    allocations: AtomicU64,
}

/// Atomic option for lock-free stack
struct AtomicOption {
    inner: AtomicUsize,
}

impl AtomicOption {
    fn new() -> Self {
        Self {
            inner: AtomicUsize::new(0),
        }
    }

    fn compare_exchange(&self, current: Option<NonNull<u8>>, new: Option<NonNull<u8>>) -> Result<Option<NonNull<u8>>, Option<NonNull<u8>>> {
        let current_val = current.map_or(0, |p| p.as_ptr() as usize);
        let new_val = new.map_or(0, |p| p.as_ptr() as usize);

        self.inner
            .compare_exchange(current_val, new_val, Ordering::AcqRel, Ordering::Relaxed)
            .map(|v| if v != 0 { Some(NonNull::new(v as *mut u8).unwrap()) } else { None })
            .map_err(|v| if v != 0 { Some(NonNull::new(v as *mut u8).unwrap()) } else { None })
    }
}

// SAFETY: AtomicOption is used for lock-free synchronization and only stores pointers
unsafe impl Send for AtomicOption {}
unsafe impl Sync for AtomicOption {}

impl AtomicAllocationPool {
    /// Create a new atomic allocation pool
    pub fn new() -> Self {
        Self {
            free_list: AtomicOption::new(),
            size: AtomicUsize::new(0),
            allocations: AtomicU64::new(0),
        }
    }

    /// Allocate from pool (lock-free)
    pub fn alloc(&self) -> Option<NonNull<u8>> {
        loop {
            let current_ptr = self.free_list.inner.load(Ordering::Acquire);
            if current_ptr == 0 {
                return None;
            }
            let current = NonNull::new(current_ptr as *mut u8);

            match self.free_list.compare_exchange(current, None) {
                Ok(_) => {
                    self.size.fetch_sub(1, Ordering::Relaxed);
                    self.allocations.fetch_add(1, Ordering::Relaxed);
                    return current;
                }
                Err(_) => continue,
            }
        }
    }

    /// Free to pool (lock-free)
    pub fn free(&self, ptr: NonNull<u8>) {
        loop {
            let current_ptr = self.free_list.inner.load(Ordering::Acquire);
            let current = if current_ptr != 0 {
                Some(NonNull::new(current_ptr as *mut u8).unwrap())
            } else {
                None
            };

            match self.free_list.compare_exchange(current, Some(ptr)) {
                Ok(_) => {
                    self.size.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                Err(_) => continue,
            }
        }
    }

    /// Get pool size
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Get allocations
    pub fn allocations(&self) -> u64 {
        self.allocations.load(Ordering::Relaxed)
    }
}

/// Quicklist for small object allocation
pub struct Quicklist {
    /// Object size
    object_size: usize,
    /// Free objects
    free_list: Vec<NonNull<u8>>,
    /// Maximum list size
    max_size: usize,
    /// Allocations
    allocations: AtomicU64,
    /// Hits
    hits: AtomicU64,
}

// SAFETY: Quicklist only contains raw pointers and atomic values,
// and is used internally in a thread-safe manner through Mutex
unsafe impl Send for Quicklist {}

// SAFETY: Quicklist only contains raw pointers and atomic values,
// and is used internally in a thread-safe manner through Mutex
unsafe impl Sync for Quicklist {}

impl Quicklist {
    /// Create a new quicklist
    pub fn new(object_size: usize, max_size: usize) -> Self {
        Self {
            object_size,
            free_list: Vec::new(),
            max_size,
            allocations: AtomicU64::new(0),
            hits: AtomicU64::new(0),
        }
    }

    /// Allocate from quicklist
    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        self.allocations.fetch_add(1, Ordering::Relaxed);

        if let Some(ptr) = self.free_list.pop() {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(ptr)
        } else {
            None
        }
    }

    /// Free to quicklist
    pub fn free(&mut self, ptr: NonNull<u8>) {
        if self.free_list.len() < self.max_size {
            self.free_list.push(ptr);
        }
    }

    /// Get statistics
    pub fn stats(&self) -> QuicklistStats {
        QuicklistStats {
            object_size: self.object_size,
            free_objects: self.free_list.len(),
            max_size: self.max_size,
            allocations: self.allocations.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
        }
    }
}

/// Quicklist statistics
#[derive(Debug, Clone)]
pub struct QuicklistStats {
    pub object_size: usize,
    pub free_objects: usize,
    pub max_size: usize,
    pub allocations: u64,
    pub hits: u64,
}

/// Memory cgroup tracker
pub struct MemoryCgroup {
    /// Cgroup ID
    id: usize,
    /// Memory limit
    limit: usize,
    /// Current usage
    usage: AtomicUsize,
    /// Peak usage
    peak_usage: AtomicUsize,
    /// Fail count
    fail_count: AtomicU64,
}

impl MemoryCgroup {
    /// Create a new memory cgroup
    pub fn new(id: usize, limit: usize) -> Self {
        Self {
            id,
            limit,
            usage: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
            fail_count: AtomicU64::new(0),
        }
    }

    /// Try to charge memory
    pub fn charge(&self, size: usize) -> Result<(), KmemError> {
        let mut current = self.usage.load(Ordering::Relaxed);

        loop {
            let new = current.saturating_add(size);

            if new > self.limit {
                self.fail_count.fetch_add(1, Ordering::Relaxed);
                return Err(KmemError::CgroupLimitExceeded);
            }

            match self.usage.compare_exchange_weak(
                current,
                new,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // Update peak
                    let mut peak = self.peak_usage.load(Ordering::Relaxed);
                    loop {
                        if new <= peak {
                            break;
                        }
                        match self.peak_usage.compare_exchange_weak(
                            peak,
                            new,
                            Ordering::AcqRel,
                            Ordering::Relaxed,
                        ) {
                            Ok(_) => break,
                            Err(p) => peak = p,
                        }
                    }
                    return Ok(());
                }
                Err(c) => current = c,
            }
        }
    }

    /// Uncharge memory
    pub fn uncharge(&self, size: usize) {
        let current = self.usage.fetch_sub(size, Ordering::Relaxed);
        // Prevent underflow
        if current < size {
            self.usage.fetch_add(size - current, Ordering::Relaxed);
        }
    }

    /// Get usage
    pub fn usage(&self) -> usize {
        self.usage.load(Ordering::Relaxed)
    }

    /// Get peak usage
    pub fn peak_usage(&self) -> usize {
        self.peak_usage.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> CgroupStats {
        CgroupStats {
            id: self.id,
            limit: self.limit,
            usage: self.usage(),
            peak_usage: self.peak_usage(),
            fail_count: self.fail_count.load(Ordering::Relaxed),
        }
    }
}

/// Cgroup statistics
#[derive(Debug, Clone)]
pub struct CgroupStats {
    pub id: usize,
    pub limit: usize,
    pub usage: usize,
    pub peak_usage: usize,
    pub fail_count: u64,
}

/// Kernel memory cache
pub struct KmemCache {
    /// Cache name
    name: String,
    /// Object size
    object_size: usize,
    /// Objects per slab
    objects_per_slab: usize,
    /// Slabs
    slabs: Vec<Box<Slab>>,
    /// Free objects
    free_objects: AtomicUsize,
    /// Total allocations
    total_allocations: AtomicU64,
}

impl KmemCache {
    /// Create a new kmem cache
    pub fn new(name: &str, object_size: usize) -> Result<Self, KmemError> {
        let objects_per_slab = PAGE_SIZE / object_size;

        Ok(Self {
            name: name.to_string(),
            object_size,
            objects_per_slab,
            slabs: Vec::new(),
            free_objects: AtomicUsize::new(0),
            total_allocations: AtomicU64::new(0),
        })
    }

    /// Allocate object from cache
    pub fn alloc(&mut self) -> Result<NonNull<u8>, KmemError> {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);

        // Try to find free object in existing slabs
        for slab in &mut self.slabs {
            if let Some(ptr) = slab.alloc() {
                self.free_objects.fetch_sub(1, Ordering::Relaxed);
                return Ok(ptr);
            }
        }

        // Need to allocate new slab
        let mut slab = Box::new(Slab::new(self.object_size, self.objects_per_slab)?);
        let ptr = slab.alloc().ok_or(KmemError::AllocationFailed)?;
        self.slabs.push(slab);

        Ok(ptr)
    }

    /// Free object to cache
    pub fn free(&mut self, ptr: NonNull<u8>) {
        for slab in &mut self.slabs {
            if slab.contains(ptr) {
                slab.free(ptr);
                self.free_objects.fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
    }

    /// Shrink cache (reclaim empty slabs)
    pub fn shrink(&mut self) -> usize {
        let before = self.slabs.len();
        self.slabs.retain(|slab| !slab.is_empty());
        let after = self.slabs.len();
        before - after
    }

    /// Reap cache (reclaim some free objects)
    pub fn reap(&mut self) -> usize {
        let before = self.free_objects.load(Ordering::Relaxed);
        self.shrink();
        let after = self.free_objects.load(Ordering::Relaxed);
        before.saturating_sub(after)
    }

    /// Get statistics
    pub fn stats(&self) -> KmemCacheStats {
        KmemCacheStats {
            name: self.name.clone(),
            object_size: self.object_size,
            num_slabs: self.slabs.len(),
            objects_per_slab: self.objects_per_slab,
            free_objects: self.free_objects.load(Ordering::Relaxed),
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
        }
    }
}

/// Kmem cache statistics
#[derive(Debug, Clone)]
pub struct KmemCacheStats {
    pub name: String,
    pub object_size: usize,
    pub num_slabs: usize,
    pub objects_per_slab: usize,
    pub free_objects: usize,
    pub total_allocations: u64,
}

/// Slab for kmem cache
pub struct Slab {
    /// Base pointer
    base: NonNull<u8>,
    /// Free bitmap
    free_bitmap: Vec<u64>,
    /// Object size
    object_size: usize,
    /// Objects per slab
    objects_per_slab: usize,
}

impl Slab {
    /// Create a new slab
    pub fn new(object_size: usize, objects_per_slab: usize) -> Result<Self, KmemError> {
        let layout = Layout::from_size_align(PAGE_SIZE, PAGE_SIZE)
            .map_err(|_| KmemError::InvalidSize)?;

        let ptr = unsafe { alloc(layout) };

        if ptr.is_null() {
            return Err(KmemError::OutOfMemory);
        }

        let base = NonNull::new(ptr).ok_or(KmemError::AllocationFailed)?;

        // Initialize free bitmap
        let bitmap_size = (objects_per_slab + 63) / 64;
        let mut free_bitmap = Vec::with_capacity(bitmap_size);
        for _ in 0..bitmap_size {
            free_bitmap.push(u64::MAX);
        }

        Ok(Self {
            base,
            free_bitmap,
            object_size,
            objects_per_slab,
        })
    }

    /// Allocate object from slab
    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        for (idx, bitmap) in self.free_bitmap.iter_mut().enumerate() {
            if *bitmap != 0 {
                let bit = bitmap.trailing_zeros() as usize;
                *bitmap &= !(1 << bit);
                let obj_idx = idx * 64 + bit;

                if obj_idx < self.objects_per_slab {
                    let offset = obj_idx * self.object_size;
                    let ptr = unsafe { self.base.as_ptr().add(offset) };
                    return Some(NonNull::new(ptr).unwrap());
                }
            }
        }
        None
    }

    /// Free object to slab
    pub fn free(&mut self, ptr: NonNull<u8>) {
        let offset = unsafe { ptr.as_ptr().offset_from(self.base.as_ptr()) } as usize;
        let obj_idx = offset / self.object_size;

        let idx = obj_idx / 64;
        let bit = obj_idx % 64;

        if let Some(bitmap) = self.free_bitmap.get_mut(idx) {
            *bitmap |= 1 << bit;
        }
    }

    /// Check if slab contains pointer
    pub fn contains(&self, ptr: NonNull<u8>) -> bool {
        let start = self.base.as_ptr() as usize;
        let end = start + PAGE_SIZE;
        let addr = ptr.as_ptr() as usize;
        addr >= start && addr < end
    }

    /// Check if slab is empty
    pub fn is_empty(&self) -> bool {
        self.free_bitmap.iter().all(|&b| b == u64::MAX)
    }
}

/// Central kernel memory allocator
pub struct CentralKmemAllocator {
    /// Quicklists for small sizes
    quicklists: Vec<Quicklist>,
    /// Large allocations
    large_allocs: Vec<LargeAllocation>,
    /// Total allocations
    total_allocations: AtomicU64,
    /// Total frees
    total_frees: AtomicU64,
}

/// Large allocation tracking
#[derive(Debug, Clone)]
struct LargeAllocation {
    ptr: NonNull<u8>,
    size: usize,
}

// SAFETY: LargeAllocation is owned exclusively by the allocator and
// the allocator ensures thread-safe access through its Mutex
unsafe impl Send for LargeAllocation {}
unsafe impl Sync for LargeAllocation {}

impl CentralKmemAllocator {
    /// Create a new central allocator
    pub fn new() -> Self {
        let mut quicklists = Vec::new();

        // Quicklists for sizes: 32, 64, 128, 256, 512, 1024 bytes
        for size in [32, 64, 128, 256, 512, 1024] {
            quicklists.push(Quicklist::new(size, 256));
        }

        Self {
            quicklists,
            large_allocs: Vec::new(),
            total_allocations: AtomicU64::new(0),
            total_frees: AtomicU64::new(0),
        }
    }

    /// Allocate from central allocator
    pub fn allocate(&mut self, size: usize) -> Option<NonNull<u8>> {
        self.total_allocations.fetch_add(1, Ordering::Relaxed);

        // Try quicklists for small sizes
        for ql in &mut self.quicklists {
            if size <= ql.object_size {
                if let Some(ptr) = ql.alloc() {
                    return Some(ptr);
                }
                break;
            }
        }

        // Large allocation
        let layout = Layout::from_size_align(size, 8).ok()?;
        let ptr = unsafe { alloc(layout) };

        if ptr.is_null() {
            return None;
        }

        let ptr = NonNull::new(ptr)?;
        self.large_allocs.push(LargeAllocation { ptr, size });
        Some(ptr)
    }

    /// Deallocate to central allocator
    pub fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) {
        self.total_frees.fetch_add(1, Ordering::Relaxed);

        // Try quicklists for small sizes
        for ql in &mut self.quicklists {
            if size <= ql.object_size {
                ql.free(ptr);
                return;
            }
        }

        // Large free
        self.large_allocs.retain(|a| a.ptr != ptr);
        unsafe { dealloc(ptr.as_ptr(), Layout::from_size_align_unchecked(size, 8)) }
    }

    /// Get statistics
    pub fn stats(&self) -> CentralKmemStats {
        CentralKmemStats {
            total_allocations: self.total_allocations.load(Ordering::Relaxed),
            total_frees: self.total_frees.load(Ordering::Relaxed),
            large_allocs: self.large_allocs.len(),
        }
    }
}

/// Central kmem statistics
#[derive(Debug, Clone)]
pub struct CentralKmemStats {
    pub total_allocations: u64,
    pub total_frees: u64,
    pub large_allocs: usize,
}

/// Global kernel memory allocator
pub struct KmemAllocator {
    /// Per-CPU caches
    percpu_caches: Vec<Mutex<PerCpuKmemCache>>,
    /// Atomic pools
    atomic_pools: Vec<Mutex<AtomicAllocationPool>>,
    /// Central allocator
    central: Mutex<CentralKmemAllocator>,
    /// Memory cgroups
    cgroups: Vec<MemoryCgroup>,
    /// Default cgroup ID
    default_cgroup: AtomicUsize,
}

impl KmemAllocator {
    /// Create a new kmem allocator
    pub fn new(num_cpus: usize) -> Self {
        let mut percpu_caches = Vec::new();

        for cpu in 0..num_cpus {
            percpu_caches.push(Mutex::new(PerCpuKmemCache::new(cpu, 64)));
        }

        let mut atomic_pools = Vec::new();
        for _ in 0..num_cpus {
            atomic_pools.push(Mutex::new(AtomicAllocationPool::new()));
        }

        Self {
            percpu_caches,
            atomic_pools,
            central: Mutex::new(CentralKmemAllocator::new()),
            cgroups: Vec::new(),
            default_cgroup: AtomicUsize::new(0),
        }
    }

    /// Allocate kernel memory
    pub fn kmalloc(&self, size: usize) -> Result<*mut u8, KmemError> {
        let cpu_id = 0; // GH-#1278: Get actual CPU ID
        // See: https://github.com/npos/kernel/issues/1278

        // Try per-CPU cache
        if let Some(cache) = self.percpu_caches.get(cpu_id) {
            if let Some(ptr) = cache.lock().alloc() {
                return Ok(ptr.as_ptr());
            }
        }

        // Fall back to central allocator
        let mut central = self.central.lock();
        let ptr = central.allocate(size).ok_or(KmemError::OutOfMemory)?;
        Ok(ptr.as_ptr())
    }

    /// Free kernel memory
    pub fn kfree(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }

        let cpu_id = 0; // GH-#1279: Get actual CPU ID
        // See: https://github.com/npos/kernel/issues/1279

        // Try per-CPU cache
        if let Some(cache) = self.percpu_caches.get(cpu_id) {
            cache.lock().free(NonNull::new(ptr).unwrap());
            return;
        }

        // Fall back to central allocator
        let mut central = self.central.lock();
        central.deallocate(NonNull::new(ptr).unwrap(), 0);
    }

    /// Create kmem cache
    pub fn kmem_cache_create(&self, name: &str, size: usize) -> Result<KmemCache, KmemError> {
        KmemCache::new(name, size)
    }

    /// Get statistics
    pub fn stats(&self) -> KmemStats {
        KmemStats {
            percpu_caches: self.percpu_caches.len(),
            atomic_pools: self.atomic_pools.len(),
            cgroups: self.cgroups.len(),
        }
    }
}

/// Kernel memory statistics
#[derive(Debug, Clone)]
pub struct KmemStats {
    pub percpu_caches: usize,
    pub atomic_pools: usize,
    pub cgroups: usize,
}

/// Global kmem allocator instance
static GLOBAL_KMEM_ALLOCATOR: Mutex<Option<KmemAllocator>> = Mutex::new(None);

/// Initialize kernel memory allocator
pub fn init_kmem_allocator() {
    log::info!("Initializing kernel memory allocator...");

    let num_cpus = 1; // GH-#1280: Get actual CPU count
    // See: https://github.com/npos/kernel/issues/1280
    let allocator = KmemAllocator::new(num_cpus);

    *GLOBAL_KMEM_ALLOCATOR.lock() = Some(allocator);

    log::info!("Kernel memory allocator initialized");
}

/// Get global kmem allocator
pub fn get_kmem_allocator() -> Option<&'static Mutex<Option<KmemAllocator>>> {
    Some(&GLOBAL_KMEM_ALLOCATOR)
}

/// Public API: kmalloc
pub fn kmalloc(size: usize) -> Result<*mut u8, KmemError> {
    if let Some(allocator) = GLOBAL_KMEM_ALLOCATOR.lock().as_ref() {
        allocator.kmalloc(size)
    } else {
        Err(KmemError::OutOfMemory)
    }
}

/// Public API: kfree
pub fn kfree(ptr: *mut u8) {
    if let Some(allocator) = GLOBAL_KMEM_ALLOCATOR.lock().as_ref() {
        allocator.kfree(ptr);
    }
}

/// Public API: kmem_cache_create
pub fn kmem_cache_create(name: &str, size: usize) -> Result<KmemCache, KmemError> {
    if let Some(allocator) = GLOBAL_KMEM_ALLOCATOR.lock().as_ref() {
        allocator.kmem_cache_create(name, size)
    } else {
        Err(KmemError::SlabCacheError)
    }
}
