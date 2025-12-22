//! Comprehensive Performance Benchmarks for NOS Kernel
//!
//! This module provides performance benchmarks for critical kernel subsystems:
//! - System call dispatch (fast-path, RCU-optimized dispatcher)
//! - Error handling (fast errno conversion)
//! - Memory allocators (buddy, slab, tiered)
//!
//! These benchmarks help identify performance bottlenecks and measure the impact
//! of optimizations like RCU, fast-path dispatch, and optimized error handling.

#![no_std]

extern crate alloc;

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

// Note: These benchmarks require the kernel to be compiled as a library
// and may need adjustments for no_std environment

// ============================================================================
// System Call Dispatch Performance Benchmarks
// ============================================================================

/// Benchmark fast-path syscall dispatch (RCU-optimized)
#[cfg(feature = "syscalls")]
fn bench_fast_path_dispatch(c: &mut Criterion) {
    use kernel::subsystems::syscalls::dispatch::unified::{init_unified_dispatcher, get_unified_dispatcher, UnifiedDispatcherConfig};
    
    // Initialize dispatcher
    let config = UnifiedDispatcherConfig {
        enable_fast_path: true,
        enable_per_cpu_cache: true,
        enable_monitoring: false, // Disable monitoring for pure dispatch benchmark
        enable_adaptive_optimization: false,
        fast_path_update_interval: 1000,
    };
    init_unified_dispatcher(config);
    
    // Register a fast-path handler
    if let Some(dispatcher_mutex) = get_unified_dispatcher() {
        let dispatcher = dispatcher_mutex.lock();
        if let Some(ref d) = *dispatcher {
            let _ = d.register_fast_path(0x1004, |_num, _args| Ok(12345));
        }
    }
    
    c.bench_function("fast_path_dispatch", |b| {
        b.iter(|| {
            if let Some(dispatcher_mutex) = get_unified_dispatcher() {
                let dispatcher = dispatcher_mutex.lock();
                if let Some(ref d) = *dispatcher {
                    let _ = black_box(d.dispatch(0x1004, &[]));
                }
            }
        });
    });
}

/// Benchmark regular syscall dispatch (RCU-optimized handlers map)
#[cfg(feature = "syscalls")]
fn bench_regular_dispatch(c: &mut Criterion) {
    use kernel::subsystems::syscalls::dispatch::unified::{init_unified_dispatcher, get_unified_dispatcher, UnifiedDispatcherConfig};
    use kernel::subsystems::syscalls::interface::{SyscallHandler, SyscallError, SyscallResult};
    use alloc::sync::Arc;
    
    // Initialize dispatcher
    let config = UnifiedDispatcherConfig {
        enable_fast_path: false, // Disable fast-path to test regular dispatch
        enable_per_cpu_cache: true,
        enable_monitoring: false,
        enable_adaptive_optimization: false,
        fast_path_update_interval: 1000,
    };
    init_unified_dispatcher(config);
    
    // Create a simple handler
    struct TestHandler;
    impl SyscallHandler for TestHandler {
        fn handle(&self, _args: &[u64]) -> SyscallResult {
            Ok(12345)
        }
        
        fn get_syscall_number(&self) -> u32 {
            0x2000
        }
        
        fn get_name(&self) -> &'static str {
            "test_handler"
        }
    }
    
    // Register handler
    if let Some(dispatcher_mutex) = get_unified_dispatcher() {
        let dispatcher = dispatcher_mutex.lock();
        if let Some(ref d) = *dispatcher {
            let handler = Arc::new(TestHandler);
            let _ = d.register_handler(0x2000, handler);
        }
    }
    
    c.bench_function("regular_dispatch", |b| {
        b.iter(|| {
            if let Some(dispatcher_mutex) = get_unified_dispatcher() {
                let dispatcher = dispatcher_mutex.lock();
                if let Some(ref d) = *dispatcher {
                    let _ = black_box(d.dispatch(0x2000, &[]));
                }
            }
        });
    });
}

