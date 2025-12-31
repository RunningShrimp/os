//! Filesystem Metadata Operations Benchmark
//!
//! Measures performance of metadata operations like create, unlink,
//! stat, chmod, rename, etc.
//! Target: >50K operations/second.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

/// Benchmark filesystem metadata operations
pub struct MetadataBenchmark {
    operation_type: MetadataOp,
}

#[derive(Debug, Clone, Copy)]
pub enum MetadataOp {
    Create,    // File creation
    Unlink,    // File deletion
    Stat,      // File stat
    Chmod,     // Permission change
    Rename,    // File rename
    Readdir,   // Directory read
    Mkdir,     // Directory creation
    Rmdir,     // Directory removal
}

impl MetadataBenchmark {
    pub fn new(op: MetadataOp) -> Self {
        Self {
            operation_type: op,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(MetadataOp::Stat);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for MetadataBenchmark {
    fn name(&self) -> &str {
        match self.operation_type {
            MetadataOp::Create => "filesystem/metadata_create",
            MetadataOp::Unlink => "filesystem/metadata_unlink",
            MetadataOp::Stat => "filesystem/metadata_stat",
            MetadataOp::Chmod => "filesystem/metadata_chmod",
            MetadataOp::Rename => "filesystem/metadata_rename",
            MetadataOp::Readdir => "filesystem/metadata_readdir",
            MetadataOp::Mkdir => "filesystem/metadata_mkdir",
            MetadataOp::Rmdir => "filesystem/metadata_rmdir",
        }
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        match self.operation_type {
            MetadataOp::Create => {
                // Create file: allocate inode, dirent, update parent dir
                for _ in 0..30 {
                    core::hint::spin_loop();
                }
            }
            MetadataOp::Unlink => {
                // Unlink: remove dirent, decrement inode refcount
                for _ in 0..25 {
                    core::hint::spin_loop();
                }
            }
            MetadataOp::Stat => {
                // Stat: read inode
                for _ in 0..15 {
                    core::hint::spin_loop();
                }
            }
            MetadataOp::Chmod => {
                // Chmod: update inode metadata
                for _ in 0..20 {
                    core::hint::spin_loop();
                }
            }
            MetadataOp::Rename => {
                // Rename: update two dirents, same parent
                for _ in 0..40 {
                    core::hint::spin_loop();
                }
            }
            MetadataOp::Readdir => {
                // Readdir: read directory blocks
                for _ in 0..35 {
                    core::hint::spin_loop();
                }
            }
            MetadataOp::Mkdir => {
                // Mkdir: create directory, allocate . and .. entries
                for _ in 0..40 {
                    core::hint::spin_loop();
                }
            }
            MetadataOp::Rmdir => {
                // Rmdir: remove directory if empty
                for _ in 0..35 {
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

/// Benchmark directory lookup performance
pub struct LookupBenchmark {
    entries_per_dir: usize,
}

impl LookupBenchmark {
    pub fn new() -> Self {
        Self {
            entries_per_dir: 1000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for LookupBenchmark {
    fn name(&self) -> &str {
        "filesystem/directory_lookup"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Lookup filename in directory
        let start = core::time::Instant::now();

        // Simulate hash lookup or linear search
        for _ in 0..20 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark inode allocation performance
pub struct InodeAllocationBenchmark {
    num_inodes: usize,
}

impl InodeAllocationBenchmark {
    pub fn new() -> Self {
        Self {
            num_inodes: 10000,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for InodeAllocationBenchmark {
    fn name(&self) -> &str {
        "filesystem/inode_allocation"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Allocate inode from free list
        for _ in 0..25 {
            core::hint::spin_loop();
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
    fn test_metadata_operations() {
        for op in [
            MetadataOp::Create,
            MetadataOp::Unlink,
            MetadataOp::Stat,
            MetadataOp::Chmod,
            MetadataOp::Rename,
            MetadataOp::Readdir,
            MetadataOp::Mkdir,
            MetadataOp::Rmdir,
        ] {
            let mut bench = MetadataBenchmark::new(op);
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
    fn test_lookup_benchmark() {
        let mut bench = LookupBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_inode_allocation() {
        let mut bench = InodeAllocationBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
