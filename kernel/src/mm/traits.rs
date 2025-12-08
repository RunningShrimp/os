//! Unified memory allocator traits
//!
//! Provides a common interface for all memory allocators in the system,
//! allowing different modules (libc, GLib, kernel) to use a unified allocator.

use core::alloc::Layout;
use core::ffi::c_void;

/// Unified memory allocator trait
/// 
/// This trait provides a common interface for all memory allocators,
/// allowing different parts of the system to use the same underlying allocator
/// while maintaining their specific interfaces.
/// 
/// # Safety
/// Implementations must ensure that:
/// - `allocate` returns a valid pointer or null
/// - `deallocate` is called with a pointer previously returned by `allocate`
/// - All operations are thread-safe
pub unsafe trait UnifiedAllocator {
    /// Allocate memory with the given layout
    /// 
    /// # Arguments
    /// * `layout` - Memory layout specifying size and alignment
    /// 
    /// # Returns
    /// * Pointer to allocated memory on success, null on failure
    unsafe fn allocate(&self, layout: Layout) -> *mut u8;

    /// Deallocate memory previously allocated with `allocate`
    /// 
    /// # Arguments
    /// * `ptr` - Pointer to memory to deallocate
    /// * `layout` - Original layout used for allocation
    unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout);

    /// Allocate zero-initialized memory
    /// 
    /// # Arguments
    /// * `layout` - Memory layout specifying size and alignment
    /// 
    /// # Returns
    /// * Pointer to zero-initialized memory on success, null on failure
    unsafe fn allocate_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.allocate(layout);
        if !ptr.is_null() {
            core::ptr::write_bytes(ptr, 0, layout.size());
        }
        ptr
    }

    /// Reallocate memory to a new size
    /// 
    /// # Arguments
    /// * `ptr` - Pointer to previously allocated memory (can be null)
    /// * `old_layout` - Original layout
    /// * `new_size` - New size in bytes
    /// 
    /// # Returns
    /// * Pointer to reallocated memory on success, null on failure
    unsafe fn reallocate(
        &self,
        ptr: *mut u8,
        old_layout: Layout,
        new_size: usize,
    ) -> *mut u8 {
        if ptr.is_null() {
            let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
                Ok(l) => l,
                Err(_) => return core::ptr::null_mut(),
            };
            return self.allocate(new_layout);
        }

        let new_layout = match Layout::from_size_align(new_size, old_layout.align()) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };

        let new_ptr = self.allocate(new_layout);
        if new_ptr.is_null() {
            return core::ptr::null_mut();
        }

        let copy_size = old_layout.size().min(new_size);
        core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        self.deallocate(ptr, old_layout);

        new_ptr
    }
}

/// Extension trait for C-compatible allocation functions
pub trait CAllocator: UnifiedAllocator {
    /// C-compatible malloc
    /// 
    /// # Arguments
    /// * `size` - Size in bytes to allocate
    /// 
    /// # Returns
    /// * Pointer to allocated memory, or null on failure
    fn malloc(&self, size: usize) -> *mut c_void {
        if size == 0 {
            return core::ptr::null_mut();
        }

        let layout = match Layout::from_size_align(size, 1) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };
        
        unsafe {
            self.allocate(layout) as *mut c_void
        }
    }

    /// C-compatible free
    /// 
    /// # Arguments
    /// * `ptr` - Pointer to memory to free (can be null)
    fn free(&self, ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }

        // Note: In a real implementation, we would need to track the layout
        // For now, this is a simplified version that assumes we can infer it
        // TODO: Implement proper layout tracking for C allocations
        unsafe {
            // This is a placeholder - real implementation needs layout tracking
            let layout = Layout::from_size_align(1, 1).unwrap();
            self.deallocate(ptr as *mut u8, layout);
        }
    }

    /// C-compatible calloc
    /// 
    /// # Arguments
    /// * `nmemb` - Number of elements
    /// * `size` - Size of each element
    /// 
    /// # Returns
    /// * Pointer to zero-initialized memory, or null on failure
    fn calloc(&self, nmemb: usize, size: usize) -> *mut c_void {
        let total_size = match nmemb.checked_mul(size) {
            Some(s) => s,
            None => return core::ptr::null_mut(),
        };
        if total_size == 0 {
            return core::ptr::null_mut();
        }

        let layout = match Layout::from_size_align(total_size, 1) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };
        
        unsafe {
            self.allocate_zeroed(layout) as *mut c_void
        }
    }

    /// C-compatible realloc
    /// 
    /// # Arguments
    /// * `ptr` - Pointer to previously allocated memory (can be null)
    /// * `size` - New size in bytes
    /// 
    /// # Returns
    /// * Pointer to reallocated memory, or null on failure
    fn realloc(&self, ptr: *mut c_void, size: usize) -> *mut c_void {
        if ptr.is_null() {
            return self.malloc(size);
        }

        if size == 0 {
            self.free(ptr);
            return core::ptr::null_mut();
        }

        // Note: This is simplified - real implementation needs layout tracking
        let old_layout = Layout::from_size_align(1, 1).unwrap();
        let _new_layout = match Layout::from_size_align(size, 1) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };

        unsafe {
            self.reallocate(ptr as *mut u8, old_layout, size) as *mut c_void
        }
    }
}

/// Default implementation: CAllocator for all UnifiedAllocator types
impl<T: UnifiedAllocator> CAllocator for T {}

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

/// Trait for allocators that provide statistics
pub trait AllocatorWithStats: UnifiedAllocator {
    /// Get current allocator statistics
    fn stats(&self) -> AllocatorStats;
}

