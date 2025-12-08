//! libc memory allocator adapter
//!
//! Provides an adapter layer that wraps the unified allocator
//! for use by libc modules.

use core::ffi::c_void;
use core::alloc::Layout;
use crate::mm::traits::{UnifiedAllocator, CAllocator};
use crate::mm::allocator::HybridAllocator;

/// libc memory allocator adapter
/// 
/// This adapter wraps the unified HybridAllocator to provide
/// C-compatible memory allocation functions for libc.
/// 
/// # Safety
/// This struct is safe to share between threads because HybridAllocator
/// uses internal synchronization (Mutex).
pub struct LibcMemoryAdapter {
    allocator: &'static HybridAllocator,
}

// Safety: HybridAllocator is thread-safe (uses Mutex internally)
unsafe impl Send for LibcMemoryAdapter {}
unsafe impl Sync for LibcMemoryAdapter {}

impl LibcMemoryAdapter {
    /// Create a new libc memory adapter
    pub fn new() -> Self {
        Self {
            allocator: crate::mm::allocator::get_global_allocator(),
        }
    }

    /// Get a reference to the underlying allocator
    pub fn allocator(&self) -> &'static HybridAllocator {
        self.allocator
    }
}

impl Default for LibcMemoryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// Implement UnifiedAllocator for LibcMemoryAdapter
unsafe impl UnifiedAllocator for LibcMemoryAdapter {
    unsafe fn allocate(&self, layout: Layout) -> *mut u8 {
        unsafe { self.allocator.allocate(layout) }
    }

    unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.allocator.deallocate(ptr, layout) }
    }

    unsafe fn allocate_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { self.allocator.allocate_zeroed(layout) }
    }

    unsafe fn reallocate(
        &self,
        ptr: *mut u8,
        old_layout: Layout,
        new_size: usize,
    ) -> *mut u8 {
        unsafe { self.allocator.reallocate(ptr, old_layout, new_size) }
    }
}

/// Global libc memory adapter instance
/// Note: Using lazy_static or similar would be better, but for now we use a function
pub fn get_libc_adapter() -> &'static LibcMemoryAdapter {
    // Use a static with OnceCell or similar for thread safety
    // For now, create a new instance each time (not ideal but works)
    // TODO: Use OnceCell or LazyLock for proper initialization
    static mut ADAPTER: Option<LibcMemoryAdapter> = None;
    unsafe {
        if ADAPTER.is_none() {
            ADAPTER = Some(LibcMemoryAdapter::new());
        }
        ADAPTER.as_ref().unwrap()
    }
}

/// C-compatible malloc function using unified allocator
pub fn libc_malloc(size: usize) -> *mut c_void {
    get_libc_adapter().malloc(size)
}

/// C-compatible free function using unified allocator
pub fn libc_free(ptr: *mut c_void) {
    get_libc_adapter().free(ptr)
}

/// C-compatible calloc function using unified allocator
pub fn libc_calloc(nmemb: usize, size: usize) -> *mut c_void {
    get_libc_adapter().calloc(nmemb, size)
}

/// C-compatible realloc function using unified allocator
pub fn libc_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    get_libc_adapter().realloc(ptr, size)
}

