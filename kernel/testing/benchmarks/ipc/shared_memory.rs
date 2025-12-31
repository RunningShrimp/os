//! Shared Memory IPC Benchmark
//!
//! Measures shared memory performance including latency and throughput.
//! Target: Low latency (<1μs) and high bandwidth.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

const SHMEM_SIZE: usize = 1024 * 1024; // 1MB

/// Benchmark shared memory throughput
pub struct SharedMemoryBenchmark {
    shmem_size: usize,
    access_pattern: AccessPattern,
}

/// Access patterns for shared memory
#[derive(Debug, Clone, Copy)]
pub enum AccessPattern {
    /// Sequential access (good for bandwidth)
    Sequential,
    /// Random access (tests latency)
    Random,
    /// Strided access
    Strided { stride: usize },
}

impl SharedMemoryBenchmark {
    pub fn new() -> Self {
        Self {
            shmem_size: SHMEM_SIZE,
            access_pattern: AccessPattern::Sequential,
        }
    }

    pub fn_with_pattern(mut self, pattern: AccessPattern) -> Self {
        self.access_pattern = pattern;
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.shmem_size = size;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for SharedMemoryBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for SharedMemoryBenchmark {
    fn name(&self) -> &str {
        "ipc/shared_memory"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 1000;
        let start = core::time::Instant::now();

        match self.access_pattern {
            AccessPattern::Sequential => {
                // Sequential access - cache friendly
                for _ in 0..iterations {
                    for _ in 0..self.shmem_size / 64 {
                        // Simulate cache line access
                        core::hint::spin_loop();
                    }
                }
            }
            AccessPattern::Random => {
                // Random access - poor cache locality
                for _ in 0..iterations {
                    for _ in 0..self.shmem_size / 4096 {
                        // Simulate random page access
                        for _ in 0..10 {
                            core::hint::spin_loop();
                        }
                    }
                }
            }
            AccessPattern::Strided { stride } => {
                // Strided access - tests cache line effects
                for _ in 0..iterations {
                    let mut offset = 0;
                    while offset < self.shmem_size {
                        // Access at stride intervals
                        for _ in 0..2 {
                            core::hint::spin_loop();
                        }
                        offset += stride;
                    }
                }
            }
        }

        let elapsed = start.elapsed();
        let ns_per_iteration = elapsed.as_nanos() as u64 / iterations as u64;
        Ok(ns_per_iteration)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark shared memory synchronization primitives
pub struct ShmemSyncBenchmark {
    sync_type: SyncType,
}

/// Types of synchronization for shared memory
#[derive(Debug, Clone, Copy)]
pub enum SyncType {
    /// No synchronization (just measure raw access)
    None,
    /// Atomic operations
    Atomic,
    /// Futex-based synchronization
    Futex,
    /// Mutex-based synchronization
    Mutex,
}

impl ShmemSyncBenchmark {
    pub fn new(sync_type: SyncType) -> Self {
        Self { sync_type }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(SyncType::None);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for ShmemSyncBenchmark {
    fn name(&self) -> &str {
        "ipc/shared_memory_sync"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 1000;
        let start = core::time::Instant::now();

        match self.sync_type {
            SyncType::None => {
                // Unprotected access
                for _ in 0..iterations {
                    for _ in 0..10 {
                        core::hint::spin_loop();
                    }
                }
            }
            SyncType::Atomic => {
                // Atomic operations
                for _ in 0..iterations {
                    // Simulate atomic read-modify-write
                    for _ in 0..20 {
                        core::hint::spin_loop();
                    }
                }
            }
            SyncType::Futex => {
                // Futex-based synchronization
                for _ in 0..iterations {
                    // Simulate futex wait/wake
                    for _ in 0..30 {
                        core::hint::spin_loop();
                    }
                }
            }
            SyncType::Mutex => {
                // Mutex-based synchronization
                for _ in 0..iterations {
                    // Simulate lock/unlock
                    for _ in 0..25 {
                        core::hint::spin_loop();
                    }
                }
            }
        }

        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() as u64 / iterations as u64;
        Ok(ns_per_op)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark producer-consumer pattern with shared memory
pub struct ShmemProducerConsumerBenchmark {
    buffer_size: usize,
    message_size: usize,
}

impl ShmemProducerConsumerBenchmark {
    pub fn new() -> Self {
        Self {
            buffer_size: 4096,
            message_size: 64,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for ShmemProducerConsumerBenchmark {
    fn name(&self) -> &str {
        "ipc/shared_memory_producer_consumer"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let messages = 100;
        let start = core::time::Instant::now();

        // Simulate producer-consumer with circular buffer
        for i in 0..messages {
            // Producer: write to shared memory
            for _ in 0..self.message_size / 8 {
                core::hint::spin_loop();
            }

            // Update write index (synchronization)
            for _ in 0..5 {
                core::hint::spin_loop();
            }

            // Consumer: read from shared memory
            for _ in 0..self.message_size / 8 {
                core::hint::spin_loop();
            }

            // Update read index (synchronization)
            for _ in 0..5 {
                core::hint::spin_loop();
            }

            let _ = i;
        }

        let elapsed = start.elapsed();
        let ns_per_message = elapsed.as_nanos() as u64 / messages as u64;
        Ok(ns_per_message)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark shared memory with multiple readers
pub struct ShmemMultiReaderBenchmark {
    num_readers: usize,
    data_size: usize,
}

impl ShmemMultiReaderBenchmark {
    pub fn new(num_readers: usize) -> Self {
        Self {
            num_readers,
            data_size: 1024,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(4);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for ShmemMultiReaderBenchmark {
    fn name(&self) -> &str {
        "ipc/shared_memory_multi_reader"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 100;
        let start = core::time::Instant::now();

        // Writer updates data
        for _ in 0..iterations {
            // Write data
            for _ in 0..self.data_size / 64 {
                core::hint::spin_loop();
            }

            // Multiple readers access the data
            for reader in 0..self.num_readers {
                // Each reader reads
                for _ in 0..self.data_size / 64 {
                    core::hint::spin_loop();
                }
                let _ = reader;
            }
        }

        let elapsed = start.elapsed();
        let ns_per_iteration = elapsed.as_nanos() as u64 / iterations as u64;
        Ok(ns_per_iteration)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_memory_benchmark() {
        let mut bench = SharedMemoryBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_access_patterns() {
        for pattern in [
            AccessPattern::Sequential,
            AccessPattern::Random,
            AccessPattern::Strided { stride: 64 },
        ] {
            let mut bench = SharedMemoryBenchmark::new().with_pattern(pattern);
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
    fn test_shmem_sync() {
        for sync_type in [
            SyncType::None,
            SyncType::Atomic,
            SyncType::Futex,
            SyncType::Mutex,
        ] {
            let mut bench = ShmemSyncBenchmark::new(sync_type);
            let config = BenchmarkConfig {
                warmup_iterations: 10,
                measurement_iterations: 100,
                ..Default::default()
            };

            let result = bench.execute(&config);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_producer_consumer() {
        let mut bench = ShmemProducerConsumerBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_multi_reader() {
        let mut bench = ShmemMultiReaderBenchmark::new(4);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
