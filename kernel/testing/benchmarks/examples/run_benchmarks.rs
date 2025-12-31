//! Example: Running the Kernel Benchmark Suite
//!
//! This example demonstrates how to run and use the benchmarking framework.

#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use kernel::testing::benchmarks::{
    run_all_benchmarks, BenchmarkConfig, BenchmarkSuite, BenchmarkResult,
};

/// Example 1: Run all benchmarks with default configuration
fn example_run_all() {
    println!("Running all kernel benchmarks...\n");

    match run_all_benchmarks() {
        Ok(summary) => {
            print!("{}", summary.report());

            // Check for failures
            if summary.failed > 0 {
                eprintln!("\n⚠️  Warning: {} benchmarks failed to meet thresholds", summary.failed);
            }

            // Check for regressions
            if !summary.regressions.is_empty() {
                eprintln!("\n⚠️  Warning: {} potential regressions detected:", summary.regressions.len());
                for regression in &summary.regressions {
                    eprintln!("  - {}", regression);
                }
            }

            println!("\n✅ Benchmark suite completed successfully");
        }
        Err(e) => {
            eprintln!("❌ Benchmark suite failed: {}", e);
        }
    }
}

/// Example 2: Run specific benchmark categories
fn example_run_specific() {
    use kernel::testing::benchmarks::{memory, scheduler};

    let config = BenchmarkConfig {
        warmup_iterations: 50,
        measurement_iterations: 500,
        ..Default::default()
    };

    // Run only scheduler benchmarks
    println!("Running scheduler benchmarks...\n");
    match scheduler::run_all(&config) {
        Ok(results) => {
            for result in results {
                print!("{}", result.report());
            }
        }
        Err(e) => {
            eprintln!("Scheduler benchmarks failed: {}", e);
        }
    }

    // Run only memory benchmarks
    println!("\nRunning memory benchmarks...\n");
    match memory::run_all(&config) {
        Ok(results) => {
            for result in results {
                print!("{}", result.report());

                // Check against thresholds
                if result.name.contains("page_allocation") {
                    let target_throughput = 1_000_000.0; // 1M pages/sec
                    if result.throughput_ops_per_sec < target_throughput {
                        eprintln!("⚠️  Warning: Below target throughput of {} ops/sec",
                                  target_throughput);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Memory benchmarks failed: {}", e);
        }
    }
}

/// Example 3: Custom benchmark configuration
fn example_custom_config() {
    use core::time::Duration;

    // Strict configuration for performance testing
    let strict_config = BenchmarkConfig {
        warmup_iterations: 1000,
        measurement_iterations: 10000,
        max_duration: Duration::from_secs(30),
        check_regressions: true,
        regression_threshold: 3.0, // More strict: 3% threshold
    };

    // Quick configuration for development
    let quick_config = BenchmarkConfig {
        warmup_iterations: 10,
        measurement_iterations: 100,
        max_duration: Duration::from_secs(5),
        check_regressions: false,
        regression_threshold: 10.0,
    };

    println!("Running quick benchmarks for development...\n");
    // Use quick_config for rapid iteration during development
}

/// Example 4: Creating a custom benchmark
struct CustomBenchmark {
    iterations: usize,
}

impl BenchmarkSuite for CustomBenchmark {
    fn name(&self) -> &str {
        "custom/my_operation"
    }

    fn setup(&mut self) -> Result<(), String> {
        // Initialize resources
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Measure your operation here
        let start = core::time::Instant::now();

        // ... perform operation ...

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        // Cleanup resources
        Ok(())
    }
}

fn example_custom_benchmark() {
    let mut bench = CustomBenchmark { iterations: 1000 };
    let config = BenchmarkConfig::default();

    match bench.execute(&config) {
        Ok(result) => {
            print!("{}", result.report());

            // Check if it meets your performance target
            let target_ns = 1000; // 1μs target
            if result.p95_ns > target_ns as f64 {
                eprintln!("⚠️  Warning: P95 latency ({:.2} ns) exceeds target ({} ns)",
                          result.p95_ns, target_ns);
            }
        }
        Err(e) => {
            eprintln!("Custom benchmark failed: {}", e);
        }
    }
}

/// Example 5: Comparing results
fn example_comparison() {
    use kernel::testing::benchmarks::memory::run_page_allocation_benchmark;

    let config = BenchmarkConfig::default();

    // Run baseline
    println!("Running baseline...\n");
    let baseline = match run_page_allocation_benchmark(&config) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Baseline failed: {}", e);
            return;
        }
    };

    // ... make changes to kernel ...

    // Run after changes
    println!("\nRunning after changes...\n");
    let current = match run_page_allocation_benchmark(&config) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Current run failed: {}", e);
            return;
        }
    };

    // Compare results
    let regression = current.regression_from_baseline(baseline.p95_ns as u64);
    println!("\nRegression analysis:");
    println!("  Baseline P95: {:.2} ns", baseline.p95_ns);
    println!("  Current P95:  {:.2} ns", current.p95_ns);
    println!("  Difference:  {:+.2}%", regression);

    if regression > 5.0 {
        eprintln!("⚠️  Warning: Performance regression detected!");
    } else if regression < -5.0 {
        println!("✅ Performance improved!");
    } else {
        println!("✓ Performance stable");
    }
}

/// Example 6: Generating reports in different formats
fn example_reports(result: &BenchmarkResult) {
    // Human-readable report
    print!("{}", result.report());

    // Compact format
    println!("\nCompact: {} | mean: {:.2} ns | p95: {:.2} ns | throughput: {:.2} Mops/sec",
             result.name,
             result.mean_ns,
             result.p95_ns,
             result.throughput_ops_per_sec / 1_000_000.0);

    // JSON-like format (manual construction since no std)
    let json = format!(
        "{{\"name\": \"{}\", \"mean_ns\": {:.2}, \"p95_ns\": {:.2}, \"throughput\": {:.2}}}",
        result.name,
        result.mean_ns,
        result.p95_ns,
        result.throughput_ops_per_sec
    );
    println!("\nJSON: {}", json);
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║        Kernel Performance Benchmarking Suite               ║");
    println!("╚═══════════════════════════════════════════════════════════╝\n");

    // Run examples
    example_run_all();

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
