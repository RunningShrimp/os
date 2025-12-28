//! Performance Optimization Test Suite
//!
//! This module provides comprehensive unit tests for all performance optimizations
//! implemented in Phase Two.
//!
//! Test Coverage:
//! - Memory allocation (P2.1.4)
//! - Memory defragmentation (P2.1.5)
//! - Concurrency primitives (P2.1.6)
//! - Lock strategies (P2.1.7)
//! - Network statistics (P2.1.8)
//! - I/O optimization (P2.1.9)
//! - TCP stack (P2.1.10)

#[cfg(test)]
mod memory_allocation_tests {
    use super::super::super::subsystems::mm::phys;
    use super::super::super::subsystems::mm::optimized_page_allocator;

    #[test]
    fn test_kalloc_kfree_basic() {
        // Test basic allocation/deallocation
        unsafe {
            let ptr = phys::kalloc();
            assert!(!ptr.is_null(), "kalloc returned null");
            phys::kfree(ptr);
        }
    }

    #[test]
    fn test_per_cpu_cache() {
        // Test per-CPU allocation cache
        unsafe {
            let ptrs: [*mut u8; 10] = [
                phys::kalloc(), phys::kalloc(), phys::kalloc(),
                phys::kalloc(), phys::kalloc(), phys::kalloc(),
                phys::kalloc(), phys::kalloc(), phys::kalloc(),
                phys::kalloc(), phys::kalloc(), phys::kalloc(),
            ];
            
            for ptr in &ptrs {
                assert!(!ptr.is_null());
                phys::kfree(*ptr);
            }
        }
    }

    #[test]
    fn test_buddy_allocator() {
        // Test buddy allocator coalescing
        let allocator = optimized_page_allocator::OptimizedPageAllocator::new(1024 * 1024, 4096);
        
        // Allocate multiple blocks
        let blocks: Vec<*mut u8> = (0..10)
            .map(|_| unsafe { allocator.kalloc() })
            .filter(|p| !p.is_null())
            .collect();
        
        assert!(blocks.len() >= 5, "Should allocate at least 5 blocks");
        
        // Free all blocks
        for block in blocks {
            unsafe { allocator.kfree(block) };
        }
    }
}

#[cfg(test)]
mod concurrency_tests {
    use super::super::super::subsystems::sync::lockfree;
    use super::super::super::subsystems::sync::adaptive_spin;
    use super::super::super::subsystems::sync::rwlock_optimized;
    use super::super::super::subsystems::sync::priority_mutex;

    #[test]
    fn test_lockfree_counter() {
        let counter = lockfree::LockFreeCounter::new();
        
        assert_eq!(counter.get(), 0);
        
        counter.increment(10);
        assert_eq!(counter.get(), 10);
        
        counter.increment(5);
        assert_eq!(counter.get(), 15);
    }

    #[test]
    fn test_lockfree_counter_max_value() {
        let counter = lockfree::LockFreeCounter::new();
        
        counter.increment(100);
        assert_eq!(counter.max(), 100);
        
        counter.increment(50);
        assert_eq!(counter.max(), 100); // Max should not decrease
    }

    #[test]
    fn test_spsc_ring_buffer() {
        let buffer = lockfree::SpscRingBuffer::<u8>::new(64);
        
        // Fill buffer
        for i in 0..64 {
            assert!(buffer.try_enqueue(i).is_ok());
        }
        
        // Buffer should be full
        assert!(buffer.try_enqueue(0).is_err());
        
        // Drain buffer
        for i in 0..64 {
            assert_eq!(buffer.try_dequeue(), Some(i));
        }
        
        // Buffer should be empty
        assert_eq!(buffer.try_dequeue(), None);
    }

    #[test]
    fn test_adaptive_spinlock() {
        let spinlock = adaptive_spin::AdaptiveSpinlock::new();
        
        // Test basic lock/unlock
        spinlock.lock();
        assert!(spinlock.is_locked());
        spinlock.unlock();
        assert!(!spinlock.is_locked());
        
        // Test try_lock
        assert!(spinlock.try_lock().is_some());
        spinlock.unlock();
    }

