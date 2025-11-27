//! Slab Allocator for efficient small object allocation
//! 
//! Implements a SLAB allocator similar to Linux kernel's SLUB allocator:
//! - Per-CPU caches for fast allocation/deallocation without locking
//! - Multiple size classes for different object sizes
//! - Memory reclamation through slab coalescing
//! - Cache coloring to reduce cache conflicts

extern crate alloc;

use core::alloc::Layout;
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::mm::{kalloc, kfree, PAGE_SIZE};
use crate::sync::Mutex;
use crate::cpu::cpuid as cpu_id;

// ============================================================================
// Configuration
// ============================================================================

/// Maximum number of CPUs supported
const MAX_CPUS: usize = 8;

/// Maximum object size handled by slab (larger objects go to page allocator)
const SLAB_MAX_SIZE: usize = 2048;

/// Number of objects to cache per CPU
const CPU_CACHE_SIZE: usize = 32;

/// Minimum slab size class
const MIN_SIZE_CLASS: usize = 8;

/// Number of size classes (8, 16, 32, 64, 128, 256, 512, 1024, 2048)
const NUM_SIZE_CLASSES: usize = 9;

/// Size classes array
const SIZE_CLASSES: [usize; NUM_SIZE_CLASSES] = [8, 16, 32, 64, 128, 256, 512, 1024, 2048];

// ============================================================================
// Free Object Header
// ============================================================================

/// Header for free objects in slab
/// When an object is free, its first bytes contain a pointer to the next free object
#[repr(C)]
struct FreeObject {
    next: *mut FreeObject,
}

// ============================================================================
// Slab Page Header  
// ============================================================================

/// Metadata for each slab page
#[repr(C)]
struct SlabPage {
    /// Pointer to the slab cache this page belongs to
    cache: *mut SlabCache,
    /// Free list head
    freelist: AtomicPtr<FreeObject>,
    /// Number of objects in use
    inuse: AtomicUsize,
    /// Total objects in this page
    total: usize,
    /// Next slab page in the partial/full list
    next: *mut SlabPage,
    /// Previous slab page
    prev: *mut SlabPage,
}

impl SlabPage {
    /// Initialize a new slab page
    unsafe fn init(
        page: *mut u8,
        cache: *mut SlabCache,
        obj_size: usize,
    ) -> *mut SlabPage {
        let header = page as *mut SlabPage;
        let header_size = mem::size_of::<SlabPage>();
        let aligned_header = (header_size + obj_size - 1) & !(obj_size - 1);
        
        // Calculate number of objects that fit in the page
        let available = PAGE_SIZE - aligned_header;
        let total = available / obj_size;
        
        (*header).cache = cache;
        (*header).freelist = AtomicPtr::new(ptr::null_mut());
        (*header).inuse = AtomicUsize::new(0);
        (*header).total = total;
        (*header).next = ptr::null_mut();
        (*header).prev = ptr::null_mut();
        
        // Initialize free list
        let base = page.add(aligned_header);
        let mut prev: *mut FreeObject = ptr::null_mut();
        
        for i in (0..total).rev() {
            let obj = base.add(i * obj_size) as *mut FreeObject;
            (*obj).next = prev;
            prev = obj;
        }
        
        (*header).freelist.store(prev, Ordering::Release);
        
        header
    }
    
    /// Allocate an object from this slab
    unsafe fn alloc(&mut self) -> *mut u8 {
        loop {
            let head = self.freelist.load(Ordering::Acquire);
            if head.is_null() {
                return ptr::null_mut();
            }
            
            let next = (*head).next;
            if self.freelist.compare_exchange_weak(
                head,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ).is_ok() {
                self.inuse.fetch_add(1, Ordering::Relaxed);
                return head as *mut u8;
            }
        }
    }
    
    /// Free an object back to this slab
    unsafe fn free(&mut self, ptr: *mut u8) {
        let obj = ptr as *mut FreeObject;
        
        loop {
            let head = self.freelist.load(Ordering::Acquire);
            (*obj).next = head;
            
            if self.freelist.compare_exchange_weak(
                head,
                obj,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ).is_ok() {
                self.inuse.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        }
    }
    
    /// Check if slab is empty (all objects free)
    fn is_empty(&self) -> bool {
        self.inuse.load(Ordering::Relaxed) == 0
    }
    
    /// Check if slab is full (no free objects)
    fn is_full(&self) -> bool {
        self.inuse.load(Ordering::Relaxed) == self.total
    }
}

// ============================================================================
// Per-CPU Cache
// ============================================================================

/// Per-CPU object cache for lock-free fast path
struct CpuCache {
    /// Cached free objects
    objects: [AtomicPtr<u8>; CPU_CACHE_SIZE],
    /// Number of cached objects
    count: AtomicUsize,
}

impl CpuCache {
    const fn new() -> Self {
        const NULL_PTR: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
        Self {
            objects: [NULL_PTR; CPU_CACHE_SIZE],
            count: AtomicUsize::new(0),
        }
    }
    
