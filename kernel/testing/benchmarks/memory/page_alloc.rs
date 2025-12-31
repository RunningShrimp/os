//! Page Allocation Benchmark
//!
//! Measures page allocation throughput and latency.
//! Target: >1M pages/sec allocation rate, <100ns average latency.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

const PAGE_SIZE: usize = 4096;
const ALLOCATION_BATCH: usize = 100;

/// Benchmark page allocation performance
pub struct PageAllocationBenchmark {
    pages_to_allocate: usize,
}

impl PageAllocationBenchmark {
    pub fn new() -> Self {
        Self {
            pages_to_allocate: ALLOCATION_BATCH,
        }
    }

    pub fn with_pages(mut self, count: usize) -> Self {
        self.pages_to_allocate = count;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for PageAllocationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for PageAllocationBenchmark {
    fn name(&self) -> &str {
        "memory/page_allocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        // Initialize page allocator
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Measure time to allocate batch of pages
        let (allocated, elapsed_ns) = measure_time(|| {
            // Simulate page allocation
            let mut allocations = Vec::with_capacity(self.pages_to_allocate);

            for _ in 0..self.pages_to_allocate {
                // Simulate allocation: find free page, mark as used
                // In real implementation: allocate_pages(1, flags)
                let page = simulate_page_allocation();
                allocations.push(page);
            }

            allocations
        });

        // Return nanoseconds per allocation
        let ns_per_alloc = elapsed_ns / allocated.len().max(1) as u64;
        Ok(ns_per_alloc)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Simulate page allocation
fn simulate_page_allocation() -> *mut u8 {
    // Placeholder: In real implementation, this would:
    // 1. Acquire allocator lock
    // 2. Find free page in appropriate zone
    // 3. Mark page as used
    // 4. Return physical/virtual address

    // Simulate allocation work
    for _ in 0..5 {
        core::hint::spin_loop();
    }

    0x1000 as *mut u8 // Placeholder address
}

/// Benchmark page deallocation performance
pub struct PageDeallocationBenchmark {
    pages_to_free: usize,
}

impl PageDeallocationBenchmark {
    pub fn new() -> Self {
        Self {
            pages_to_free: ALLOCATION_BATCH,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for PageDeallocationBenchmark {
    fn name(&self) -> &str {
        "memory/page_deallocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        // Pre-allocate pages to free
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Simulate freeing pages
        for _ in 0..self.pages_to_free {
            simulate_page_deallocation();
        }

        let elapsed = start.elapsed();
        let ns_per_free = elapsed.as_nanos() as u64 / self.pages_to_free as u64;
        Ok(ns_per_free)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Simulate page deallocation
fn simulate_page_deallocation() {
    // Simulate deallocation work
    for _ in 0..3 {
        core::hint::spin_loop();
    }
}

/// Benchmark large page (huge page) allocation
pub struct HugePageAllocationBenchmark {
    page_size: usize,
}

impl HugePageAllocationBenchmark {
    pub fn new() -> Self {
        Self {
            page_size: 2 * 1024 * 1024, // 2MB huge page
        }
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for HugePageAllocationBenchmark {
    fn name(&self) -> &str {
        "memory/hugepage_allocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Simulate huge page allocation (requires contiguous physical pages)
        simulate_page_allocation();

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Mixed allocation/deallocation workload
pub struct MixedAllocationBenchmark {
    alloc_ratio: f64, // Percentage of allocations vs deallocations
}

impl MixedAllocationBenchmark {
    pub fn new() -> Self {
        Self {
            alloc_ratio: 0.7,
        }
    }

    pub fn with_ratio(mut self, ratio: f64) -> Self {
        self.alloc_ratio = ratio;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for MixedAllocationBenchmark {
    fn name(&self) -> &str {
        "memory/mixed_allocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let operations = 100;
        let start = core::time::Instant::now();

        for i in 0..operations {
            if (i as f64 / operations as f64) < self.alloc_ratio {
                simulate_page_allocation();
            } else {
                simulate_page_deallocation();
            }
        }

        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() as u64 / operations as u64;
        Ok(ns_per_op)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_allocation_benchmark() {
        let mut bench = PageAllocationBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
        assert!(result.throughput_ops_per_sec > 0.0);
    }

    #[test]
    fn test_page_deallocation_benchmark() {
        let mut bench = PageDeallocationBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_hugepage_allocation() {
        let mut bench = HugePageAllocationBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_mixed_allocation() {
        let mut bench = MixedAllocationBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
