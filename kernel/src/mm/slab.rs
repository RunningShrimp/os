//! Slab allocator for efficient allocation of fixed-size objects
//! Reduces fragmentation for common object sizes

extern crate alloc;

use core::alloc::Layout;
use core::ptr::null_mut;
use alloc::vec::Vec;

/// Object size classes: 8, 16, 32, 64, 128, 256, 512, 1024, 2048
const SLAB_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// A slab object header (placed before each allocated object)
struct SlabObjectHeader {
    slab_idx: u8,  // Index of the parent slab
    size_class: u8, // Size class index
    in_use: bool,
}

/// A slab containing multiple objects of the same size
struct Slab {
    /// Base address of this slab
    base: *mut u8,
    /// Size of this slab in bytes
    capacity: usize,
    /// Object size for this slab
    object_size: usize,
    /// Bitmap for tracking used objects (simple version)
    used_count: usize,
    /// Free list of objects in this slab
    free_list: *mut u8,
}

/// Slab allocator statistics
#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    pub used: usize,
    pub allocated: usize,
}

/// Main slab allocator
pub struct SlabAllocator {
    /// Slabs for each size class (initialized later)
    slabs: Option<Vec<Vec<Slab>>>,
    /// Heap management
    heap_ptr: *mut u8,
    heap_size: usize,
    allocated: usize,
}

impl SlabAllocator {
    /// Create an uninitialized slab allocator (for const contexts)
    pub const fn uninitialized() -> Self {
        Self {
            slabs: None,
            heap_ptr: null_mut(),
            heap_size: 0,
            allocated: 0,
        }
    }

    /// Create a new slab allocator
    pub fn new() -> Self {
        let mut slabs = Vec::new();
        for _ in SLAB_SIZES {
            slabs.push(Vec::new());
        }

        Self {
            slabs: Some(slabs),
            heap_ptr: null_mut(),
            heap_size: 0,
            allocated: 0,
        }
    }

    /// Get mutable reference to slabs, initializing if necessary
    fn get_slabs_mut(&mut self) -> &mut Vec<Vec<Slab>> {
        if self.slabs.is_none() {
            let mut slabs = Vec::new();
            for _ in SLAB_SIZES {
                slabs.push(Vec::new());
            }
            self.slabs = Some(slabs);
        }
        self.slabs.as_mut().unwrap()
    }

    /// Get reference to slabs, initializing if necessary
    fn get_slabs(&self) -> &Vec<Vec<Slab>> {
        // This is unsafe but needed for const initialization patterns
        // In practice, this should only be called after init()
        unsafe { self.slabs.as_ref().unwrap_unchecked() }
    }

    /// Initialize the slab allocator with a memory region
    pub unsafe fn init(&mut self, heap_ptr: *mut u8, heap_size: usize) {
        self.heap_ptr = heap_ptr;
        self.heap_size = heap_size;
        self.allocated = 0;

        // Initialize slabs if not already done
        if self.slabs.is_none() {
            let mut slabs = Vec::new();
            for _ in SLAB_SIZES {
                slabs.push(Vec::new());
            }
            self.slabs = Some(slabs);
        }

        // Collect slab creations first, then add them to avoid borrow conflicts
        let mut new_slabs: Vec<(usize, Slab)> = Vec::new();

        for (size_class_idx, &size) in SLAB_SIZES.iter().enumerate() {
            // Allocate a few slabs per size class
            for _ in 0..4 {
                if let Some(slab) = self.create_slab(size_class_idx, size) {
                    new_slabs.push((size_class_idx, slab));
                }
            }
        }

        // Now add the created slabs
        if let Some(slabs) = &mut self.slabs {
            for (size_class_idx, slab) in new_slabs {
                slabs[size_class_idx].push(slab);
            }
        }
    }

    /// Create a new slab for a given size class
    unsafe fn create_slab(&mut self, _size_class_idx: usize, object_size: usize) -> Option<Slab> {
        // Calculate slab size (4 objects per slab, minimum 4KB)
        let objects_per_slab = 4;
        let slab_capacity = (object_size * objects_per_slab).max(4096);
        
        if self.allocated + slab_capacity > self.heap_size {
            return None;
        }

        let slab_base = self.heap_ptr.add(self.allocated);
        self.allocated += slab_capacity;

        // Initialize free list
        let free_list = slab_base;
        for i in 0..objects_per_slab {
            let obj = slab_base.add(i * object_size);
            let _header = obj as *mut SlabObjectHeader;
            if i < objects_per_slab - 1 {
                let next = slab_base.add((i + 1) * object_size);
                *(obj as *mut *mut u8) = next;
            } else {
                *(obj as *mut *mut u8) = null_mut();
            }
        }

        Some(Slab {
            base: slab_base,
            capacity: slab_capacity,
            object_size,
            used_count: 0,
            free_list,
        })
    }

