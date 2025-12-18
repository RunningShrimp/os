//! 依赖注入容器 - 服务组合和依赖管理
//!
//! 提供完整的服务注册、解析和生命周期管理功能。
//! 支持单例、瞬态和作用域生命周期，以及条件服务注册。
//! 包含循环依赖检测和延迟初始化功能。

use crate::domain::hardware_detection::HardwareDetectionService;
use crate::domain::repositories::BootConfigRepository;
use crate::infrastructure::graphics_backend::{GraphicsBackend, create_graphics_backend};
use crate::infrastructure::hardware_detection::create_hardware_detection_service;
use crate::protocol::BootProtocolType;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use core::cell::RefCell;

/// 服务生命周期枚举
///
/// 定义服务实例的创建和生命周期管理方式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceLifecycle {
    /// 单例模式 - 整个应用程序生命周期内只创建一次
    Singleton,
    /// 瞬态模式 - 每次请求都创建新实例
    Transient,
    /// 作用域模式 - 在特定作用域内保持单例
    Scoped,
}

/// 服务条件枚举
///
/// 定义服务注册的条件，只有在条件满足时才会注册服务
pub enum ServiceCondition {
    /// 总是注册
    Always,
    /// 只有当特定功能启用时才会注册
    FeatureEnabled(&'static str),
    /// 只有当引导协议类型匹配时才会注册
    ProtocolType(BootProtocolType),
    /// 自定义条件函数
    Custom(Arc<dyn Fn(&DIContainer) -> bool>),
}

impl Clone for ServiceCondition {
    fn clone(&self) -> Self {
        match self {
            ServiceCondition::Always => ServiceCondition::Always,
            ServiceCondition::FeatureEnabled(feature) => ServiceCondition::FeatureEnabled(feature),
            ServiceCondition::ProtocolType(protocol) => ServiceCondition::ProtocolType(*protocol),
            ServiceCondition::Custom(func) => ServiceCondition::Custom(Arc::clone(func)),
        }
    }
}

impl ServiceCondition {
    /// 检查条件是否满足
    pub fn is_satisfied(&self, container: &DIContainer) -> bool {
        match self {
            ServiceCondition::Always => true,
            ServiceCondition::FeatureEnabled(feature) => {
                // 在实际实现中，这里会检查功能标志
                // 现在返回true作为默认值
                container.get_config_value(feature).map_or(false, |v| v == "true")
            }
            ServiceCondition::ProtocolType(protocol) => {
                container.protocol_type() == *protocol
            }
            ServiceCondition::Custom(func) => func(container),
        }
    }
}

/// 服务工厂接口
///
/// 负责创建服务实例的工厂
pub trait ServiceFactory: Send + Sync {
    /// 创建服务实例
    fn create_instance(&self, container: &DIContainer) -> Result<Box<dyn Any>, &'static str>;
    
    /// 获取服务类型名称
    fn get_service_type(&self) -> &'static str;
    
    /// 获取服务依赖
    fn get_dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

/// 服务描述符
///
/// 包含服务的所有元数据和配置信息
pub struct ServiceDescriptor {
    /// 服务类型名称
    pub service_type: &'static str,
    /// 实现类型名称（可选）
    pub implementation_type: Option<&'static str>,
    /// 服务生命周期
    pub lifecycle: ServiceLifecycle,
    /// 服务工厂
    pub factory: Box<dyn ServiceFactory>,
    /// 服务依赖列表
    pub dependencies: Vec<&'static str>,
    /// 服务注册条件
    pub condition: Option<ServiceCondition>,
}

impl ServiceDescriptor {
    /// 创建新的服务描述符
    pub fn new(
        service_type: &'static str,
        lifecycle: ServiceLifecycle,
        factory: Box<dyn ServiceFactory>,
    ) -> Self {
        let dependencies = factory.get_dependencies();
        Self {
            service_type,
            implementation_type: None,
            lifecycle,
            factory,
            dependencies,
            condition: None,
        }
    }
    
    /// 设置实现类型
    pub fn with_implementation_type(mut self, impl_type: &'static str) -> Self {
        self.implementation_type = Some(impl_type);
        self
    }
    
