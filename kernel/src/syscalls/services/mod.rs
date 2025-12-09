//! 服务管理模块
//! 
//! 本模块提供了完整的服务管理架构，包括：
//! - 服务特征定义（traits）
//! - 服务注册表（registry）
//! - 系统调用分发器（dispatcher）
//! 
//! 这个模块是重构架构的核心，实现了依赖注入和服务发现机制。
//! 
//! # 模块结构
//! 
//! ```text
//! services/
//! ├── mod.rs          # 模块入口，统一导出
//! ├── traits.rs        # 服务特征定义
//! ├── registry.rs      # 服务注册表
//! └── dispatcher.rs    # 系统调用分发器
//! ```
//! 
//! # 使用示例
//! 
//! ```rust
//! use kernel::syscalls::services::*;
//! use alloc::sync::Arc;
//! 
//! // 创建服务注册表
//! let registry = Arc::new(ServiceRegistry::new());
//! 
//! // 创建分发器
//! let dispatcher = SyscallDispatcher::with_default_config(registry.clone());
//! 
//! // 注册服务
//! // let service = Box::new(MySystemService::new());
//! // registry.register_service(service, ServiceMetadata::default())?;
//! 
//! // 分发系统调用
//! // let result = dispatcher.dispatch(syscall_number, &args)?;
//! ```

// Import commonly used alloc types
use alloc::boxed::Box;
use alloc::string::String;

// Submodule declarations
pub mod traits;
pub mod registry;
pub mod dispatcher;

// 导出所有公共接口
pub use traits::*;
pub use registry::*;
pub use dispatcher::*;

// 重新导出常用的类型和特征，方便外部使用
pub use traits::{
    Service, SyscallService, ServiceLifecycle, ServiceFactory, ServiceProvider,
    ServiceStatus, ServiceHealth,
};
pub use registry::ServiceType;

pub use registry::{
    ServiceRegistry, ServiceEntry, ServiceMetadata, DependencyGraph,
    ServiceRegistryError, ServiceAsAny,
};

pub use dispatcher::{
    SyscallDispatcher, DispatchResult, DispatchStats, DispatcherConfig,
    CachedServiceInfo, DispatcherError,
};

/// 模块初始化函数
/// 
/// 初始化服务管理系统的所有组件。
/// 
/// # 返回值
/// 
/// * `(Arc<ServiceRegistry>, Arc<SyscallDispatcher>)` - 注册表和分发器实例
/// * `Err(KernelError)` - 初始化失败
pub fn init_service_system() -> Result<(alloc::sync::Arc<ServiceRegistry>, alloc::sync::Arc<SyscallDispatcher>), crate::error_handling::unified::KernelError> {
    use alloc::sync::Arc;
    
    // 创建服务注册表
    let registry = Arc::new(ServiceRegistry::new());
    
    // 创建系统调用分发器
    let dispatcher = Arc::new(SyscallDispatcher::with_default_config(registry.clone()));
    
    Ok((registry, dispatcher))
}

/// 创建默认配置的服务系统
/// 
/// 使用默认配置创建完整的服务管理系统。
/// 
/// # 返回值
/// 
/// * `ServiceSystem` - 包装的服务系统实例
pub fn create_default_service_system() -> ServiceSystem {
    let (registry, dispatcher) = init_service_system()
        .expect("Failed to initialize service system");
    
    ServiceSystem::new(registry, dispatcher)
}

/// 服务系统包装器
/// 
/// 提供服务管理的统一接口。
pub struct ServiceSystem {
    /// 服务注册表
    registry: alloc::sync::Arc<ServiceRegistry>,
    /// 系统调用分发器
    dispatcher: alloc::sync::Arc<SyscallDispatcher>,
}

impl ServiceSystem {
    /// 创建新的服务系统
    /// 
    /// # 参数
    /// 
    /// * `registry` - 服务注册表
    /// * `dispatcher` - 系统调用分发器
    pub fn new(
        registry: alloc::sync::Arc<ServiceRegistry>,
        dispatcher: alloc::sync::Arc<SyscallDispatcher>,
    ) -> Self {
        Self { registry, dispatcher }
    }
    
    /// 获取注册表引用
    pub fn registry(&self) -> &alloc::sync::Arc<ServiceRegistry> {
        &self.registry
    }
    
    /// 获取分发器引用
    pub fn dispatcher(&self) -> &alloc::sync::Arc<SyscallDispatcher> {
        &self.dispatcher
    }
    
    /// 分发系统调用的便捷方法
    /// 
    /// # 参数
    /// 
    /// * `syscall_number` - 系统调用号
    /// * `args` - 系统调用参数
    /// 
    /// # 返回值
    /// 
    /// * `Result<u64, KernelError>` - 系统调用结果
    pub fn handle_syscall(&self, syscall_number: u32, args: &[u64]) -> Result<u64, crate::error_handling::unified::KernelError> {
        let result = self.dispatcher.dispatch(syscall_number, args)?;
        
        if result.success {
            Ok(result.return_value)
        } else {
            Err(result.error.unwrap_or(crate::error_handling::unified::KernelError::Unknown("Unknown syscall error".into())))
        }
    }
    
