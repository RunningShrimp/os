//! 进程管理模块
//! 
//! 本模块提供进程相关的系统调用服务，包括：
//! - 进程创建和终止
//! - 进程状态管理
//! - 进程调度和优先级
//! - 进程间同步
//! 
//! 模块采用分层架构设计，通过服务接口与系统调用分发器集成。

pub mod handlers;
pub mod service;
pub mod types;

// 重新导出主要接口
pub use service::ProcessService;
pub use types::*;

use crate::syscalls::services::SyscallService;

/// 获取进程系统调用服务实例
/// 
/// 创建并返回一个进程系统调用服务的实例。
/// 
/// # 返回值
/// 
/// * `Box<dyn SyscallService>` - 进程系统调用服务实例
pub fn create_process_service() -> Box<dyn SyscallService> {
    Box::new(ProcessService::new())
}

/// 模块初始化函数
/// 
/// 初始化进程模块，注册必要的系统调用处理程序。
/// 
/// # 返回值
/// 
/// * `Result<(), crate::error::KernelError>` - 初始化结果
pub fn initialize_process_module() -> Result<(), crate::error::KernelError> {
    // TODO: 实现模块初始化逻辑
    crate::log_info!("Initializing process module");
    Ok(())
}