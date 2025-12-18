//! Optimized slab allocator with improved performance

extern crate alloc;

use core::alloc::Layout;
use core::ptr::null_mut;
use core::mem;
use alloc::vec::Vec;

/// Object size classes: 8, 16, 32, 64, 128, 256, 512, 1024, 2048
pub const SLAB_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// A slab object header (placed before each allocated object)
#[repr(C, packed)]
struct SlabObjectHeader {
    size_class: u8, // Size class index
    prev: *mut u8,   // Previous free object for double-linked free list
    next: *mut u8,   // Next free object in free list
}

/// A slab containing multiple objects of the same size
struct Slab {
    /// Base address of this slab
    base: *mut u8,
    /// Size of this slab in bytes
    capacity: usize,
    /// Object size for this slab
    object_size: usize,
    /// Number of objects in this slab
    total_objects: usize,
    /// Number of used objects in this slab
    used_count: usize,
    /// Free list head - direct pointer without header indirection
    free_list_head: *mut u8,
    /// Free list tail - for faster appends
    free_list_tail: *mut u8,
    /// Bitmap for quick occupancy check
    occupancy: u64, // TODO: support more than 64 objects per slab
}

/// Slab allocator statistics.
///
/// Tracks memory usage and slab count for the slab allocator.
#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    /// Number of bytes currently in use by allocated objects.
    pub used: usize,
    /// Total number of bytes allocated across all slabs.
    pub allocated: usize,
    /// Total number of slabs created.
    pub slab_count: usize,
}

/// Main slab allocator
pub struct OptimizedSlabAllocator {
    /// Slabs for each size class (initialized later)
    slabs: Option<Vec<Vec<Slab>>>,
    /// Heap management
    heap_ptr: *mut u8,
    heap_size: usize,
    allocated: usize,
    /// Pre-allocated slab cache for different size classes
    slab_caches: Vec<usize>,
}

impl OptimizedSlabAllocator {
    /// Create an uninitialized slab allocator (for const contexts)
    pub const fn uninitialized() -> Self {
        Self {
            slabs: None,
            heap_ptr: null_mut(),
            heap_size: 0,
            allocated: 0,
            slab_caches: Vec::new(),
        }
    }

    /// Create a new slab allocator
    pub fn new() -> Self {
        let mut slabs = Vec::new();
        let mut slab_caches = Vec::new();
        
        for _ in SLAB_SIZES {
            slabs.push(Vec::new());
            // Initialize slab cache size for each size class
            slab_caches.push(10); // Default cache size: 10 slabs per size class
        }

        Self {
            slabs: Some(slabs),
            heap_ptr: null_mut(),
            heap_size: 0,
            allocated: 0,
            slab_caches,
        }
    }

    /// Get mutable reference to slabs, initializing if necessary
    fn get_slabs_mut(&mut self) -> &mut Vec<Vec<Slab>> {
        if self.slabs.is_none() || self.slab_caches.is_empty() {
            let mut slabs = Vec::new();
            let mut slab_caches = Vec::new();
            
            for _ in SLAB_SIZES {
                slabs.push(Vec::new());
                slab_caches.push(10); // Default cache size: 10 slabs per size class
            }
            
            self.slabs = Some(slabs);
            self.slab_caches = slab_caches;
        }
        self.slabs.as_mut().unwrap()
    }

    /// Initialize the slab allocator with a memory region
    ///
    /// # Safety
    /// - `heap_ptr` must point to valid, contiguous memory that is accessible
    /// - `heap_size` must be the size of the memory region in bytes
    /// - The memory region must be aligned to the system's page size
    /// - The caller must ensure no other code accesses this memory region while the allocator is in use
    pub unsafe fn init(&mut self, heap_ptr: *mut u8, heap_size: usize) {
        self.heap_ptr = heap_ptr;
        self.heap_size = heap_size;
        self.allocated = 0;

        // Initialize slabs and slab_caches if not already done
        if self.slabs.is_none() || self.slab_caches.is_empty() {
            let mut slabs = Vec::new();
            let mut slab_caches = Vec::new();
            
            for _ in SLAB_SIZES {
                slabs.push(Vec::new());
                slab_caches.push(10); // Default cache size: 10 slabs per size class
            }
            
            self.slabs = Some(slabs);
            self.slab_caches = slab_caches;
        }
    }

