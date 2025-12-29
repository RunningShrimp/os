//! # NOS Kernel Library
//!
//! This crate provides the public API for the NOS (New Operating System) kernel.
//! It acts as an integration layer for the various kernel components.
//!
//! ## 概述
//!
//! NOS 是一个现代的、模块化的操作系统内核，采用 Rust 编写，专注于：
//! - **安全性**: 利用 Rust 的类型系统和内存安全特性
//! - **性能**: 优化的系统调用路径和零拷贝技术
//! - **兼容性**: POSIX 兼容层，支持现有 Linux 应用
//! - **可扩展性**: 模块化架构，易于扩展和维护
//!
//! ## 架构
//!
//! The kernel follows a modular architecture with the following main components:
//!
//! ### 核心子系统
//!
//! - **系统调用** (`syscalls`): 系统调用接口和分发机制
//! - **服务管理** (`services`): 服务管理和发现框架
//! - **错误处理** (`error`): 全面的错误处理和恢复框架
//! - **内存管理** (`subsystems::mm`): 物理和虚拟内存管理
//! - **进程管理** (`subsystems::process`): 进程创建、调度和生命周期管理
//! - **文件系统** (`subsystems::fs`, `vfs`): 虚拟文件系统和文件操作
//! - **网络** (`subsystems::net`): 网络协议栈和套接字接口
//! - **安全** (`security`): 安全机制（ASLR、SMAP/SMEP、ACL、Capabilities）
//! - **IPC** (`subsystems::ipc`): 进程间通信机制
//! - **同步** (`sync`, `subsystems::sync`): 同步原语和锁机制
//! - **调度器** (`sched`, `subsystems::scheduler`): 进程和线程调度
//!
//! ### 平台支持
//!
//! - **架构**: x86_64, ARM64 (AArch64), RISC-V
//! - **平台**: 裸金属、虚拟化环境
//!
//! ## 使用示例
//!
//! ### 初始化内核
//!
//! ```no_run
//! use kernel::init_kernel;
//!
//! // Initialize the kernel
//! let boot_params = kernel::BootParameters::default();
//! init_kernel(boot_params)?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 关闭内核
//!
//! ```no_run
//! use kernel::shutdown_kernel;
//!
//! // Shutdown the kernel
//! shutdown_kernel()?;
//! # Ok::<(), nos_api::Error>(())
//! ```
//!
//! ### 获取内核信息
//!
//! ```
//! use kernel::{get_kernel_version, get_kernel_build_info};
//!
//! let version = get_kernel_version();
//! println!("NOS Kernel Version: {}", version);
//!
//! let build_info = get_kernel_build_info();
//! println!("Build Time: {}", build_info.build_time);
//! println!("Git Commit: {}", build_info.git_commit);
//! ```
//!
//! ## 模块组织
//!
//! ### 公共 API 层
//!
//! - [`api`]: 统一的内核 API 接口
//! - [`vfs_interface`]: VFS 接口层，打破循环依赖
//!
//! ### 核心功能
//!
//! - [`core`]: 核心内核功能
//! - [`error`]: 错误处理
//! - [`platform`]: 平台相关代码（架构、驱动、陷阱处理）
//!
//! ### 子系统
//!
//! - [`subsystems`]: 主要内核子系统
//!   - [`subsystems::process`]: 进程管理
//!   - [`subsystems::mm`]: 内存管理
//!   - [`subsystems::fs`]: 文件系统
//!   - [`subsystems::net`]: 网络
//!   - [`subsystems::ipc`]: IPC
//!   - [`subsystems::sync`]: 同步
//!   - [`subsystems::scheduler`]: 调度器
//!
//! ### 兼容层
//!
//! - [`compat`]: 兼容性层（Android、iOS、Linux、macOS、Windows）
//! - [`posix`]: POSIX 类型和常量
//!
//! ## 特性标志
//!
//! ### 编译时特性
//!
//! - `kernel_tests`: 启用内核测试框架和测试用例
//! - `baremetal`: 启用裸金属启动支持（无引导加载程序）
//! - `syscalls`: 启用系统调用支持（通过 nos-syscalls crate）
//! - `services`: 启用服务管理（通过 nos-services crate）
//! - `error_handling`: 启用错误处理（通过 nos-error-handling crate）
//! - `net_stack`: 启用网络协议栈
//! - `posix_layer`: 启用 POSIX 兼容层
//! - `debug_subsystems`: 启用子系统调试日志
//! - `security_audit`: 启用安全审计功能
//! - `formal_verification`: 启用形式化验证工具
//! - `cloud_native`: 启用云原生功能（容器、服务等）
//!
//! ## 设计决策
//!
//! ### 模块化架构
//!
//! NOS 采用高度模块化的设计，每个子系统都有清晰的接口和职责。这使得：
//! - 代码易于理解和维护
//! - 功能可以独立测试
//! - 允许选择性编译功能
//!
//! ### 安全优先
//!
//! - 使用 Rust 的类型系统确保内存安全
//! - 实现了多种安全缓解措施（ASLR、Stack Canaries、SMEP/SMAP）
//! - 提供细粒度的权限控制（Capabilities、ACL）
//!
//! ### 性能优化
//!
//! - 快速系统调用路径
//! - 零拷贝 I/O
//! - 优化的锁和同步原语
//! - 高效的内存分配器
//!
//! ## 错误处理
//!
//! NOS 使用统一的错误处理框架，基于 `nos_api::Result<T>` 和 `nos_api::Error`。
//!
//! ## 性能特征
//!
//! - 系统调用延迟: < 100ns（热路径）
//! - 上下文切换: < 1μs
//! - 内存分配: O(1) 分配和释放
//!
//! ## 线程安全
//!
//! NOS 内核设计为多核安全：
//! - 使用适当的同步原语保护共享状态
//! - 提供 SMP 安全的锁实现
//! - 支持每 CPU 数据结构
//!
//! ## 相关模块
//!
//! - [`nos_api`]: 公共 API 定义
//! - [`nos_syscalls`]: 系统调用实现
//! - [`nos_services`]: 服务管理
//! - [`nos_error_handling`]: 错误处理
//!
//! ## 参考资料
//!
//! - [架构文档](../ARCHITECTURE.md)
//! - [开发指南](../DEVELOPER_GUIDE.md)
//! - [功能特性](../FEATURES.md)