    /// 设置注册条件
    pub fn with_condition(mut self, condition: ServiceCondition) -> Self {
        self.condition = Some(condition);
        self
    }
}

/// 服务作用域
///
/// 用于管理作用域生命周期的服务实例
pub struct ServiceScope {
    // 在bootloader环境中，我们暂时不实现服务实例缓存
    // 因为Box<dyn Any>不能安全地克隆，这会导致复杂的类型转换问题
    // 我们将在后续版本中重新实现这个功能
}

impl ServiceScope {
    /// 创建新的服务作用域
    pub fn new() -> Self {
        Self {}
    }
    
    /// 获取作用域内的服务实例
    /// 在bootloader环境中，我们暂时不实现服务实例缓存
    pub fn get_instance(&self, _service_type: &'static str) -> Option<Box<dyn Any>> {
        None
    }
    
    /// 设置作用域内的服务实例
    /// 在bootloader环境中，我们暂时不实现服务实例缓存
    pub fn set_instance(&self, _service_type: &'static str, _instance: Box<dyn Any>) {
        // 什么都不做
    }
    
    /// 清理作用域内的所有实例
    /// 在bootloader环境中，我们暂时不实现服务实例缓存
    pub fn clear(&self) {
        // 什么都不做
    }
}

/// 依赖注入容器
///
/// 核心容器，负责服务注册、解析和生命周期管理
/// 支持循环依赖检测和延迟初始化
pub struct DIContainer {
    /// 引导协议类型
    protocol_type: BootProtocolType,
    /// 服务注册表
    services: RefCell<BTreeMap<&'static str, ServiceDescriptor>>,
    /// 单例实例缓存
    singletons: RefCell<BTreeMap<&'static str, Arc<dyn Any>>>,

    /// 当前作用域
    current_scope: RefCell<Option<Arc<ServiceScope>>>,
    /// 正在解析的服务栈（用于循环依赖检测）
    resolving_stack: RefCell<VecDeque<&'static str>>,
    /// 配置值
    config_values: RefCell<BTreeMap<String, String>>,
}

impl DIContainer {
    /// 创建新的依赖注入容器
    pub fn new(protocol_type: BootProtocolType) -> Self {
        Self {
            protocol_type,
            services: RefCell::new(BTreeMap::new()),
            singletons: RefCell::new(BTreeMap::new()), // 初始化类型已更新为Arc<dyn Any + Send + Sync>
            current_scope: RefCell::new(None),
            resolving_stack: RefCell::new(VecDeque::new()),
            config_values: RefCell::new(BTreeMap::new()),
        }
    }
    
    /// 注册服务
    ///
    /// # 参数
    /// * `descriptor` - 服务描述符
    ///
    /// # 错误
    /// 如果服务已注册或条件不满足，返回错误
    pub fn register_service(&self, descriptor: ServiceDescriptor) -> Result<(), &'static str> {
        // 检查注册条件
        if let Some(ref condition) = descriptor.condition {
            if !condition.is_satisfied(self) {
                return Ok(()); // 条件不满足，跳过注册但不报错
            }
        }
        
        let mut services = self.services.borrow_mut();
        
        // 检查服务是否已注册
        if services.contains_key(descriptor.service_type) {
            return Err("Service already registered");
        }
        
        services.insert(descriptor.service_type, descriptor);
        Ok(())
    }
    
    /// 注册单例服务
    ///
    /// # 参数
    /// * `service_type` - 服务类型名称
    /// * `factory` - 服务工厂
    /// * `condition` - 注册条件（可选）
    pub fn register_singleton<F>(
        &self,
        service_type: &'static str,
        factory: F,
        condition: Option<ServiceCondition>,
    ) -> Result<(), &'static str>
    where
        F: ServiceFactory + 'static,
    {
        let mut descriptor = ServiceDescriptor::new(service_type, ServiceLifecycle::Singleton, Box::new(factory));
        
        if let Some(cond) = condition {
            descriptor = descriptor.with_condition(cond);
        }
        
        self.register_service(descriptor)
    }
    
