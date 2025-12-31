//! IPC Performance Benchmarks
//!
//! This module contains benchmarks for various IPC mechanisms including
//! pipes, shared memory, and message queues.

pub mod pipe;
pub mod shared_memory;
pub mod message_queue;

use crate::testing::benchmarks::{BenchmarkConfig, BenchmarkResult};
use alloc::vec::Vec;

/// Run all IPC benchmarks
pub fn run_all(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>, String> {
    let mut results = Vec::new();

    // Pipe benchmark
    match pipe::PipeBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Pipe benchmark failed: {}", e),
    }

    // Shared memory benchmark
    match shared_memory::SharedMemoryBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Shared memory benchmark failed: {}", e),
    }

    // Message queue benchmark
    match message_queue::MessageQueueBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Message queue benchmark failed: {}", e),
    }

    Ok(results)
}

/// Run pipe benchmark (convenience function)
pub fn run_pipe_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    pipe::PipeBenchmark::run(config)
}

/// Run shared memory benchmark (convenience function)
pub fn run_shared_memory_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    shared_memory::SharedMemoryBenchmark::run(config)
}

/// Run message queue benchmark (convenience function)
pub fn run_message_queue_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    message_queue::MessageQueueBenchmark::run(config)
}
