//! 服务注册表模块
//!
//! 本模块实现了服务的注册、发现和管理功能，包括：
//! - ServiceRegistry: 核心服务注册表
//! - 服务依赖关系管理
//! - 服务生命周期状态跟踪
//! - 服务查询和过滤功能
//!
//! 服务注册表是整个服务管理系统的核心，负责维护所有已注册服务的信息。

use alloc::{string::ToString, 
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use spin::Mutex;

use crate::{error_handling::unified::KernelError, syscalls::services::traits::*};

/// 服务注册表
///
/// 核心服务管理组件，负责：
/// - 服务的注册和注销
/// - 服务依赖关系管理
/// - 服务状态跟踪
/// - 服务查询和发现
///
/// 使用线程安全的内部状态，支持并发访问。
pub struct ServiceRegistry {
    /// 已注册的服务映射
    ///
    /// 键：服务名称，值：服务实例和元数据
    services: Arc<Mutex<BTreeMap<String, ServiceEntry>>>,

    /// 系统调用服务映射
    ///
    /// 键：系统调用号，值：服务名称
    /// 用于快速查找处理特定系统调用的服务
    syscall_mapping: Arc<Mutex<BTreeMap<u32, String>>>,

    /// 服务依赖图
    ///
    /// 用于管理服务间的依赖关系和启动顺序
    dependency_graph: Arc<Mutex<DependencyGraph>>,
}

/// 服务注册条目
///
/// 包含服务实例及其相关元数据。
#[derive(Debug)]
pub struct ServiceEntry {
    /// 服务实例
    pub service: Box<dyn Service>,
    /// 服务元数据
    pub metadata: ServiceMetadata,
    /// 注册时间戳
    pub registration_time: u64,
}

/// 服务元数据
///
/// 包含服务的描述性信息。
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    /// 服务类型
    pub service_type: ServiceType,
    /// 服务优先级
    pub priority: u32,
    /// 是否为系统调用服务
    pub is_syscall_service: bool,
    /// 自定义标签
    pub tags: Vec<String>,
}

/// 服务类型枚举
///
/// 定义不同类型的服务。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    /// 核心系统服务
    Core,
    /// 系统调用服务
    Syscall,
    /// 网络服务
    Network,
    /// 文件系统服务
    FileSystem,
    /// 进程管理服务
    Process,
    /// 内存管理服务
    Memory,
    /// 设备驱动服务
    Device,
    /// 用户自定义服务
    Custom,
}

/// 依赖图
///
/// 管理服务间的依赖关系。
#[derive(Debug)]
pub struct DependencyGraph {
    /// 依赖关系映射
    ///
    /// 键：服务名称，值：依赖的服务名称列表
    dependencies: BTreeMap<String, Vec<String>>,
    /// 反向依赖映射
    ///
    /// 键：服务名称，值：依赖此服务的服务名称列表
    dependents: BTreeMap<String, Vec<String>>,
}