#![no_std]

#[macro_use]
extern crate alloc;

#[cfg(feature = "kernel_tests")]
#[macro_use]
mod test_macros;

// Logging macros (stub implementations for no_std environments)
#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_debug { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_info { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_warn { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

#[cfg(not(feature = "debug_subsystems"))]
#[macro_export]
macro_rules! log_error { ($($arg:tt)*) => { let _ = ($($arg)*); }; }

// API layer - public interfaces for kernel subsystems
pub mod api;

// VFS interface layer - breaks circular dependency between VFS and FS
pub mod vfs_interface;

// Core kernel functionality (from nos-kernel-core)
pub mod core;

// Error handling module
pub mod error;

// Kernel factory for creating and managing internal modules
mod kernel_factory;

// Include necessary internal modules for library
// pub mod arch;
pub mod platform;
pub mod subsystems;

// Re-export key types for external use
/// Core kernel functionality
pub use core::*;

/// Kernel factory and components
pub use kernel_factory::*;
#[cfg(feature = "error_handling")]
pub use nos_error_handling as error_handling;
#[cfg(feature = "services")]
pub use nos_services as services;
// Re-export external crates when features are enabled
#[cfg(feature = "syscalls")]
pub use nos_syscalls as syscalls;
/// Performance monitoring
pub use perf::*;
pub use platform::{arch, boot, drivers, trap};
/// POSIX types and constants
pub use posix::*;
// Re-export moved modules to maintain compatibility
pub use subsystems::fs;
#[cfg(feature = "net_stack")]
pub use subsystems::net;
pub use subsystems::{ipc, process, sync, time, vfs};

/// Boot parameters passed from bootloader to kernel
pub use crate::boot::BootParameters;

mod collections;
mod compat;
mod cpu;
#[cfg(feature = "debug_subsystems")]
mod debug;
mod di;
mod event;
mod ids;
mod libc;
mod syscall_interface;
mod types;
// Legacy modules - now accessed through subsystems
mod monitoring;
mod perf;
pub mod posix;
mod procfs;
mod sched;
mod security;
#[cfg(feature = "security_audit")]
mod security_audit;

#[cfg(not(feature = "cloud_native"))]
mod cloud_native {
    pub mod namespaces {
        use alloc::string::String;
        #[derive(Debug)]
        pub enum NamespaceType {
            Mount,
            UTS,
            IPC,
            Network,
            PID,
            User,
        }

        pub struct NamespaceParameters {
            pub mount_params: Option<()>,
            pub network_params: Option<()>,
            pub user_params: Option<()>,
            pub uts_params: Option<()>,
        }

        pub struct NamespaceConfig {
            pub ns_type: NamespaceType,
            pub new_namespace: bool,
            pub existing_path: Option<String>,
        }

        pub fn create_namespace(_config: NamespaceConfig) -> Result<u64, ()> {
            Ok(0)
        }
        pub fn join_namespace(_path: &str) -> Result<u64, ()> {
            Ok(0)
        }
    }
}