    /// Static version of create_slab that doesn't borrow self
    unsafe fn create_slab_static(
        heap_ptr: *mut u8,
        allocated: usize,
        heap_size: usize,
        _size_class_idx: usize,
        object_size: usize,
    ) -> Option<(Slab, usize)> {
        // Calculate slab size (4 objects per slab, minimum 4KB)
        let objects_per_slab = 4;
        let slab_capacity = (object_size * objects_per_slab).max(4096);

        if allocated + slab_capacity > heap_size {
            return None;
        }

        let slab_base = heap_ptr.add(allocated);
        let new_allocated = allocated + slab_capacity;

        // Initialize free list
        let free_list = slab_base;
        for i in 0..objects_per_slab {
            let obj = slab_base.add(i * object_size);
            let _header = obj as *mut SlabObjectHeader;
            if i < objects_per_slab - 1 {
                let next = slab_base.add((i + 1) * object_size);
                *(obj as *mut *mut u8) = next;
            } else {
                *(obj as *mut *mut u8) = null_mut();
            }
        }

        Some((Slab {
            base: slab_base,
            capacity: slab_capacity,
            object_size,
            used_count: 0,
            free_list,
        }, new_allocated))
    }

    /// Allocate memory with given layout
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // Find the appropriate size class
        let size_class_idx = self.find_size_class(size);
        if size_class_idx >= SLAB_SIZES.len() {
            return null_mut(); // Allocation too large for slab allocator
        }

        let slab_size = SLAB_SIZES[size_class_idx];
        // Initialize slabs if needed
        if self.slabs.is_none() {
            self.slabs = Some(Vec::new());
            for _ in SLAB_SIZES {
                self.slabs.as_mut().unwrap().push(Vec::new());
            }
        }

        // Try to allocate from an existing slab
        if let Some(ref mut slabs) = self.slabs {
            if size_class_idx < slabs.len() {
                for slab in slabs[size_class_idx].iter_mut() {
                    if !slab.free_list.is_null() {
                        unsafe {
                            let obj = slab.free_list;
                            slab.free_list = *(obj as *mut *mut u8);
                            slab.used_count += 1;

                            // Write header
                            let header = obj as *mut SlabObjectHeader;
                            (*header).slab_idx = (slabs[size_class_idx].len() - 1) as u8;
                            (*header).size_class = size_class_idx as u8;
                            (*header).in_use = true;

                            // Return pointer after header
                            return obj.add(core::mem::size_of::<SlabObjectHeader>());
                        }
                    }
                }
            }
        }

        // Create a new slab if needed - collect required data first
        let heap_ptr = self.heap_ptr;
        let allocated = self.allocated;
        let heap_size = self.heap_size;

        let new_slab = unsafe { Self::create_slab_static(heap_ptr, allocated, heap_size, size_class_idx, slab_size) };
        if let Some((new_slab, new_allocated)) = new_slab {
            // Update allocated counter
            self.allocated = new_allocated;

            // Add to slabs and allocate from the new slab
            if let Some(ref mut slabs) = self.slabs {
                if size_class_idx < slabs.len() {
                    slabs[size_class_idx].push(new_slab);

                    // Then allocate from the new slab
                    let slab_idx = slabs[size_class_idx].len() - 1;
                    let slab = &mut slabs[size_class_idx][slab_idx];
                    unsafe {
                        let obj = slab.free_list;
                        slab.free_list = *(obj as *mut *mut u8);
                        slab.used_count += 1;

                        // Write header
                        let header = obj as *mut SlabObjectHeader;
                        (*header).slab_idx = slab_idx as u8;
                        (*header).size_class = size_class_idx as u8;
                        (*header).in_use = true;

                        // Return pointer after header
                        return obj.add(core::mem::size_of::<SlabObjectHeader>());
                    }
                }
            }
        }

