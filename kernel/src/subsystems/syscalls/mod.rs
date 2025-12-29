//! # 系统调用子系统
//!
//! 提供用户空间和内核空间之间的接口，实现系统调用分发和处理。
//!
//! ## 概述
//!
//! 系统调用子系统是内核最重要的接口之一，提供：
//! - **系统调用接口**: 用户程序访问内核服务的标准接口
//! - **参数验证**: 安全的参数传递和验证
//! - **快速路径**: 热门系统调用的优化实现
//! - **异步操作**: 支持异步和批量系统调用
//! - **错误处理**: 统一的错误处理和返回
//!
//! ## 主要组件
//!
//! ### 核心模块
//!
//! - [`core`]: 核心分发逻辑和系统调用注册表
//! - [`dispatch`]: 系统调用分发器
//! - [`types`]: 系统调用类型定义
//!
//! ### 功能域
//!
//! - [`fs`]: 文件系统相关系统调用（open, read, write, close 等）
//! - [`process`]: 进程管理相关系统调用（fork, exec, wait 等）
//! - [`memory`]: 内存管理（mmap, munmap, brk 等）
//! - [`network`]: 网络系统调用（socket, bind, listen 等）
//! - [`ipc`]: IPC 相关系统调用（shm, msgq, sem 等）
//! - [`signal`]: 信号处理（kill, sigaction 等）
//!

// Import kernel prelude for common types
use crate::prelude::*;
// ### 高级特性
//
// - [`async_ops`]: 异步系统调用操作
// - [`epoll`]: 事件轮询机制
// - [`fast_path`]: 快速系统调用路径
// - [`optimization`]: 性能优化框架
// - [`security`]: 系统调用安全验证
// - [`eventfd`]: 事件通知
// - [`signalfd`]: 信号文件描述符
// - [`timerfd`]: 定时器文件描述符
//
// ## 架构
//
// ```
// 用户空间
//     ├── 系统调用触发 (syscall 指令)
// 系统调用处理
//     ├── 参数复制和验证
//     ├── 快速路径检查 (fast_path)
//     ├── 分发器 (dispatch)
//     │   ├── FS 处理器 (fs)
//     │   ├── 进程处理器 (process)
//     │   ├── 内存处理器 (memory)
//     │   ├── 网络处理器 (network)
//     │   ├── IPC 处理器 (ipc)
//     │   └── 信号处理器 (signal)
//     └── 结果返回
// ```
//
// ## 使用示例
//
// ### 发起系统调用
//
// 系统调用通常通过 libc wrapper 调用：
//
// ```c
// // 用户空间代码
// #include <unistd.h>
// #include <sys/types.h>
//
// int main() {
//     // write 系统调用
//     write(1, "Hello, World!\n", 13);
//
//     // fork 系统调用
//     pid_t pid = fork();
//
//     return 0;
// }
// ```
//
// ### 注册新系统调用
//
// ```no_run
// use kernel::subsystems::syscalls::{SyscallHandler, SyscallId, SyscallResult);
//
// struct MySyscall;
//
// impl SyscallHandler for MySyscall {
//     fn handle(&self, args: &[usize]) -> SyscallResult<i64>{
//         // 处理系统调用
//         Ok(0)
//     }
// }
//
// // 注册系统调用
// // register_syscall(SyscallId::Custom, MySyscall);
// ```
//
// ## 设计决策
//
// ### 按功能域拆分
//
// 系统调用按功能域拆分为多个模块：
// - 代码组织清晰
// - 便于维护和测试
// - 支持条件编译
//
// ### 快速路径优化
//
// 热门系统调用使用快速路径：
// - 减少参数复制
// - 内联关键代码
// - 避免 switch 开销
//
// ## 性能特征
//
// - **系统调用延迟**:
//   - 热路径（getpid, gettimeofday）: < 50ns
//   - 普通路径（read, write）: < 100ns
//   - 复杂路径（fork, exec）: < 10μs
//
// ## POSIX 兼容性
//
// NOS 提供 POSIX 兼容的系统调用接口：
// - Linux 系统调用号
// - 标准错误码
// - 标准返回值
//
// ## 线程安全
//
// 系统调用处理是线程安全的：
// - 每个系统调用独立的执行上下文
// - 适当的锁保护共享资源
// - 支持多核并行执行
//
// ## 相关模块
//
// - [`crate::api::syscall`]: 系统调用 API 定义
// - [`nos_syscalls`]: 系统调用 crate
// - [`crate::subsystems::process`]: 进程管理
// - [`crate::subsystems::fs`]: 文件系统
// - [`crate::subsystems::mm`]: 内存管理

pub mod aio;
pub mod api;
pub mod async_ops; // avoid keyword clash
pub mod common;
pub mod core;
pub mod dispatch;
pub mod epoll;
pub mod eventfd;
pub mod fast_path;
pub mod fs;
pub mod glib;
pub mod ipc;
pub mod memory;
pub mod network;
pub mod object;
pub mod optimization;
pub mod posix_fd;
pub mod process;
pub mod security;
pub mod signal;
pub mod signalfd;
pub mod signal_service;
pub mod thread;
pub mod thread_futex;
pub mod timerfd;
pub mod types;

// 重新导出主要接口
pub use core::*;

pub use aio::*;
pub use async_ops::*;
pub use common::*;
pub use dispatch::*;
pub use epoll::*;
pub use fast_path::*;
pub use fs::*;
pub use glib::*;
pub use ipc::*;
pub use memory::*;
pub use network::*;
// Note: memory module not glob-imported due to dispatch function ambiguity
pub use object::*;
pub use process::*;
pub use security::*;
pub use signal::*;
pub use thread::*;
pub use thread_futex::*;
pub use types::*;

// Export syscall constants from api module
pub use api::syscall_id::{SYS_BATCH, SYS_CLOSE, SYS_GETPID, SYS_READ, SYS_WRITE};

pub mod interface;
pub mod services;
