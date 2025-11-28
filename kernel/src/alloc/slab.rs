//! Slab allocator for efficient allocation of fixed-size objects
//! Reduces fragmentation for common object sizes

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
    /// Slabs for each size class
    slabs: Vec<Vec<Slab>>,
    /// Heap management
    heap_ptr: *mut u8,
    heap_size: usize,
    allocated: usize,
}

impl SlabAllocator {
    /// Create a new slab allocator
    pub fn new() -> Self {
        let mut slabs = Vec::new();
        for _ in SLAB_SIZES {
            slabs.push(Vec::new());
        }
        
        Self {
            slabs,
            heap_ptr: null_mut(),
            heap_size: 0,
            allocated: 0,
        }
    }

    /// Initialize the slab allocator with a memory region
    pub unsafe fn init(&mut self, heap_ptr: *mut u8, heap_size: usize) {
        self.heap_ptr = heap_ptr;
        self.heap_size = heap_size;
        self.allocated = 0;
        
        // Pre-allocate some slabs for each size class
        for (size_class_idx, &size) in SLAB_SIZES.iter().enumerate() {
            // Allocate a few slabs per size class
            for _ in 0..4 {
                if let Some(slab) = self.create_slab(size_class_idx, size) {
                    self.slabs[size_class_idx].push(slab);
                }
            }
        }
    }

    /// Create a new slab for a given size class
    unsafe fn create_slab(&mut self, size_class_idx: usize, object_size: usize) -> Option<Slab> {
        // Calculate slab size (4 objects per slab, minimum 4KB)
        let objects_per_slab = 4;
        let slab_capacity = (object_size * objects_per_slab).max(4096);
        
        if self.allocated + slab_capacity > self.heap_size {
            return None;
        }

        let slab_base = self.heap_ptr.add(self.allocated);
        self.allocated += slab_capacity;

        // Initialize free list
        let mut free_list = slab_base;
        for i in 0..objects_per_slab {
            let obj = slab_base.add(i * object_size);
            let header = obj as *mut SlabObjectHeader;
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

    /// Allocate memory with given layout
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // Find the appropriate size class
        let size_class_idx = self.find_size_class(size);
        if size_class_idx >= SLAB_SIZES.len() {
            return null_mut(); // Allocation too large for slab allocator
        }

        let slab_size = SLAB_SIZES[size_class_idx];
        let slabs = &mut self.slabs[size_class_idx];

        // Try to allocate from an existing slab
        for slab in slabs.iter_mut() {
            if !slab.free_list.is_null() {
                unsafe {
                    let obj = slab.free_list;
                    slab.free_list = *(obj as *mut *mut u8);
                    slab.used_count += 1;
                    
                    // Write header
                    let header = obj as *mut SlabObjectHeader;
                    (*header).slab_idx = (slabs.len() - 1) as u8;
                    (*header).size_class = size_class_idx as u8;
                    (*header).in_use = true;
                    
                    // Return pointer after header
                    return obj.add(core::mem::size_of::<SlabObjectHeader>());
                }
            }
        }

        // Create a new slab if needed
        if let Some(new_slab) = unsafe { self.create_slab(size_class_idx, slab_size) } {
            slabs.push(new_slab);
            
            let slab = &mut slabs[slabs.len() - 1];
            unsafe {
                let obj = slab.free_list;
                slab.free_list = *(obj as *mut *mut u8);
                slab.used_count += 1;
                
                // Write header
                let header = obj as *mut SlabObjectHeader;
                (*header).slab_idx = (slabs.len() - 1) as u8;
                (*header).size_class = size_class_idx as u8;
                (*header).in_use = true;
                
                // Return pointer after header
                return obj.add(core::mem::size_of::<SlabObjectHeader>());
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
            
            if size_class_idx < self.slabs.len() && slab_idx < self.slabs[size_class_idx].len() {
                let slab = &mut self.slabs[size_class_idx][slab_idx];
                
                // Add back to free list
                *(obj as *mut *mut u8) = slab.free_list;
                slab.free_list = obj;
                slab.used_count = slab.used_count.saturating_sub(1);
                (*header).in_use = false;
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
        
        for (size_class_idx, slabs) in self.slabs.iter().enumerate() {
            let size = SLAB_SIZES[size_class_idx];
            for slab in slabs {
                total_allocated += slab.capacity;
                total_used += slab.used_count * size;
            }
        }
        
        AllocatorStats { used: total_used, allocated: total_allocated }
    }
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