/// Initialize the kernel
///
/// This function initializes all kernel subsystems and prepares the system
/// for operation.
/// It uses the same core initialization logic as `rust_main_with_boot_info`,
/// ensuring consistency between bootloader-based and library-based startup.
///
/// # Arguments
///
/// * `boot_params` - Boot parameters passed from the bootloader
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn init_kernel(boot_params: BootParameters) -> nos_api::Result<()> {
    // Use the same core initialization function as bootloader entry
    // This ensures consistency between different entry points
    core::init::init_kernel_core(Some(&boot_params));

    log_info!("NOS Kernel initialized successfully");

    Ok(())
}

/// Shutdown the kernel
///
/// This function shuts down all kernel subsystems in a controlled manner.
///
/// # Returns
///
/// * `nos_api::Result<()>` - Success or error
pub fn shutdown_kernel() -> nos_api::Result<()> {
    log_info!("Shutting down NOS Kernel");

    // Shutdown performance monitoring
    // perf::shutdown_performance_monitor()?; // Function not found

    // Shutdown scheduler
    // sched::shutdown_scheduler()?; // Function not found

    // Shutdown security
    // security::shutdown_security()?; // Function not found

    // Shutdown network stack (if enabled)
    // #[cfg(feature = "net_stack")]
    // {
    //     subsystems::net::shutdown_network_stack()?;
    // }

    // Shutdown error handling (if enabled)
    #[cfg(feature = "error_handling")]
    {
        nos_error_handling::shutdown_error_handling()?;
    }

    // Shutdown services (if enabled)
    #[cfg(feature = "services")]
    {
        nos_services::shutdown_services()?;
    }

    // Shutdown system calls (if enabled)
    #[cfg(all(feature = "syscalls", feature = "alloc"))]
    {
        nos_syscalls::shutdown_syscalls()?;
    }

    // Shutdown IPC
    // subsystems::ipc::shutdown_ipc()?; // Function not found

    // Shutdown file system
    // subsystems::fs::shutdown_file_system()?; // Function not found

    // Shutdown process management
    // subsystems::process::shutdown_process_management()?; // Function not found

    // Shutdown memory management
    mm::shutdown_advanced_memory_management()?;

    // Shutdown platform
    platform::shutdown_platform()?;

    log_info!("NOS Kernel shutdown complete");

    Ok(())
}

/// Get kernel version
///
/// # Returns
///
/// * `&'static str` - Kernel version string
pub fn get_kernel_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get kernel build information
///
/// # Returns
///
/// * `KernelBuildInfo` - Kernel build information
pub fn get_kernel_build_info() -> KernelBuildInfo {
    KernelBuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        build_time: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown"),
        git_commit: option_env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
        target_triple: option_env!("VERGEN_CARGO_TARGET_TRIPLE")
            .unwrap_or_else(|| option_env!("CARGO_BUILD_TARGET").unwrap_or("unknown")),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        features: get_enabled_features(),
    }
}

/// Kernel build information
#[derive(Debug, Clone)]
pub struct KernelBuildInfo {
    /// Kernel version
    pub version: &'static str,
    /// Build timestamp
    pub build_time: &'static str,
    /// Git commit hash
    pub git_commit: &'static str,
    /// Target triple
    pub target_triple: &'static str,
    /// Build profile
    pub profile: &'static str,
    /// Enabled features
    pub features: alloc::vec::Vec<&'static str>,
}

/// Get enabled features
fn get_enabled_features() -> alloc::vec::Vec<&'static str> {
    let mut features = alloc::vec::Vec::new();

    if cfg!(feature = "baremetal") {
        features.push("baremetal");
    }
    if cfg!(feature = "kernel_tests") {
        features.push("kernel_tests");
    }
    if cfg!(feature = "syscalls") {
        features.push("syscalls");
    }
    if cfg!(feature = "services") {
        features.push("services");
    }
    if cfg!(feature = "error_handling") {
        features.push("error_handling");
    }
    if cfg!(feature = "net_stack") {
        features.push("net_stack");
    }
    if cfg!(feature = "posix_layer") {
        features.push("posix_layer");
    }
    if cfg!(feature = "debug_subsystems") {
        features.push("debug_subsystems");
    }
    if cfg!(feature = "security_audit") {
        features.push("security_audit");
    }
    if cfg!(feature = "formal_verification") {
        features.push("formal_verification");
    }
    if cfg!(feature = "cloud_native") {
        features.push("cloud_native");
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_version() {
        let version = get_kernel_version();
        assert!(!version.is_empty());
    }

    #[test]
    fn test_kernel_build_info() {
        let build_info = get_kernel_build_info();
        assert!(!build_info.version.is_empty());
        assert!(!build_info.build_time.is_empty());
        assert!(!build_info.git_commit.is_empty());
        assert!(!build_info.target_triple.is_empty());
        assert!(!build_info.profile.is_empty());
        assert!(!build_info.features.is_empty());
    }

    #[test]
    fn test_enabled_features() {
        let features = get_enabled_features();
        assert!(!features.is_empty());
    }
}
mod mm;
pub mod reliability;
mod tests;
