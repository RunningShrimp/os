//! Scheduler Performance Benchmarks
//!
//! This module contains comprehensive benchmarks for the kernel scheduler,
//! measuring context switch latency, scheduling latency, and throughput.

pub mod context_switch;
pub mod schedule_latency;
pub mod throughput;

use crate::testing::benchmarks::{BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

/// Run all scheduler benchmarks
pub fn run_all(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>, String> {
    let mut results = Vec::new();

    // Context switch benchmark
    match context_switch::ContextSwitchBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Context switch benchmark failed: {}", e),
    }

    // Schedule latency benchmark
    match schedule_latency::ScheduleLatencyBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Schedule latency benchmark failed: {}", e),
    }

    // Throughput benchmark
    match throughput::ThroughputBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Throughput benchmark failed: {}", e),
    }

    Ok(results)
}

/// Run context switch benchmark (convenience function)
pub fn run_context_switch_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    context_switch::ContextSwitchBenchmark::run(config)
}

/// Run schedule latency benchmark (convenience function)
pub fn run_schedule_latency_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    schedule_latency::ScheduleLatencyBenchmark::run(config)
}

/// Run throughput benchmark (convenience function)
pub fn run_throughput_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    throughput::ThroughputBenchmark::run(config)
}
