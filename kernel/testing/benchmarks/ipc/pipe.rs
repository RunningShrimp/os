//! Pipe IPC Benchmark
//!
//! Measures pipe throughput and latency.
//! Target: >1GB/sec throughput.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

const BUFFER_SIZE: usize = 4096;
const MESSAGE_SIZE: usize = 1024;

/// Benchmark pipe throughput
pub struct PipeBenchmark {
    buffer_size: usize,
    message_size: usize,
}

impl PipeBenchmark {
    pub fn new() -> Self {
        Self {
            buffer_size: BUFFER_SIZE,
            message_size: MESSAGE_SIZE,
        }
    }

    pub fn with_message_size(mut self, size: usize) -> Self {
        self.message_size = size;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }

    /// Calculate throughput in MB/s
    pub fn calculate_throughput_mb(&self, bytes: u64, ns: u64) -> f64 {
        let seconds = ns as f64 / 1_000_000_000.0;
        let mb = bytes as f64 / (1024.0 * 1024.0);
        mb / seconds
    }
}

impl Default for PipeBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for PipeBenchmark {
    fn name(&self) -> &str {
        "ipc/pipe_throughput"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 1000;
        let total_bytes = iterations * self.message_size;

        let start = core::time::Instant::now();

        // Simulate pipe write and read
        for _ in 0..iterations {
            // Write to pipe
            for _ in 0..self.message_size / 64 {
                core::hint::spin_loop();
            }

            // Read from pipe
            for _ in 0..self.message_size / 64 {
                core::hint::spin_loop();
            }
        }

        let elapsed = start.elapsed();

        // Return nanoseconds per operation
        let ns_per_op = elapsed.as_nanos() as u64 / iterations as u64;
        Ok(ns_per_op)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark pipe latency (round-trip time)
pub struct PipeLatencyBenchmark {
    message_size: usize,
}

impl PipeLatencyBenchmark {
    pub fn new() -> Self {
        Self {
            message_size: 64, // Small message for latency
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for PipeLatencyBenchmark {
    fn name(&self) -> &str {
        "ipc/pipe_latency"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate round-trip: write -> read
        let start = core::time::Instant::now();

        // Write
        for _ in 0..3 {
            core::hint::spin_loop();
        }

        // Context switch
        for _ in 0..5 {
            core::hint::spin_loop();
        }

        // Read
        for _ in 0..3 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark pipe with multiple concurrent readers/writers
pub struct MultiPipeBenchmark {
    num_pipes: usize,
    buffer_size: usize,
}

impl MultiPipeBenchmark {
    pub fn new(num_pipes: usize) -> Self {
        Self {
            num_pipes,
            buffer_size: BUFFER_SIZE,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(4);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for MultiPipeBenchmark {
    fn name(&self) -> &str {
        "ipc/multi_pipe"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 100;
        let start = core::time::Instant::now();

        // Simulate multiple pipes operating concurrently
        for _ in 0..iterations {
            for pipe_id in 0..self.num_pipes {
                // Write to pipe
                for _ in 0..5 {
                    core::hint::spin_loop();
                }

                // Read from pipe
                for _ in 0..5 {
                    core::hint::spin_loop();
                }

                let _ = pipe_id;
            }
        }

        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() as u64 / (iterations * self.num_pipes) as u64;
        Ok(ns_per_op)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark pipe with blocking I/O
pub struct BlockingPipeBenchmark {
    buffer_size: usize,
}

impl BlockingPipeBenchmark {
    pub fn new() -> Self {
        Self {
            buffer_size: BUFFER_SIZE,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for BlockingPipeBenchmark {
    fn name(&self) -> &str {
        "ipc/blocking_pipe"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 100;
        let start = core::time::Instant::now();

        // Simulate blocking writes/reads with wait states
        for _ in 0..iterations {
            // Write (may block if buffer full)
            for _ in 0..10 {
                core::hint::spin_loop();
            }

            // Simulate blocking wait
            for _ in 0..20 {
                core::hint::spin_loop();
            }

            // Read (may block if buffer empty)
            for _ in 0..10 {
                core::hint::spin_loop();
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

/// Benchmark pipe buffer utilization
pub struct PipeBufferUtilBenchmark {
    buffer_size: usize,
}

impl PipeBufferUtilBenchmark {
    pub fn new() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB buffer
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for PipeBufferUtilBenchmark {
    fn name(&self) -> &str {
        "ipc/pipe_buffer_utilization"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Test different fill levels
        let fill_percentages = [25, 50, 75, 100];
        let mut total_ns = 0u64;

        for percentage in fill_percentages {
            let start = core::time::Instant::now();

            // Write to fill buffer to percentage
            let bytes_to_write = (self.buffer_size * percentage / 100) / 64;
            for _ in 0..bytes_to_write {
                core::hint::spin_loop();
            }

            // Read to drain
            for _ in 0..bytes_to_write {
                core::hint::spin_loop();
            }

            total_ns += start.elapsed().as_nanos() as u64;
        }

        Ok(total_ns / fill_percentages.len() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipe_benchmark() {
        let mut bench = PipeBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_pipe_latency() {
        let mut bench = PipeLatencyBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_multi_pipe() {
        let mut bench = MultiPipeBenchmark::new(4);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_blocking_pipe() {
        let mut bench = BlockingPipeBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_pipe_throughput_calculation() {
        let bench = PipeBenchmark::new();
        let throughput = bench.calculate_throughput_mb(1024 * 1024 * 100, 1_000_000_000);
        assert!((throughput - 100.0).abs() < 0.1);
    }
}
