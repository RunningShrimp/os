//! 服务特征定义模块
//!
//! 本模块定义了服务管理系统的核心特征接口，包括：
//! - Service: 基础服务特征
//! - SyscallService: 系统调用服务特征
//! - ServiceLifecycle: 服务生命周期管理特征
//!
//! 这些特征为依赖注入和服务发现机制提供了统一的接口规范。

use alloc::{boxed::Box, string::String, vec::Vec};
use core::any::Any;

use crate::error_handling::unified::KernelError;

/// 基础服务特征
///
/// 所有服务都必须实现此特征，提供基本的服务标识和状态管理功能。
/// 支持服务的初始化、启动、停止和销毁等生命周期操作。
pub trait Service: Send + Sync {
    /// 获取服务的Any引用，用于向下转型
    ///
    /// 这个方法允许将服务实例向下转型为具体类型。
    /// 实现者应该返回self的Any引用。
    fn as_any(&self) -> &dyn Any;

    /// 获取服务的SyscallService引用（如果适用）
    ///
    /// 这个方法允许将服务向下转型为SyscallService类型。
    /// 默认实现返回None，需要实现SyscallService trait的服务应覆盖此方法。
    fn as_syscall_service(&self) -> Option<&dyn SyscallService> {
        None
    }
    /// 获取服务名称
    ///
    /// 返回服务的唯一标识符，用于服务注册和发现。
    /// 服务名称应该是唯一的，并且能够描述服务的功能。
    fn name(&self) -> &str;

    /// 获取服务版本
    ///
    /// 返回服务的版本信息，用于服务兼容性检查。
    /// 版本格式建议使用语义化版本（semantic versioning）。
    fn version(&self) -> &str;

    /// 获取服务描述
    ///
    /// 返回服务的功能描述，用于服务文档和管理。
    fn description(&self) -> &str;

    /// 初始化服务
    ///
    /// 在服务启动前进行必要的初始化工作。
    /// 如果初始化失败，服务将无法启动。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 初始化成功
    /// * `Err(KernelError)` - 初始化失败，包含错误信息
    fn initialize(&mut self) -> Result<(), KernelError>;

    /// 启动服务
    ///
    /// 启动服务并开始提供服务。
    /// 服务必须先成功初始化才能启动。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 启动成功
    /// * `Err(KernelError)` - 启动失败，包含错误信息
    fn start(&mut self) -> Result<(), KernelError>;

    /// 停止服务
    ///
    /// 停止服务并清理运行时资源。
    /// 停止后的服务可以通过start()重新启动。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 停止成功
    /// * `Err(KernelError)` - 停止失败，包含错误信息
    fn stop(&mut self) -> Result<(), KernelError>;

    /// 销毁服务
    ///
    /// 完全销毁服务并释放所有资源。
    /// 销毁后的服务无法重新使用，需要重新创建实例。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 销毁成功
    /// * `Err(KernelError)` - 销毁失败，包含错误信息
    fn destroy(&mut self) -> Result<(), KernelError>;

    /// 检查服务状态
    ///
    /// 返回服务当前的状态信息。
    fn status(&self) -> ServiceStatus;

    /// 获取服务依赖
    ///
    /// 返回此服务依赖的其他服务名称列表。
    /// 服务注册器会根据依赖关系确定启动顺序。
    fn dependencies(&self) -> Vec<&str>;
}

/// 系统调用服务特征
///
/// 扩展基础服务特征，添加系统调用特有的功能。
/// 系统调用服务负责处理特定类别的系统调用请求。
pub trait SyscallService: Service {
    /// 获取支持的系统调用号列表
    ///
    /// 返回此服务能够处理的系统调用号列表。
    /// 系统调用分发器使用此信息进行路由。
    fn supported_syscalls(&self) -> Vec<u32>;

    /// 处理系统调用
    ///
    /// 处理指定的系统调用请求。
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 系统调用号
    /// * `args` - 系统调用参数
    ///
    /// # 返回值
    ///
    /// * `Ok(u64)` - 系统调用执行成功，返回结果值
    /// * `Err(KernelError)` - 系统调用执行失败，包含错误信息
    fn handle_syscall(&mut self, syscall_number: u32, args: &[u64]) -> Result<u64, KernelError>;

    /// 检查是否支持指定的系统调用
    ///
    /// 快速检查服务是否支持指定的系统调用。
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 要检查的系统调用号
    ///
    /// # 返回值
    ///
    /// * `true` - 支持该系统调用
    /// * `false` - 不支持该系统调用
    fn supports_syscall(&self, syscall_number: u32) -> bool {
        self.supported_syscalls().contains(&syscall_number)
    }

    /// 获取系统调用服务优先级
    ///
    /// 返回服务的优先级，用于服务注册时的排序。
    /// 数值越小优先级越高。
    fn priority(&self) -> u32 {
        100 // 默认优先级
    }