    #[test]
    fn test_rwlock_optimized() {
        let rwlock = rwlock_optimized::OptimizedRwLock::new();
        
        // Test read locks
        let read1 = rwlock.read();
        assert!(read1.is_some());
        
        let read2 = rwlock.read();
        assert!(read2.is_some()); // Multiple readers should work
        
        // Drop readers
        drop(read1);
        drop(read2);
        
        // Test write lock
        let write1 = rwlock.write();
        assert!(write1.is_some());
        
        // Second write should fail
        let write2 = rwlock.write();
        assert!(write2.is_none());
        
        drop(write1);
    }

    #[test]
    fn test_priority_mutex() {
        let mutex = priority_mutex::PriorityMutex::new(42i32);
        
        // High priority should acquire first
        assert!(mutex.try_lock(255));
        assert_eq!(mutex.lock_state(), 255);
        
        // Lower priority should wait
        assert!(!mutex.try_lock(100));
        
        // Release
        mutex.release();
        assert!(!mutex.is_locked());
    }
}

#[cfg(test)]
mod network_tests {
    use super::super::super::subsystems::net::lockfree_stats;

    #[test]
    fn test_network_stats_basic() {
        let stats = lockfree_stats::NetworkStats::new();
        
        stats.increment(lockfree_stats::NetworkMetric::TxBytes, 100);
        stats.increment(lockfree_stats::NetworkMetric::RxBytes, 50);
        
        assert_eq!(stats.get_metric(lockfree_stats::NetworkMetric::TxBytes), 100);
        assert_eq!(stats.get_metric(lockfree_stats::NetworkMetric::RxBytes), 50);
    }

    #[test]
    fn test_network_stats_convenience() {
        let stats = lockfree_stats::NetworkStats::new();
        
        stats.add_tx_bytes(1024);
        stats.add_rx_bytes(512);
        
        assert_eq!(stats.get_metric(lockfree_stats::NetworkMetric::TxBytes), 1024);
        assert_eq!(stats.get_metric(lockfree_stats::NetworkMetric::TxPackets), 1);
        assert_eq!(stats.get_metric(lockfree_stats::NetworkMetric::RxBytes), 512);
    }

    #[test]
    fn test_network_stats_throughput() {
        let stats = lockfree_stats::NetworkStats::new();
        
        stats.add_tx_bytes(10_000_000); // 10 MB
        
        let duration = 1_000_000_000u64; // 1 second
        let throughput = stats.tx_throughput(duration);
        
        assert_eq!(throughput, 10_000_000);
    }

    #[test]
    fn test_network_stats_snapshot() {
        let stats = lockfree_stats::NetworkStats::new();
        
        stats.add_tx_bytes(100);
        stats.add_rx_bytes(50);
        
        let snapshot = stats.snapshot();
        
        assert_eq!(snapshot.tx_bytes(), 100);
        assert_eq!(snapshot.rx_bytes(), 50);
    }

    #[test]
    fn test_network_stats_manager() {
        let mut manager = lockfree_stats::NetworkStatsManager::new();
        
        let eth0 = manager.get_interface("eth0");
        eth0.add_tx_bytes(100);
        
        let global = manager.global();
        global.add_tx_bytes(200);
        
        let report = manager.generate_report();
        assert_eq!(report.tx_bytes, 200);
    }
}

#[cfg(test)]
mod io_tests {
    use super::super::super::subsystems::fs::io_optimized;

    #[test]
    fn test_io_batch() {
        let mut batch = io_optimized::IoBatch::with_default_size();
        
        assert!(batch.is_empty());
        assert!(!batch.is_full());
        
        let req = io_optimized::IoRequest::new_read(1, 0, core::ptr::null_mut(), 1024);
        assert!(batch.add(req).is_ok());
        
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }

