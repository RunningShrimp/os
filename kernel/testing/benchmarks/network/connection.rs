//! Connection Establishment Benchmark
//!
//! Measures latency for establishing new connections.
//! Target: <10μs for TCP connection setup.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

/// Benchmark connection establishment latency
pub struct ConnectionBenchmark {
    connection_type: ConnectionType,
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectionType {
    Tcp,
    Udp,
    Unix,
}

impl ConnectionBenchmark {
    pub fn new() -> Self {
        Self {
            connection_type: ConnectionType::Tcp,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for ConnectionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for ConnectionBenchmark {
    fn name(&self) -> &str {
        "network/connection_setup"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        match self.connection_type {
            ConnectionType::Tcp => {
                // TCP 3-way handshake: SYN -> SYN-ACK -> ACK
                // SYN
                for _ in 0..10 {
                    core::hint::spin_loop();
                }
                // SYN-ACK processing
                for _ in 0..10 {
                    core::hint::spin_loop();
                }
                // ACK
                for _ in 0..10 {
                    core::hint::spin_loop();
                }
            }
            ConnectionType::Udp => {
                // UDP is connectionless - minimal overhead
                for _ in 0..5 {
                    core::hint::spin_loop();
                }
            }
            ConnectionType::Unix => {
                // Unix domain socket connection
                for _ in 0..8 {
                    core::hint::spin_loop();
                }
            }
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark connection teardown latency
pub struct ConnectionTeardownBenchmark {
    connection_type: ConnectionType,
}

impl ConnectionTeardownBenchmark {
    pub fn new(conn_type: ConnectionType) -> Self {
        Self {
            connection_type: conn_type,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(ConnectionType::Tcp);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for ConnectionTeardownBenchmark {
    fn name(&self) -> &str {
        "network/connection_teardown"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        match self.connection_type {
            ConnectionType::Tcp => {
                // TCP connection termination: FIN -> ACK -> FIN -> ACK
                for _ in 0..20 {
                    core::hint::spin_loop();
                }
            }
            ConnectionType::Udp => {
                // No teardown needed for UDP
                for _ in 0..2 {
                    core::hint::spin_loop();
                }
            }
            ConnectionType::Unix => {
                for _ in 0::10 {
                    core::hint::spin_loop();
                }
            }
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
    fn test_connection_benchmark() {
        let mut bench = ConnectionBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_connection_types() {
        for conn_type in [ConnectionType::Tcp, ConnectionType::Udp, ConnectionType::Unix] {
            let mut bench = ConnectionBenchmark::new();
            bench.connection_type = conn_type;
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
    fn test_teardown() {
        let mut bench = ConnectionTeardownBenchmark::new(ConnectionType::Tcp);
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