impl ServiceRegistry {
    /// 创建新的服务注册表
    ///
    /// 初始化一个空的服务注册表。
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(BTreeMap::new())),
            syscall_mapping: Arc::new(Mutex::new(BTreeMap::new())),
            dependency_graph: Arc::new(Mutex::new(DependencyGraph::new())),
        }
    }

    /// 注册服务
    ///
    /// 将服务注册到注册表中。
    ///
    /// # 参数
    ///
    /// * `service` - 要注册的服务实例
    /// * `metadata` - 服务元数据
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 注册成功
    /// * `Err(KernelError)` - 注册失败，包含错误信息
    pub fn register_service(
        &self,
        service: Box<dyn Service>,
        metadata: ServiceMetadata,
    ) -> Result<(), KernelError> {
        // println removed for no_std compatibility

        // 检查服务是否已存在
        {
            // println removed for no_std compatibility
            if self.services.contains_key(&metadata.name) {
                return Err(KernelError::ServiceAlreadyExists(metadata.name.clone()));
            }
        }

        // 验证依赖关系
        self.validate_dependencies(&service)?;

        // 创建服务条目
        let entry = ServiceEntry {
            service,
            metadata: metadata.clone(),
            registration_time: self.get_current_timestamp(),
        };

        // 注册服务
        {
            // println removed for no_std compatibility
            self.services.insert(metadata.name.clone(), entry);
        }

        // 更新依赖图
        {
            // println removed for no_std compatibility
            // println removed for no_std compatibility
        }

        // 如果是系统调用服务，更新系统调用映射
        if metadata.is_syscall_service {
            self.update_syscall_mapping(&metadata.name)?;
        }

        Ok(())
    }

    /// 注销服务
    ///
    /// 从注册表中移除服务。
    ///
    /// # 参数
    ///
    /// * `name` - 要注销的服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 注销成功
    /// * `Err(KernelError)` - 注销失败，包含错误信息
    pub fn unregister_service(&self, name: &str) -> Result<(), KernelError> {
        // 检查是否有其他服务依赖此服务
        {
            // println removed for no_std compatibility
            if let Some(dependents) = self.dependency_graph.lock().get_dependents(name) {
                if !dependents.is_empty() {
                    return Err(KernelError::ServiceHasDependents(name.to_string(), dependents));
                }
            }
        }

        // 移除服务
        {
            // println removed for no_std compatibility
            if let Some(entry) = self.services.remove(name) {
                // 停止服务
                // println removed for no_std compatibility
            } else {
                return Err(KernelError::ServiceNotFound(name.to_string()));
            }
        }

        // 更新依赖图
        {
            // println removed for no_std compatibility
            // println removed for no_std compatibility
        }

        // 清理系统调用映射
        {
            // println removed for no_std compatibility
            // println removed for no_std compatibility
        }

        Ok(())
    }

    /// 获取服务
    ///
    /// 根据名称获取已注册的服务。
    ///
    /// # 参数
    ///
    /// * `name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(Some(Box<dyn Service>))` - 找到服务
    /// * `Ok(None)` - 服务不存在
    /// * `Err(KernelError)` - 获取失败
    pub fn get_service(&self, name: &str) -> Result<Option<Box<dyn Service>>, KernelError> {
        // println removed for no_std compatibility
        if let Some(entry) = self.services.get(name) {
            // 这里需要克隆服务实例，但Service trait不是Clone
            // 实际实现中可能需要使用Arc或其他共享机制
            // 暂时返回None作为占位符
            Ok(None)
        } else {
            Ok(None)
        }
    }

    /// 获取系统调用服务
    ///
    /// 根据系统调用号获取对应的服务。
    ///
    /// # 参数
    ///
    /// * `syscall_number` - 系统调用号
    ///
    /// # 返回值
    ///
    /// * `Ok(Some(String))` - 找到服务名称
    /// * `Ok(None)` - 未找到对应服务
    /// * `Err(KernelError)` - 查询失败
    pub fn get_syscall_service(&self, syscall_number: u32) -> Result<Option<String>, KernelError> {
        // println removed for no_std compatibility
        Ok(self.syscall_mapping.lock().get(&syscall_number).cloned())
    }

    /// 列出所有服务
    ///
    /// 返回所有已注册服务的名称列表。
    ///
    /// # 返回值
    ///
    /// * `Vec<String>` - 服务名称列表
    pub fn list_services(&self) -> Vec<String> {
        // println removed for no_std compatibility
        self.services.keys().cloned().collect()
    }

    /// 按类型列出服务
    ///
    /// 返回指定类型的所有服务。
    ///
    /// # 参数
    ///
    /// * `service_type` - 服务类型
    ///
    /// # 返回值
    ///
    /// * `Vec<String>` - 服务名称列表
    pub fn list_services_by_type(&self, service_type: ServiceType) -> Vec<String> {
        // println removed for no_std compatibility
        self.services
            .lock()
            .iter()
            .filter(|(_, entry)| entry.metadata.service_type == service_type)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// 获取服务状态
    ///
    /// 返回指定服务的当前状态。
    ///
    /// # 参数
    ///
    /// * `name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(ServiceStatus)` - 服务状态
    /// * `Err(KernelError)` - 获取失败
    pub fn get_service_status(&self, name: &str) -> Result<ServiceStatus, KernelError> {
        // println removed for no_std compatibility
        if let Some(entry) = self.services.get(name) {
            Ok(entry.service.status())
        } else {
            Err(KernelError::ServiceNotFound(name.to_string()))
        }
    }

    /// 获取服务依赖关系
    ///
    /// 返回指定服务的依赖列表。
    ///
    /// # 参数
    ///
    /// * `name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(Vec<String>)` - 依赖服务列表
    /// * `Err(KernelError)` - 获取失败
    pub fn get_service_dependencies(&self, name: &str) -> Result<Vec<String>, KernelError> {
        // println removed for no_std compatibility
        Ok(self
            .dependency_graph
            .lock()
            .get_dependencies(name)
            .unwrap_or_default())
    }

    /// 计算服务启动顺序
    ///
    /// 根据依赖关系计算服务的启动顺序。
    ///
    /// # 返回值
    ///
    /// * `Ok(Vec<String>)` - 按启动顺序排列的服务名称列表
    /// * `Err(KernelError)` - 计算失败（如存在循环依赖）
    pub fn calculate_startup_order(&self) -> Result<Vec<String>, KernelError> {
        // println removed for no_std compatibility
        self.dependency_graph.lock().topological_sort()
    }

    /// 验证依赖关系
    ///
    /// 验证服务的依赖关系是否有效。
    ///
    /// # 参数
    ///
    /// * `service` - 要验证的服务
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 验证通过
    /// * `Err(KernelError)` - 验证失败
    fn validate_dependencies(&self, service: &Box<dyn Service>) -> Result<(), KernelError> {
        // println removed for no_std compatibility
        // println removed for no_std compatibility

        let dependencies = service.dependencies();
        for dep in dependencies {
            if !self.services.contains_key(dep) {
                return Err(KernelError::DependencyNotFound(
                    service.name().to_string(),
                    dep.to_string(),
                ));
            }
        }

        Ok(())
    }

    /// 更新系统调用映射
    ///
    /// 为系统调用服务更新系统调用号映射。
    ///
    /// # 参数
    ///
    /// * `service_name` - 服务名称
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 更新成功
    /// * `Err(KernelError)` - 更新失败
    fn update_syscall_mapping(&self, service_name: &str) -> Result<(), KernelError> {
        // println removed for no_std compatibility
        if let Some(entry) = self.services.lock().get(service_name) {
            // 尝试将服务转换为SyscallService
            if let Some(syscall_service) =
                entry.service.as_any().downcast_ref::<dyn SyscallService>()
            {
                // println removed for no_std compatibility
                for syscall_num in syscall_service.supported_syscalls() {
                    self.syscall_mapping
                        .lock()
                        .insert(syscall_num, service_name.to_string());
                }
            }
        }
        Ok(())
    }

    /// 获取当前时间戳
    ///
    /// 返回当前的时间戳（简化实现）。
    fn get_current_timestamp(&self) -> u64 {
        // 这里应该实现真实的时间戳获取
        // 暂时返回固定值
        0
    }
}

