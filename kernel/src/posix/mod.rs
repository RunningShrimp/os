//! # POSIX 兼容层
//!
//! 提供 POSIX 标准类型、常量和接口，确保与 POSIX 兼容的应用程序兼容。
//!
//! ## 概述
//!
//! POSIX 兼容层提供：
//! - **标准类型**: POSIX 定义的类型（pid_t, uid_t, gid_t 等）
//! - **文件标志**: 文件打开和访问标志（O_RDONLY, O_WRONLY 等）
//! - **文件权限**: Unix 文件权限位
//! - **信号**: POSIX 信号定义和处理
//! - **线程**: pthread 兼容的线程接口
//! - **IPC**: System V IPC 兼容接口
//! - **定时器**: POSIX 定时器
//!
//! ## 主要组件
//!
//! ### 类型定义
//!
//! - [`types`]: POSIX 基本类型定义
//! - [`stat`]: 文件状态类型
//! - [`fcntl`]: 文件控制标志
//! - [`open_flags`]: 文件打开标志
//! - [`file_modes`]: 文件权限模式
//! - [`seek`]: 文件寻址常量
//!
//! ### 高级功能
//!
//! - [`thread`]: 线程支持和 pthreads
//! - [`sync`]: 线程同步原语（mutex, rwlock, condition）
//! - [`advanced_signal`]: 高级信号处理
//! - [`advanced_thread`]: 高级线程功能
//! - [`realtime`]: 实时扩展
//! - [`timer`]: POSIX 定时器
//! - [`aio`]: 异步 I/O
//! - [`mqueue`]: 消息队列
//! - [`semaphore`]: 信号量
//! - [`shm`]: 共享内存
//! - [`session`]: 会话和进程组
//!
//! ## 使用示例
//!
//! ### 文件操作
//!
//! ```c
//! #include <fcntl.h>
//! #include <unistd.h>
//! #include <sys/stat.h>
//!
//! int main() {
//!     // 打开文件
//!     int fd = open("/etc/hostname", O_RDONLY);
//!
//!     // 获取文件状态
//!     struct stat st;
//!     fstat(fd, &st);
//!
//!     // 关闭文件
//!     close(fd);
//!     return 0;
//! }
//! ```
//!
//! ### 线程创建
//!
//! ```c
//! #include <pthread.h>
//!
//! void* thread_func(void* arg) {
//!     // 线程代码
//!     return NULL;
//! }
//!
//! int main() {
//!     pthread_t thread;
//!     pthread_create(&thread, NULL, thread_func, NULL);
//!     pthread_join(thread, NULL);
//!     return 0;
//! }
//! ```
//!
//! ## 设计决策
//!
//! ### 标准兼容
//!
//! 严格遵循 POSIX 标准：
//! - IEEE Std 1003.1
//! - IEEE Std 1003.1b（实时扩展）
//! - IEEE Std 1003.1c（线程）
//!
//! ### 模块化组织
//!
//! 按功能组织为子模块：
//! - 便于查找和维护
//! - 支持条件编译
//! - 清晰的依赖关系
//!
//! ## 性能特征
//!
//! - **类型映射**: 零开销，类型别名
//! - **系统调用**: 与原生系统调用相同性能
//! - **线程**: 用户级线程，上下文切换 < 1μs
//!
//! ## 兼容性级别
//!
//! NOS POSIX 兼容性：
//! - **Base**: POSIX.1-2008 基础
//! - **Realtime**: 实时扩展（部分）
//! - **Threads**: 线程扩展（大部分）
//!
//! ## 相关模块
//!
//! - [`crate::subsystems::process`]: 进程和线程管理
//! - [`crate::subsystems::syscalls`]: 系统调用实现
//! - [`crate::libc`]: libc 接口

//! POSIX Types and Constants
//!
//! Standard types and constants for POSIX compliance
//!
//! This module is organized into submodules for better maintainability:

pub mod aio;
pub mod fcntl;
pub mod file_modes;
pub mod open_flags;
pub mod seek;
pub mod stat;
pub mod types;

// ============================================================================
// Public Exports
// ============================================================================

pub use self::{aio::*, fcntl::*, file_modes::*, open_flags::*, seek::*, stat::*, types::*};

// ============================================================================
// Thread support
// ============================================================================

pub mod sync;
pub mod thread;

// ============================================================================
// IPC and Synchronization modules
// ============================================================================

pub mod advanced_signal;
pub mod advanced_thread;
pub mod fd_flags;
pub mod mqueue;
pub mod realtime;
pub mod security;
pub mod semaphore;
pub mod session;
pub mod shm;
pub mod timer;

pub use self::{mqueue::*, semaphore::*, shm::*, thread::*, timer::*};
// ============================================================================
// Re-export from libc for compatibility
// ============================================================================
pub use crate::libc::interface::size_t;
pub use crate::libc::ssize_t;

// ============================================================================
// Memory Protection Constants
// ============================================================================
pub const PROT_READ: i32 = 0x1;
pub const PROT_WRITE: i32 = 0x2;
pub const PROT_EXEC: i32 = 0x4;
pub const PROT_NONE: i32 = 0x0;

// ============================================================================
// Mapping Flags
// ============================================================================
pub const MAP_SHARED: i32 = 0x01;
pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_ANONYMOUS: i32 = 0x20;
pub const MAP_FIXED: i32 = 0x10;

// ============================================================================
// Semaphore Types
// ============================================================================
pub type SemT = *mut core::ffi::c_void;

// ============================================================================
// Shared Memory Types
// ============================================================================
#[repr(C)]
pub struct ShmidDs {
    pub shm_perm: IpcPerm,
    pub shm_segsz: usize,
    pub shm_atime: i64,
    pub shm_dtime: i64,
    pub shm_ctime: i64,
    pub shm_cpid: i32,
    pub shm_lpid: i32,
    pub shm_nattch: u64,
}

#[repr(C)]
pub struct IpcPerm {
    pub key: i32,
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u32,
    pub seq: u16,
}

// ============================================================================
// Timer Types
// ============================================================================
pub type TimerT = i32;

#[repr(C)]
pub struct SigEvent {
    pub sigev_value: usize,
    pub sigev_signo: i32,
    pub sigev_notify: i32,
}

#[repr(C)]
pub struct Itimerspec {
    pub it_interval: Timespec,
    pub it_value: Timespec,
}

// ============================================================================
// Signal Types
// ============================================================================
#[repr(C)]
pub struct SigSet {
    pub bits: [u64; 1],
}

#[repr(C)]
pub struct SigInfoT {
    pub si_signo: i32,
    pub si_errno: i32,
    pub si_code: i32,
    pub _pad: [i32; 29],
}

pub const SIGRTMIN: i32 = 34;
pub const SIGRTMAX: i32 = 64;
