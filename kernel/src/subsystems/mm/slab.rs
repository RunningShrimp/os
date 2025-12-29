//! Slab Allocator
//!
//! This module implements slab allocator for efficient small object allocation.

use core::{alloc::Layout, ptr};

/// Slab sizes (powers of 2 from 32 to 2048 bytes)
pub const SLAB_SIZES: [usize; 7] = [32, 64, 128, 256, 512, 1024, 2048];

/// Slab object
#[derive(Debug, Clone)]
pub struct SlabObject {
    pub data: *mut u8,
    pub in_use: bool,
}

/// Slab for a specific object size
#[derive(Debug)]
pub struct Slab {
    pub object_size: usize,
    pub objects: [SlabObject; 64], // Each slab manages 64 objects
    pub next_free: *mut SlabObject,
}

impl Slab {
    pub fn new(object_size: usize) -> Self {
        Self {
            object_size,
            objects: unsafe { core::mem::zeroed() },
            next_free: core::ptr::null_mut(),
        }
    }
}

/// Statistics for slab allocator
#[derive(Debug, Clone)]
pub struct AllocatorStats {
    pub used: usize,
    pub allocated: usize,
}

impl AllocatorStats {
    pub const fn new() -> Self {
        Self { used: 0, allocated: 0 }
    }
}

impl Default for AllocatorStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Optimized Slab Allocator
pub struct OptimizedSlabAllocator {
    slabs: [Option<*mut Slab>; 7], // One slab for each SLAB_SIZE
    stats: AllocatorStats,
}

unsafe impl Send for OptimizedSlabAllocator {}

impl OptimizedSlabAllocator {
    pub const fn uninitialized() -> Self {
        Self { slabs: [None; 7], stats: AllocatorStats::new() }
    }

    /// Initialize the slab allocator with a memory region
    pub unsafe fn init(&mut self, start: *mut u8, size: usize) {
        // Divide memory among all slab sizes
        let slab_size = size / SLAB_SIZES.len();
        let mut current_ptr = start;

        for (i, &object_size) in SLAB_SIZES.iter().enumerate() {
            let slab_ptr = current_ptr as *mut Slab;
            (*slab_ptr).object_size = object_size;
            (*slab_ptr).next_free = &mut (*slab_ptr).objects[0];

            // Initialize free list
            for j in 0..64 {
                (*slab_ptr).objects[j].data = if j == 0 {
                    ptr::null_mut()
                } else {
                    let offset = j * object_size + 64 * object_size; // Skip slab header
                    (slab_ptr as usize + offset) as *mut u8
                };
                (*slab_ptr).objects[j].in_use = false;

                if j < 63 {
                    (*slab_ptr).objects[j].data =
                        &mut (*slab_ptr).objects[j + 1] as *mut SlabObject as *mut u8;
                }
            }

            self.slabs[i] = Some(slab_ptr);
            current_ptr = (current_ptr as usize + slab_size) as *mut u8;
        }
    }

    /// Allocate memory from slab
    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();

        // Find appropriate slab size
        for (i, &slab_size) in SLAB_SIZES.iter().enumerate() {
            if size <= slab_size {
                if let Some(slab_ptr) = self.slabs[i] {
                    if !(*slab_ptr).next_free.is_null() {
                        let obj = (*slab_ptr).next_free;
                        (*slab_ptr).next_free = (*obj).data as *mut SlabObject;
                        (*obj).in_use = true;
                        self.stats.used += 1;
                        self.stats.allocated += slab_size;
                        return obj as *mut u8;
                    }
                }
                break;
            }
        }

        ptr::null_mut()
    }

    /// Deallocate memory to slab
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        // Find which slab this pointer belongs to
        for i in 0..SLAB_SIZES.len() {
            if let Some(slab_ptr) = self.slabs[i] {
                let slab_start = slab_ptr as usize;
                let slab_end = slab_start + 64 * SLAB_SIZES[i];

                let ptr_addr = ptr as usize;
                if ptr_addr >= slab_start && ptr_addr < slab_end {
                    // This pointer belongs to this slab
                    let obj = ptr as *mut SlabObject;
                    (*obj).in_use = false;
                    (*obj).data = (*slab_ptr).next_free as *mut u8;
                    (*slab_ptr).next_free = obj;
                    self.stats.used -= 1;
                    return;
                }
            }
        }
    }

    /// Get allocator statistics
    pub fn stats(&self) -> AllocatorStats {
        self.stats.clone()
    }
}

impl Default for OptimizedSlabAllocator {
    fn default() -> Self {
        Self::uninitialized()
    }
}
