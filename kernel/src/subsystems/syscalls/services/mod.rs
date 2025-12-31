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
//! use alloc::sync::Arc;
//!
//! use kernel::syscalls::services::*;
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
use alloc::{boxed::Box, string::String, string::ToString, sync::Arc, vec::Vec};

// Import nos_api types
use nos_api::{Result, Error};

// Import syscall_interface traits
use crate::syscall_interface::{ServiceManager, ServiceStats};

// Import local traits and types
use crate::subsystems::syscalls::services::traits::Service as LocalService;

// Submodule declarations
pub mod dispatcher;
pub mod registry;
pub mod traits;

// 导出所有公共接口
#[allow(unused_imports)]
pub use dispatcher::*;
pub use dispatcher::{
    CachedServiceInfo, DispatchResult, DispatchStats, DispatcherConfig, DispatcherError,
    SyscallDispatcher,
};
#[allow(unused_imports)]
pub use registry::*;
pub use registry::{
    DependencyGraph, ServiceAsAny, ServiceEntry, ServiceMetadata, ServiceRegistry,
    ServiceRegistryError, ServiceType,
};
#[allow(unused_imports)]
pub use traits::*;
// 重新导出常用的类型和特征，方便外部使用
pub use traits::{
    Service as BaseService, ServiceFactory, ServiceHealth, ServiceLifecycle, ServiceProvider,
    ServiceStatus, SyscallService,
};

