//! Memory Fragmentation Benchmark
//!
//! Analyzes memory fragmentation under various allocation patterns.
//! Target: <20% external fragmentation ratio.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

/// Benchmark external fragmentation
pub struct FragmentationBenchmark {
    total_memory: usize,
    allocation_pattern: AllocationPattern,
}

/// Different allocation patterns to test fragmentation
#[derive(Debug, Clone, Copy)]
pub enum AllocationPattern {
    /// Random sized allocations
    Random,
    /// Alternating large and small allocations
    Alternating,
    /// Gradually increasing then decreasing sizes
    Wave,
    /// Realistic workload (mix of sizes)
    Realistic,
}

impl FragmentationBenchmark {
    pub fn new() -> Self {
        Self {
            total_memory: 1024 * 1024 * 10, // 10 MB
            allocation_pattern: AllocationPattern::Random,
        }
    }

    pub fn with_pattern(mut self, pattern: AllocationPattern) -> Self {
        self.allocation_pattern = pattern;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }

    /// Calculate fragmentation ratio
    pub fn calculate_fragmentation(&self, free_memory: usize, largest_free_block: usize) -> f64 {
        if free_memory == 0 {
            return 0.0;
        }
        // External fragmentation: 1 - (largest_free / total_free)
        1.0 - (largest_free_block as f64 / free_memory as f64)
    }
}

