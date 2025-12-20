//! 文件系统模块
//! 
//! 本模块提供文件系统相关的系统调用服务，包括：
//! - 文件和目录操作
//! - 文件描述符管理
//! - 文件权限和属性
//! - 虚拟文件系统接口
//! 
//! 模块采用分层架构设计，通过服务接口与系统调用分发器集成。

pub mod handlers;
pub mod service;
pub mod types;

// 重新导出主要接口
pub use service::FileSystemService;
pub use types::*;

use crate::syscalls::services::SyscallService;

/// 获取文件系统系统调用服务实例
/// 
/// 创建并返回一个文件系统系统调用服务的实例。
/// 
/// # 返回值
/// 
/// * `Box<dyn SyscallService>` - 文件系统系统调用服务实例
pub fn create_fs_service() -> Box<dyn SyscallService> {
    Box::new(FileSystemService::new())
}

/// 模块初始化函数
/// 
/// 初始化文件系统模块，注册必要的系统调用处理程序。
/// 
/// # 返回值
/// 
/// * `Result<(), crate::error::KernelError>` - 初始化结果
pub fn initialize_fs_module() -> Result<(), crate::error::KernelError> {
    // TODO: 实现模块初始化逻辑
    crate::log_info!("Initializing filesystem module");
    Ok(())
}