/// Benchmark syscall dispatch with multiple concurrent readers (RCU benefit)
#[cfg(feature = "syscalls")]
fn bench_concurrent_dispatch(c: &mut Criterion) {
    use kernel::subsystems::syscalls::dispatch::unified::{init_unified_dispatcher, get_unified_dispatcher, UnifiedDispatcherConfig};
    
    // Initialize dispatcher
    let config = UnifiedDispatcherConfig {
        enable_fast_path: true,
        enable_per_cpu_cache: true,
        enable_monitoring: false,
        enable_adaptive_optimization: false,
        fast_path_update_interval: 1000,
    };
    init_unified_dispatcher(config);
    
    // Register fast-path handlers
    if let Some(dispatcher_mutex) = get_unified_dispatcher() {
        let dispatcher = dispatcher_mutex.lock();
        if let Some(ref d) = *dispatcher {
            for i in 0..10 {
                let _ = d.register_fast_path(0x1000 + i, |_num, _args| Ok(i as u64));
            }
        }
    }
    
    c.bench_function("concurrent_dispatch", |b| {
        b.iter(|| {
            // Simulate concurrent reads (no locks needed with RCU)
            for i in 0..10 {
                if let Some(dispatcher_mutex) = get_unified_dispatcher() {
                    let dispatcher = dispatcher_mutex.lock();
                    if let Some(ref d) = *dispatcher {
                        let _ = black_box(d.dispatch(0x1000 + (i % 10), &[]));
                    }
                }
            }
        });
    });
}

// ============================================================================
// Error Handling Performance Benchmarks
// ============================================================================

/// Benchmark fast errno conversion (optimized error mapping)
fn bench_fast_errno_conversion(c: &mut Criterion) {
    use kernel::error::unified_mapping::fast_unified_error_to_errno;
    use nos_api::core::types::KernelError;
    
    let errors = [
        KernelError::PermissionDenied,
        KernelError::NotFound,
        KernelError::IoError,
        KernelError::NoDevice,
        KernelError::InvalidArgument,
        KernelError::OutOfMemory,
        KernelError::Busy,
        KernelError::WouldBlock,
    ];
    
    c.bench_function("fast_errno_conversion", |b| {
        b.iter(|| {
            for error in &errors {
                let _ = black_box(fast_unified_error_to_errno(error));
            }
        });
    });
}

/// Benchmark unified error to errno conversion (static match)
fn bench_unified_error_to_errno(c: &mut Criterion) {
    use kernel::error::unified_mapping::unified_error_to_errno;
    use kernel::error::unified::UnifiedError;
    
    let errors = [
        UnifiedError::InvalidArgument,
        UnifiedError::NotFound,
        UnifiedError::PermissionDenied,
        UnifiedError::IoError,
        UnifiedError::OutOfMemory,
        UnifiedError::Busy,
        UnifiedError::WouldBlock,
    ];
    
    c.bench_function("unified_error_to_errno", |b| {
        b.iter(|| {
            for error in &errors {
                let _ = black_box(unified_error_to_errno(error));
            }
        });
    });
}

/// Benchmark error handling overhead (with and without fast path)
fn bench_error_handling_overhead(c: &mut Criterion) {
    use kernel::error::unified_mapping::{fast_unified_error_to_errno, unified_error_to_errno};
    use nos_api::core::types::KernelError;
    use kernel::error::unified::UnifiedError;
    
    c.bench_function("error_handling_overhead", |b| {
        b.iter(|| {
            // Fast path (direct KernelError -> Errno)
            let kernel_error = KernelError::PermissionDenied;
            let _errno1 = black_box(fast_unified_error_to_errno(&kernel_error));
            
            // Regular path (UnifiedError -> Errno)
            let unified_error = UnifiedError::PermissionDenied;
            let _errno2 = black_box(unified_error_to_errno(&unified_error));
        });
    });
}

// ============================================================================
// Memory Allocator Performance Benchmarks
// ============================================================================

/// Benchmark buddy allocator performance (optimized with bitmap)
#[cfg(feature = "memory_management")]
fn bench_buddy_allocator(c: &mut Criterion) {
    use kernel::subsystems::mm::optimized_page_allocator::{OptimizedPageAllocator, BuddyAllocator};
    
    c.bench_function("buddy_allocator_optimized", |b| {
        // Initialize allocator with 1MB
        let mut allocator = OptimizedPageAllocator::new(1, 0);
        unsafe {
            allocator.init(0x100000, 0x200000); // 1MB range
        }
        
        b.iter(|| {
            let pfn = allocator.allocate_page();
            if let Some(pfn) = pfn {
                black_box(pfn);
                allocator.deallocate_page(pfn);
            }
        });
    });
}

/// Benchmark slab allocator performance
#[cfg(feature = "memory_management")]
fn bench_slab_allocator(c: &mut Criterion) {
    c.bench_function("slab_allocator", |b| {
    use nos_memory_management::allocator::slab::OptimizedSlabAllocator;
    
    let mut allocator = OptimizedSlabAllocator::new();
    
        b.iter(|| {
            let ptr = allocator.allocate(64).unwrap();
            black_box(ptr);
            allocator.deallocate(ptr, 64);
        });
    });
}

