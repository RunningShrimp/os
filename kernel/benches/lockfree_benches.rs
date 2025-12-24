//! Benchmarks for lock-free data structures

extern crate alloc;
use alloc::vec::Vec;

use crate::benches::{Benchmark, BenchmarkRunner, BenchmarkConfig, Timer};

/// Benchmark for LockFreeHashMap insertion
pub struct HashMapInsertBenchmark {
    iterations: u64,
}

impl HashMapInsertBenchmark {
    pub fn new(iterations: u64) -> Self {
        Self { iterations }
    }
}

impl Benchmark for HashMapInsertBenchmark {
    fn name(&self) -> &str {
        "hashmap_insert"
    }

    fn setup(&mut self) {
        crate::subsystems::sync::lockfree_hashmap::LockFreeHashMap::new();
    }

    fn teardown(&mut self) {}

    fn run(&mut self) -> core::time::Duration {
        use crate::subsystems::sync::lockfree_hashmap::LockFreeHashMap;
        let map = LockFreeHashMap::new();

        let _timer = Timer::new();
        for i in 0..self.iterations {
            map.insert(i, i * 2).ok();
        }
        _timer.elapsed()
    }
}

/// Benchmark for LockFreeHashMap lookup
pub struct HashMapLookupBenchmark {
    iterations: u64,
}

impl HashMapLookupBenchmark {
    pub fn new(iterations: u64) -> Self {
        Self { iterations }
    }
}

impl Benchmark for HashMapLookupBenchmark {
    fn name(&self) -> &str {
        "hashmap_lookup"
    }

    fn setup(&mut self) {
        use crate::subsystems::sync::lockfree_hashmap::LockFreeHashMap;
        let map = LockFreeHashMap::new();
        for i in 0..self.iterations {
            map.insert(i, i * 2).ok();
        }
    }

    fn teardown(&mut self) {}

    fn run(&mut self) -> core::time::Duration {
        use crate::subsystems::sync::lockfree_hashmap::LockFreeHashMap;
        let map = LockFreeHashMap::new();

        for i in 0..self.iterations {
            map.insert(i, i * 2).ok();
        }

        let _timer = Timer::new();
        for i in 0..self.iterations {
            let _ = map.get(&i);
        }
        _timer.elapsed()
    }
}

/// Benchmark for WorkStealingDeque push/pop
pub struct WorkStealingDequeBenchmark {
    iterations: u64,
}

impl WorkStealingDequeBenchmark {
    pub fn new(iterations: u64) -> Self {
        Self { iterations }
    }
}

impl Benchmark for WorkStealingDequeBenchmark {
    fn name(&self) -> &str {
        "work_stealing_deque"
    }

    fn setup(&mut self) {}

    fn teardown(&mut self) {}

    fn run(&mut self) -> core::time::Duration {
        use crate::subsystems::sync::work_stealing_queue::WorkStealingDeque;
        let deque = WorkStealingDeque::new();

        let _timer = Timer::new();
        for i in 0..self.iterations {
            deque.push_bottom(i);
        }
        for _ in 0..self.iterations {
            let _ = deque.pop_bottom();
        }
        _timer.elapsed()
    }
}

/// Run all lock-free data structure benchmarks
pub fn run_lockfree_benchmarks() -> Vec<crate::benches::BenchmarkResult> {
    let runner = BenchmarkRunner::with_config(BenchmarkConfig {
        warmup_iterations: 10,
        measurement_iterations: 100,
        sample_size: 1000,
    });

    let mut benchmarks: Vec<&mut dyn Benchmark> = Vec::new();

    let mut insert_bench = HashMapInsertBenchmark::new(1000);
    benchmarks.push(&mut insert_bench);

    let mut lookup_bench = HashMapLookupBenchmark::new(1000);
    benchmarks.push(&mut lookup_bench);

    let mut deque_bench = WorkStealingDequeBenchmark::new(1000);
    benchmarks.push(&mut deque_bench);

    runner.run_all(&mut benchmarks)
}
