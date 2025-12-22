//! 系统调用模块
//! 
//! 本模块提供系统调用接口和分发机制，按功能域拆分为：
//! - core: 核心分发逻辑
//! - fs: 文件系统相关系统调用
//! - process: 进程管理相关系统调用
//! - network: 网络相关系统调用
//! - ipc: IPC相关系统调用
//! - signal: 信号处理
//! - async: 异步操作
//! - epoll: 事件轮询
//! - memory: 内存管理
//! - object: 对象管理

pub mod core;
pub mod fs;
pub mod process;
pub mod network;
pub mod ipc;
pub mod signal;
pub mod async_ops; // avoid keyword clash
pub mod epoll;
pub mod memory;
pub mod object;
pub mod common;
pub mod types;
pub mod security;
pub mod fast_path;
pub mod timerfd;
pub mod eventfd;
pub mod signalfd;
pub mod posix_fd;

// 重新导出主要接口
pub use core::*;
pub use fs::*;
pub use process::*;
pub use network::*;
pub use ipc::*;
pub use signal::*;
pub use async_ops::*;
pub use epoll::*;
pub use memory::*;
pub use object::*;
pub use common::*;
pub use types::*;
pub use security::*;
pub use fast_path::*;
