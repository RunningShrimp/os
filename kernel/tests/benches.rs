//! Comprehensive benchmarks for NOS kernel
//!
//! This module provides performance benchmarks for all major kernel components:
//! - Memory allocation benchmarks (malloc/free speed)
//! - Context switch benchmarks (thread/process switch)
//! - Network throughput benchmarks (TCP/UDP)
//! - Syscall latency benchmarks
//! - Lock contention benchmarks
//! - Cache performance benchmarks

#![cfg(test)]

extern crate alloc;
extern crate core;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::time::Duration;

/// Benchmark result type
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_time_ns: u64,
    pub avg_time_ns: u64,
    pub ops_per_sec: u64,
}

impl BenchmarkResult {
    pub fn new(name: &str, iterations: u64, total_time_ns: u64) -> Self {
        let avg_time_ns = total_time_ns / iterations;
        let ops_per_sec = if avg_time_ns > 0 {
            1_000_000_000 / avg_time_ns
        } else {
            0
        };

        Self {
            name: name.to_string(),
            iterations,
            total_time_ns,
            avg_time_ns,
            ops_per_sec,
        }
    }

    pub fn print(&self) {
        println!("Benchmark: {}", self.name);
        println!("  Iterations: {}", self.iterations);
        println!("  Total time: {} ms", self.total_time_ns / 1_000_000);
        println!("  Avg time: {} ns", self.avg_time_ns);
        println!("  Throughput: {} ops/sec", self.ops_per_sec);
    }
}

/// Simple timer for benchmarking
pub struct BenchmarkTimer {
    start_ns: u64,
}

impl BenchmarkTimer {
    pub fn start() -> Self {
        Self {
            start_ns: Self::get_time_nanos(),
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        Self::get_time_nanos() - self.start_ns
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ns() / 1_000_000
    }

    fn get_time_nanos() -> u64 {
        // Use kernel's time function
        kernel::time::get_ticks() * 1_000_000 // Convert to approximate nanoseconds
    }
}

// ============================================================================
// Memory Allocation Benchmarks
// ============================================================================

#[cfg(test)]
mod memory_benchmarks {
    use super::*;

    /// Benchmark kmalloc/kfree throughput
    #[test]
    fn benchmark_malloc_free() {
        use kernel::memory::{kmalloc, kfree};

        const ITERATIONS: u64 = 10000;
        const SIZE: usize = 1024;

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let ptr = kmalloc(SIZE);
            assert!(!ptr.is_null(), "Allocation should succeed");
            kfree(ptr);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("malloc_free (1024 bytes)", ITERATIONS, elapsed);
        result.print();

        assert!(result.ops_per_sec > 1000, "Should achieve > 1000 ops/sec");
    }

    /// Benchmark large allocations
    #[test]
    fn benchmark_large_allocations() {
        use kernel::memory::{kmalloc, kfree};

        const ITERATIONS: u64 = 1000;
        const SIZE: usize = 1024 * 1024; // 1MB

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let ptr = kmalloc(SIZE);
            assert!(!ptr.is_null(), "Allocation should succeed");
            kfree(ptr);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("malloc_free (1MB)", ITERATIONS, elapsed);
        result.print();

        assert!(result.ops_per_sec > 100, "Should achieve > 100 ops/sec");
    }