impl DependencyGraph {
    /// 创建新的依赖图
    pub fn new() -> Self {
        Self { dependencies: BTreeMap::new(), dependents: BTreeMap::new() }
    }

    /// 添加服务
    ///
    /// 将服务添加到依赖图中。
    pub fn add_service(&mut self, name: &str, metadata: &ServiceMetadata) {
        // 初始化依赖关系（这里需要从服务实例获取）
        self.dependencies.insert(name.to_string(), Vec::new());
        self.dependents.insert(name.to_string(), Vec::new());
    }

    /// 移除服务
    ///
    /// 从依赖图中移除服务。
    pub fn remove_service(&mut self, name: &str) {
        // println removed for no_std compatibility
        // println removed for no_std compatibility

        // 从其他服务的依赖列表中移除
        for deps in self.dependencies.values_mut() {
            // println removed for no_std compatibility
        }

        // 从其他服务的依赖者列表中移除
        for dependents in self.dependents.values_mut() {
            // println removed for no_std compatibility
        }
    }

    /// 获取服务的依赖
    pub fn get_dependencies(&self, name: &str) -> Option<&Vec<String>> {
        self.dependencies.get(name)
    }

    /// 获取服务的依赖者
    pub fn get_dependents(&self, name: &str) -> Option<&Vec<String>> {
        self.dependents.get(name)
    }