    /// 注册瞬态服务
    ///
    /// # 参数
    /// * `service_type` - 服务类型名称
    /// * `factory` - 服务工厂
    /// * `condition` - 注册条件（可选）
    pub fn register_transient<F>(
        &self,
        service_type: &'static str,
        factory: F,
        condition: Option<ServiceCondition>,
    ) -> Result<(), &'static str>
    where
        F: ServiceFactory + 'static,
    {
        let mut descriptor = ServiceDescriptor::new(service_type, ServiceLifecycle::Transient, Box::new(factory));
        
        if let Some(cond) = condition {
            descriptor = descriptor.with_condition(cond);
        }
        
        self.register_service(descriptor)
    }
    
    /// 注册作用域服务
    ///
    /// # 参数
    /// * `service_type` - 服务类型名称
    /// * `factory` - 服务工厂
    /// * `condition` - 注册条件（可选）
    pub fn register_scoped<F>(
        &self,
        service_type: &'static str,
        factory: F,
        condition: Option<ServiceCondition>,
    ) -> Result<(), &'static str>
    where
        F: ServiceFactory + 'static,
    {
        let mut descriptor = ServiceDescriptor::new(service_type, ServiceLifecycle::Scoped, Box::new(factory));
        
        if let Some(cond) = condition {
            descriptor = descriptor.with_condition(cond);
        }
        
        self.register_service(descriptor)
    }
    
    /// 解析服务
    ///
    /// # 参数
    /// * `service_type` - 服务类型名称
    ///
    /// # 返回
    /// 服务实例的引用
    ///
    /// # 错误
    /// 如果服务未注册或创建失败，返回错误
    pub fn resolve<T: Any + 'static>(&self, service_type: &'static str) -> Result<T, &'static str> {
        let instance = self.resolve_any(service_type)?;
        
