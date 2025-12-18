//! NOS System Calls
//!
//! This crate provides the system call interface and dispatch mechanism for the NOS operating system.
//! It includes system call handlers, dispatch logic, and related utilities.
//!
//! # Architecture
//!
//! The system call module is organized into several functional domains:
//!
//! - **Core**: Core dispatch logic and system call infrastructure
//! - **FS**: File system related system calls
//! - **Process**: Process management related system calls
//! - **Network**: Network related system calls
//! - **IPC**: Inter-process communication system calls
//! - **Signal**: Signal handling system calls
//! - **Memory**: Memory management system calls
//! - **Time**: Time-related system calls
//!
//! # Usage
//!
//! ```rust
//! use nos_syscalls::{dispatch, SyscallId};
//!
//! // Dispatch a system call
//! let result = dispatch(SyscallId::Read, &[
//!     fd as usize,
//!     buffer.as_ptr() as usize,
//!     count as usize
//! ]);
//! ```

#![no_std]
#![allow(dead_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

// Removed unused imports to fix warnings
// Core modules
pub mod core;
pub mod fs;
pub mod process;
pub mod network;
pub mod ipc;
pub mod signal;
pub mod memory;
pub mod time;
pub mod common;
pub mod types;

// Advanced system call modules
pub mod advanced_mmap;
#[cfg(feature = "alloc")]
pub mod async_ops;
#[cfg(feature = "alloc")]
pub mod epoll;
#[cfg(feature = "alloc")]
pub mod zero_copy_network;
#[cfg(feature = "alloc")]
pub mod optimized_syscall_path;
#[cfg(feature = "alloc")]
pub mod adaptive_scheduler;
#[cfg(feature = "alloc")]
pub mod modular_framework;
#[cfg(feature = "alloc")]
pub mod testing_framework;
#[cfg(feature = "alloc")]
pub mod performance_monitor;
#[cfg(feature = "alloc")]
pub mod zero_copy_network_impl;

// Re-export commonly used items
pub use core::{SyscallRegistry, SyscallHandler, get_registry};
#[cfg(feature = "alloc")]
pub use core::{SyscallInfo, SyscallDispatcher, get_dispatcher, get_stats};
// Note: fs, process, network, ipc, signal, memory, time, common modules are not re-exported to avoid unused import warnings
pub use types::*;
pub use advanced_mmap::*;
// Note: async_ops and epoll modules are not re-exported as they may not be available in all configurations

// Note: zero_copy_network is re-exported with explicit imports to avoid conflicts
#[cfg(feature = "alloc")]
pub use zero_copy_network::{ZeroCopySendHandler, ZeroCopyRecvHandler, ZeroCopyConfig};

// Note: zero_copy_network_impl is not re-exported to avoid ambiguous glob re-exports with zero_copy_network
// Only specific items should be imported directly from zero_copy_network_impl when needed
// #[cfg(feature = "alloc")]
// pub use zero_copy_network_impl::*;

#[cfg(feature = "alloc")]
pub use optimized_syscall_path::{register_handlers as register_optimized_syscall_handlers};
#[cfg(feature = "alloc")]
pub use adaptive_scheduler::{register_handlers as register_adaptive_scheduler_handlers, get_scheduler_report, TaskSchedulerHandler, TaskPriority, TaskState, TaskStats, TaskControlBlock, AdaptiveScheduler, SchedulerStats, AdaptiveParameters};
#[cfg(feature = "alloc")]
pub use modular_framework::{register_standard_modules, SyscallCategory, SyscallMetadata, SyscallModule, ModuleInfo, ModularDispatcher, DispatcherStats, ModuleBuilder};
#[cfg(feature = "alloc")]
pub use testing_framework::{run_all_tests as run_all_framework_tests, create_standard_test_suites, TestStatus, TestResult, TestSuite, TestSuiteResult, TestCase, SimpleTestCase, TestRunner, TestRunnerStats, SyscallTestCase};
#[cfg(feature = "alloc")]
pub use performance_monitor::{register_handlers as register_performance_monitor_handlers, MetricType, PerformanceMetric, PerformanceMonitor, MonitorStats, PerformanceAnalyzer, AnalysisResult, AnalysisType, AnalysisValue, TrendDirection, SyscallMonitor};

/// Initialize the system call subsystem
#[cfg(feature = "alloc")]
pub fn init_syscalls() -> nos_api::Result<()> {
    // Initialize core dispatch mechanism
    core::init_dispatcher()?;
    
    // Register all system call handlers
    core::register_handlers()?;
    
    // Register modular framework
    let mut modular_dispatcher = modular_framework::ModularDispatcher::new();
    modular_framework::register_standard_modules(&mut modular_dispatcher)?;
    
    // Register testing framework
    testing_framework::run_all_tests()?;
    
    Ok(())
}

/// Shutdown the system call subsystem
///
/// This function shuts down the system call dispatch mechanism
/// and cleans up resources.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
#[cfg(feature = "alloc")]
pub fn shutdown_syscalls() -> nos_api::Result<()> {
    // Shutdown core dispatch mechanism
    core::shutdown_dispatcher()?;
    
    Ok(())
}

/// Get system call statistics
///
/// # Returns
///
/// * `SyscallStats` - System call statistics
#[cfg(feature = "alloc")]
pub fn get_syscall_stats() -> SyscallStats {
    core::get_stats()
}

/// System call statistics
#[derive(Debug, Clone)]
#[cfg(feature = "alloc")]
pub struct SyscallStats {
    /// Total number of system calls
    pub total_calls: u64,
    /// Number of calls by type
    pub calls_by_type: alloc::collections::BTreeMap<u32, u64>,
    /// Average execution time (microseconds)
    pub avg_execution_time: u64,
    /// Number of errors
    pub error_count: u64,
}

#[cfg(feature = "alloc")]
impl Default for SyscallStats {
    fn default() -> Self {
        Self {
            total_calls: 0,
            calls_by_type: alloc::collections::BTreeMap::new(),
            avg_execution_time: 0,
            error_count: 0,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_stats() {
        let stats = SyscallStats::default();
        assert_eq!(stats.total_calls, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.avg_execution_time, 0);
        assert!(stats.calls_by_type.is_empty());
    }
}