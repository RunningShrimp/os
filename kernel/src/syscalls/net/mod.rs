//! 网络模块
//! 
//! 本模块提供网络相关的系统调用服务，包括：
//! - 套接字创建和管理
//! - 网络连接和数据传输
//! - 网络配置和统计
//! - 网络协议支持
//! 
//! 模块采用分层架构设计，通过服务接口与系统调用分发器集成。

pub mod handlers;
pub mod service;
pub mod types;

// 重新导出主要接口
pub use service::NetworkService;
pub use types::*;

use alloc::boxed::Box;
use crate::syscalls::services::SyscallService;

/// 获取网络系统调用服务实例
/// 
/// 创建并返回一个网络系统调用服务的实例。
/// 
/// # 返回值
/// 
/// * `Box<dyn SyscallService>` - 网络系统调用服务实例
pub fn create_net_service() -> Box<dyn SyscallService> {
    Box::new(NetworkService::new())
}

/// 模块初始化函数
/// 
/// 初始化网络模块，注册必要的系统调用处理程序。
/// 
/// # 返回值
/// 
/// * `Result<(), crate::error_handling::unified::KernelError>` - 初始化结果
pub fn initialize_net_module() -> Result<(), crate::error_handling::unified::KernelError> {
    // TODO: 实现模块初始化逻辑
    crate::log_info!("Initializing network module");
    Ok(())
}