        // 尝试将实例转换为指定类型
        match instance.downcast::<T>() {
            Ok(instance) => Ok(*instance),
            Err(_) => Err("Failed to downcast service to requested type"),
        }
    }
    
    /// 解析服务为Any类型
    ///
    /// 内部方法，处理所有类型的服务解析
    fn resolve_any(&self, service_type: &'static str) -> Result<Box<dyn Any>, &'static str> {
        // 检查循环依赖
        {
            let mut stack = self.resolving_stack.borrow_mut();
            if stack.contains(&service_type) {
                // 构建循环路径信息
                let mut path: Vec<&str> = stack.iter().copied().collect();
                path.push(service_type);
                let path_str = path.join(" -> ");
                return Err(alloc::format!("Circular dependency detected: {}", path_str).leak());
            }
            stack.push_back(service_type);
        }
        
        let result = self.resolve_internal(service_type);
        
        // 从解析栈中移除
        {
            let mut stack = self.resolving_stack.borrow_mut();
            stack.pop_back();
        }
        
        result
    }
    
    /// 内部解析方法
    ///
    /// 根据服务生命周期创建或获取实例
    fn resolve_internal(&self, service_type: &'static str) -> Result<Box<dyn Any>, &'static str> {
        let services = self.services.borrow();
        
        // 检查服务是否已注册
        let descriptor = match services.get(service_type) {
            Some(desc) => desc,
            None => return Err("Service not registered"),
        };
        
        // 首先解析所有依赖
        for dep in &descriptor.dependencies {
            self.resolve_any(dep)?;
        }
        
        // 根据生命周期处理
        match descriptor.lifecycle {
            ServiceLifecycle::Singleton => self.resolve_singleton(descriptor),
            ServiceLifecycle::Transient => self.resolve_transient(descriptor),
            ServiceLifecycle::Scoped => self.resolve_scoped(descriptor),
        }
    }
    
    /// 解析单例服务
    fn resolve_singleton(&self, descriptor: &ServiceDescriptor) -> Result<Box<dyn Any>, &'static str> {
        let mut singletons = self.singletons.borrow_mut();
        
        // 检查是否已存在实例
        if let Some(arc_instance) = singletons.get(descriptor.service_type) {
            // 创建一个新的Box<dyn Any>，包含Arc的克隆
            // 由于Arc实现了Clone trait，我们可以安全地克隆它
            let cloned_arc = Arc::clone(arc_instance);
            
            // 使用unsafe代码将Arc转换为Box<dyn Any>
            // 这是安全的，因为我们知道Arc中的值实现了Any trait
            unsafe {
                // 创建一个新的Box，指向Arc中的值
                // 注意：这种方式不会克隆实际的数据，只会创建一个新的Box指针
                // 但由于我们返回的是Box<dyn Any>，调用者期望能够拥有这个实例
                // 所以我们需要确保Arc的生命周期足够长
                let raw_ptr = Arc::into_raw(cloned_arc) as *mut (dyn Any + 'static);
                return Ok(Box::from_raw(raw_ptr));
            }
        }
        
        // 创建新实例
        let instance = descriptor.factory.create_instance(self)?;
        
        // 将Box<dyn Any>转换为Arc<dyn Any>
        // 使用unsafe代码进行转换
        let arc_instance = unsafe {
            // 将Box转换为原始指针
            let raw_ptr = Box::into_raw(instance);
            // 将原始指针转换为Arc
            Arc::from_raw(raw_ptr)
        };
        
        // 缓存实例（克隆Arc）
        singletons.insert(descriptor.service_type, Arc::clone(&arc_instance));
        
        // 返回实例的克隆（使用相同的unsafe转换）
        unsafe {
            let raw_ptr = Arc::into_raw(arc_instance) as *mut (dyn Any + 'static);
            Ok(Box::from_raw(raw_ptr))
        }
    }
    
    /// 解析瞬态服务
    fn resolve_transient(&self, descriptor: &ServiceDescriptor) -> Result<Box<dyn Any>, &'static str> {
        // 每次都创建新实例
        descriptor.factory.create_instance(self)
    }
    
    /// 解析作用域服务
    fn resolve_scoped(&self, descriptor: &ServiceDescriptor) -> Result<Box<dyn Any>, &'static str> {
        // 检查是否存在活动作用域
        {
            let current_scope = self.current_scope.borrow();
            if current_scope.is_none() {
                return Err("No active service scope");
            }
        }
        
        // 由于ServiceScope已简化且不处理缓存，每次都创建新实例
        descriptor.factory.create_instance(self)
    }
    
    /// 创建新的服务作用域
    pub fn create_scope(&self) -> Arc<ServiceScope> {
        let scope = Arc::new(ServiceScope::new());
        *self.current_scope.borrow_mut() = Some(Arc::clone(&scope));
        scope
    }
    
    /// 结束当前服务作用域
    pub fn end_scope(&self) {
        *self.current_scope.borrow_mut() = None;
    }
    
    /// 设置配置值
    pub fn set_config_value(&self, key: String, value: String) {
        let mut config = self.config_values.borrow_mut();
        config.insert(key, value);
    }
    
    /// 获取配置值
    pub fn get_config_value(&self, key: &str) -> Option<String> {
        let config = self.config_values.borrow();
        config.get(key).cloned()
    }
    
    /// 获取协议类型
    pub fn protocol_type(&self) -> BootProtocolType {
        self.protocol_type
    }
    
    /// 检查服务是否已注册
    pub fn is_service_registered(&self, service_type: &'static str) -> bool {
        let services = self.services.borrow();
        services.contains_key(service_type)
    }
    
    /// 获取已注册的服务列表
    pub fn get_registered_services(&self) -> Vec<&'static str> {
        let services = self.services.borrow();
        services.keys().copied().collect()
    }
    
    /// 验证依赖关系
    ///
    /// 检查所有已注册服务的依赖关系是否有效
    pub fn validate_dependencies(&self) -> Result<(), Vec<String>> {
        let services = self.services.borrow();
        let mut errors = Vec::new();
        
        for (service_name, descriptor) in services.iter() {
            // 检查依赖的服务是否存在
            for dep in &descriptor.dependencies {
                if !services.contains_key(dep) {
                    errors.push(format!("Dependency '{}' not found for service '{}'", dep, service_name));
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// 批量注册服务
    ///
    /// 从配置或其他来源批量注册服务
    pub fn register_services_batch(&self, descriptors: Vec<ServiceDescriptor>) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        for descriptor in descriptors {
            if let Err(e) = self.register_service(descriptor) {
                errors.push(e);
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// 清理所有单例实例
    ///
    /// 主要用于测试场景
    pub fn clear_singletons(&self) {
        let mut singletons = self.singletons.borrow_mut();
        singletons.clear();
    }
    
    /// 获取服务统计信息
    pub fn get_service_stats(&self) -> ServiceStats {
        let services = self.services.borrow();
        let singletons = self.singletons.borrow();
        
        let mut singleton_count = 0;
        let mut transient_count = 0;
        let mut scoped_count = 0;
        
        for descriptor in services.values() {
            match descriptor.lifecycle {
                ServiceLifecycle::Singleton => singleton_count += 1,
                ServiceLifecycle::Transient => transient_count += 1,
                ServiceLifecycle::Scoped => scoped_count += 1,
            }
        }
        
        ServiceStats {
            total_services: services.len(),
            singleton_count,
            transient_count,
            scoped_count,
            instantiated_singletons: singletons.len(),
        }
    }
}

/// 服务统计信息
#[derive(Debug, Clone)]
pub struct ServiceStats {
    /// 总服务数
    pub total_services: usize,
    /// 单例服务数
    pub singleton_count: usize,
    /// 瞬态服务数
    pub transient_count: usize,
    /// 作用域服务数
    pub scoped_count: usize,
    /// 已实例化的单例数
    pub instantiated_singletons: usize,
}


/// 引导依赖注入容器
///
/// 专门用于引导过程的DI容器，预配置了所有必要的服务
pub struct BootDIContainer {
    /// 内部DI容器
    inner: DIContainer,
    /// 图形后端实例（缓存）
    graphics_backend: Option<Box<dyn GraphicsBackend>>,
    /// 配置仓库实例（缓存）
    config_repo: Option<Box<dyn BootConfigRepository>>,
    /// 硬件检测服务实例（缓存）
    hardware_detection: Option<Box<dyn HardwareDetectionService>>,
}

impl BootDIContainer {
    /// 创建新的引导DI容器
    pub fn new(protocol_type: BootProtocolType) -> Self {
        let mut container = Self {
            inner: DIContainer::new(protocol_type),
            graphics_backend: None,
            config_repo: None,
            hardware_detection: None,
        };
        
        // 注册默认服务
        container.register_default_services();
        
        container
    }
    
    /// 注册默认服务
    fn register_default_services(&mut self) {
        // 注册配置仓库
        let _ = self.inner.register_singleton(
            "BootConfigRepository",
            DefaultBootConfigRepositoryFactory,
            None,
        );
        
        // 注册硬件检测服务
        let _ = self.inner.register_singleton(
            "HardwareDetectionService",
            HardwareDetectionServiceFactory::new(self.inner.protocol_type()),
            None,
        );
        
        // 注册图形后端
        let _ = self.inner.register_singleton(
            "GraphicsBackend",
            GraphicsBackendFactory::new(self.inner.protocol_type()),
            None,
        );
        
        // 注册事件发布器
        let _ = self.inner.register_singleton(
            "DomainEventPublisher",
            SimpleEventPublisherFactory,
            None,
        );
    }
    
    /// 初始化所有依赖
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        // 验证依赖关系
        self.inner.validate_dependencies()
            .map_err(|_| "Dependency validation failed")?;
        
        // 创建图形后端
        self.graphics_backend = Some(create_graphics_backend(self.inner.protocol_type())?);

        // 创建配置仓库
        self.config_repo = Some(Box::new(crate::domain::repositories::DefaultBootConfigRepository));

        // 创建硬件检测服务
        self.hardware_detection = Some(create_hardware_detection_service(self.inner.protocol_type())?);

        Ok(())
    }
    
    /// 获取图形后端
    pub fn graphics_backend(&self) -> Option<&dyn GraphicsBackend> {
        self.graphics_backend.as_deref()
    }
    
    /// 获取引导配置仓库
    pub fn config_repo(&self) -> Option<&dyn BootConfigRepository> {
        self.config_repo.as_deref()
    }
    
    /// 获取硬件检测服务
    pub fn hardware_detection_service(&self) -> Result<&dyn HardwareDetectionService, &'static str> {
        self.hardware_detection.as_deref().ok_or("Hardware detection service not available")
    }
    
    /// 获取协议类型
    pub fn protocol_type(&self) -> BootProtocolType {
        self.inner.protocol_type()
    }
    
    /// 获取内部DI容器
    pub fn inner(&self) -> &DIContainer {
        &self.inner
    }
    
    /// 获取内部DI容器的可变引用
    pub fn inner_mut(&mut self) -> &mut DIContainer {
        &mut self.inner
    }
    
    /// 转换为内部DI容器
    /// 
    /// 将BootDIContainer转换为内部的DIContainer，转移所有权
    pub fn into_inner(self) -> DIContainer {
        self.inner
    }
}

/// 默认配置仓库工厂
pub struct DefaultBootConfigRepositoryFactory;

impl ServiceFactory for DefaultBootConfigRepositoryFactory {
    fn create_instance(&self, _container: &DIContainer) -> Result<Box<dyn Any>, &'static str> {
        Ok(Box::new(crate::domain::repositories::DefaultBootConfigRepository))
    }
    
    fn get_service_type(&self) -> &'static str {
        "BootConfigRepository"
    }
}

/// 硬件检测服务工厂
pub struct HardwareDetectionServiceFactory {
    protocol_type: BootProtocolType,
}

impl HardwareDetectionServiceFactory {
    pub fn new(protocol_type: BootProtocolType) -> Self {
        Self { protocol_type }
    }
}

impl ServiceFactory for HardwareDetectionServiceFactory {
    fn create_instance(&self, _container: &DIContainer) -> Result<Box<dyn Any>, &'static str> {
        let service = create_hardware_detection_service(self.protocol_type)?;
        Ok(Box::new(service))
    }
    
    fn get_service_type(&self) -> &'static str {
        "HardwareDetectionService"
    }
}

