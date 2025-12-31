//! # 进程管理子系统
//!
//! 负责进程和线程的创建、调度、生命周期管理和资源管理。
//!
//! ## 概述
//!
//! 进程管理子系统是内核的核心组件之一，提供：
//! - **进程创建**: 通过 `fork` 和 `exec` 创建新进程
//! - **线程管理**: 支持 POSIX 线程（pthreads）
//! - **资源管理**: 文件描述符、内存、信号等资源的管理
//! - **上下文切换**: 高效的进程和线程上下文切换
//! - **进程间通信**: 支持多种 IPC 机制
//!
//! ## 主要组件
//!
//! - [`Manager`]: 进程管理器，维护全局进程表
//! - [`Process`]: 进程结构体，包含进程状态和资源
//! - [`Thread`]: 线程结构体，支持多线程
//! - [`FdCache`]: 文件描述符缓存，优化文件访问
//! - [`context_switch`]: 上下文切换优化
//! - [`elf`]: ELF 二进制格式加载器
//! - [`dynamic_linker`]: 动态链接器支持
//! - [`exec`]: exec 系统调用实现
//!
//! ## 架构
//!
//! ```
//! 进程管理器 (Manager)
//!     ├── 进程表 (Process Table)
//!     │   ├── 进程1 (Process)
//!     │   │   ├── 主线程 (Thread)
//!     │   │   ├── 文件描述符表
//!     │   │   ├── 虚拟地址空间
//!     │   │   └── 信号处理
//!     │   └── 进程2 (Process)
//!     │       └── ...
//!     ├── 调度队列
//!     └── RCU 表 (rcu_table)
//! ```
//!
//! ## 使用示例
//!
//! ### 创建新进程
//!
//! ```no_run
//! use kernel::subsystems::process::{fork, exec};
//!
//! // Fork a new process
//! match fork() {
//!     Ok(0) => {
//!         // Child process
//!         exec("/bin/program")?;
//!     }
//!     Ok(pid) => {
//!         // Parent process
//!         println!("Child PID: {}", pid);
//!     }
//!     Err(e) => {
//!         eprintln!("Fork failed: {:?}", e);
//!     }
//! }
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 等待进程退出
//!
//! ```no_run
//! use kernel::subsystems::process::{waitpid, WaitStatus};
//!
//! match waitpid(None) {
//!     Ok(WaitStatus::Exited(pid, status)) => {
//!         println!("Process {} exited with status {}", pid, status);
//!     }
//!     Ok(WaitStatus::Signaled(pid, signal)) => {
//!         println!("Process {} killed by signal {}", pid, signal);
//!     }
//!     _ => {}
//! }
//! ```
//!
//! ## 设计决策
//!
//! ### 进程 vs 线程
//!
//! NOS 使用统一的进程/线程模型：
//! - 进程是资源分配的单位
//! - 线程是调度的单位
//! - 同一进程的线程共享资源
//!
//! ### RCU 优化
//!
//! 使用读-复制-更新（RCU）机制优化进程表的访问：
//! - 读操作无锁
//! - 写操作使用延迟释放
//! - 减少锁竞争，提高性能
//!
//! ## 性能特征
//!
//! - **Fork 延迟**: O(n) 其中 n 是内存页数（使用写时复制优化）
//! - **上下文切换**: < 1μs
//! - **进程查找**: O(1) 通过 PID
//!
//! ## 线程安全
//!
//! 进程管理器使用多种同步机制：
//! - 全局进程表使用 `Mutex` 保护
//! - RCU 表提供无锁读取
//! - 每个进程有独立的锁
//!
//! ## 相关模块
//!
//! - [`crate::sched`]: 调度器
//! - [`crate::subsystems::mm`]: 内存管理
//! - [`crate::vfs`]: 文件系统（文件描述符）
//! - [`crate::subsystems::syscalls::process`]: 进程相关系统调用

pub mod context_switch;
pub mod credentials;
pub mod dynamic_linker;
pub mod elf;
pub mod exec;
pub mod fd_cache;
// lock_optimized removed during cleanup - optimizations integrated into sync module
pub mod manager;
pub mod rcu_table;
pub mod rlimit;
pub mod thread;
pub mod thread_cancellation;
pub mod vfork;

#[cfg(feature = "kernel_tests")]
pub mod tests;

pub use fd_cache::*;
// lock_optimized removed - optimizations integrated into sync module
pub use manager::*;
pub use rcu_table::*;

use crate::types::stubs::{GidT as gid_t, UidT as uid_t};

/// Get current real user ID
pub fn getuid() -> uid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.uid;
        }
    }
    0
}

/// Get current real group ID
pub fn getgid() -> gid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.gid;
        }
    }
    0
}

/// Get current effective user ID
pub fn geteuid() -> uid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.euid;
        }
    }
    0
}

/// Get current effective group ID
pub fn getegid() -> gid_t {
    if let Some(pid) = myproc() {
        let table = PROC_TABLE.lock();
        if let Some(proc) = table.find_ref(pid) {
            return proc.egid;
        }
    }
    0
}

/// Initialize process management subsystem
///
/// This function initializes all process management components including:
/// - Process table
/// - Thread subsystem
/// - Context switching
/// - Process manager
pub fn init() -> nos_api::Result<()> {
    // Initialize process manager
    manager::init();

    // Initialize thread subsystem
    thread::init();

    // Initialize context switching
    context_switch::init();

    crate::println!("[process] Process management subsystem initialized");
    Ok(())
}

/// Shutdown process management subsystem
///
/// This function cleans up process management resources.
/// Note: In a production system, this should gracefully terminate all processes.
pub fn shutdown() -> nos_api::Result<()> {
    // TODO: Implement graceful shutdown
    // - Terminate all user processes
    // - Wait for processes to exit
    // - Clean up process table
    // - Release resources

    crate::println!("[process] Process management subsystem shutdown");
    Ok(())
}
pub mod types;
pub use types::{Process, ProcessId, Thread, get_current_process, get_process_by_pid};