    /// Create a new slab for a given size class with optimized sizing
    unsafe fn create_slab(&mut self, size_class_idx: usize, object_size: usize) -> Option<Slab> {
        unsafe {
            // Calculate optimized slab size based on object size
            let objects_per_slab = match object_size {
                8..=64 => 256,    // Small objects: many per slab
                128..=256 => 128,  // Medium objects
                512..=1024 => 64,  // Large objects
                _ => 32,           // Very large objects
            };
            
            // Add header size to object size
            let actual_object_size = mem::size_of::<SlabObjectHeader>() + object_size;
            
            // Calculate slab size, round up to page boundary (4KB)
            let slab_size = (actual_object_size * objects_per_slab + 4095) & !4095;
            
            if self.allocated + slab_size > self.heap_size {
                return None;
            }

            let slab_base = self.heap_ptr.add(self.allocated);
            self.allocated += slab_size;

            // Initialize free list
            let mut free_list_head = null_mut();
            let mut free_list_tail = null_mut();
            
            for i in 0..objects_per_slab {
                let obj = slab_base.add(i * actual_object_size);
                
                if i < objects_per_slab - 1 {
                    let next = slab_base.add((i + 1) * actual_object_size);
                    let header = obj as *mut SlabObjectHeader;
                    
                    // Initialize header
                    (*header).size_class = size_class_idx as u8;
                    (*header).prev = free_list_tail;
                    (*header).next = next;
                    
                    // Update tail
                    if i == 0 {
                        free_list_head = obj;
                    } else {
                        let prev_header = free_list_tail as *mut SlabObjectHeader;
                        (*prev_header).next = obj;
                    }
                    
                    free_list_tail = obj;
                } else {
                    // Last object
                    let header = obj as *mut SlabObjectHeader;
                    (*header).size_class = size_class_idx as u8;
                    (*header).prev = free_list_tail;
                    (*header).next = null_mut();
                    
                    let prev_header = free_list_tail as *mut SlabObjectHeader;
                    (*prev_header).next = obj;
                    
                    free_list_tail = obj;
                }
            }

            Some(Slab {
                base: slab_base,
                capacity: slab_size,
                object_size: actual_object_size,
                total_objects: objects_per_slab,
                used_count: 0,
                free_list_head,
                free_list_tail,
                occupancy: 0,
            })
        }
    }

    /// Allocate memory with given layout
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // Find the appropriate size class
        let size_class_idx = Self::find_size_class(size);
        if size_class_idx >= SLAB_SIZES.len() {
            return null_mut(); // Allocation too large for slab allocator
        }

        { // First scope: try to allocate from existing slabs
            let slabs = self.get_slabs_mut();
            if size_class_idx >= slabs.len() {
                return null_mut();
            }
            
            // Try to allocate from an existing slab
            for slab in slabs[size_class_idx].iter_mut() {
                if slab.free_list_head.is_null() {
                    continue;
                }
                
                unsafe {
                    // Take first free object from the free list
                    let obj = slab.free_list_head;
                    slab.free_list_head = (*(obj as *mut SlabObjectHeader)).next;
                    
                    // Update the next object's prev pointer if it exists
                    if !slab.free_list_head.is_null() {
                        (*(slab.free_list_head as *mut SlabObjectHeader)).prev = null_mut();
                    }
                    
                    // Calculate the object index within the slab
                    let object_idx = (obj as usize - slab.base as usize) / slab.object_size;
                    
                    // Increment used count and update occupancy bitmap
                    slab.used_count += 1;
                    slab.occupancy |= 1 << object_idx; // Mark as used
                    
                    // Return pointer after header
                    return obj.add(mem::size_of::<SlabObjectHeader>());
                }
            }
        } // Slab borrow ends here
        
        // No free slab found, create a new one
        let slab_size = SLAB_SIZES[size_class_idx];
        let new_slab = unsafe { self.create_slab(size_class_idx, slab_size) };
        if let Some(mut slab) = new_slab {
            // Second scope: add new slab and allocate from it
            let slabs = self.get_slabs_mut();
            // Allocate from the newly created slab
            let obj = slab.free_list_head;
            slab.free_list_head = unsafe { (*(obj as *mut SlabObjectHeader)).next };
            
            // Update the next object's prev pointer if it exists
            if !slab.free_list_head.is_null() {
                unsafe {
                    (*(slab.free_list_head as *mut SlabObjectHeader)).prev = null_mut();
                }
            }
            
            // Calculate the object index within the slab
            let object_idx = (obj as usize - slab.base as usize) / slab.object_size;
            
            // Increment used count and update occupancy bitmap
            slab.used_count += 1;
            slab.occupancy |= 1 << object_idx; // Mark as used
            
            // Add the new slab to the slab list
            slabs[size_class_idx].push(slab);
            
            // Return pointer after header
            unsafe {
                return obj.add(mem::size_of::<SlabObjectHeader>());
            }
        }

