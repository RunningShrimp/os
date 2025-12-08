//! Adaptive Prefetching Benchmark
//!
//! This benchmark measures the performance improvement of adaptive prefetching.

#![feature(test)]

extern crate test;

use test::Bencher;
use kernel::mm;
use kernel::mm::prefetch;

#[bench]
fn bench_sequential_memory_access(b: &mut Bencher) {
    // Simulate sequential memory access pattern
    // This should benefit from sequential prefetching
    
    b.iter(|| {
        let mut sum = 0;
        let base = 0x10000000; // Simulated user space address
        
        // Access 1MB sequentially
        for i in 0..256 { // 256 * 4KB = 1MB
            let addr = base + i * mm::PAGE_SIZE;
            
            // Notify prefetch module about this access
            prefetch::process_memory_access(addr, mm::PAGE_SIZE, prefetch::AccessType::Read);
            
            // Simulate memory access latency
            sum += addr;
        }
        
        sum
    });
}

#[bench]
fn bench_stride_memory_access(b: &mut Bencher) {
    // Simulate stride memory access pattern (every 2 pages)
    
    b.iter(|| {
        let mut sum = 0;
        let base = 0x10000000;
        let stride = 2 * mm::PAGE_SIZE;
        
        for i in 0..256 {
            let addr = base + i * stride;
            
            prefetch::process_memory_access(addr, mm::PAGE_SIZE, prefetch::AccessType::Read);
            
            sum += addr;
        }
        
        sum
    });
}