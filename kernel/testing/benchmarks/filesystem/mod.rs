//! Filesystem Performance Benchmarks
//!
//! This module contains benchmarks for various filesystem operations including
//! read/write throughput and metadata operations.

pub mod ext4_read;
pub mod ext4_write;
pub mod metadata;

use crate::testing::benchmarks::{BenchmarkConfig, BenchmarkResult};
use alloc::vec::Vec;

/// Run all filesystem benchmarks
pub fn run_all(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>, String> {
    let mut results = Vec::new();

    // Ext4 read benchmark
    match ext4_read::Ext4ReadBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Ext4 read benchmark failed: {}", e),
    }

    // Ext4 write benchmark
    match ext4_write::Ext4WriteBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Ext4 write benchmark failed: {}", e),
    }

    // Metadata benchmark
    match metadata::MetadataBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Metadata benchmark failed: {}", e),
    }

    Ok(results)
}

/// Run ext4 read benchmark (convenience function)
pub fn run_ext4_read_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    ext4_read::Ext4ReadBenchmark::run(config)
}

/// Run ext4 write benchmark (convenience function)
pub fn run_ext4_write_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    ext4_write::Ext4WriteBenchmark::run(config)
}

/// Run metadata benchmark (convenience function)
pub fn run_metadata_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    metadata::MetadataBenchmark::run(config)
}