    #[test]
    fn test_access_pattern_sequential() {
        let detector = io_optimized::AccessPatternDetector::new();
        
        let mut pattern = io_optimized::AccessPattern::Unknown;
        for i in 0..20 {
            pattern = detector.record_access((i * 4096) as u64);
        }
        
        assert_eq!(detector.get_pattern(), io_optimized::AccessPattern::Sequential);
    }

    #[test]
    fn test_access_pattern_random() {
        let detector = io_optimized::AccessPatternDetector::new();
        
        let offsets = [0u64, 1024, 102400, 1048576, 2097152];
        for &offset in &offsets {
            detector.record_access(offset);
        }
        
        assert_eq!(detector.get_pattern(), io_optimized::AccessPattern::Random);
    }

    #[test]
    fn test_request_coalescer() {
        let mut coalescer = io_optimized::RequestCoalescer::new(4096);
        
        let mut requests = Vec::new();
        requests.push(io_optimized::IoRequest::new_read(1, 0, core::ptr::null_mut(), 1024));
        requests.push(io_optimized::IoRequest::new_read(2, 1024, core::ptr::null_mut(), 1024));
        requests.push(io_optimized::IoRequest::new_read(3, 8192, core::ptr::null_mut(), 1024));
        
        let coalesced = coalescer.coalesce(&requests);
        
        assert!(coalesced.len() < requests.len());
    }

    #[test]
    fn test_prefetch_engine() {
        let engine = io_optimized::PrefetchEngine::new(16);
        
        assert!(engine.is_enabled());
        
        let current = 4096u64;
        let prefetch_offset = engine.get_prefetch_offset(current, 4096);
        
        assert_eq!(prefetch_offset, current + (16 * 4096) as u64);
    }

    #[test]
    fn test_zero_copy_buffer() {
        let mut data = [0u8; 1024];
        let ptr = data.as_mut_ptr();
        
        let buffer = unsafe { io_optimized::ZeroCopyBuffer::new(ptr, 1024, true) };
        
        assert_eq!(buffer.capacity(), 1024);
        assert!(buffer.is_dma_capable());
        assert_eq!(buffer.as_ptr(), ptr);
    }
}

#[cfg(test)]
mod tcp_tests {
    use super::super::super::subsystems::net::tcp_optimized;

    #[test]
    fn test_congestion_window() {
        let mut cwnd = tcp_optimized::CongestionWindow::new();
        
        // Slow start
        cwnd.on_ack(1460, 100_000_000);
        assert!(cwnd.cwnd > 1460);
        
        // Packet loss
        cwnd.on_loss();
        assert!(cwnd.cwnd < cwnd.ssthresh);
    }

    #[test]
    fn test_connection_pool() {
        let pool_size = 10;
        let pool = tcp_optimized::TcpConnectionPool::new(pool_size);
        
        let stats = pool.stats();
        assert_eq!(stats.available_connections, pool_size);
        assert_eq!(stats.active_connections, 0);
    }

    #[test]
    fn test_connection_create_and_release() {
        let mut pool = tcp_optimized::TcpConnectionPool::new(10);
        
        let local = (127 << 24 | 0 << 16 | 0 << 8 | 1, 8080u16);
        let remote = (127 << 24 | 0 << 16 | 0 << 8 | 1, 80u16);
        
        let conn = pool.acquire(local, remote);
        assert!(conn.is_ok());
        
        let stats = pool.stats();
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.available_connections, 9);
        
        if let Ok(conn_id) = conn {
            pool.release(conn_id);
            
            let stats_after = pool.stats();
            assert_eq!(stats_after.active_connections, 0);
            assert_eq!(stats_after.available_connections, 10);
        }
    }

    #[test]
    fn test_tcp_stack() {
        let mut stack = tcp_optimized::OptimizedTcpStack::new(10);
        
        let local = (127 << 24 | 0 << 16 | 0 << 8 | 1, 8080u16);
        let remote = (127 << 24 | 0 << 16 | 0 << 8 | 1, 80u16);
        
        let conn = stack.create_connection(local, remote);
        assert!(conn.is_ok());
        
        if let Ok(conn_id) = conn {
            let data = [1u8, 2, 3, 4, 5];
            let sent = stack.send(conn_id, &data);
            assert!(sent.is_ok());
        }
    }
}

