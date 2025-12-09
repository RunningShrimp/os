//! 内存管理模块
//! 
//! 本模块提供内存管理相关的系统调用服务，包括：
//! - 内存映射和取消映射
//! - 内存分配和释放
//! - 内存保护和管理
//! - 虚拟内存操作
//! 
//! 模块采用分层架构设计，通过服务接口与系统调用分发器集成。

pub mod handlers;
pub mod service;
pub mod types;

// 重新导出主要接口
pub use service::MemoryService;
pub use types::*;

use alloc::boxed::Box;
use crate::syscalls::services::SyscallService;

/// 获取内存管理系统调用服务实例
/// 
/// 创建并返回一个内存管理系统调用服务的实例。
/// 
/// # 返回值
/// 
/// * `Box<dyn SyscallService>` - 内存管理系统调用服务实例
pub fn create_mm_service() -> Box<dyn SyscallService> {
    Box::new(MemoryService::new())
}

/// 模块初始化函数
/// 
/// 初始化内存管理模块，注册必要的系统调用处理程序。
/// 
/// # 返回值
/// 
/// * `Result<(), crate::error_handling::unified::KernelError>` - 初始化结果
pub fn initialize_mm_module() -> Result<(), crate::error_handling::unified::KernelError> {
    // TODO: 实现模块初始化逻辑
    crate::log_info!("Initializing memory management module");
    Ok(())
}