    /// 获取服务的SyscallService引用
    ///
    /// 这个方法允许将服务向下转型为SyscallService类型。
    /// SyscallService trait的实现者必须提供此方法。
    fn as_syscall_service(&self) -> Option<&dyn SyscallService>;
}

/// 服务生命周期管理特征
///
/// 提供更细粒度的服务生命周期控制。
/// 支持服务的暂停、恢复和重启等高级操作。
pub trait ServiceLifecycle: Service {
    /// 暂停服务
    ///
    /// 临时暂停服务，但保持服务状态。
    /// 暂停的服务可以通过resume()恢复。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 暂停成功
    /// * `Err(KernelError)` - 暂停失败，包含错误信息
    fn pause(&mut self) -> Result<(), KernelError>;

    /// 恢复服务
    ///
    /// 恢复被暂停的服务。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 恢复成功
    /// * `Err(KernelError)` - 恢复失败，包含错误信息
    fn resume(&mut self) -> Result<(), KernelError>;

    /// 重启服务
    ///
    /// 重启服务，先停止再启动。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 重启成功
    /// * `Err(KernelError)` - 重启失败，包含错误信息
    fn restart(&mut self) -> Result<(), KernelError> {
        self.stop()?;
        self.start()
    }

    /// 获取服务健康状态
    ///
    /// 返回服务的详细健康状态信息。
    fn health_check(&self) -> ServiceHealth;
}

/// 服务状态枚举
///
/// 定义服务可能的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// 未初始化状态
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 已初始化，未启动
    Initialized,
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 暂停中
    Pausing,
    /// 已暂停
    Paused,
    /// 错误状态
    Error,
}

/// 服务健康状态
///
/// 包含服务的详细健康信息。
#[derive(Debug, Clone)]
pub struct ServiceHealth {
    /// 服务是否健康
    pub is_healthy: bool,
    /// 健康检查时间戳
    pub check_timestamp: u64,
    /// 健康状态描述
    pub status_message: String,
    /// 额外的健康指标
    pub metrics: Vec<(String, String)>,
}

impl ServiceHealth {
    /// 创建新的健康状态
    ///
    /// # 参数
    ///
    /// * `is_healthy` - 是否健康
    /// * `status_message` - 状态描述
    pub fn new(is_healthy: bool, status_message: String) -> Self {
        Self {
            is_healthy,
            check_timestamp: 0, // 需要从外部设置
            status_message,
            metrics: Vec::new(),
        }
    }

    /// 添加健康指标
    ///
    /// # 参数
    ///
    /// * `name` - 指标名称
    /// * `value` - 指标值
    pub fn add_metric(&mut self, name: String, value: String) {
        self.metrics.push((name, value));
    }
}

/// 服务工厂特征
///
/// 用于创建服务实例的工厂特征。
/// 支持依赖注入和服务实例的动态创建。
pub trait ServiceFactory: Send + Sync {
    /// 创建服务实例
    ///
    /// 创建一个新的服务实例。
    ///
    /// # 参数
    ///
    /// * `dependencies` - 依赖的服务实例
    ///
    /// # 返回值
    ///
    /// * `Ok(Box<dyn Service>)` - 创建成功，返回服务实例
    /// * `Err(KernelError)` - 创建失败，包含错误信息
    fn create_service(
        &self,
        dependencies: Vec<Box<dyn Service>>,
    ) -> Result<Box<dyn Service>, KernelError>;

    /// 获取工厂支持的服务类型
    ///
    /// 返回此工厂能够创建的服务类型名称。
    fn service_type(&self) -> &str;
}

/// 服务提供者特征
///
/// 用于服务注册和发现的提供者特征。
/// 管理服务的注册、查找和生命周期。
pub trait ServiceProvider: Send + Sync {
    /// 注册服务
    ///
    /// 将服务实例注册到提供者中。
    ///
    /// # 参数
    ///
    /// * `service` - 要注册的服务实例
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 注册成功
    /// * `Err(KernelError)` - 注册失败，包含错误信息
    fn register_service(&mut self, service: Box<dyn Service>) -> Result<(), KernelError>;

    /// 查找服务
    ///
    /// 根据服务名称查找已注册的服务。
    ///
    /// # 参数
    ///
    /// * `name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(Some(Box<dyn Service>))` - 找到服务，返回服务实例
    /// * `Ok(None)` - 未找到服务
    /// * `Err(KernelError)` - 查找失败，包含错误信息
    fn get_service(&self, name: &str) -> Result<Option<Box<dyn Service>>, KernelError>;

    /// 列出所有已注册的服务
    ///
    /// 返回所有已注册服务的名称列表。
    fn list_services(&self) -> Vec<String>;

    /// 移除服务
    ///
    /// 从提供者中移除指定的服务。
    ///
    /// # 参数
    ///
    /// * `name` - 要移除的服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 移除成功
    /// * `Err(KernelError)` - 移除失败，包含错误信息
    fn unregister_service(&mut self, name: &str) -> Result<(), KernelError>;
}