impl Default for FragmentationBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for FragmentationBenchmark {
    fn name(&self) -> &str {
        "memory/fragmentation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate allocation/deallocation pattern
        let mut allocations = Vec::new();
        let fragmentation = match self.allocation_pattern {
            AllocationPattern::Random => self.simulate_random_pattern(&mut allocations),
            AllocationPattern::Alternating => self.simulate_alternating_pattern(&mut allocations),
            AllocationPattern::Wave => self.simulate_wave_pattern(&mut allocations),
            AllocationPattern::Realistic => self.simulate_realistic_pattern(&mut allocations),
        };

        // Return fragmentation percentage as the metric
        Ok((fragmentation * 100.0) as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

impl FragmentationBenchmark {
    fn simulate_random_pattern(&self, allocations: &mut Vec<(usize, usize)>) -> f64 {
        use core::hint::spin_loop;

        let mut allocated = 0;
        let mut free_memory = self.total_memory;
        let mut largest_free = self.total_memory;

        // Random allocations
        for i in 0..100 {
            let size = ((i * 137) % 4096 + 1) * 8; // Pseudo-random sizes

            if allocated + size < self.total_memory {
                allocations.push((i, size));
                allocated += size;
                free_memory -= size;

                // Update largest free block estimate
                largest_free = largest_free.min(free_memory);

                // Simulate allocation overhead
                for _ in 0..3 {
                    spin_loop();
                }
            }
        }

        // Free half the allocations randomly
        for j in (0..allocations.len()).step_by(2) {
            if let Some((idx, size)) = allocations.get(j) {
                free_memory += size;
                allocated -= size;
            }
        }

        self.calculate_fragmentation(free_memory, largest_free)
    }

    fn simulate_alternating_pattern(&self, allocations: &mut Vec<(usize, usize)>) -> f64 {
        let mut allocated = 0;
        let mut free_memory = self.total_memory;
        let mut largest_free = self.total_memory;

        // Alternate between large and small allocations
        for i in 0..50 {
            // Large allocation
            let large_size = 4096 * 4;
            if allocated + large_size < self.total_memory {
                allocations.push((i * 2, large_size));
                allocated += large_size;
                free_memory -= large_size;
                largest_free = largest_free.min(free_memory);
            }

            // Small allocation
            let small_size = 64;
            if allocated + small_size < self.total_memory {
                allocations.push((i * 2 + 1, small_size));
                allocated += small_size;
                free_memory -= small_size;
                largest_free = largest_free.min(free_memory);
            }
        }

        // Free every other allocation
        for j in (0..allocations.len()).step_by(2) {
            if let Some((_, size)) = allocations.get(j) {
                free_memory += size;
            }
        }

        self.calculate_fragmentation(free_memory, largest_free)
    }

    fn simulate_wave_pattern(&self, allocations: &mut Vec<(usize, usize)>) -> f64 {
        let mut allocated = 0;
        let mut free_memory = self.total_memory;
        let mut largest_free = self.total_memory;

        // Increasing then decreasing allocation sizes
        for i in 0..50 {
            let size = if i < 25 {
                (i + 1) * 64 // Increasing
            } else {
                (50 - i) * 64 // Decreasing
            };

            if allocated + size < self.total_memory {
                allocations.push((i, size));
                allocated += size;
                free_memory -= size;
                largest_free = largest_free.min(free_memory);
            }
        }

        self.calculate_fragmentation(free_memory, largest_free)
    }

    fn simulate_realistic_pattern(&self, allocations: &mut Vec<(usize, usize)>) -> f64 {
        let mut allocated = 0;
        let mut free_memory = self.total_memory;
        let mut largest_free = self.total_memory;

        // Mix of small, medium, and large allocations (simulating real workloads)
        let sizes = &[64, 128, 256, 512, 1024, 2048, 4096, 8192];

        for i in 0..100 {
            // More small allocations than large ones
            let size_idx = if i % 10 < 6 {
                0 // 60% small allocations
            } else if i % 10 < 9 {
                4 // 30% medium allocations
            } else {
                7 // 10% large allocations
            };

            let size = sizes[size_idx];

            if allocated + size < self.total_memory {
                allocations.push((i, size));
                allocated += size;
                free_memory -= size;
                largest_free = largest_free.min(free_memory);
            }
        }

        // Free random allocations
        for j in (0..allocations.len()).step_by(3) {
            if let Some((_, size)) = allocations.get(j) {
                free_memory += size;
            }
        }

        self.calculate_fragmentation(free_memory, largest_free)
    }
}

/// Benchmark internal fragmentation (wasted space within allocated blocks)
pub struct InternalFragmentationBenchmark {
    allocation_size: usize,
    request_size: usize,
}

impl InternalFragmentationBenchmark {
    pub fn new(request_size: usize) -> Self {
        // Calculate actual allocation size (usually rounded up to power of 2 or page size)
        let allocation_size = (request_size + 63) & !63; // Round up to 64-byte boundary
        Self {
            allocation_size,
            request_size,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(100);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for InternalFragmentationBenchmark {
    fn name(&self) -> &str {
        "memory/internal_fragmentation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Internal fragmentation: (allocated - requested) / allocated
        let wasted = self.allocation_size - self.request_size;
        let frag_ratio = (wasted as f64 / self.allocation_size as f64) * 100.0;

        Ok(frag_ratio as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark memory compaction effectiveness
pub struct CompactionBenchmark {
    iterations: usize,
}

impl CompactionBenchmark {
    pub fn new() -> Self {
        Self {
            iterations: 100,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for CompactionBenchmark {
    fn name(&self) -> &str {
        "memory/compaction"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate fragmentation then compaction
        let mut allocated = 0;
        let total_memory = 1024 * 1024;

        // Create fragmentation
        let mut allocations = Vec::new();
        for i in 0..50 {
            let size = if i % 2 == 0 { 64 } else { 4096 };
            if allocated + size < total_memory {
                allocations.push(size);
                allocated += size;
            }
        }

        // Measure compaction time
        let start = core::time::Instant::now();

        // Simulate compaction (moving allocations to consolidate free space)
        let mut compacted = 0;
        for size in &allocations {
            // Simulate moving allocation
            for _ in 0..(*size / 64) {
                core::hint::spin_loop();
            }
            compacted += size;
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragmentation_benchmark() {
        let mut bench = FragmentationBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_allocation_patterns() {
        for pattern in [
            AllocationPattern::Random,
            AllocationPattern::Alternating,
            AllocationPattern::Wave,
            AllocationPattern::Realistic,
        ] {
            let mut bench = FragmentationBenchmark::new().with_pattern(pattern);
            let config = BenchmarkConfig {
                warmup_iterations: 5,
                measurement_iterations: 20,
                ..Default::default()
            };

            let result = bench.execute(&config);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_internal_fragmentation() {
        let mut bench = InternalFragmentationBenchmark::new(100);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_compaction_benchmark() {
        let mut bench = CompactionBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 20,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
