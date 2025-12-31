# Kernel Performance Benchmarking Suite

A comprehensive, production-ready benchmarking framework for the kernel with statistical analysis, regression detection, and detailed reporting.

## Overview

This benchmarking suite provides:

- **Comprehensive Coverage**: Scheduler, memory, IPC, filesystem, and network benchmarks
- **Statistical Analysis**: Mean, median, P95, P99, standard deviation
- **Regression Detection**: Automatic detection of performance degradations >5%
- **Human-Readable Reports**: Clear, actionable performance reports
- **Production-Ready**: Proper error handling and accurate timing

## Directory Structure

```
benchmarks/
├── mod.rs                    # Main entry point
├── metrics.yaml              # Performance thresholds
├── README.md                 # This file
├── scheduler/
│   ├── mod.rs
│   ├── context_switch.rs     # Context switch latency
│   ├── schedule_latency.rs   # Scheduling latency
│   └── throughput.rs         # Scheduler throughput
├── memory/
│   ├── mod.rs
│   ├── page_alloc.rs         # Page allocation
│   ├── slab_alloc.rs         # Slab allocator
│   ├── fragmentation.rs      # Memory fragmentation
│   └── numa_locality.rs      # NUMA performance
├── ipc/
│   ├── mod.rs
│   ├── pipe.rs               # Pipe IPC
│   ├── shared_memory.rs      # Shared memory
│   └── message_queue.rs      # Message queues
├── filesystem/
│   ├── mod.rs
│   ├── ext4_read.rs          # Read performance
│   ├── ext4_write.rs         # Write performance
│   └── metadata.rs           # Metadata operations
└── network/
    ├── mod.rs
    ├── tcp.rs                # TCP performance
    ├── udp.rs                # UDP performance
    └── connection.rs         # Connection establishment
```

## Usage

### Running All Benchmarks

```rust
use kernel::testing::benchmarks::run_all_benchmarks;

fn main() {
    match run_all_benchmarks() {
        Ok(summary) => {
            print!("{}", summary.report());
            if summary.failed > 0 {
                eprintln!("WARNING: {} benchmarks failed thresholds", summary.failed);
            }
            if !summary.regressions.is_empty() {
                eprintln!("WARNING: {} regressions detected", summary.regressions.len());
            }
        }
        Err(e) => {
            eprintln!("Benchmark execution failed: {}", e);
        }
    }
}
```

### Running Specific Benchmark Suites

```rust
use kernel::testing::benchmarks::{BenchmarkConfig, memory};

fn main() {
    let config = BenchmarkConfig::default();

    // Run memory benchmarks
    match memory::run_all(&config) {
        Ok(results) => {
            for result in results {
                print!("{}", result.report());
            }
        }
        Err(e) => {
            eprintln!("Memory benchmarks failed: {}", e);
        }
    }
}
```

### Custom Benchmark Configuration

```rust
use kernel::testing::benchmarks::BenchmarkConfig;
use core::time::Duration;

let config = BenchmarkConfig {
    warmup_iterations: 500,
    measurement_iterations: 5000,
    max_duration: Duration::from_secs(30),
    check_regressions: true,
    regression_threshold: 10.0, // 10% threshold
};
```

## Benchmark Targets

### Scheduler
- **Context Switch Latency**: <5μs
- **Schedule Latency**: <10μs
- **Throughput**: >100,000 switches/sec

### Memory
- **Page Allocation**: >1M pages/sec, <100ns latency
- **Slab Allocation**: <50ns latency
- **Fragmentation**: <20%
- **NUMA Locality**: >90% local accesses

### IPC
- **Pipe Throughput**: >1GB/sec
- **Pipe Latency**: <1.5μs
- **Shared Memory**: >5GB/sec, <500ns latency
- **Message Queue**: >500K messages/sec

### Filesystem
- **Ext4 Read**: >500MB/sec sequential
- **Ext4 Write**: >300MB/sec sequential
- **Metadata Operations**: >50K ops/sec

### Network
- **TCP Throughput**: >5GB/sec
- **TCP Latency**: <2μs
- **UDP Throughput**: >6GB/sec
- **Connection Setup**: <10μs

## Understanding Results

### Sample Output

```
## Benchmark: memory/page_allocation
  Iterations: 1000
  Total Time: 150.32 μs
  Throughput: 6.65 Mops/sec

  Latency Statistics:
    Mean:   150.32 ns
    Median: 148.00 ns
    Min:    120.00 ns
    Max:    320.00 ns
    P95:    180.00 ns
    P99:    240.00 ns
    StdDev: 25.43 ns
```

### Metrics Explained

- **Mean**: Average execution time
- **Median**: Middle value (50th percentile)
- **P95/P99**: 95th/99th percentiles (important for tail latency)
- **StdDev**: Standard deviation (measure of consistency)
- **Throughput**: Operations per second

## Regression Detection

The suite automatically detects regressions by comparing against:

1. **Thresholds**: Defined in `metrics.yaml`
2. **Baseline**: Previous benchmark run (if available)
3. **Trend Analysis**: Statistical significance testing

A regression is flagged when:
- Performance degrades by >5% (configurable)
- With 95% confidence interval
- Across sufficient sample size

## Creating Custom Benchmarks

```rust
use kernel::testing::benchmarks::{BenchmarkSuite, BenchmarkConfig, BenchmarkResult};

struct MyCustomBenchmark {
    iterations: usize,
}

impl BenchmarkSuite for MyCustomBenchmark {
    fn name(&self) -> &str {
        "custom/my_benchmark"
    }

    fn setup(&mut self) -> Result<(), String> {
        // Setup code
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Benchmark code - return nanoseconds
        let start = core::time::Instant::now();

        // ... perform operation ...

        Ok(start.elapsed().as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        // Cleanup code
        Ok(())
    }
}
```

## Best Practices

1. **Warmup**: Always run warmup iterations to stabilize caches
2. **Multiple Runs**: Collect at least 1000 samples for statistical validity
3. **Stable Environment**: Ensure consistent system load during benchmarks
4. **Frequency**: Run benchmarks regularly (e.g., per commit, daily)
5. **Trend Analysis**: Track results over time to catch gradual degradation

## Configuration

Edit `metrics.yaml` to customize:

- Performance thresholds
- Regression detection sensitivity
- Benchmark execution parameters
- Reporting format

## Integration with CI/CD

```yaml
# .github/workflows/benchmarks.yml
name: Performance Benchmarks

on: [push, pull_request]

jobs:
  benchmark:
    runs-on: [self-hosted, kernel-test]
    steps:
      - uses: actions/checkout@v2
      - name: Run Benchmarks
        run: cargo test --release --bench '*'
      - name: Check Regressions
        run: |
          if benchmark-summary | grep -q "REGRESSION"; then
            exit 1
          fi
```

## License

Licensed under the same terms as the kernel project.
