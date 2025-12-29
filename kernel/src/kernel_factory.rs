//! 内核工厂模块
//!
//! 本模块提供工厂模式创建和管理内核内部模块，减少lib.rs的直接依赖。

use alloc::{sync::Arc, vec::Vec};
use spin::Mutex;
use nos_api::{di::Container, Result};
use nos_services::core::{Service as ServiceTrait, ServiceInfo as ServiceInfoTrait, ServiceStats as ServiceStatsTrait};

// Import API adapter types for cleaner interfaces
use crate::api::adapter::{
    SyscallHandler, EventPublisher,
};

/// Simple service locator
pub struct ServiceLocator {
    container: Arc<Container>,
}

impl ServiceLocator {
    pub fn new(container: Arc<Container>) -> Self {
        Self { container }
    }
}

use crate::subsystems::syscalls::interface::{SyscallDispatcher, SyscallNumber, SyscallArgs, SyscallResult};
use crate::syscall_interface::ServiceManager;

/// 内核工厂，负责创建和管理内核组件
pub struct KernelFactory {
    container: Arc<Container>,
    components: KernelComponents,
}

/// 内核组件集合
pub struct KernelComponents {
    /// 系统调用分发器
    pub syscall_dispatcher: Option<Arc<dyn SyscallDispatcher>>,
    /// 服务管理器
    pub service_manager: Option<Arc<dyn ServiceManager>>,
    /// 事件发布器
    pub event_publisher: Option<Arc<dyn EventPublisher>>,
}

impl KernelFactory {
    /// 创建新的内核工厂
    pub fn new() -> Self {
        let container = Arc::new(Container::new());
        Self {
            container,
            components: KernelComponents {
                syscall_dispatcher: None,
                service_manager: None,
                event_publisher: None,
            },
        }
    }

    /// 使用自定义容器创建内核工厂
    pub fn with_container(container: Arc<Container>) -> Self {
        Self {
            container,
            components: KernelComponents {
                syscall_dispatcher: None,
                service_manager: None,
                event_publisher: None,
            },
        }
    }

    /// 获取服务定位器
    pub fn service_locator(&self) -> ServiceLocator {
        ServiceLocator::new(self.container.clone())
    }

    /// 获取容器
    pub fn container(&self) -> Arc<Container> {
        self.container.clone()
    }

    /// 初始化所有内核组件
    pub fn initialize_components(&mut self) -> Result<()> {
        // 初始化系统调用分发器
        self.initialize_syscall_dispatcher()?;

        // 初始化服务管理器
        self.initialize_service_manager()?;

        // 初始化事件发布器
        self.initialize_event_publisher()?;

        Ok(())
    }

    /// 初始化系统调用分发器
    fn initialize_syscall_dispatcher(&mut self) -> Result<()> {
        // 这里应该创建实际的系统调用分发器实现
        // 暂时使用占位符
        self.components.syscall_dispatcher = Some(Arc::new(PlaceholderSyscallDispatcher::new()));
        Ok(())
    }

    /// 初始化服务管理器
    fn initialize_service_manager(&mut self) -> Result<()> {
        // 这里应该创建实际的服务管理器实现
        // 暂时使用占位符
        self.components.service_manager = Some(Arc::new(PlaceholderServiceManager::new()));
        Ok(())
    }

    /// 初始化事件发布器
    fn initialize_event_publisher(&mut self) -> Result<()> {
        // 这里应该创建实际的事件发布器实现
        // 暂时使用占位符
        self.components.event_publisher = Some(Arc::new(PlaceholderEventPublisher::new()));
        Ok(())
    }

    /// 获取系统调用分发器
    pub fn get_syscall_dispatcher(&self) -> Option<Arc<dyn SyscallDispatcher>> {
        self.components.syscall_dispatcher.clone()
    }

    /// 获取服务管理器
    pub fn get_service_manager(&self) -> Option<Arc<dyn ServiceManager>> {
        self.components.service_manager.clone()
    }

    /// 获取事件发布器
    pub fn get_event_publisher(&self) -> Option<Arc<dyn EventPublisher>> {
        self.components.event_publisher.clone()
    }

    /// 获取所有组件
    pub fn get_components(&self) -> &KernelComponents {
        &self.components
    }
}

/// 占位符系统调用分发器
struct PlaceholderSyscallDispatcher {
    // 实际实现中这里会有具体字段
}

