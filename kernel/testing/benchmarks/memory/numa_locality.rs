//! NUMA Locality Benchmark
//!
//! Measures memory access patterns and performance in NUMA systems.
//! Target: >90% of accesses should be to local memory.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

/// Number of NUMA nodes to simulate
const NUM_NUMA_NODES: usize = 4;

/// Benchmark NUMA locality
pub struct NumaLocalityBenchmark {
    num_nodes: usize,
    access_pattern: AccessPattern,
}

/// Different memory access patterns
#[derive(Debug, Clone, Copy)]
pub enum AccessPattern {
    /// All accesses to local memory
    Local,
    /// All accesses to remote memory
    Remote,
    /// Mixed local and remote
    Mixed { local_percent: f64 },
    /// Realistic workload with locality
    Realistic,
}

impl NumaLocalityBenchmark {
    pub fn new() -> Self {
        Self {
            num_nodes: NUM_NUMA_NODES,
            access_pattern: AccessPattern::Local,
        }
    }

    pub fn with_pattern(mut self, pattern: AccessPattern) -> Self {
        self.access_pattern = pattern;
        self
    }

    pub fn with_nodes(mut self, nodes: usize) -> Self {
        self.num_nodes = nodes;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for NumaLocalityBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for NumaLocalityBenchmark {
    fn name(&self) -> &str {
        "memory/numa_locality"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let access_count = 1000;
        let (local_percent, elapsed_ns) = measure_time(|| {
            match self.access_pattern {
                AccessPattern::Local => self.simulate_local_accesses(access_count),
                AccessPattern::Remote => self.simulate_remote_accesses(access_count),
                AccessPattern::Mixed { local_percent } => {
                    self.simulate_mixed_accesses(access_count, local_percent)
                }
                AccessPattern::Realistic => self.simulate_realistic_accesses(access_count),
            }
        });

        // Return latency per access
        let ns_per_access = elapsed_ns / access_count as u64;
        Ok(ns_per_access)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

impl NumaLocalityBenchmark {
    fn simulate_local_accesses(&self, count: usize) -> f64 {
        // Local memory access is fast
        for _ in 0..count {
            // Simulate local access latency (~60ns on real systems)
            for _ in 0..3 {
                core::hint::spin_loop();
            }
        }
        100.0 // 100% local
    }

    fn simulate_remote_accesses(&self, count: usize) -> f64 {
        // Remote memory access is slower
        for _ in 0..count {
            // Simulate remote access latency (~120-200ns on real systems)
            for _ in 0..8 {
                core::hint::spin_loop();
            }
        }
        0.0 // 0% local
    }

    fn simulate_mixed_accesses(&self, count: usize, local_percent: f64) -> f64 {
        let local_count = (count as f64 * local_percent / 100.0) as usize;

        for i in 0..count {
            if i < local_count {
                // Local access
                for _ in 0..3 {
                    core::hint::spin_loop();
                }
            } else {
                // Remote access
                for _ in 0..8 {
                    core::hint::spin_loop();
                }
            }
        }

        local_percent
    }

    fn simulate_realistic_accesses(&self, count: usize) -> f64 {
        // Realistic: most accesses are local (~90%), some remote
        let local_count = (count as f64 * 0.9) as usize;

        for i in 0..count {
            if i < local_count {
                // Local access
                for _ in 0..3 {
                    core::hint::spin_loop();
                }
            } else {
                // Remote access
                for _ in 0..8 {
                    core::hint::spin_loop();
                }
            }
        }

        90.0
    }
}

/// Benchmark cross-node memory allocation
pub struct CrossNodeAllocationBenchmark {
    source_node: usize,
    target_node: usize,
    size: usize,
}

impl CrossNodeAllocationBenchmark {
    pub fn new(source: usize, target: usize) -> Self {
        Self {
            source_node: source,
            target_node: target,
            size: 4096, // Page size
        }
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(0, 1);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for CrossNodeAllocationBenchmark {
    fn name(&self) -> &str {
        "memory/cross_node_allocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Simulate cross-node allocation
        // This is more expensive than local allocation
        for _ in 0..15 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark memory bandwidth per NUMA node
pub struct NumaBandwidthBenchmark {
    node_id: usize,
    buffer_size: usize,
}

impl NumaBandwidthBenchmark {
    pub fn new(node: usize) -> Self {
        Self {
            node_id: node,
            buffer_size: 1024 * 1024, // 1MB
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(0);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for NumaBandwidthBenchmark {
    fn name(&self) -> &str {
        "memory/numa_bandwidth"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Simulate sequential memory access (good bandwidth)
        let iterations = self.buffer_size / 64;
        for _ in 0..iterations {
            // Simulate cache line access
            for _ in 0..2 {
                core::hint::spin_loop();
            }
        }

        let elapsed = start.elapsed();

        // Return nanoseconds per MB
        let ns_per_mb = elapsed.as_nanos() as u64;
        Ok(ns_per_mb)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark page migration between NUMA nodes
pub struct PageMigrationBenchmark {
    source_node: usize,
    target_node: usize,
    num_pages: usize,
}

impl PageMigrationBenchmark {
    pub fn new(source: usize, target: usize) -> Self {
        Self {
            source_node: source,
            target_node: target,
            num_pages: 100,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(0, 1);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for PageMigrationBenchmark {
    fn name(&self) -> &str {
        "memory/page_migration"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Simulate page migration overhead
        for _ in 0..self.num_pages {
            // Page migration involves:
            // 1. Copy page data
            // 2. Update page tables
            // 3. Invalidate TLB entries
            for _ in 0..50 {
                core::hint::spin_loop();
            }
        }

        let elapsed = start.elapsed();
        let ns_per_page = elapsed.as_nanos() as u64 / self.num_pages as u64;
        Ok(ns_per_page)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark interleaved memory allocation
pub struct InterleavedAllocationBenchmark {
    num_nodes: usize,
    num_allocations: usize,
}

impl InterleavedAllocationBenchmark {
    pub fn new(nodes: usize) -> Self {
        Self {
            num_nodes: nodes,
            num_allocations: 1000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(4);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for InterleavedAllocationBenchmark {
    fn name(&self) -> &str {
        "memory/interleaved_allocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Allocate pages interleaved across nodes
        for i in 0..self.num_allocations {
            let node = i % self.num_nodes;

            // Simulate allocation on specific node
            for _ in 0..5 {
                core::hint::spin_loop();
            }

            // Access pattern depends on node
            let _ = node;
        }

        let elapsed = start.elapsed();
        let ns_per_alloc = elapsed.as_nanos() as u64 / self.num_allocations as u64;
        Ok(ns_per_alloc)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numa_locality_benchmark() {
        let mut bench = NumaLocalityBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_access_patterns() {
        for pattern in [
            AccessPattern::Local,
            AccessPattern::Remote,
            AccessPattern::Mixed { local_percent: 70.0 },
            AccessPattern::Realistic,
        ] {
            let mut bench = NumaLocalityBenchmark::new().with_pattern(pattern);
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
    fn test_cross_node_allocation() {
        let mut bench = CrossNodeAllocationBenchmark::new(0, 1);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_numa_bandwidth() {
        let mut bench = NumaBandwidthBenchmark::new(0);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_page_migration() {
        let mut bench = PageMigrationBenchmark::new(0, 1);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 20,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