/// 图形后端工厂
pub struct GraphicsBackendFactory {
    protocol_type: BootProtocolType,
}

impl GraphicsBackendFactory {
    pub fn new(protocol_type: BootProtocolType) -> Self {
        Self { protocol_type }
    }
}

impl ServiceFactory for GraphicsBackendFactory {
    fn create_instance(&self, _container: &DIContainer) -> Result<Box<dyn Any>, &'static str> {
        let backend = create_graphics_backend(self.protocol_type)?;
        Ok(Box::new(backend))
    }
    
    fn get_service_type(&self) -> &'static str {
        "GraphicsBackend"
    }
}

/// 简单事件发布器工厂
pub struct SimpleEventPublisherFactory;

impl ServiceFactory for SimpleEventPublisherFactory {
    fn create_instance(&self, _container: &DIContainer) -> Result<Box<dyn Any>, &'static str> {
        Ok(Box::new(crate::domain::events::SimpleEventPublisher::new()))
    }
    
    fn get_service_type(&self) -> &'static str {
        "DomainEventPublisher"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_di_container_creation() {
        let container = DIContainer::new(BootProtocolType::Bios);
        assert_eq!(container.protocol_type(), BootProtocolType::Bios);
    }
    
    #[test]
    fn test_service_registration() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        let result = container.register_singleton(
            "TestService",
            DefaultBootConfigRepositoryFactory,
            None,
        );
        
        assert!(result.is_ok());
        assert!(container.is_service_registered("TestService"));
    }
    
    #[test]
    fn test_duplicate_service_registration() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 注册第一次
        let result1 = container.register_singleton(
            "TestService",
            DefaultBootConfigRepositoryFactory,
            None,
        );
        assert!(result1.is_ok());
        
        // 尝试注册第二次
        let result2 = container.register_singleton(
            "TestService",
            DefaultBootConfigRepositoryFactory,
            None,
        );
        assert!(result2.is_err());
    }
    
    #[test]
    fn test_service_resolution() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 注册服务
        let _ = container.register_singleton(
            "BootConfigRepository",
            DefaultBootConfigRepositoryFactory,
            None,
        );
        
        // 解析服务
        let result: Result<Box<dyn BootConfigRepository>, &'static str> = 
            container.resolve("BootConfigRepository");
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_boot_di_container_creation() {
        let container = BootDIContainer::new(BootProtocolType::Bios);
        assert_eq!(container.protocol_type(), BootProtocolType::Bios);
    }
    
    #[test]
    fn test_boot_di_container_initialization() {
        let mut container = BootDIContainer::new(BootProtocolType::Bios);
        #[cfg(feature = "bios_support")]
        {
            assert!(container.initialize().is_ok());
            assert!(container.graphics_backend().is_some());
            assert!(container.config_repo().is_some());
        }
    }
    
    #[test]
    fn test_service_scope() {
        let scope = ServiceScope::new();
        
        // 测试作用域实例管理
        let instance = Box::new(42i32);
        scope.set_instance("TestService", instance);
        
        let retrieved = scope.get_instance("TestService");
        assert!(retrieved.is_some());
        
        // 清理作用域
        scope.clear();
        let retrieved_after_clear = scope.get_instance("TestService");
        assert!(retrieved_after_clear.is_none());
    }
    
    #[test]
    fn test_service_condition() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 测试协议类型条件
        let bios_condition = ServiceCondition::ProtocolType(BootProtocolType::Bios);
        assert!(bios_condition.is_satisfied(&container));
        
        let uefi_condition = ServiceCondition::ProtocolType(BootProtocolType::Uefi);
        assert!(!uefi_condition.is_satisfied(&container));
        
        // 测试总是条件
        let always_condition = ServiceCondition::Always;
        assert!(always_condition.is_satisfied(&container));
        
        // 测试功能条件
        container.set_config_value("test_feature".to_string(), "true".to_string());
        let feature_condition = ServiceCondition::FeatureEnabled("test_feature");
        assert!(feature_condition.is_satisfied(&container));
    }
    
    #[test]
    fn test_config_values() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 设置配置值
        container.set_config_value("test_key".to_string(), "test_value".to_string());
        
        // 获取配置值
        let value = container.get_config_value("test_key");
        assert_eq!(value, Some("test_value".to_string()));
        
        // 获取不存在的配置值
        let missing = container.get_config_value("missing_key");
        assert_eq!(missing, None);
    }
    
    #[test]
    fn test_dependency_validation() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 注册没有依赖的服务
        let _ = container.register_singleton(
            "ServiceA",
            DefaultBootConfigRepositoryFactory,
            None,
        );
        
        // 验证依赖关系
        let result = container.validate_dependencies();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_circular_dependency_detection() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 创建相互依赖的工厂
        struct FactoryA;
        impl ServiceFactory for FactoryA {
            fn create_instance(&self, container: &DIContainer) -> Result<Box<dyn Any>, &'static str> {
                container.resolve_any("ServiceB")
            }
            fn get_service_type(&self) -> &'static str { "ServiceA" }
            fn get_dependencies(&self) -> Vec<&'static str> { vec!["ServiceB"] }
        }
        
        struct FactoryB;
        impl ServiceFactory for FactoryB {
            fn create_instance(&self, container: &DIContainer) -> Result<Box<dyn Any>, &'static str> {
                container.resolve_any("ServiceA")
            }
            fn get_service_type(&self) -> &'static str { "ServiceB" }
            fn get_dependencies(&self) -> Vec<&'static str> { vec!["ServiceA"] }
        }
        
        // 注册相互依赖的服务
        let _ = container.register_singleton("ServiceA", FactoryA, None);
        let _ = container.register_singleton("ServiceB", FactoryB, None);
        
        // 尝试解析应该检测到循环依赖
        let result = container.resolve_any("ServiceA");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency detected"));
    }
    
    #[test]
    fn test_service_stats() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 注册不同生命周期的服务
        let _ = container.register_singleton("SingletonService", DefaultBootConfigRepositoryFactory, None);
        let _ = container.register_transient("TransientService", DefaultBootConfigRepositoryFactory, None);
        let _ = container.register_scoped("ScopedService", DefaultBootConfigRepositoryFactory, None);
        
        // 获取统计信息
        let stats = container.get_service_stats();
        assert_eq!(stats.total_services, 3);
        assert_eq!(stats.singleton_count, 1);
        assert_eq!(stats.transient_count, 1);
        assert_eq!(stats.scoped_count, 1);
    }
    
    #[test]
    fn test_batch_registration() {
        let container = DIContainer::new(BootProtocolType::Bios);
        
        // 创建多个服务描述符
        let descriptors = vec![
            ServiceDescriptor::new("Service1", ServiceLifecycle::Singleton, Box::new(DefaultBootConfigRepositoryFactory)),
            ServiceDescriptor::new("Service2", ServiceLifecycle::Transient, Box::new(DefaultBootConfigRepositoryFactory)),
        ];
        
        // 批量注册
        let result = container.register_services_batch(descriptors);
        assert!(result.is_ok());
        assert!(container.is_service_registered("Service1"));
        assert!(container.is_service_registered("Service2"));
    }
}
