//! 进程间通信模块
//! 
//! 本模块提供进程间通信相关的系统调用服务，包括：
//! - 管道和命名管道
//! - 共享内存
//! - 消息队列
//! - 信号量
//! 
//! 模块采用分层架构设计，通过服务接口与系统调用分发器集成。

pub mod handlers;
pub mod service;
pub mod types;

// 重新导出主要接口
pub use service::IpcService;
pub use types::*;

use alloc::boxed::Box;
use crate::syscalls::services::SyscallService;

/// 获取IPC系统调用服务实例
/// 
/// 创建并返回一个IPC系统调用服务的实例。
/// 
/// # 返回值
/// 
/// * `Box<dyn SyscallService>` - IPC系统调用服务实例
pub fn create_ipc_service() -> Box<dyn SyscallService> {
    Box::new(IpcService::new())
}

/// 模块初始化函数
/// 
/// 初始化IPC模块，注册必要的系统调用处理程序。
/// 
/// # 返回值
/// 
/// * `Result<(), crate::error_handling::unified::KernelError>` - 初始化结果
pub fn initialize_ipc_module() -> Result<(), crate::error_handling::unified::KernelError> {
    // TODO: 实现模块初始化逻辑
    crate::log_info!("Initializing IPC module");
    Ok(())
}