impl PlaceholderSyscallDispatcher {
    pub fn new() -> Self {
        Self {}
    }
}

impl SyscallDispatcher for PlaceholderSyscallDispatcher {
    fn dispatch(&mut self, _syscall_num: SyscallNumber, _args: &SyscallArgs) -> Result<SyscallResult> {
        // 占位符实现
        Ok(SyscallResult::success(-1))
    }

    fn get_stats(&self) -> crate::subsystems::syscalls::interface::SyscallStats {
        // 占位符实现
        crate::subsystems::syscalls::interface::SyscallStats {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            avg_execution_time_ns: 0,
        }
    }

    fn register_handler(
        &mut self,
        _syscall_num: SyscallNumber,
        _handler: Box<dyn SyscallHandler>,
    ) -> Result<()> {
        // 占位符实现
        Ok(())
    }

    fn unregister_handler(&mut self, _syscall_num: SyscallNumber) {
        // 占位符实现
    }

    fn handler_count(&self) -> usize {
        // 占位符实现
        0
    }

    fn list_handlers(&self) -> Vec<(usize, &str)> {
        // 占位符实现
        Vec::new()
    }
}

/// 占位符服务管理器
struct PlaceholderServiceManager {
    // 实际实现中这里会有具体字段
}

impl PlaceholderServiceManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl nos_services::core::ServiceManager for PlaceholderServiceManager {
    fn register_service(
        &mut self,
        _name: &str,
        _service: Box<dyn ServiceTrait>,
    ) -> Result<u32> {
        // 占位符实现
        Ok(0)
    }

    fn unregister_service(&mut self, _id: u32) -> Result<()> {
        // 占位符实现
        Ok(())
    }

    fn start_service(&mut self, _id: u32) -> Result<()> {
        // 占位符实现
        Ok(())
    }

    fn stop_service(&mut self, _id: u32) -> Result<()> {
        // 占位符实现
        Ok(())
    }

    fn get_service(&self, _id: u32) -> Option<&dyn ServiceTrait> {
        // 占位符实现
        None
    }

    fn get_service_by_name(&self, _name: &str) -> Option<&dyn ServiceTrait> {
        // 占位符实现
        None
    }

    fn list_services(&self) -> Vec<ServiceInfoTrait> {
        // 占位符实现
        Vec::new()
    }

    fn get_stats(&self) -> ServiceStatsTrait {
        // 占位符实现
        ServiceStatsTrait::default()
    }
}

/// 占位符事件发布器
struct PlaceholderEventPublisher {
    // 实际实现中这里会有具体字段
}

impl PlaceholderEventPublisher {
    pub fn new() -> Self {
        Self {}
    }
}

impl EventPublisher for PlaceholderEventPublisher {
    fn publish(&self, _event: alloc::sync::Arc<BasicEvent>) -> Result<()> {
        // 占位符实现
        Ok(())
    }

    fn publish_batch(
        &self,
        _events: Vec<alloc::sync::Arc<BasicEvent>>,
    ) -> Result<()> {
        // 占位符实现
        Ok(())
    }
}

/// 全局内核工厂实例
static mut GLOBAL_KERNEL_FACTORY: Option<KernelFactory> = None;
static KERNEL_FACTORY_INIT: Mutex<bool> = Mutex::new(false);

/// 初始化全局内核工厂
pub fn init_kernel_factory() -> Result<()> {
    let mut is_init = KERNEL_FACTORY_INIT.lock();
    if *is_init {
        return Ok(());
    }

    let mut factory = KernelFactory::new();
    factory.initialize_components()?;

    unsafe {
        GLOBAL_KERNEL_FACTORY = Some(factory);
    }
    *is_init = true;
    Ok(())
}

/// 获取全局内核工厂
pub fn get_kernel_factory() -> &'static mut KernelFactory {
    unsafe {
        GLOBAL_KERNEL_FACTORY
            .as_mut()
            .expect("Kernel factory not initialized")
    }
}

/// 获取系统调用分发器
pub fn get_syscall_dispatcher() -> Option<Arc<dyn SyscallDispatcher>> {
    get_kernel_factory().get_syscall_dispatcher()
}

/// 获取服务管理器
pub fn get_service_manager() -> Option<Arc<dyn ServiceManager>> {
    get_kernel_factory().get_service_manager()
}

/// 获取事件发布器
pub fn get_event_publisher() -> Option<Arc<dyn EventPublisher>> {
    get_kernel_factory().get_event_publisher()
}
