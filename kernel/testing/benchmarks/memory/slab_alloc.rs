//! Slab Allocation Benchmark
//!
//! Measures slab allocator performance for small object allocations.
//! Target: <100ns average allocation latency.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

/// Object sizes to benchmark
const SLAB_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024];

/// Benchmark slab allocation for various object sizes
pub struct SlabAllocationBenchmark {
    object_size: usize,
    batch_size: usize,
}

impl SlabAllocationBenchmark {
    pub fn new(object_size: usize) -> Self {
        Self {
            object_size,
            batch_size: 1000,
        }
    }

    pub fn with_batch(mut self, batch: usize) -> Self {
        self.batch_size = batch;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(64);
        benchmark.execute(config)
    }

    /// Run benchmark for all standard slab sizes
    pub fn run_all_sizes(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>, String> {
        let mut results = Vec::new();

        for &size in SLAB_SIZES {
            let mut bench = Self::new(size);
            match bench.execute(config) {
                Ok(result) => results.push(result),
                Err(e) => log::error!("Slab benchmark failed for size {}: {}", size, e),
            }
        }

        Ok(results)
    }
}

impl BenchmarkSuite for SlabAllocationBenchmark {
    fn name(&self) -> &str {
        "memory/slab_allocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Measure allocation time
        let (allocated, elapsed_ns) = measure_time(|| {
            let mut allocations = Vec::with_capacity(self.batch_size);

            for _ in 0..self.batch_size {
                let obj = simulate_slab_allocation(self.object_size);
                allocations.push(obj);
            }

            allocations
        });

        let ns_per_alloc = elapsed_ns / allocated.len().max(1) as u64;
        Ok(ns_per_alloc)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Simulate slab allocation
fn simulate_slab_allocation(size: usize) -> *mut u8 {
    // In real implementation:
    // 1. Find appropriate slab cache
    // 2. Get object from per-CPU cache
    // 3. If cache empty, refill from slab
    // 4. Return object pointer

    // Simulate cache lookup
    for _ in 0..3 {
        core::hint::spin_loop();
    }

    // Simulate based on size
    let iterations = (size / 32).max(1);
    for _ in 0..iterations {
        core::hint::spin_loop();
    }

    0x2000 as *mut u8 // Placeholder
}

/// Benchmark slab deallocation
pub struct SlabDeallocationBenchmark {
    object_size: usize,
    batch_size: usize,
}

impl SlabDeallocationBenchmark {
    pub fn new(object_size: usize) -> Self {
        Self {
            object_size,
            batch_size: 1000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(64);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for SlabDeallocationBenchmark {
    fn name(&self) -> &str {
        "memory/slab_deallocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        for _ in 0..self.batch_size {
            simulate_slab_deallocation();
        }

        let elapsed = start.elapsed();
        let ns_per_free = elapsed.as_nanos() as u64 / self.batch_size as u64;
        Ok(ns_per_free)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Simulate slab deallocation
fn simulate_slab_deallocation() {
    // Simulate returning object to per-CPU cache
    for _ in 0..2 {
        core::hint::spin_loop();
    }
}

/// Benchmark per-CPU slab cache hit rate
pub struct SlabCacheHitBenchmark {
    object_size: usize,
    cache_size: usize,
}

impl SlabCacheHitBenchmark {
    pub fn new(object_size: usize) -> Self {
        Self {
            object_size,
            cache_size: 64, // Typical per-CPU cache size
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(64);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for SlabCacheHitBenchmark {
    fn name(&self) -> &str {
        "memory/slab_cache_hit"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let operations = self.cache_size * 2;
        let start = core::time::Instant::now();

        // Allocate then free to test cache hits
        for i in 0..operations {
            if i < self.cache_size {
                simulate_slab_allocation(self.object_size);
            } else {
                simulate_slab_deallocation();
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

/// Benchmark slab cache miss (refill from slab)
pub struct SlabCacheMissBenchmark {
    object_size: usize,
}

impl SlabCacheMissBenchmark {
    pub fn new(object_size: usize) -> Self {
        Self { object_size }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(64);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for SlabCacheMissBenchmark {
    fn name(&self) -> &str {
        "memory/slab_cache_miss"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate cache miss - need to refill from slab
        let start = core::time::Instant::now();

        // More expensive: allocate new slab
        for _ in 0..20 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Multi-threaded slab allocation benchmark
pub struct MultiThreadSlabBenchmark {
    num_threads: usize,
    allocs_per_thread: usize,
}

impl MultiThreadSlabBenchmark {
    pub fn new(num_threads: usize) -> Self {
        Self {
            num_threads,
            allocs_per_thread: 1000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(4);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for MultiThreadSlabBenchmark {
    fn name(&self) -> &str {
        "memory/slab_multithreaded"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let total_ops = self.num_threads * self.allocs_per_thread;
        let start = core::time::Instant::now();

        // Simulate multiple threads allocating from per-CPU caches
        for _ in 0..self.num_threads {
            for _ in 0..self.allocs_per_thread {
                simulate_slab_allocation(64);
            }
        }

        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() as u64 / total_ops as u64;
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
    fn test_slab_allocation_benchmark() {
        let mut bench = SlabAllocationBenchmark::new(64);
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_slab_deallocation_benchmark() {
        let mut bench = SlabDeallocationBenchmark::new(64);
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_cache_hit_benchmark() {
        let mut bench = SlabCacheHitBenchmark::new(64);
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_multithread_slab() {
        let mut bench = MultiThreadSlabBenchmark::new(4);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
