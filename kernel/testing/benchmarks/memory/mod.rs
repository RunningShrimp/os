//! Memory Performance Benchmarks
//!
//! This module contains comprehensive benchmarks for memory allocation,
//! including page allocation, slab allocation, fragmentation analysis, and NUMA.

pub mod page_alloc;
pub mod slab_alloc;
pub mod fragmentation;
pub mod numa_locality;

use crate::testing::benchmarks::{BenchmarkConfig, BenchmarkResult};
use alloc::vec::Vec;

/// Run all memory benchmarks
pub fn run_all(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>, String> {
    let mut results = Vec::new();

    // Page allocation benchmark
    match page_alloc::PageAllocationBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Page allocation benchmark failed: {}", e),
    }

    // Slab allocation benchmark
    match slab_alloc::SlabAllocationBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Slab allocation benchmark failed: {}", e),
    }

    // Fragmentation benchmark
    match fragmentation::FragmentationBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Fragmentation benchmark failed: {}", e),
    }

    // NUMA locality benchmark
    match numa_locality::NumaLocalityBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("NUMA locality benchmark failed: {}", e),
    }

    Ok(results)
}

/// Run page allocation benchmark (convenience function)
pub fn run_page_allocation_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    page_alloc::PageAllocationBenchmark::run(config)
}

/// Run slab allocation benchmark (convenience function)
pub fn run_slab_allocation_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    slab_alloc::SlabAllocationBenchmark::run(config)
}

/// Run fragmentation benchmark (convenience function)
pub fn run_fragmentation_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    fragmentation::FragmentationBenchmark::run(config)
}

/// Run NUMA locality benchmark (convenience function)
pub fn run_numa_locality_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    numa_locality::NumaLocalityBenchmark::run(config)
}