    /// Try to get an object from the CPU cache
    fn pop(&self) -> *mut u8 {
        loop {
            let count = self.count.load(Ordering::Acquire);
            if count == 0 {
                return ptr::null_mut();
            }
            
            if self.count.compare_exchange_weak(
                count,
                count - 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ).is_ok() {
                return self.objects[count - 1].swap(ptr::null_mut(), Ordering::AcqRel);
            }
        }
    }
    
    /// Try to put an object into the CPU cache
    fn push(&self, ptr: *mut u8) -> bool {
        loop {
            let count = self.count.load(Ordering::Acquire);
            if count >= CPU_CACHE_SIZE {
                return false;
            }
            
            if self.count.compare_exchange_weak(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ).is_ok() {
                self.objects[count].store(ptr, Ordering::Release);
                return true;
            }
        }
    }
}

// ============================================================================
// Slab Cache
// ============================================================================

/// A cache for objects of a specific size
pub struct SlabCache {
    /// Object size
    obj_size: usize,
    /// Alignment
    align: usize,
    /// Name for debugging
    name: &'static str,
    /// Per-CPU caches
    cpu_caches: [CpuCache; MAX_CPUS],
    /// List of partial slabs (have some free objects)
    partial: Mutex<SlabList>,
    /// List of full slabs (no free objects)
    full: Mutex<SlabList>,
    /// Statistics
    stats: CacheStats,
}

/// Slab list management
struct SlabList {
    head: *mut SlabPage,
    tail: *mut SlabPage,
    count: usize,
}

impl SlabList {
    const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
            count: 0,
        }
    }
    
    unsafe fn push_front(&mut self, slab: *mut SlabPage) {
        (*slab).next = self.head;
        (*slab).prev = ptr::null_mut();
        
        if !self.head.is_null() {
            (*self.head).prev = slab;
        } else {
            self.tail = slab;
        }
        
        self.head = slab;
        self.count += 1;
    }
    
    unsafe fn remove(&mut self, slab: *mut SlabPage) {
        let prev = (*slab).prev;
        let next = (*slab).next;
        
        if !prev.is_null() {
            (*prev).next = next;
        } else {
            self.head = next;
        }
        
        if !next.is_null() {
            (*next).prev = prev;
        } else {
            self.tail = prev;
        }
        
        (*slab).prev = ptr::null_mut();
        (*slab).next = ptr::null_mut();
        self.count -= 1;
    }
    
    fn pop_front(&mut self) -> *mut SlabPage {
        if self.head.is_null() {
            return ptr::null_mut();
        }
        
        let slab = self.head;
        unsafe { self.remove(slab); }
        slab
    }
}

/// Cache statistics
struct CacheStats {
    allocs: AtomicUsize,
    frees: AtomicUsize,
    cache_hits: AtomicUsize,
    slab_allocs: AtomicUsize,
    slab_frees: AtomicUsize,
}

impl CacheStats {
    const fn new() -> Self {
        Self {
            allocs: AtomicUsize::new(0),
            frees: AtomicUsize::new(0),
            cache_hits: AtomicUsize::new(0),
            slab_allocs: AtomicUsize::new(0),
            slab_frees: AtomicUsize::new(0),
        }
    }
}

impl SlabCache {
    /// Create a new slab cache
    pub fn new(obj_size: usize, align: usize, name: &'static str) -> Self {
        const EMPTY_CPU_CACHE: CpuCache = CpuCache::new();
        Self {
            obj_size,
            align,
            name,
            cpu_caches: [EMPTY_CPU_CACHE; MAX_CPUS],
            partial: Mutex::new(SlabList::new()),
            full: Mutex::new(SlabList::new()),
            stats: CacheStats::new(),
        }
    }
    
    /// Allocate an object from the cache
    pub fn alloc(&self) -> *mut u8 {
        self.stats.allocs.fetch_add(1, Ordering::Relaxed);
        
        // Fast path: try CPU cache first
        let cpu = cpu_id() % MAX_CPUS;
        let ptr = self.cpu_caches[cpu].pop();
        if !ptr.is_null() {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return ptr;
        }
        
        // Slow path: allocate from slab
        self.alloc_slow()
    }
    