    /// 注册服务的便捷方法
    /// 
    /// # 参数
    /// 
    /// * `service` - 要注册的服务
    /// * `metadata` - 服务元数据
    /// 
    /// # 返回值
    /// 
    /// * `Result<(), KernelError>` - 注册结果
    pub fn register_service(
        &self,
        service: Box<dyn Service>,
        metadata: ServiceMetadata,
    ) -> Result<(), crate::error_handling::unified::KernelError> {
        self.registry.register_service(service, metadata)
    }
    
    /// 获取系统统计信息
    /// 
    /// # 返回值
    /// 
    /// * `SystemStats` - 系统统计信息
    pub fn get_system_stats(&self) -> SystemStats {
        SystemStats {
            dispatch_stats: self.dispatcher.get_stats(),
            registered_services: self.registry.list_services(),
            cache_size: self.dispatcher.cache_size(),
        }
    }
    
    /// 启动所有服务
    /// 
    /// 按依赖顺序启动所有已注册的服务。
    /// 
    /// # 返回值
    /// 
    /// * `Result<(), KernelError>` - 启动结果
    pub fn start_all_services(&self) -> Result<(), crate::error_handling::unified::KernelError> {
        let startup_order = self.registry.calculate_startup_order()?;
        
        for service_name in startup_order {
            // 这里需要获取服务实例并启动
            // 由于所有权问题，实际实现可能需要不同的方法
            // 暂时为空实现
        }
        
        Ok(())
    }
    
    /// 停止所有服务
    /// 
    /// 按相反的依赖顺序停止所有服务。
    /// 
    /// # 返回值
    /// 
    /// * `Result<(), KernelError>` - 停止结果
    pub fn stop_all_services(&self) -> Result<(), crate::error_handling::unified::KernelError> {
        let startup_order = self.registry.calculate_startup_order()?;
        
        // 按相反顺序停止服务
        for service_name in startup_order.iter().rev() {
            // 这里需要获取服务实例并停止
            // 暂时为空实现
        }
        
        Ok(())
    }
}

/// 系统统计信息
/// 
/// 包含整个服务系统的统计信息。
#[derive(Debug, Clone)]
pub struct SystemStats {
    /// 分发统计
    pub dispatch_stats: DispatchStats,
    /// 已注册的服务列表
    pub registered_services: alloc::vec::Vec<String>,
    /// 缓存大小
    pub cache_size: usize,
}

impl SystemStats {
    /// 创建新的系统统计信息
    pub fn new(
        dispatch_stats: DispatchStats,
        registered_services: alloc::vec::Vec<String>,
        cache_size: usize,
    ) -> Self {
        Self {
            dispatch_stats,
            registered_services,
            cache_size,
        }
    }
    
    /// 获取总服务数
    pub fn total_services(&self) -> usize {
        self.registered_services.len()
    }
    
    /// 获取总系统调用数
    pub fn total_syscalls(&self) -> u64 {
        self.dispatch_stats.total_dispatches
    }
    
    /// 获取成功率
    /// 
    /// 返回系统调用的成功率（百分比）
    pub fn success_rate(&self) -> f64 {
        if self.dispatch_stats.total_dispatches == 0 {
            0.0
        } else {
            (self.dispatch_stats.successful_dispatches as f64 / self.dispatch_stats.total_dispatches as f64) * 100.0
        }
    }
    
    /// 获取缓存命中率
    /// 
    /// 返回缓存的命中率（百分比）
    pub fn cache_hit_rate(&self) -> f64 {
        let total_requests = self.dispatch_stats.cache_hits + self.dispatch_stats.cache_misses;
        if total_requests == 0 {
            0.0
        } else {
            (self.dispatch_stats.cache_hits as f64 / total_requests as f64) * 100.0
        }
    }
}

/// 模块版本信息
pub const MODULE_VERSION: &str = "1.0.0";
/// 模块名称
pub const MODULE_NAME: &str = "services";

/// 获取模块信息
/// 
/// # 返回值
/// 
/// * `(&str, &str)` - (模块名称, 版本)
pub fn get_module_info() -> (&'static str, &'static str) {
    (MODULE_NAME, MODULE_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_info() {
        let (name, version) = get_module_info();
        assert_eq!(name, "services");
        assert_eq!(version, "1.0.0");
    }
    
    #[test]
    fn test_system_stats() {
        let dispatch_stats = DispatchStats::default();
        let services = vec!["service1".to_string(), "service2".to_string()];
        let stats = SystemStats::new(dispatch_stats, services, 10);
        
        assert_eq!(stats.total_services(), 2);
        assert_eq!(stats.total_syscalls(), 0);
        assert_eq!(stats.success_rate(), 0.0);
        assert_eq!(stats.cache_hit_rate(), 0.0);
    }
}