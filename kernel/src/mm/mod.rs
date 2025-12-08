pub mod phys;
pub mod vm;
pub mod allocator;
pub mod buddy;
pub mod slab;
pub mod optimized_buddy;
pub mod optimized_slab;
pub mod mempool;
pub mod compress;
pub mod hugepage;
pub mod traits;
pub mod optimized_allocator;
pub mod prefetch;
pub mod numa;

#[cfg(feature = "kernel_tests")]
pub mod tests;

pub use phys::*;
pub use traits::*;
pub use optimized_buddy::OptimizedBuddyAllocator;
pub use optimized_slab::OptimizedSlabAllocator;
pub use mempool::*;
pub use optimized_allocator::OptimizedHybridAllocator;
pub use numa::*;

/// Align up to the given alignment
pub const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

/// Align down to the given alignment
pub const fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}

/// Check if an address is aligned to the given alignment
pub const fn is_aligned(addr: usize, align: usize) -> bool {
    (addr & (align - 1)) == 0
}

/// Round up to the next power of 2
pub const fn round_up_power_of_2(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        let mut v = n - 1;
        v |= v >> 1;
        v |= v >> 2;
        v |= v >> 4;
        v |= v >> 8;
        v |= v >> 16;
        v |= v >> 32;
        v + 1
    }
}

/// Get the log2 of a power-of-2 number
pub const fn log2_pow2(n: usize) -> u32 {
    if n == 0 {
        panic!("log2_pow2(0) is undefined");
    }
    (usize::BITS - 1) - n.leading_zeros()
}

/// Get the order (log2) of a size, rounded up to the nearest power of 2
pub const fn get_order(size: usize, min_order: usize) -> usize {
    let mut order = min_order;
    let mut current_size = 1 << min_order;
    
    while current_size < size {
        current_size *= 2;
        order += 1;
    }
    
    order
}