        null_mut()
    }

    /// Deallocate memory
    pub fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        unsafe {
            let obj = ptr.sub(core::mem::size_of::<SlabObjectHeader>());
            let header = obj as *mut SlabObjectHeader;
            
            if !(*header).in_use {
                return; // Already freed
            }

            let size_class_idx = (*header).size_class as usize;
            let slab_idx = (*header).slab_idx as usize;

            // Check bounds first
            let can_access = if let Some(ref slabs) = self.slabs {
                size_class_idx < slabs.len() && slab_idx < slabs[size_class_idx].len()
            } else {
                false
            };

            if can_access {
                // Now access mutably
                if let Some(ref mut slabs) = self.slabs {
                    let slab = &mut slabs[size_class_idx][slab_idx];

                    // Add back to free list
                    *(obj as *mut *mut u8) = slab.free_list;
                    slab.free_list = obj;
                    slab.used_count = slab.used_count.saturating_sub(1);
                    (*header).in_use = false;
                }
            }
        }
    }

    /// Find the appropriate size class for a given size
    fn find_size_class(&self, size: usize) -> usize {
        for (idx, &class_size) in SLAB_SIZES.iter().enumerate() {
            if size <= class_size {
                return idx;
            }
        }
        SLAB_SIZES.len()
    }

    /// Get allocator statistics
    pub fn stats(&self) -> AllocatorStats {
        let mut total_allocated = 0;
        let mut total_used = 0;
        
        if let Some(ref slabs_array) = self.slabs {
            for (size_class_idx, slabs) in slabs_array.iter().enumerate() {
                let size = SLAB_SIZES[size_class_idx];
                for slab in slabs {
                    total_allocated += slab.capacity;
                    total_used += slab.used_count * size;
                }
            }
        }
        
        AllocatorStats { used: total_used, allocated: total_allocated }
    }

    /// Shrink slab allocator by removing completely empty slabs
    pub fn shrink(&mut self) -> usize {
        let mut freed_slabs = 0;

        if let Some(ref mut slabs) = self.slabs {
            for (size_class_idx, size_class_slabs) in slabs.iter_mut().enumerate() {
                // Remove completely empty slabs from this size class
                let initial_count = size_class_slabs.len();
                size_class_slabs.retain(|slab| {
                    // Keep slab if it has any objects in use
                    slab.used_count > 0
                });
                let removed_count = initial_count - size_class_slabs.len();
                freed_slabs += removed_count;

                if removed_count > 0 {
                    crate::println!("[slab] Freed {} empty slabs from size class {} ({} bytes)",
                                   removed_count, size_class_idx, SLAB_SIZES[size_class_idx]);
                }
            }
        }

        freed_slabs
    }
}


// Implement Send/Sync for slab allocator types since they are thread-safe when protected by Mutex
unsafe impl Send for SlabObjectHeader {}
unsafe impl Sync for SlabObjectHeader {}
unsafe impl Send for Slab {}
unsafe impl Sync for Slab {}
unsafe impl Send for SlabAllocator {}
unsafe impl Sync for SlabAllocator {}

use crate::sync::Mutex;

/// Global slab allocator instance
static GLOBAL_SLAB: Mutex<SlabAllocator> = Mutex::new(SlabAllocator::uninitialized());

/// Get global slab allocator statistics
pub fn slab_stats() -> AllocatorStats {
    GLOBAL_SLAB.lock().stats()
}

/// Shrink global slab allocator
pub fn slab_shrink() -> usize {
    GLOBAL_SLAB.lock().shrink()
}

/// Initialize global slab allocator
pub unsafe fn init_slab(heap_ptr: *mut u8, heap_size: usize) {
    GLOBAL_SLAB.lock().init(heap_ptr, heap_size);
}

/// Allocate from global slab allocator
pub fn slab_alloc(layout: Layout) -> *mut u8 {
    GLOBAL_SLAB.lock().alloc(layout)
}

/// Deallocate to global slab allocator
pub unsafe fn slab_dealloc(ptr: *mut u8, layout: Layout) {
    GLOBAL_SLAB.lock().dealloc(ptr, layout);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_alloc() {
        let mut alloc = SlabAllocator::new();
        unsafe { alloc.init(0x1000 as *mut u8, 0x10000); }
        
        let layout = Layout::from_size_align(64, 8).unwrap();
        let ptr = alloc.alloc(layout);
        assert!(!ptr.is_null());
        
        alloc.dealloc(ptr, layout);
    }
}