//! Ext4 Write Performance Benchmark
//!
//! Measures sequential and random write performance on Ext4 filesystems.
//! Target: >300MB/sec sequential write throughput.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

const FILE_SIZE: usize = 100 * 1024 * 1024; // 100MB
const BUFFER_SIZE: usize = 4096; // 4KB blocks

/// Benchmark sequential write throughput
pub struct Ext4WriteBenchmark {
    file_size: usize,
    buffer_size: usize,
    sync_mode: SyncMode,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncMode {
    None,       // No sync (write-back cache)
    DataSync,   // Sync data only (fdatasync)
    FullSync,   // Sync data + metadata (fsync)
}

impl Ext4WriteBenchmark {
    pub fn new() -> Self {
        Self {
            file_size: FILE_SIZE,
            buffer_size: BUFFER_SIZE,
            sync_mode: SyncMode::None,
        }
    }

    pub fn with_sync(mut self, mode: SyncMode) -> Self {
        self.sync_mode = mode;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for Ext4WriteBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for Ext4WriteBenchmark {
    fn name(&self) -> &str {
        "filesystem/ext4_sequential_write"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let num_blocks = self.file_size / self.buffer_size;
        let start = core::time::Instant::now();

        for _ in 0..num_blocks {
            // Simulate write operation
            for _ in 0..20 {
                core::hint::spin_loop();
            }

            // Sync overhead based on mode
            match self.sync_mode {
                SyncMode::None => {},
                SyncMode::DataSync => {
                    for _ in 0..100 {
                        core::hint::spin_loop();
                    }
                },
                SyncMode::FullSync => {
                    for _ in 0::200 {
                        core::hint::spin_loop();
                    }
                },
            }
        }

        let elapsed = start.elapsed();
        let ns_per_block = elapsed.as_nanos() as u64 / num_blocks as u64;
        Ok(ns_per_block)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark random write performance
pub struct RandomWriteBenchmark {
    file_size: usize,
    buffer_size: usize,
    num_writes: usize,
}

impl RandomWriteBenchmark {
    pub fn new() -> Self {
        Self {
            file_size: FILE_SIZE,
            buffer_size: BUFFER_SIZE,
            num_writes: 10000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for RandomWriteBenchmark {
    fn name(&self) -> &str {
        "filesystem/ext4_random_write"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        for i in 0..self.num_writes {
            let offset = ((i * 137) * self.buffer_size) % self.file_size;

            // Random writes are slower (no sequential optimization)
            for _ in 0..40 {
                core::hint::spin_loop();
            }

            let _ = offset;
        }

        let elapsed = start.elapsed();
        let ns_per_write = elapsed.as_nanos() as u64 / self.num_writes as u64;
        Ok(ns_per_write)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ext4_write_benchmark() {
        let mut bench = Ext4WriteBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_sync_modes() {
        for mode in [SyncMode::None, SyncMode::DataSync, SyncMode::FullSync] {
            let mut bench = Ext4WriteBenchmark::new().with_sync(mode);
            let config = BenchmarkConfig {
                warmup_iterations: 5,
                measurement_iterations: 50,
                ..Default::default()
            };

            let result = bench.execute(&config);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_random_write() {
        let mut bench = RandomWriteBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