    /// Slow path allocation from slab pages
    fn alloc_slow(&self) -> *mut u8 {
        // Try partial slabs first
        {
            let mut partial = self.partial.lock();
            let mut slab = partial.head;
            
            while !slab.is_null() {
                unsafe {
                    let ptr = (*slab).alloc();
                    if !ptr.is_null() {
                        // If slab is now full, move to full list
                        if (*slab).is_full() {
                            partial.remove(slab);
                            self.full.lock().push_front(slab);
                        }
                        return ptr;
                    }
                    slab = (*slab).next;
                }
            }
        }
        
        // No space in partial slabs, allocate new slab
        self.grow()
    }
    
    /// Allocate a new slab page
    fn grow(&self) -> *mut u8 {
        let page = kalloc();
        if page.is_null() {
            return ptr::null_mut();
        }
        
        self.stats.slab_allocs.fetch_add(1, Ordering::Relaxed);
        
        unsafe {
            let slab = SlabPage::init(
                page,
                self as *const _ as *mut _,
                self.obj_size,
            );
            
            // Allocate one object immediately
            let ptr = (*slab).alloc();
            
            // Add to partial list
            self.partial.lock().push_front(slab);
            
            ptr
        }
    }
    
    /// Free an object back to the cache
    pub fn free(&self, ptr: *mut u8) {
        if ptr.is_null() {
            return;
        }
        
        self.stats.frees.fetch_add(1, Ordering::Relaxed);
        
        // Fast path: try CPU cache first
        let cpu = cpu_id() % MAX_CPUS;
        if self.cpu_caches[cpu].push(ptr) {
            return;
        }
        
        // Slow path: return to slab
        self.free_slow(ptr);
    }
    
    /// Slow path free back to slab
    fn free_slow(&self, ptr: *mut u8) {
        // Find the slab page this object belongs to
        let page_addr = (ptr as usize) & !(PAGE_SIZE - 1);
        let slab = page_addr as *mut SlabPage;
        
        unsafe {
            let was_full = (*slab).is_full();
            (*slab).free(ptr);
            
            // If slab was full, move to partial list
            if was_full {
                self.full.lock().remove(slab);
                self.partial.lock().push_front(slab);
            }
            
            // If slab is now empty, consider freeing it
            if (*slab).is_empty() {
                self.try_free_slab(slab);
            }
        }
    }
    
    /// Try to free an empty slab back to the page allocator
    unsafe fn try_free_slab(&self, slab: *mut SlabPage) {
        let mut partial = self.partial.lock();
        
        // Keep at least one empty slab for hysteresis
        if partial.count > 1 && (*slab).is_empty() {
            partial.remove(slab);
            self.stats.slab_frees.fetch_add(1, Ordering::Relaxed);
            kfree(slab as *mut u8);
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.stats.allocs.load(Ordering::Relaxed),
            self.stats.frees.load(Ordering::Relaxed),
            self.stats.cache_hits.load(Ordering::Relaxed),
            self.stats.slab_allocs.load(Ordering::Relaxed),
            self.stats.slab_frees.load(Ordering::Relaxed),
        )
    }
    
    /// Shrink cache by freeing empty slabs
    pub fn shrink(&self) -> usize {
        let mut freed = 0;
        let mut partial = self.partial.lock();
        
        // Free all empty slabs except one
        let mut slab = partial.head;
        while !slab.is_null() {
            unsafe {
                let next = (*slab).next;
                if (*slab).is_empty() && partial.count > 1 {
                    partial.remove(slab);
                    kfree(slab as *mut u8);
                    freed += 1;
                }
                slab = next;
            }
        }
        
        freed
    }
}

// ============================================================================
// Global Slab Allocator
// ============================================================================

/// Global size-class based slab allocator
pub struct SlabAllocator {
    /// Caches for each size class
    caches: [SlabCache; NUM_SIZE_CLASSES],
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub fn new() -> Self {
        Self {
            caches: [
                SlabCache::new(8, 8, "size-8"),
                SlabCache::new(16, 8, "size-16"),
                SlabCache::new(32, 8, "size-32"),
                SlabCache::new(64, 8, "size-64"),
                SlabCache::new(128, 8, "size-128"),
                SlabCache::new(256, 8, "size-256"),
                SlabCache::new(512, 8, "size-512"),
                SlabCache::new(1024, 8, "size-1024"),
                SlabCache::new(2048, 8, "size-2048"),
            ],
        }
    }
    
    /// Find the appropriate cache for a given size
    fn find_cache(&self, size: usize) -> Option<&SlabCache> {
        for (i, &class_size) in SIZE_CLASSES.iter().enumerate() {
            if size <= class_size {
                return Some(&self.caches[i]);
            }
        }
        None
    }
    
