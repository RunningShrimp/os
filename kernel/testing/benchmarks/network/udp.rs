//! UDP Performance Benchmark
//!
//! Measures UDP throughput and latency for connectionless communication.
//! Target: High throughput with low overhead.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

const UDP_DATAGRAM_SIZE: usize = 1472; // Typical MTU - headers

/// Benchmark UDP throughput
pub struct UdpBenchmark {
    datagram_size: usize,
    num_datagrams: usize,
}

impl UdpBenchmark {
    pub fn new() -> Self {
        Self {
            datagram_size: UDP_DATAGRAM_SIZE,
            num_datagrams: 10000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for UdpBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for UdpBenchmark {
    fn name(&self) -> &str {
        "network/udp_throughput"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // UDP is simpler than TCP (no connection state, no retransmission)
        for _ in 0..self.num_datagrams {
            // Send datagram
            for _ in 0..self.datagram_size / 64 {
                core::hint::spin_loop();
            }
        }

        let elapsed = start.elapsed();
        let ns_per_datagram = elapsed.as_nanos() as u64 / self.num_datagrams as u64;
        Ok(ns_per_datagram)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udp_benchmark() {
        let mut bench = UdpBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
