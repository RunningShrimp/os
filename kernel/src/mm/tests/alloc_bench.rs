//! Benchmarks for memory allocation performance
//! 
//! This module contains benchmarks for comparing the performance of the
//! original allocators vs the optimized allocators.

use core::alloc::Layout;
use crate::mm::{
    buddy::BuddyAllocator,
    slab::SlabAllocator,
    optimized_buddy::OptimizedBuddyAllocator,
    optimized_slab::OptimizedSlabAllocator,
    optimized_allocator::OptimizedHybridAllocator,
    hybrid::HybridAllocator, // If the original was named HybridAllocator
    traits::{UnifiedAllocator, AllocatorWithStats},
};

use crate::benchmark::memory::*;

/// Benchmark the original buddy allocator
pub fn bench_original_buddy() -> AllocatorStats {
    let mut alloc = BuddyAllocator::new();
    
    // Initialize with a 16MB heap
    unsafe { alloc.init(0x1000000, 0x1100000, 4096); }
    
    let layout = Layout::from_size_align(1024, 8).unwrap();
    
    // Run the benchmark
    let stats = bench_allocator(&alloc, layout, 100000);
    
    stats
}

/// Benchmark the optimized buddy allocator
pub fn bench_optimized_buddy() -> AllocatorStats {
    let mut alloc = OptimizedBuddyAllocator::new();
    
    // Initialize with a 16MB heap
    unsafe { alloc.init(0x1000000, 0x1100000, 4096); }
    
    let layout = Layout::from_size_align(1024, 8).unwrap();
    
    // Run the benchmark
    let stats = bench_allocator(&alloc, layout, 100000);
    
    stats
}

/// Benchmark the original slab allocator
pub fn bench_original_slab() -> AllocatorStats {
    let mut alloc = SlabAllocator::new();
    
    // Initialize with a 4MB heap
    unsafe { alloc.init(0x1000000 as *mut u8, 0x1040000); }
    
    let layout = Layout::from_size_align(64, 8).unwrap();
    
    // Run the benchmark
    let stats = bench_allocator(&alloc, layout, 100000);
    
    stats
}

/// Benchmark the optimized slab allocator
pub fn bench_optimized_slab() -> AllocatorStats {
    let mut alloc = OptimizedSlabAllocator::new();
    
    // Initialize with a 4MB heap
    unsafe { alloc.init(0x1000000 as *mut u8, 0x1040000); }
    
    let layout = Layout::from_size_align(64, 8).unwrap();
    
    // Run the benchmark
    let stats = bench_allocator(&alloc, layout, 100000);
    
    stats
}

/// Benchmark the original hybrid allocator
pub fn bench_original_hybrid() -> AllocatorStats {
    // Note: This may need adjustment based on the actual original allocator name
    // and initialization requirements
    
    let stats = AllocatorStats {
        operations: 0,
        time_ns: 0,
        throughput_mb_s: 0.0,
        fragmentation_rate: 0.0,
    };
    
    stats
}

/// Benchmark the optimized hybrid allocator
pub fn bench_optimized_hybrid() -> AllocatorStats {
    let alloc = OptimizedHybridAllocator::new();
    
    // Initialize with a 16MB buddy heap and 4MB slab heap
    unsafe {
        alloc.init(0x1000000, 0x1040000, 0x1040000, 0x1100000, 4096);
    }
    
    // Benchmark with a mix of small and medium allocations
    let stats = bench_mixed_allocations(&alloc, 100000);
    
    stats
}

/// Run all benchmarks and print results
pub fn run_all_benchmarks() {
    // Note: This function is intended for use in a test or benchmark context
    
    crate::println!("Running memory allocation benchmarks...\n");
    
    // Buddy allocator benchmarks
    //let original_buddy_stats = bench_original_buddy();
    //crate::println!("Original Buddy Allocator: {:#?}", original_buddy_stats);
    
    let optimized_buddy_stats = bench_optimized_buddy();
    crate::println!("Optimized Buddy Allocator: {:#?}", optimized_buddy_stats);
    
    crate::println!("\n---\n");
    
    // Slab allocator benchmarks
    //let original_slab_stats = bench_original_slab();
    //crate::println!("Original Slab Allocator: {:#?}", original_slab_stats);
    
    let optimized_slab_stats = bench_optimized_slab();
    crate::println!("Optimized Slab Allocator: {:#?}", optimized_slab_stats);
    
    crate::println!("\n---\n");
    
    // Hybrid allocator benchmarks
    let optimized_hybrid_stats = bench_optimized_hybrid();
    crate::println!("Optimized Hybrid Allocator: {:#?}", optimized_hybrid_stats);
    
    crate::println!("\nBenchmarks complete!");
}

/// Benchmark results structure
#[derive(Debug)]
pub struct AllocatorStats {
    pub operations: usize,
    pub time_ns: u64,
    pub throughput_mb_s: f64,
    pub fragmentation_rate: f64,
}

/// Benchmark a single allocator with fixed-size allocations
pub unsafe fn bench_allocator<A: UnifiedAllocator>(
    allocator: &A,
    layout: Layout,
    num_ops: usize,
) -> AllocatorStats {
    // This is a placeholder implementation. A real benchmark would need to:
    // 1. Measure time accurately
    // 2. Track actual allocation performance
    
    let mut ptrs = Vec::with_capacity(num_ops);
    let start_time = crate::benchmark::get_time_ns();
    
    // Allocate
    for _ in 0..num_ops {
        let ptr = allocator.allocate(layout);
        ptrs.push(ptr);
    }
    
    // Deallocate
    for ptr in ptrs {
        allocator.deallocate(ptr, layout);
    }
    
    let end_time = crate::benchmark::get_time_ns();
    let time_ns = end_time - start_time;
    let total_data = num_ops * layout.size();
    
    AllocatorStats {
        operations: num_ops,
        time_ns,
        throughput_mb_s: (total_data as f64 * 1e9) / (time_ns as f64 * 1024.0 * 1024.0),
        fragmentation_rate: 0.0, // Not yet implemented
    }
}

/// Benchmark with a mix of different allocation sizes
pub unsafe fn bench_mixed_allocations<A: UnifiedAllocator>(
    allocator: &A,
    num_ops: usize,
) -> AllocatorStats {
    // Mix of common allocation sizes
    let sizes = [16, 32, 64, 128, 256, 512, 1024, 2048];
    
    let mut ptrs = Vec::with_capacity(num_ops);
    let start_time = crate::benchmark::get_time_ns();
    
    // Allocate
    for i in 0..num_ops {
        let size = sizes[i % sizes.len()];
        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = allocator.allocate(layout);
        ptrs.push((ptr, layout));
    }
    
    // Deallocate
    for (ptr, layout) in ptrs {
        allocator.deallocate(ptr, layout);
    }
    
    let end_time = crate::benchmark::get_time_ns();
    let time_ns = end_time - start_time;
    
    // Calculate average data size
    let avg_size = sizes.iter().sum::<usize>() as f64 / sizes.len() as f64;
    let total_data = (num_ops as f64 * avg_size) as usize;
    
    AllocatorStats {
        operations: num_ops,
        time_ns,
        throughput_mb_s: (total_data as f64 * 1e9) / (time_ns as f64 * 1024.0 * 1024.0),
        fragmentation_rate: 0.0, // Not yet implemented
    }
}