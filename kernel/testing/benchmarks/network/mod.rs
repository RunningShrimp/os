//! Network Performance Benchmarks
//!
//! This module contains benchmarks for network operations including
//! TCP/UDP throughput and connection establishment.

pub mod tcp;
pub mod udp;
pub mod connection;

use crate::testing::benchmarks::{BenchmarkConfig, BenchmarkResult};
use alloc::vec::Vec;

/// Run all network benchmarks
pub fn run_all(config: &BenchmarkConfig) -> Result<Vec<BenchmarkResult>, String> {
    let mut results = Vec::new();

    // TCP benchmark
    match tcp::TcpBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("TCP benchmark failed: {}", e),
    }

    // UDP benchmark
    match udp::UdpBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("UDP benchmark failed: {}", e),
    }

    // Connection benchmark
    match connection::ConnectionBenchmark::run(config) {
        Ok(result) => results.push(result),
        Err(e) => log::error!("Connection benchmark failed: {}", e),
    }

    Ok(results)
}

/// Run TCP benchmark (convenience function)
pub fn run_tcp_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    tcp::TcpBenchmark::run(config)
}

/// Run UDP benchmark (convenience function)
pub fn run_udp_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    udp::UdpBenchmark::run(config)
}

/// Run connection benchmark (convenience function)
pub fn run_connection_benchmark(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
    connection::ConnectionBenchmark::run(config)
}