/// Benchmark tiered allocator performance
#[cfg(feature = "memory_management")]
fn bench_tiered_allocator(c: &mut Criterion) {
    c.bench_function("tiered_allocator", |b| {
    use nos_memory_management::allocator::tiered::TieredMemoryAllocator;
    
    let mut allocator = TieredMemoryAllocator::new();
    
        b.iter(|| {
            let ptr = allocator.allocate(256).unwrap();
            black_box(ptr);
            allocator.deallocate(ptr, 256);
        });
    });
}

/// Benchmark memory allocator for different sizes
#[cfg(feature = "memory_management")]
fn bench_allocator_sizes(c: &mut Criterion) {
    c.bench_function("allocator_sizes", |b| {
    use nos_memory_management::allocator::buddy::OptimizedBuddyAllocator;
    
    let mut allocator = OptimizedBuddyAllocator::new(4 * 1024 * 1024); // 4MB
    
    let sizes = [64, 256, 1024, 4096, 16384, 65536];
    
        b.iter(|| {
            for &size in &sizes {
                let ptr = allocator.allocate(size).unwrap();
                black_box(ptr);
                allocator.deallocate(ptr, size);
            }
        });
    });
}

/// Benchmark memory allocator fragmentation
#[cfg(feature = "memory_management")]
fn bench_allocator_fragmentation(c: &mut Criterion) {
    use nos_memory_management::allocator::buddy::OptimizedBuddyAllocator;
    use alloc::vec::Vec;
    
    let mut allocator = OptimizedBuddyAllocator::new(1024 * 1024); // 1MB
    let mut ptrs = Vec::new();
    
    // Allocate many small blocks to create fragmentation
    for _ in 0..100 {
        if let Ok(ptr) = allocator.allocate(64) {
            ptrs.push(ptr);
        }
    }
    
    c.bench_function("allocator_fragmentation", |b| {
        b.iter(|| {
            // Try to allocate a large block (should be harder with fragmentation)
            let ptr = allocator.allocate(4096);
            black_box(ptr);
        });
    });
    
    // Cleanup
    for ptr in ptrs {
        allocator.deallocate(ptr, 64);
    }
}

// ============================================================================
// Combined Performance Benchmarks
// ============================================================================

/// Benchmark complete syscall path (dispatch + error handling)
#[cfg(feature = "syscalls")]
fn bench_complete_syscall_path(c: &mut Criterion) {
    use kernel::subsystems::syscalls::dispatch::unified::{init_unified_dispatcher, get_unified_dispatcher, UnifiedDispatcherConfig};
    use kernel::error::unified_mapping::fast_unified_error_to_errno;
    use nos_api::core::types::KernelError;
    
    // Initialize dispatcher
    let config = UnifiedDispatcherConfig::default();
    init_unified_dispatcher(config);
    
    // Register handler that returns error
    if let Some(dispatcher_mutex) = get_unified_dispatcher() {
        let dispatcher = dispatcher_mutex.lock();
        if let Some(ref d) = *dispatcher {
            let _ = d.register_fast_path(0x3000, |_num, _args| {
                Err(kernel::subsystems::syscalls::interface::SyscallError::InvalidArguments)
            });
        }
    }
    
    c.bench_function("complete_syscall_path", |b| {
        b.iter(|| {
            if let Some(dispatcher_mutex) = get_unified_dispatcher() {
                let dispatcher = dispatcher_mutex.lock();
                if let Some(ref d) = *dispatcher {
                    let result = d.dispatch(0x3000, &[]);
                    // Convert error to errno (simulating real syscall path)
                    if result.is_err() {
                        let kernel_error = KernelError::InvalidArgument;
                        let _errno = black_box(fast_unified_error_to_errno(&kernel_error));
                    }
                    black_box(result);
                }
            }
        });
    });
}

// ============================================================================
// Benchmark Registration
// ============================================================================

criterion_group!(
    benches,
    #[cfg(feature = "syscalls")]
    bench_fast_path_dispatch,
    #[cfg(feature = "syscalls")]
    bench_regular_dispatch,
    #[cfg(feature = "syscalls")]
    bench_concurrent_dispatch,
    bench_fast_errno_conversion,
    bench_unified_error_to_errno,
    bench_error_handling_overhead,
    #[cfg(feature = "memory_management")]
    bench_buddy_allocator,
    #[cfg(feature = "memory_management")]
    bench_slab_allocator,
    #[cfg(feature = "memory_management")]
    bench_tiered_allocator,
    #[cfg(feature = "memory_management")]
    bench_allocator_sizes,
    #[cfg(feature = "memory_management")]
    bench_allocator_fragmentation,
    #[cfg(feature = "syscalls")]
    bench_complete_syscall_path,
);

criterion_main!(benches);

