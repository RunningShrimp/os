//! 核心POSIX系统调用测试
//!
//! 本模块提供核心POSIX系统调用的全面测试，包括：
//! - 文件系统操作测试
//! - 进程管理测试
//! - 内存管理测试
//! - 网络操作测试
//! - 信号处理测试
//! - 线程管理测试

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void};

use crate::posix_tests::{PosixTestResult, PosixTestResults, PerformanceMetric};
use crate::syscalls;
use crate::posix;

// 导出子模块
pub mod basic_tests;
pub mod signal_tests;
pub mod thread_tests;

// 重新导出测试函数
pub use basic_tests::*;
pub use signal_tests::*;
pub use thread_tests::*;

/// 文件系统相关系统调用测试
pub fn test_filesystem_syscalls(results: &mut PosixTestResults) {
    crate::println!("  📁 文件系统系统调用测试:");

    let start_time = crate::subsystems::time::get_time_ns();

    // 测试stat系列系统调用
    test_stat_syscalls(results);

    // 测试文件操作系统调用
    test_file_operations(results);

    // 测试目录操作系统调用
    test_directory_operations(results);

    // 测试文件描述符操作
    test_fd_operations(results);

    // 测试文件权限操作
    test_file_permissions(results);

    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "filesystem_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 进程管理相关系统调用测试
pub fn test_process_syscalls(results: &mut PosixTestResults) {
    crate::println!("  ⚙️ 进程管理系统调用测试:");

    let start_time = crate::subsystems::time::get_time_ns();

    // 测试fork/vfork
    test_fork_vfork(results);

    // 测试exec系列
    test_exec_series(results);

    // 测试wait系列
    test_wait_series(results);

    // 测试exit系列
    test_exit_series(results);

    // 测试getpid/getppid
    test_getpid_getppid(results);

    // 测试进程组相关
    test_process_groups(results);

    // 测试会话相关
    test_session_management(results);

    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "process_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 内存管理相关系统调用测试
pub fn test_memory_syscalls(results: &mut PosixTestResults) {
    crate::println!("  💾 内存管理系统调用测试:");

    let start_time = crate::subsystems::time::get_time_ns();

    // 测试mmap系列
    test_mmap_series(results);

    // 测试mprotect
    test_mprotect(results);

    // 测试msync
    test_msync(results);

    // 测试mlock系列
    test_mlock_series(results);

    // 测试brk/sbrk
    test_brk_sbrk(results);

    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "memory_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 网络相关系统调用测试
pub fn test_network_syscalls(results: &mut PosixTestResults) {
    crate::println!("  🌐 网络系统调用测试:");

    let start_time = crate::subsystems::time::get_time_ns();

    // 测试socket系列
    test_socket_series(results);

    // 测试bind/listen/accept
    test_bind_listen_accept(results);

    // 测试connect
    test_connect(results);

    // 测试send/recv系列
    test_send_recv_series(results);

    // 测试shutdown
    test_shutdown(results);

    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "network_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 信号处理测试
pub fn test_signal_syscalls(results: &mut PosixTestResults) {
    crate::println!("  📶 信号处理测试:");

    let start_time = crate::subsystems::time::get_time_ns();

    // 基础信号测试
    test_basic_signals(results);

    // 信号掩码测试
    test_signal_mask(results);

    // 信号处理测试
    test_signal_handlers(results);

    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "signal_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}

/// 线程管理测试
pub fn test_thread_syscalls(results: &mut PosixTestResults) {
    crate::println!("  🧵 线程管理测试:");

    let start_time = crate::subsystems::time::get_time_ns();

    // 线程创建和销毁
    test_thread_creation(results);

    // 线程同步
    test_thread_sync(results);

    // 线程属性
    test_thread_attributes(results);

    let execution_time = crate::subsystems::time::get_time_ns() - start_time;
    results.record_performance(PerformanceMetric {
        test_name: "thread_syscalls".to_string(),
        execution_time_ns: execution_time,
        memory_used_bytes: 0,
        cpu_cycles: 0,
    });
}
