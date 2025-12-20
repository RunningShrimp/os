//! 信号模块
//! 
//! 本模块提供信号相关的系统调用服务，包括：
//! - 信号发送和处理
//! - 信号掩码管理
//! - 信号处理程序注册
//! - 信号集操作
//! 
//! 模块采用分层架构设计，通过服务接口与系统调用分发器集成。

pub mod handlers;
pub mod service;
pub mod types;

// 重新导出主要接口
pub use service::SignalService;
pub use types::*;

use crate::syscalls::services::SyscallService;

/// 获取信号系统调用服务实例
/// 
/// 创建并返回一个信号系统调用服务的实例。
/// 
/// # 返回值
/// 
/// * `Box<dyn SyscallService>` - 信号系统调用服务实例
pub fn create_signal_service() -> Box<dyn SyscallService> {
    Box::new(SignalService::new())
}

/// 模块初始化函数
/// 
/// 初始化信号模块，注册必要的系统调用处理程序。
/// 
/// # 返回值
/// 
/// * `Result<(), crate::error::KernelError>` - 初始化结果
pub fn initialize_signal_module() -> Result<(), crate::error::KernelError> {
    // TODO: 实现模块初始化逻辑
    crate::log_info!("Initializing signal module");
    Ok(())
}