/// 模块初始化函数
///
/// 初始化服务管理系统的所有组件。
///
/// # 返回值
///
/// * `(Arc<ServiceRegistry>, Arc<SyscallDispatcher>)` - 注册表和分发器实例
/// * `Err(Error)` - 初始化失败
pub fn init_service_system()
-> nos_api::Result<(alloc::sync::Arc<ServiceRegistry>, alloc::sync::Arc<SyscallDispatcher>)> {
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
    let (registry, dispatcher) =
        init_service_system().expect("Failed to initialize service system");

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
    /// * `Result<u64>` - 系统调用结果
    pub fn handle_syscall(&self, syscall_number: u32, args: &[u64]) -> nos_api::Result<u64> {
        let result = self.dispatcher.dispatch(syscall_number, args, None)
            .map_err(|e| Error::SystemError(format!("Dispatch error: {}", e)))?;

        if result.success {
            Ok(result.return_value)
        } else {
            Err(result
                .error
                .map(|e| Error::SystemError(format!("Service error: {}", e)))
                .unwrap_or_else(|| Error::SystemError("Unknown syscall error".into())))
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
    /// * `Result<()>` - 注册结果
    pub fn register_service(
        &self,
        service: Arc<dyn BaseService>,
        metadata: ServiceMetadata,
    ) -> Result<()> {
        self.registry
            .register_service(service, metadata)
            .map_err(|e| Error::SystemError(format!("Service registration failed: {:?}", e)))
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
    /// * `Result<()>` - 启动结果
    pub fn start_all_services(&self) -> Result<()> {
        let _startup_order = self
            .registry
            .calculate_startup_order()
            .map_err(|e| Error::SystemError(format!("Failed to calculate startup order: {:?}", e)))?;

        // GH-#1071: Implement service startup
        // See: https://github.com/npos/kernel/issues/1071
        // For now, just return success
        Ok(())
    }

    /// 停止所有服务
    ///
    /// 按相反的依赖顺序停止所有服务。
    ///
    /// # 返回值
    ///
    /// * `Result<()>` - 停止结果
    pub fn stop_all_services(&self) -> Result<()> {
        let _startup_order = self
            .registry
            .calculate_startup_order()
            .map_err(|e| Error::SystemError(format!("Failed to calculate startup order: {:?}", e)))?;

        // GH-#1072: Implement service shutdown
        // See: https://github.com/npos/kernel/issues/1072
        // For now, just return success
        Ok(())
    }
}

impl ServiceManager for ServiceSystem {
    /// Register a new service
    fn register_service(&mut self, service: Arc<dyn crate::syscall_interface::Service>) -> Result<()> {
        // Convert from syscall_interface::Service to LocalService
        // This is a placeholder implementation - you may need to create an adapter
        let local_service: Arc<dyn LocalService> = Arc::new(ServiceAdapter::new(service));

        let metadata = ServiceMetadata {
            service_type: ServiceType::Custom,
            priority: 50,
            is_syscall_service: true,
            tags: vec![local_service.name().to_string()],
        };

        // Convert the registry error to a nos_api error
        match self.registry.register_service(local_service, metadata) {
            Ok(()) => Ok(()),
            Err(e) => Err(Error::SystemError(format!("Service registration failed: {:?}", e))),
        }
    }

    /// Unregister a service by name
    fn unregister_service(&mut self, name: &str) -> Result<()> {
        match self.registry.unregister_service(name) {
            Ok(()) => Ok(()),
            Err(e) => Err(Error::SystemError(format!("Service unregistration failed: {:?}", e))),
        }
    }

    /// Get a service by name
    fn get_service(&self, _name: &str) -> Option<Arc<dyn crate::syscall_interface::Service>> {
        // GH-#1073: Implement proper conversion from LocalService to syscall_interface::Service
        // See: https://github.com/npos/kernel/issues/1073
        // For now, return None since we cannot directly convert between trait objects
        None
    }

    /// List all registered services
    fn list_services(&self) -> Vec<&str> {
        // Convert Vec<String> to Vec<&str> by collecting references
        self.registry.list_services()
            .into_iter()
            .map(|s| {
                // Leak the string to get a &'static str
                // Note: This is a simple workaround. A better approach would be to
                // store the strings in the ServiceSystem struct with proper lifetime management
                let leaked: &'static str = Box::leak(s.into_boxed_str());
                leaked
            })
            .collect()
    }

    /// Get service statistics
    fn get_stats(&self) -> ServiceStats {
        let registry_services = self.registry.list_services();
        ServiceStats {
            total_services: registry_services.len() as u64,
            active_services: 0, // GH-#1074: Track active services
            // See: https://github.com/npos/kernel/issues/1074
            failed_services: 0, // GH-#1075: Track failed services
            // See: https://github.com/npos/kernel/issues/1075
        }
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
        Self { dispatch_stats, registered_services, cache_size }
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
            (self.dispatch_stats.successful_dispatches as f64
                / self.dispatch_stats.total_dispatches as f64)
                * 100.0
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

/// Public type aliases for system call types
pub type SyscallNumber = u32;
pub type SyscallArgs = [u64; 6];
pub type SyscallResult = isize;

/// Thread-safe wrapper for external Service trait
///
/// Since the external Service trait doesn't implement Send + Sync,
/// we need to wrap it in a struct that does.
struct ThreadSafeServiceWrapper {
    service: Arc<dyn crate::syscall_interface::Service>,
}

// Manually implement Send and Sync for the wrapper
unsafe impl Send for ThreadSafeServiceWrapper {}
unsafe impl Sync for ThreadSafeServiceWrapper {}

/// Service adapter that implements both local and external Service traits
///
/// This adapter allows converting between syscall_interface::Service and
/// the local services::traits::Service trait.
pub struct ServiceAdapter {
    inner: Arc<ThreadSafeServiceWrapper>,
}

// Note: We are not implementing the external Service trait for ServiceAdapter
// because it would require implementing methods that don't exist in the local Service trait
// and would create circular dependencies.

impl ServiceAdapter {
    /// Create a new adapter from syscall_interface::Service
    pub fn new(service: Arc<dyn crate::syscall_interface::Service>) -> Self {
        let wrapper = Arc::new(ThreadSafeServiceWrapper { service });
        Self { inner: wrapper }
    }

    /// Create an adapter from local Service
    pub fn from_local(_service: Arc<dyn LocalService>) -> Self {
        // This is a placeholder implementation
        // In a real implementation, you would need to convert between the traits
        panic!("ServiceAdapter::from_local not implemented - need proper trait conversion");
    }

    /// Convert nos_api::Error to UnifiedError
    fn convert_error(err: nos_api::Error) -> crate::error::UnifiedError {
        use crate::error::UnifiedError;

        match err {
            nos_api::Error::InvalidArgument(_) => UnifiedError::InvalidArgument,
            nos_api::Error::InvalidState(_) => UnifiedError::InvalidState,
            nos_api::Error::NotImplemented(_) => UnifiedError::NotSupported,
            nos_api::Error::NotFound(_) => UnifiedError::NotFound,
            nos_api::Error::PermissionDenied(_) => UnifiedError::PermissionDenied,
            nos_api::Error::Busy(_) => UnifiedError::ResourceBusy,
            nos_api::Error::OutOfMemory => UnifiedError::OutOfMemory,
            nos_api::Error::IoError(_) => UnifiedError::IoError,
            nos_api::Error::Io(_) => UnifiedError::IoError,
            nos_api::Error::NetworkError(_) => UnifiedError::IoError,
            nos_api::Error::ProtocolError(_) => UnifiedError::InvalidInput,
            nos_api::Error::Timeout => UnifiedError::TimedOut,
            nos_api::Error::ConnectionError(_) => UnifiedError::IoError,
            nos_api::Error::ParseError(_) => UnifiedError::InvalidInput,
            nos_api::Error::ConfigError(_) => UnifiedError::InvalidInput,
            nos_api::Error::ServiceError(_) => UnifiedError::InvalidOperation,
            nos_api::Error::SystemError(_) => UnifiedError::InvalidOperation,
            nos_api::Error::CircularDependency(_) => UnifiedError::InvalidOperation,
            nos_api::Error::EventError(_) => UnifiedError::InvalidOperation,
            nos_api::Error::BadAddress => UnifiedError::InvalidAddress,
            // nos_api::Error doesn't have AlreadyExists, map to ResourceBusy
            nos_api::Error::NotSupported(_) => UnifiedError::NotSupported,
            _ => UnifiedError::Unknown,
        }
    }
}

impl LocalService for ServiceAdapter {
    fn name(&self) -> &str {
        // Since we can't access the service through Arc, use a placeholder name
        "service_adapter"
    }

    fn version(&self) -> &str {
        "1.0.0" // Placeholder
    }

    fn description(&self) -> &str {
        "Service adapter"
    }

    fn initialize(&mut self) -> crate::error::Result<()> {
        // For the local Service trait, initialize is a no-op
        // since the external service handles its own initialization
        Ok(())
    }

    fn start(&mut self) -> crate::error::Result<()> {
        // For external service, start is a no-op since it doesn't have start method
        // In a real implementation, this would call some appropriate method
        Ok(())
    }

    fn stop(&mut self) -> crate::error::Result<()> {
        // For external service, stop is a no-op since it doesn't have stop method
        // In a real implementation, this would call shutdown method
        Ok(())
    }

    fn destroy(&mut self) -> crate::error::Result<()> {
        // For external service, destroy is a no-op since it doesn't have destroy method
        // In a real implementation, this would call shutdown method
        Ok(())
    }

    fn status(&self) -> ServiceStatus {
        ServiceStatus::Running
    }

    fn dependencies(&self) -> Vec<&str> {
        Vec::new()
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

impl core::fmt::Debug for ServiceAdapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ServiceAdapter")
            .field("name", &self.name())
            .field("version", &self.version())
            .finish()
    }
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