    /// 拓扑排序
    ///
    /// 计算服务的启动顺序。
    pub fn topological_sort(&self) -> Result<Vec<String>, KernelError> {
        // println removed for no_std compatibility
        // println removed for no_std compatibility
        let mut visited = BTreeMap::new();
        let mut result = Vec::new();
        let temp_mark = 1;
        let perm_mark = 2;

        for service in self.dependencies.keys() {
            if !visited.contains_key(service) {
                self.visit(service, &mut visited, &mut result, temp_mark, perm_mark)?;
            }
        }

        // println removed for no_std compatibility
        Ok(result)
    }

    /// 深度优先访问
    fn visit(
        &self,
        service: &str,
        visited: &mut BTreeMap<String, u32>,
        result: &mut Vec<String>,
        temp_mark: u32,
        perm_mark: u32,
    ) -> Result<(), KernelError> {
        if let Some(mark) = visited.get(service) {
            if *mark == temp_mark {
                return Err(KernelError::CircularDependency(service.to_string()));
            }
            if *mark == perm_mark {
                return Ok(());
            }
        }

        visited.insert(service.to_string(), temp_mark);

        if let Some(deps) = self.dependencies.get(service) {
            for dep in deps {
                self.visit(dep, visited, result, temp_mark, perm_mark)?;
            }
        }

        visited.insert(service.to_string(), perm_mark);
        result.push(service.to_string());

        Ok(())
    }
}

// 为Service trait添加as_any方法，用于类型转换
pub trait ServiceAsAny {
    fn as_any(&self) -> &dyn core::any::Any;
}

impl<T: Service + core::any::Any> ServiceAsAny for T {
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

/// 服务注册错误类型
///
/// 定义服务注册过程中可能出现的错误。
#[derive(Debug, Clone)]
pub enum ServiceRegistryError {
    /// 服务已存在
    ServiceAlreadyExists(String),
    /// 服务未找到
    ServiceNotFound(String),
    /// 依赖服务未找到
    DependencyNotFound(String, String),
    /// 循环依赖
    CircularDependency(String),
    /// 服务有依赖者
    ServiceHasDependents(String, Vec<String>),
}

impl core::fmt::Display for ServiceRegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ServiceRegistryError::ServiceAlreadyExists(name) => {
                write!(f, "Service '{}' already exists", name)
            },
            ServiceRegistryError::ServiceNotFound(name) => {
                write!(f, "Service '{}' not found", name)
            },
            ServiceRegistryError::DependencyNotFound(service, dep) => {
                write!(f, "Dependency '{}' for service '{}' not found", dep, service)
            },
            ServiceRegistryError::CircularDependency(service) => {
                write!(f, "Circular dependency detected involving service '{}'", service)
            },
            ServiceRegistryError::ServiceHasDependents(service, dependents) => {
                write!(f, "Service '{}' has dependents: {:?}", service, dependents)
            },
        }
    }
}