        null_mut() // Allocation failed
    }

    /// Deallocate memory
    ///
    /// # Safety
    /// - `ptr` must have been previously allocated by this allocator
    /// - `ptr` must not be null (though this method handles null pointers safely)
    /// - `ptr` must be properly aligned and point to valid memory
    /// - The memory must not have been previously freed (double-free is undefined behavior)
    /// - `_layout` should match the layout used when allocating the memory
    pub unsafe fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            if ptr.is_null() {
                return;
            }

            // Calculate actual object pointer (subtract header)
            let obj = ptr.sub(mem::size_of::<SlabObjectHeader>());
            let header = obj as *mut SlabObjectHeader;
            
            let size_class_idx = (*header).size_class as usize;
            if size_class_idx >= SLAB_SIZES.len() {
                return; // Invalid size class
            }

            let slabs = self.get_slabs_mut();
            if size_class_idx >= slabs.len() {
                return;
            }
            
            // Find the slab containing this object
            for slab in slabs[size_class_idx].iter_mut() {
                let slab_end = slab.base.add(slab.capacity);
                if (obj as usize) >= (slab.base as usize) && (obj as usize) < (slab_end as usize) {
                    // Object found in this slab
                    
                    // Calculate the object index within the slab
                    let object_idx = (obj as usize - slab.base as usize) / slab.object_size;
                    
                    // Clear the occupancy bit
                    slab.occupancy &= !(1 << object_idx);
                    
                    // Add back to the free list
                    if slab.free_list_head.is_null() {
                        // Free list is empty, initialize it
                        slab.free_list_head = obj;
                        slab.free_list_tail = obj;
                        
                        (*header).prev = null_mut();
                        (*header).next = null_mut();
                    } else {
                        // Add to the front of the free list for faster allocation
                        (*header).prev = null_mut();
                        (*header).next = slab.free_list_head;
                        
                        // Update previous head's prev pointer
                        let head_header = slab.free_list_head as *mut SlabObjectHeader;
                        (*head_header).prev = obj;
                        
                        slab.free_list_head = obj;
                    }
                    
                    // Decrement used count
                    slab.used_count = slab.used_count.saturating_sub(1);
                    
                    return;
                }
            }
        }
    }

    /// Find the appropriate size class for a given size
    pub fn find_size_class(size: usize) -> usize {
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
        let mut slab_count = 0;
        
        if let Some(ref slabs_array) = self.slabs {
            for (size_class_idx, slabs) in slabs_array.iter().enumerate() {
                slab_count += slabs.len();
                
                let base_size = SLAB_SIZES[size_class_idx];
                
                for slab in slabs {
                    total_allocated += slab.capacity;
                    total_used += slab.used_count * base_size;
                    
                    // Validate that the slab configuration is consistent
                    debug_assert!(slab.used_count <= slab.total_objects, "Used count exceeds total objects");
                    debug_assert!(slab.object_size == mem::size_of::<SlabObjectHeader>() + base_size, "Object size mismatch");
                }
            }
        }
        
        AllocatorStats { used: total_used, allocated: total_allocated, slab_count }
    }

    /// Shrink slab allocator by removing completely empty slabs
    pub fn shrink(&mut self) -> usize {
        let mut freed_slabs = 0;

        if let Some(ref mut slabs) = self.slabs {
            for size_class_slabs in slabs.iter_mut() {
                // Remove completely empty slabs from this size class
                let initial_count = size_class_slabs.len();
                
                // Retain only slabs that have used objects
                size_class_slabs.retain(|slab| slab.used_count > 0);
                
                let removed_count = initial_count - size_class_slabs.len();
                freed_slabs += removed_count;
            }
        }

        freed_slabs
    }
}

// Implement Default for OptimizedSlabAllocator
impl Default for OptimizedSlabAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Send/Sync for slab allocator types since they are thread-safe when protected by Mutex
unsafe impl Send for SlabObjectHeader {}
unsafe impl Sync for SlabObjectHeader {}
unsafe impl Send for Slab {}
unsafe impl Sync for Slab {}
unsafe impl Send for OptimizedSlabAllocator {}
unsafe impl Sync for OptimizedSlabAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_alloc() {
        let mut alloc = OptimizedSlabAllocator::new();
        unsafe { alloc.init(0x1000 as *mut u8, 0x100000); }
        
        let layout = Layout::from_size_align(64, 8).unwrap();
        let ptr = alloc.alloc(layout);
        assert!(!ptr.is_null());
        
        unsafe {
            alloc.dealloc(ptr, layout);
        }
    }
}

/// Alias for backward compatibility
pub type SlabAllocator = OptimizedSlabAllocator;