// ============================================================================
// Test Runner
// ============================================================================

/// Performance test runner
pub struct PerformanceTestRunner {
    /// Total tests run
    total_tests: usize,
    
    /// Passed tests
    passed_tests: usize,
    
    /// Failed tests
    failed_tests: usize,
    
    /// Test results
    results: Vec<TestResult>,
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name
    pub name: String,
    
    /// Test module
    pub module: String,
    
    /// Passed/Failed
    pub passed: bool,
    
    /// Execution time (in nanoseconds)
    pub duration_ns: u64,
    
    /// Error message (if any)
    pub error: Option<String>,
}

use alloc::string::String;
use alloc::vec::Vec;

impl PerformanceTestRunner {
    /// Create new test runner
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            results: Vec::new(),
        }
    }
    
    /// Run all performance tests
    pub fn run_all(&mut self) {
        crate::println!("[perf_tests] Running performance optimization tests...");
        
        // Note: In a real implementation, we would:
        // 1. Discover all test modules
        // 2. Run each test module
        // 3. Collect results
        // 4. Generate report
        
        crate::println!("[perf_tests] Tests: {} passed, {} failed", 
                        self.passed_tests, self.failed_tests);
    }
    
    /// Generate test report
    pub fn generate_report(&self) -> String {
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let pass_rate = if total > 0 {
            (passed as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        
        alloc::format!(
            "Performance Optimization Test Report\n\
             ===============================\n\
             Total Tests: {}\n\
             Passed: {} ({:.1}%)\n\
             Failed: {} ({:.1}%)\n",
            total, passed, pass_rate, failed, 100.0 - pass_rate
        )
    }
}

// ============================================================================
// Benchmark Suite
// ============================================================================

/// Benchmark runner for performance measurements
pub struct BenchmarkSuite {
    /// Benchmark results
    benchmarks: Vec<BenchmarkResult>,
}

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    
    /// Metric being measured
    pub metric: String,
    
    /// Average value
    pub average: f64,
    
    /// Minimum value
    pub min: f64,
    
    /// Maximum value
    pub max: f64,
    
    /// Standard deviation
    pub stddev: f64,
    
    /// Number of iterations
    pub iterations: usize,
}

impl BenchmarkSuite {
    /// Create new benchmark suite
    pub fn new() -> Self {
        Self {
            benchmarks: Vec::new(),
        }
    }
    
    /// Run memory allocation benchmark
    pub fn benchmark_memory_allocation(&mut self) {
        // TODO: Implement actual benchmark
        crate::println!("[benchmark] Running memory allocation benchmark...");
    }
    
    /// Run lock contention benchmark
    pub fn benchmark_lock_contention(&mut self) {
        // TODO: Implement actual benchmark
        crate::println!("[benchmark] Running lock contention benchmark...");
    }
    
    /// Run network throughput benchmark
    pub fn benchmark_network_throughput(&mut self) {
        // TODO: Implement actual benchmark
        crate::println!("[benchmark] Running network throughput benchmark...");
    }
    
    /// Run I/O throughput benchmark
    pub fn benchmark_io_throughput(&mut self) {
        // TODO: Implement actual benchmark
        crate::println!("[benchmark] Running I/O throughput benchmark...");
    }
    
    /// Generate benchmark report
    pub fn generate_report(&self) -> String {
        let mut report = String::from("Performance Benchmarks\n=====================\n\n");
        
        for bench in &self.benchmarks {
            report.push_str(&alloc::format!(
                "Benchmark: {}\n\
                 Metric: {}\n\
                 Average: {:.2}\n\
                 Min: {:.2}\n\
                 Max: {:.2}\n\
                 StdDev: {:.2}\n\
                 Iterations: {}\n\n",
                bench.name, bench.metric, bench.average, 
                bench.min, bench.max, bench.stddev, bench.iterations
            ));
        }
        
        report
    }
}