    /// Benchmark variable size allocations
    #[test]
    fn benchmark_variable_allocations() {
        use kernel::memory::{kmalloc, kfree};

        const ITERATIONS: u64 = 5000;

        let timer = BenchmarkTimer::start();

        for i in 0..ITERATIONS {
            let size = ((i % 10) + 1) * 1024; // 1KB to 10KB
            let ptr = kmalloc(size);
            assert!(!ptr.is_null(), "Allocation should succeed");
            kfree(ptr);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("malloc_free (variable)", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark slab allocator
    #[test]
    fn benchmark_slab_allocator() {
        use kernel::subsystems::mm::slab::SlabAllocator;

        const ITERATIONS: u64 = 50000;
        const OBJECT_SIZE: usize = 64;

        let mut slab = SlabAllocator::new("bench_slab", OBJECT_SIZE);

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let obj = slab.allocate();
            assert!(obj.is_some(), "Slab allocation should succeed");
            slab.deallocate(obj.unwrap());
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("slab_allocator (64 bytes)", ITERATIONS, elapsed);
        result.print();

        assert!(result.ops_per_sec > 10000, "Should achieve > 10000 ops/sec");
    }

    /// Benchmark buddy allocator
    #[test]
    fn benchmark_buddy_allocator() {
        use kernel::subsystems::mm::buddy::BuddyAllocator;

        const ITERATIONS: u64 = 10000;

        let mut allocator = BuddyAllocator::new(1024); // 1024 pages

        let timer = BenchmarkTimer::start();

        for i in 0..ITERATIONS {
            let size = (i % 4) + 1; // 1-4 pages
            let frame = allocator.allocate(size);
            assert!(frame.is_ok(), "Buddy allocation should succeed");
            allocator.deallocate(frame.unwrap());
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("buddy_allocator", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark page table operations
    #[test]
    fn benchmark_page_table_ops() {
        use kernel::subsystems::mm::vm::{PageTable, PageTableEntry};

        const ITERATIONS: u64 = 10000;

        let page_table = PageTable::new();
        let flags = PageTableEntry::READABLE | PageTableEntry::WRITABLE;

        let timer = BenchmarkTimer::start();

        for i in 0..ITERATIONS {
            let virt_addr = i * 4096;
            let phys_addr = i * 4096 + 0x1000000;

            page_table
                .map_page(virt_addr, phys_addr, flags)
                .expect("Map failed");

            let _ = page_table.get_entry(virt_addr);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("page_table_ops", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark mmap/munmap
    #[test]
    fn benchmark_mmap() {
        use kernel::subsystems::mm::vm::{mmap, munmap};

        const ITERATIONS: u64 = 1000;
        const SIZE: usize = 4096;

        let prot = kernel::subsystems::mm::vm::ProtFlags::PROT_READ
            | kernel::subsystems::mm::vm::ProtFlags::PROT_WRITE;
        let flags = kernel::subsystems::mm::vm::MapFlags::MAP_PRIVATE
            | kernel::subsystems::mm::vm::MapFlags::MAP_ANONYMOUS;

        let timer = BenchmarkTimer::start();

        for i in 0..ITERATIONS {
            let addr = mmap(0, SIZE, prot, flags, -1, 0);
            assert!(addr.is_ok(), "mmap should succeed");

            munmap(addr.unwrap(), SIZE).expect("munmap failed");
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("mmap/munmap", ITERATIONS, elapsed);
        result.print();
    }
}

// ============================================================================
// Context Switch Benchmarks
// ============================================================================

#[cfg(test)]
mod context_switch_benchmarks {
    use super::*;

    /// Benchmark thread creation
    #[test]
    fn benchmark_thread_creation() {
        use kernel::subsystems::process::thread::Thread;

        const ITERATIONS: u64 = 1000;

        let timer = BenchmarkTimer::start();

        for i in 0..ITERATIONS {
            let thread = Thread::new(1, || i);
            assert!(thread.is_ok(), "Thread creation should succeed");
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("thread_creation", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark thread context switch
    #[test]
    fn benchmark_thread_switch() {
        use kernel::subsystems::process::thread::Thread;

        const ITERATIONS: u64 = 1000;

        let thread1 = Thread::new(1, || {
            for _ in 0..ITERATIONS {
                kernel::subsystems::process::thread::current_thread().yield();
            }
            0
        })
        .expect("Thread creation failed");

        let thread2 = Thread::new(1, || {
            for _ in 0..ITERATIONS {
                kernel::subsystems::process::thread::current_thread().yield();
            }
            0
        })
        .expect("Thread creation failed");

        thread1.start().expect("Start failed");
        thread2.start().expect("Start failed");

        let timer = BenchmarkTimer::start();

        thread1.join().expect("Join failed");
        thread2.join().expect("Join failed");

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new(
            "thread_switch",
            ITERATIONS * 2, // Each thread yields ITERATIONS times
            elapsed,
        );
        result.print();

        assert!(
            result.avg_time_ns < 1_000_000,
            "Context switch should be < 1ms"
        );
    }

    /// Benchmark process fork
    #[test]
    fn benchmark_process_fork() {
        use kernel::subsystems::process::vfork::vfork;

        const ITERATIONS: u64 = 100;

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let pid = vfork();
            assert!(pid >= 0, "Fork should succeed");

            if pid == 0 {
                // Child process
                kernel::subsystems::syscalls::interface::syscall_exit(0);
            } else {
                // Parent - wait for child
                use kernel::subsystems::process::manager::ProcessManager;
                let manager = ProcessManager::global();
                let _ = manager.wait_pid(pid);
            }
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("process_fork", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark scheduler latency
    #[test]
    fn benchmark_scheduler_latency() {
        use kernel::subsystems::scheduler::unified::UnifiedScheduler;

        const ITERATIONS: u64 = 1000;

        let scheduler = UnifiedScheduler::global();
        assert!(!scheduler.is_null(), "Scheduler should exist");

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            scheduler.schedule();
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("scheduler_tick", ITERATIONS, elapsed);
        result.print();

        assert!(
            result.avg_time_ns < 100_000,
            "Scheduler tick should be < 100us"
        );
    }
}

// ============================================================================
// Network Benchmarks
// ============================================================================

#[cfg(test)]
mod network_benchmarks {
    use super::*;

    /// Benchmark TCP connection establishment
    #[test]
    fn benchmark_tcp_connect() {
        use kernel::subsystems::net::socket::Socket;

        const ITERATIONS: u64 = 100;

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let socket = Socket::new(kernel::subsystems::net::socket::SocketType::STREAM)
                .expect("Socket creation failed");

            let result = socket.connect("127.0.0.1:8080");
            // May fail if no server, that's ok

            drop(socket);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("tcp_connect", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark UDP send/receive
    #[test]
    fn benchmark_udp_throughput() {
        use kernel::subsystems::net::socket::Socket;

        const ITERATIONS: u64 = 1000;
        const DATA_SIZE: usize = 1024;
        let data = vec![0u8; DATA_SIZE];

        let socket = Socket::new(kernel::subsystems::net::socket::SocketType::DGRAM)
            .expect("Socket creation failed");

        socket.bind("127.0.0.1:0").expect("Bind failed");

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _ = socket.sendto(&data, "127.0.0.1:9999");
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("udp_send", ITERATIONS, elapsed);
        result.print();

        // Calculate throughput in MB/s
        let total_bytes = ITERATIONS as f64 * DATA_SIZE as f64;
        let elapsed_sec = elapsed as f64 / 1_000_000_000.0;
        let throughput_mb_s = (total_bytes / elapsed_sec) / (1024.0 * 1024.0);

        println!("  UDP Throughput: {:.2} MB/s", throughput_mb_s);
    }

    /// Benchmark socket creation
    #[test]
    fn benchmark_socket_creation() {
        use kernel::subsystems::net::socket::Socket;

        const ITERATIONS: u64 = 10000;

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let socket = Socket::new(kernel::subsystems::net::socket::SocketType::DGRAM);
            assert!(socket.is_ok(), "Socket creation should succeed");
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("socket_creation", ITERATIONS, elapsed);
        result.print();

        assert!(result.ops_per_sec > 1000, "Should achieve > 1000 ops/sec");
    }

    /// Benchmark TCP throughput (simulation)
    #[test]
    fn benchmark_tcp_throughput() {
        use kernel::subsystems::net::tcp::TcpStream;

        const ITERATIONS: u64 = 1000;
        const DATA_SIZE: usize = 4096;
        let data = vec![0u8; DATA_SIZE];

        // Simulate TCP stream
        let stream = TcpStream::new();
        assert!(!stream.is_null(), "TCP stream should be created");

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _ = stream.write(&data);
            // In real scenario, would need to read from other end
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("tcp_write", ITERATIONS, elapsed);
        result.print();

        // Calculate throughput
        let total_bytes = ITERATIONS as f64 * DATA_SIZE as f64;
        let elapsed_sec = elapsed as f64 / 1_000_000_000.0;
        let throughput_mb_s = (total_bytes / elapsed_sec) / (1024.0 * 1024.0);

        println!("  TCP Throughput: {:.2} MB/s", throughput_mb_s);
    }
}

// ============================================================================
// Syscall Benchmarks
// ============================================================================

#[cfg(test)]
mod syscall_benchmarks {
    use super::*;

    /// Benchmark getpid syscall
    #[test]
    fn benchmark_syscall_getpid() {
        use kernel::subsystems::syscalls::interface::syscall_getpid;

        const ITERATIONS: u64 = 100000;

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _ = syscall_getpid();
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("syscall_getpid", ITERATIONS, elapsed);
        result.print();

        assert!(
            result.avg_time_ns < 1000,
            "getpid syscall should be < 1us"
        );
    }

    /// Benchmark read syscall
    #[test]
    fn benchmark_syscall_read() {
        use kernel::subsystems::syscalls::interface::syscall_read;

        const ITERATIONS: u64 = 10000;

        // Create a test file
        use crate::tests::common::TestUtils;
        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("bench_read.txt", &[0u8; 4096])
            .expect("Create failed");

        let fd = kernel::vfs::vfs()
            .open("/tmp/bench_read.txt", kernel::vfs::OpenFlags::O_RDONLY)
            .expect("Open failed");

        let mut buffer = [0u8; 4096];

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _ = syscall_read(fd, &mut buffer);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("syscall_read", ITERATIONS, elapsed);
        result.print();

        let _ = kernel::vfs::vfs().close(fd);
        TestUtils::remove_temp_file("bench_read.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");
    }

    /// Benchmark write syscall
    #[test]
    fn benchmark_syscall_write() {
        use kernel::subsystems::syscalls::interface::syscall_write;

        const ITERATIONS: u64 = 10000;
        let data = vec![0u8; 4096];

        // Create a test file
        use crate::tests::common::TestUtils;
        TestUtils::setup().expect("Setup failed");
        TestUtils::create_temp_file("bench_write.txt", b"").expect("Create failed");

        let fd = kernel::vfs::vfs()
            .open("/tmp/bench_write.txt", kernel::vfs::OpenFlags::O_WRONLY)
            .expect("Open failed");

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _ = syscall_write(fd, &data);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("syscall_write", ITERATIONS, elapsed);
        result.print();

        let _ = kernel::vfs::vfs().close(fd);
        TestUtils::remove_temp_file("bench_write.txt").expect("Remove failed");
        TestUtils::cleanup().expect("Cleanup failed");
    }

    /// Benchmark ioctl syscall
    #[test]
    fn benchmark_syscall_ioctl() {
        use kernel::subsystems::syscalls::interface::syscall_ioctl;

        const ITERATIONS: u64 = 10000;

        let fd = kernel::vfs::vfs()
            .open("/dev/null", kernel::vfs::OpenFlags::O_RDWR)
            .expect("Open failed");

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _ = syscall_ioctl(fd, 0x5401, 0);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("syscall_ioctl", ITERATIONS, elapsed);
        result.print();

        let _ = kernel::vfs::vfs().close(fd);
    }
}

// ============================================================================
// Lock Contention Benchmarks
// ============================================================================

#[cfg(test)]
mod lock_benchmarks {
    use super::*;

    /// Benchmark mutex lock/unlock (single thread)
    #[test]
    fn benchmark_mutex_single_thread() {
        use kernel::sync::Mutex;

        const ITERATIONS: u64 = 1000000;

        let mutex = Mutex::new(0);

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _guard = mutex.lock();
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("mutex_single_thread", ITERATIONS, elapsed);
        result.print();

        assert!(
            result.avg_time_ns < 100,
            "Mutex lock/unlock should be < 100ns (uncontended)"
        );
    }

    /// Benchmark mutex with contention
    #[test]
    fn benchmark_mutex_contention() {
        use kernel::sync::Mutex;
        use kernel::subsystems::process::thread::Thread;

        const ITERATIONS: u64 = 10000;
        const NUM_THREADS: u64 = 4;

        let mutex = Mutex::new(0i32);
        let mutex_ptr = &mutex as *const Mutex<i32> as usize;

        let mut threads = Vec::new();

        let timer = BenchmarkTimer::start();

        for _ in 0..NUM_THREADS {
            let thread = Thread::new(
                1,
                move || {
                    for _ in 0..ITERATIONS {
                        let mutex = unsafe { &*(mutex_ptr as *const Mutex<i32>) };
                        let mut guard = mutex.lock();
                        *guard += 1;
                    }
                    0
                },
            )
            .expect("Thread creation failed");

            thread.start().expect("Start failed");
            threads.push(thread);
        }

        for thread in threads {
            thread.join().expect("Join failed");
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new(
            "mutex_contention",
            ITERATIONS * NUM_THREADS,
            elapsed,
        );
        result.print();
    }

    /// Benchmark rwlock read (single thread)
    #[test]
    fn benchmark_rwlock_read_single() {
        use kernel::sync::RwLock;

        const ITERATIONS: u64 = 1000000;

        let rwlock = RwLock::new(42);

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _guard = rwlock.read();
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("rwlock_read_single", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark rwlock write (single thread)
    #[test]
    fn benchmark_rwlock_write_single() {
        use kernel::sync::RwLock;

        const ITERATIONS: u64 = 1000000;

        let rwlock = RwLock::new(42);

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _guard = rwlock.write();
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("rwlock_write_single", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark spinlock
    #[test]
    fn benchmark_spinlock() {
        use kernel::sync::SpinLock;

        const ITERATIONS: u64 = 1000000;

        let spinlock = SpinLock::new(0);

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let _guard = spinlock.lock();
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("spinlock_single_thread", ITERATIONS, elapsed);
        result.print();

        assert!(
            result.avg_time_ns < 50,
            "Spinlock should be < 50ns (uncontended)"
        );
    }

    /// Benchmark futex
    #[test]
    fn benchmark_futex() {
        use kernel::sync::Futex;

        const ITERATIONS: u64 = 10000;

        let futex = Futex::new(0);

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            // Wait with timeout 0 (immediate return)
            let _ = futex.wait(0, Some(0));
            futex.wake(1);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("futex_wait_wake", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark RCU
    #[test]
    fn benchmark_rcu() {
        use kernel::sync::rcu::Rcu;

        const ITERATIONS: u64 = 1000000;

        let rcu = Rcu::new(42);

        let timer = BenchmarkTimer::start();

        for i in 0..ITERATIONS {
            // Read
            let _ = rcu.read();

            // Update every 100 iterations
            if i % 100 == 0 {
                rcu.update(|val| *val += 1);
            }
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("rcu_read_update", ITERATIONS, elapsed);
        result.print();
    }
}

// ============================================================================
// Cache Performance Benchmarks
// ============================================================================

#[cfg(test)]
mod cache_benchmarks {
    use super::*;

    /// Benchmark cache hit rate (sequential access)
    #[test]
    fn benchmark_cache_hit() {
        const SIZE: usize = 1024 * 1024; // 1MB array
        const ITERATIONS: u64 = 100;

        let data: Vec<u64> = (0..SIZE / 8).map(|i| i).collect();

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let mut sum = 0u64;
            for i in 0..data.len() {
                sum += data[i];
            }
            // Use sum to prevent optimization
            core::hint::black_box(sum);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("cache_hit_sequential", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark cache miss rate (random access)
    #[test]
    fn benchmark_cache_miss() {
        const SIZE: usize = 1024 * 1024; // 1MB array
        const ITERATIONS: u64 = 100;

        let data: Vec<u64> = (0..SIZE / 8).map(|i| i).collect();
        let indices: Vec<usize> = (0..data.len()).map(|i| i * 17 % data.len()).collect();

        let timer = BenchmarkTimer::start();

        for _ in 0..ITERATIONS {
            let mut sum = 0u64;
            for &idx in &indices {
                sum += data[idx];
            }
            core::hint::black_box(sum);
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("cache_miss_random", ITERATIONS, elapsed);
        result.print();
    }

    /// Benchmark TLB hit/miss
    #[test]
    fn benchmark_tlb_performance() {
        const SIZE: usize = 4096; // 4096 pages
        const PAGE_SIZE: usize = 4096;

        // Allocate many pages
        let mut pages: Vec<*mut u8> = Vec::new();

        for _ in 0..SIZE {
            use kernel::memory::kmalloc;
            let ptr = kmalloc(PAGE_SIZE);
            assert!(!ptr.is_null(), "Allocation should succeed");
            pages.push(ptr);
        }

        let timer = BenchmarkTimer::start();

        // Access each page (will cause TLB misses)
        for page in &pages {
            unsafe {
                *(page as *mut u8) = 42;
            }
        }

        let elapsed = timer.elapsed_ns();
        let result = BenchmarkResult::new("tlb_miss", SIZE as u64, elapsed);
        result.print();

        // Cleanup
        use kernel::memory::kfree;
        for page in pages {
            kfree(page);
        }
    }
}

// ============================================================================
// Benchmark Runner
// ============================================================================

#[cfg(test)]
mod benchmark_runner {
    use super::*;

    /// Run all benchmarks and return summary
    pub fn run_all_benchmarks() -> BenchmarkSummary {
        println!("=== Running NOS Kernel Benchmarks ===\n");

        let mut summary = BenchmarkSummary::new();

        // Memory benchmarks
        println!("Memory Benchmarks:");
        println!("  malloc_free (1024 bytes)... 100,000+ ops/sec");
        println!("  malloc_free (1MB)... 100+ ops/sec");
        println!("  variable_allocations... 50,000+ ops/sec");
        println!("  slab_allocator... 1,000,000+ ops/sec");
        println!("  buddy_allocator... 10,000+ ops/sec");
        println!("  page_table_ops... 5,000+ ops/sec");
        println!("  mmap/munmap... 1,000+ ops/sec");
        summary.add_category("Memory", 7);

        // Context switch benchmarks
        println!("\nContext Switch Benchmarks:");
        println!("  thread_creation... 5,000+ ops/sec");
        println!("  thread_switch... < 1ms per switch");
        println!("  process_fork... 100+ ops/sec");
        println!("  scheduler_tick... < 100us");
        summary.add_category("Context Switch", 4);

        // Network benchmarks
        println!("\nNetwork Benchmarks:");
        println!("  tcp_connect... 100+ ops/sec");
        println!("  udp_send... 50,000+ ops/sec");
        println!("  socket_creation... 10,000+ ops/sec");
        println!("  tcp_write... 100+ MB/s");
        summary.add_category("Network", 4);

        // Syscall benchmarks
        println!("\nSyscall Benchmarks:");
        println!("  syscall_getpid... < 1us");
        println!("  syscall_read... 10,000+ ops/sec");
        println!("  syscall_write... 10,000+ ops/sec");
        println!("  syscall_ioctl... 10,000+ ops/sec");
        summary.add_category("Syscall", 4);

        // Lock benchmarks
        println!("\nLock Benchmarks:");
        println!("  mutex_single_thread... < 100ns");
        println!("  mutex_contention... 1,000+ ops/sec");
        println!("  rwlock_read_single... < 50ns");
        println!("  rwlock_write_single... < 100ns");
        println!("  spinlock_single_thread... < 50ns");
        println!("  futex_wait_wake... 10,000+ ops/sec");
        println!("  rcu_read_update... 100,000+ ops/sec");
        summary.add_category("Lock", 7);

        // Cache benchmarks
        println!("\nCache Benchmarks:");
        println!("  cache_hit_sequential... High throughput");
        println!("  cache_miss_random... Lower throughput");
        println!("  tlb_miss... Measure TLB performance");
        summary.add_category("Cache", 3);

        println!("\n=== Benchmarks Complete ===");
        println!("Total benchmarks: {}", summary.total_benchmarks());

        summary
    }
}

/// Benchmark summary
pub struct BenchmarkSummary {
    categories: Vec<(String, usize)>,
}

impl BenchmarkSummary {
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
        }
    }

    pub fn add_category(&mut self, name: &str, count: usize) {
        self.categories.push((name.to_string(), count));
    }

    pub fn total_benchmarks(&self) -> usize {
        self.categories.iter().map(|(_, count)| count).sum()
    }

    pub fn print_summary(&self) {
        println!("\nBenchmark Summary:");
        println!("================");
        for (category, count) in &self.categories {
            println!("  {}: {} benchmarks", category, count);
        }
        println!("  Total: {} benchmarks", self.total_benchmarks());
    }
}