    /// Allocate memory
    pub fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        
        if let Some(cache) = self.find_cache(size) {
            cache.alloc()
        } else {
            // Too large for slab, use page allocator directly
            if size <= PAGE_SIZE {
                kalloc()
            } else {
                // Multi-page allocation not supported yet
                ptr::null_mut()
            }
        }
    }
    
    /// Free memory
    pub fn free(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }
        
        let size = layout.size().max(layout.align());
        
        if let Some(cache) = self.find_cache(size) {
            cache.free(ptr);
        } else if size <= PAGE_SIZE {
            unsafe { kfree(ptr); }
        }
        // Else: multi-page, would need to track size
    }
    
    /// Get statistics for all caches
    pub fn all_stats(&self) -> [(usize, usize, usize, usize, usize); NUM_SIZE_CLASSES] {
        let mut stats = [(0, 0, 0, 0, 0); NUM_SIZE_CLASSES];
        for (i, cache) in self.caches.iter().enumerate() {
            stats[i] = cache.stats();
        }
        stats
    }
    
    /// Shrink all caches
    pub fn shrink_all(&self) -> usize {
        let mut total = 0;
        for cache in &self.caches {
            total += cache.shrink();
        }
        total
    }
}

// ============================================================================
// Global Instance
// ============================================================================

use core::sync::atomic::AtomicBool;

/// Global slab allocator instance (lazy initialized)
static mut SLAB_ALLOCATOR: Option<SlabAllocator> = None;
static SLAB_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the slab allocator
/// Must be called once during boot after heap is available
pub fn init() {
    if SLAB_INITIALIZED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }
    
    unsafe {
        SLAB_ALLOCATOR = Some(SlabAllocator::new());
    }
}

/// Get the global slab allocator
fn get_allocator() -> &'static SlabAllocator {
    unsafe {
        (*core::ptr::addr_of!(SLAB_ALLOCATOR)).as_ref().expect("Slab allocator not initialized")
    }
}

/// Allocate from slab allocator
pub fn slab_alloc(layout: Layout) -> *mut u8 {
    if !SLAB_INITIALIZED.load(Ordering::Relaxed) {
        // Fall back to page allocator if slab not initialized
        return kalloc();
    }
    get_allocator().alloc(layout)
}

/// Free to slab allocator
pub fn slab_free(ptr: *mut u8, layout: Layout) {
    if !SLAB_INITIALIZED.load(Ordering::Relaxed) {
        unsafe { kfree(ptr); }
        return;
    }
    get_allocator().free(ptr, layout)
}

/// Get slab statistics
pub fn slab_stats() -> [(usize, usize, usize, usize, usize); NUM_SIZE_CLASSES] {
    if !SLAB_INITIALIZED.load(Ordering::Relaxed) {
        return [(0, 0, 0, 0, 0); NUM_SIZE_CLASSES];
    }
    get_allocator().all_stats()
}

/// Shrink slab caches (for memory pressure)
pub fn slab_shrink() -> usize {
    if !SLAB_INITIALIZED.load(Ordering::Relaxed) {
        return 0;
    }
    get_allocator().shrink_all()
}

// ============================================================================
// Typed Object Caches
// ============================================================================

/// Create a typed cache for a specific type
pub struct TypedCache<T> {
    cache: SlabCache,
    _marker: core::marker::PhantomData<T>,
}

impl<T> TypedCache<T> {
    /// Create a new typed cache
    pub fn new(name: &'static str) -> Self {
        Self {
            cache: SlabCache::new(
                mem::size_of::<T>(),
                mem::align_of::<T>(),
                name,
            ),
            _marker: core::marker::PhantomData,
        }
    }
    
    /// Allocate an object
    pub fn alloc(&self) -> Option<NonNull<T>> {
        let ptr = self.cache.alloc();
        NonNull::new(ptr as *mut T)
    }
    
    /// Free an object
    pub fn free(&self, ptr: NonNull<T>) {
        self.cache.free(ptr.as_ptr() as *mut u8);
    }
    
    /// Allocate and initialize with a value
    pub fn alloc_init(&self, value: T) -> Option<NonNull<T>> {
        let ptr = self.alloc()?;
        unsafe {
            ptr::write(ptr.as_ptr(), value);
        }
        Some(ptr)
    }
}

// ============================================================================
// Convenience macros
// ============================================================================

/// Define a typed object cache
#[macro_export]
macro_rules! define_cache {
    ($name:ident, $type:ty, $cache_name:literal) => {
        static $name: $crate::slab::TypedCache<$type> = 
            $crate::slab::TypedCache::new($cache_name);
    };
}
