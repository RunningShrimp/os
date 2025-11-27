//! Kernel heap allocator for xv6-rust
//! Provides a simple bump allocator for early boot, later backed by page allocator

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

use crate::sync::Mutex;

/// Simple bump allocator for kernel heap
struct BumpAllocator {
    heap_start: usize,
    heap_end: usize,
    next: usize,
    allocations: usize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initialize the allocator with a memory region
    unsafe fn init(&mut self, heap_start: usize, heap_end: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_end;
        self.next = heap_start;
        self.allocations = 0;
    }

    /// Allocate memory with given layout
    fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let alloc_start = align_up(self.next, layout.align());
        let alloc_end = match alloc_start.checked_add(layout.size()) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > self.heap_end {
            return null_mut(); // Out of memory
        }

        self.next = alloc_end;
        self.allocations += 1;
        alloc_start as *mut u8
    }

    /// Deallocate memory (bump allocator doesn't actually free)
    fn dealloc(&mut self, _ptr: *mut u8, _layout: Layout) {
        self.allocations = self.allocations.saturating_sub(1);
        
        // Bump allocator doesn't actually free memory
        // In a real implementation, we'd track free blocks
        // For now, we just count allocations
        
        // Reset heap when all allocations are freed
        if self.allocations == 0 {
            self.next = self.heap_start;
        }
    }

    /// Get heap usage statistics
    fn stats(&self) -> (usize, usize, usize) {
        let used = self.next - self.heap_start;
        let total = self.heap_end - self.heap_start;
        (used, total, self.allocations)
    }
}

/// Align up to the given alignment
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

// ============================================================================
// Global Allocator
// ============================================================================

struct LockedBumpAllocator {
    inner: Mutex<BumpAllocator>,
}

impl LockedBumpAllocator {
    const fn new() -> Self {
        Self {
            inner: Mutex::new(BumpAllocator::new()),
        }
    }
}

unsafe impl GlobalAlloc for LockedBumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.lock().dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.alloc(layout);
        if !ptr.is_null() {
            unsafe {
                core::ptr::write_bytes(ptr, 0, layout.size());
            }
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = match Layout::from_size_align(new_size, layout.align()) {
            Ok(layout) => layout,
            Err(_) => return null_mut(),
        };

        let new_ptr = unsafe {
            self.alloc(new_layout)
        };
        if !new_ptr.is_null() {
            let copy_size = layout.size().min(new_size);
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
                self.dealloc(ptr, layout);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: LockedBumpAllocator = LockedBumpAllocator::new();

/// Initialize the kernel heap allocator
/// # Safety
/// Must be called exactly once with valid heap bounds
pub unsafe fn init(heap_start: usize, heap_end: usize) {
    ALLOCATOR.inner.lock().init(heap_start, heap_end);
}

/// Get heap statistics: (used, total, allocation_count)
pub fn heap_stats() -> (usize, usize, usize) {
    ALLOCATOR.inner.lock().stats()
}