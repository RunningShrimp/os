//! 内核工厂模块
//!
//! 本模块提供工厂模式创建和管理内核内部模块，减少lib.rs的直接依赖。

use nos_api::{Result, ServiceLocator, Container};
use nos_api::interfaces::{SyscallDispatcher, ServiceManager, EventPublisher};
use alloc::sync::Arc;
use alloc::vec::Vec;

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
    fn dispatch(&self, _syscall_num: usize, _args: &[usize]) -> isize {
        // 占位符实现
        -1
    }
    
    fn get_stats(&self) -> nos_api::interfaces::SyscallStats {
        // 占位符实现
        nos_api::interfaces::SyscallStats {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            avg_execution_time_ns: 0,
            calls_by_type: alloc::collections::BTreeMap::new(),
        }
    }
    
    fn register_handler(&mut self, _syscall_num: usize, _handler: alloc::sync::Arc<dyn nos_api::interfaces::SyscallHandler>) -> Result<()> {
        // 占位符实现
        Ok(())
    }
    
    fn unregister_handler(&mut self, _syscall_num: usize) -> Result<()> {
        // 占位符实现
        Ok(())
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

impl ServiceManager for PlaceholderServiceManager {
    fn register_service(&mut self, _service: alloc::sync::Arc<dyn nos_api::interfaces::Service>) -> Result<()> {
        // 占位符实现
        Ok(())
    }
    
    fn unregister_service(&mut self, _service_id: &str) -> Result<()> {
        // 占位符实现
        Ok(())
    }
    
    fn get_service(&self, _service_id: &str) -> Option<alloc::sync::Arc<dyn nos_api::interfaces::Service>> {
        // 占位符实现
        None
    }
    
    fn list_services(&self) -> Vec<nos_api::interfaces::ServiceInfo> {
        // 占位符实现
        Vec::new()
    }
    
    fn get_stats(&self) -> nos_api::interfaces::ServiceStats {
        // 占位符实现
        nos_api::interfaces::ServiceStats {
            registered_services: 0,
            running_services: 0,
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ns: 0,
        }
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
    fn publish(&self, _event: alloc::sync::Arc<dyn nos_api::event::Event>) -> Result<()> {
        // 占位符实现
        Ok(())
    }
    
    fn publish_batch(&self, _events: Vec<alloc::sync::Arc<dyn nos_api::event::Event>>) -> Result<()> {
        // 占位符实现
        Ok(())
    }
}

/// 全局内核工厂实例
static mut GLOBAL_KERNEL_FACTORY: Option<KernelFactory> = None;
static KERNEL_FACTORY_INIT: core::sync::Mutex<bool> = core::sync::Mutex::new(false);

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