//! Integration tests for the benchmarking framework
//!
//! This file verifies that all benchmarks compile and can be instantiated.

#![cfg(test)]

use kernel::testing::benchmarks::*;

#[test]
fn test_main_module_compiles() {
    // Test that the main module exports work
    let config = BenchmarkConfig::default();
    assert_eq!(config.warmup_iterations, 100);
    assert_eq!(config.measurement_iterations, 1000);
}

#[test]
fn test_benchmark_result_creation() {
    let samples = vec
![100, 200, 300, 400, 500];
    let result = BenchmarkResult::from_samples("test".to_string(), samples);
    assert_eq!(result.name, "test");
    assert_eq!(result.iterations, 5);
    assert!(result.mean_ns > 0.0);
}

#[test]
fn test_time_formatting() {
    assert_eq!(BenchmarkResult::format_time(500.0), "500.00 ns");
    assert_eq!(BenchmarkResult::format_time(5_000.0), "5.00 μs");
    assert_eq!(BenchmarkResult::format_time(5_000_000.0), "5.00 ms");
    assert_eq!(BenchmarkResult::format_time(5_000_000_000.0), "5.00 s");
}

#[test]
fn test_throughput_formatting() {
    assert_eq!(BenchmarkResult::format_throughput(500.0), "500.00 ops/sec");
    assert_eq!(BenchmarkResult::format_throughput(5_000.0), "5.00 Kops/sec");
    assert_eq!(BenchmarkResult::format_throughput(5_000_000.0), "5.00 Mops/sec");
    assert_eq!(BenchmarkResult::format_throughput(5_000_000_000.0), "5.00 Gops/sec");
}

// Scheduler benchmarks
#[test]
fn test_scheduler_context_switch_compiles() {
    let _bench = scheduler::context_switch::ContextSwitchBenchmark::new();
}

#[test]
fn test_scheduler_schedule_latency_compiles() {
    let _bench = scheduler::schedule_latency::ScheduleLatencyBenchmark::new();
}

#[test]
fn test_scheduler_throughput_compiles() {
    let _bench = scheduler::throughput::ThroughputBenchmark::new();
}

// Memory benchmarks
#[test]
fn test_memory_page_alloc_compiles() {
    let _bench = memory::page_alloc::PageAllocationBenchmark::new();
}

#[test]
fn test_memory_slab_alloc_compiles() {
    let _bench = memory::slab_alloc::SlabAllocationBenchmark::new(64);
}

#[test]
fn test_memory_fragmentation_compiles() {
    let _bench = memory::fragmentation::FragmentationBenchmark::new();
}

#[test]
fn test_memory_numa_locality_compiles() {
    let _bench = memory::numa_locality::NumaLocalityBenchmark::new();
}

// IPC benchmarks
#[test]
fn test_ipc_pipe_compiles() {
    let _bench = ipc::pipe::PipeBenchmark::new();
}

#[test]
fn test_ipc_shared_memory_compiles() {
    let _bench = ipc::shared_memory::SharedMemoryBenchmark::new();
}

#[test]
fn test_ipc_message_queue_compiles() {
    let _bench = ipc::message_queue::MessageQueueBenchmark::new();
}

// Filesystem benchmarks
#[test]
fn test_filesystem_ext4_read_compiles() {
    let _bench = filesystem::ext4_read::Ext4ReadBenchmark::new();
}

#[test]
fn test_filesystem_ext4_write_compiles() {
    let _bench = filesystem::ext4_write::Ext4WriteBenchmark::new();
}

#[test]
fn test_filesystem_metadata_compiles() {
    let _bench = filesystem::metadata::MetadataBenchmark::new(
        filesystem::metadata::MetadataOp::Stat
    );
}

// Network benchmarks
#[test]
fn test_network_tcp_compiles() {
    let _bench = network::tcp::TcpBenchmark::new();
}

#[test]
fn test_network_udp_compiles() {
    let _bench = network::udp::UdpBenchmark::new();
}

#[test]
fn test_network_connection_compiles() {
    let _bench = network::connection::ConnectionBenchmark::new();
}

// Test threshold checking
#[test]
fn test_threshold_checking() {
    let samples = vec
![1000; 100];
    let result = BenchmarkResult::from_samples("test".to_string(), samples);

    // Should pass 2000ns threshold
    assert!(result.meets_threshold(2000));

    // Should fail 500ns threshold
    assert!(!result.meets_threshold(500));
}

// Test regression detection
#[test]
fn test_regression_detection() {
    let samples = vec
![1050; 100];
    let result = BenchmarkResult::from_samples("test".to_string(), samples);

    // 5% regression from 1000 baseline
    let regression = result.regression_from_baseline(1000);
    assert!((regression - 5.0).abs() < 0.1);

    // Should detect regression at 5% threshold
    assert!(regression >= 5.0);
}

// Test statistics calculation
#[test]
fn test_statistics_calculation() {
    let samples = vec
![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
    let result = BenchmarkResult::from_samples("stats_test".to_string(), samples);

    // Verify basic statistics
    assert_eq!(result.min_ns, 100);
    assert_eq!(result.max_ns, 1000);
    assert_eq!(result.median_ns, 550.0); // Average of middle two
    assert!((result.mean_ns - 550.0).abs() < 0.01);
}

// Test report generation
#[test]
fn test_report_generation() {
    let samples = vec
![1000; 100];
    let result = BenchmarkResult::from_samples("report_test".to_string(), samples);
    let report = result.report();

    assert!(report.contains("Benchmark: report_test"));
    assert!(report.contains("Mean:"));
    assert!(report.contains("Median:"));
    assert!(report.contains("P95:"));
    assert!(report.contains("P99:"));
}
